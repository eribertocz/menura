//! One common table expression per plan node, in plan order (E6-4.4).

use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::Expression;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, SortItem,
};
use menura_representations::names::ColumnPrefix;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::{Columns, Environment, ExpressionWriter};
use crate::identifiers::quote;
use crate::join::JoinInputs;
use crate::parameters::Parameters;
use crate::scan::ScanRole;
use crate::types::physical_type;

/// A node already written: the name of its step and the `ORDER BY` its rows keep.
#[derive(Debug, Clone)]
pub(crate) struct WrittenNode {
    /// The quoted name of its common table expression.
    pub(crate) name: String,
    /// The `ORDER BY` clause of the nearest `Sort` below a `Limit`, restated wherever the
    /// order must hold, since SQL keeps no order between steps.
    pub(crate) ordering: Option<String>,
}

/// Writes the steps of one statement.
pub(crate) struct StepWriter<'input> {
    pub(crate) environment: &'input Environment<'input>,
    pub(crate) parameters: Parameters,
    pub(crate) steps: Vec<(String, SelectBody)>,
    /// The `right_prefix` of the `Join` whose right input is being written, for its `Scan`.
    pub(crate) column_prefix: Option<&'input ColumnPrefix>,
}

impl<'input> StepWriter<'input> {
    /// A writer with no step yet.
    pub(crate) fn new(environment: &'input Environment<'input>) -> Self {
        Self {
            environment,
            parameters: Parameters::default(),
            steps: Vec::new(),
            column_prefix: None,
        }
    }

    /// Writes the steps of `node`'s inputs, then the step of `node`.
    pub(crate) fn write(&mut self, node: &'input PlanNode) -> Result<WrittenNode, GenerateError> {
        let (body, ordering, kind) = match node.operation() {
            Operation::Filter { input, predicate } => {
                (self.filter(node, input, predicate)?, None, "filter")
            }
            Operation::Aggregate {
                input,
                keys,
                components,
            } => (
                self.aggregate(node, input, keys, components)?,
                None,
                "aggregate",
            ),
            Operation::Project { input, expressions } => {
                (self.project(node, input, expressions)?, None, "project")
            }
            Operation::Sort { input, items } => {
                let (body, ordering) = self.sort(node, input, items)?;
                (body, Some(ordering), "sort")
            }
            Operation::Limit { input, rows } => {
                let (body, ordering) = self.limit(node, input, *rows)?;
                (body, ordering, "limit")
            }
            _ => {
                let (body, kind) = self.relation_step(node)?;
                (body, None, kind)
            }
        };
        let name = quote(&format!("menura_{}_{kind}", self.steps.len() + 1));
        self.steps.push((name.clone(), body));
        Ok(WrittenNode { name, ordering })
    }

    /// The body and kind of a node that reads, joins or deduplicates rows: `Scan`, `Join` and
    /// `Deduplicate`. `SemiJoin` and `Stitch` are not generated yet.
    fn relation_step(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<(SelectBody, &'static str), GenerateError> {
        match node.operation() {
            Operation::Scan {
                dataset,
                row_identity,
            } => {
                let role = ScanRole {
                    column_prefix: self.column_prefix.take(),
                    row_identity: row_identity.as_ref(),
                };
                Ok((self.scan(node, dataset, role)?, "scan"))
            }
            Operation::Join {
                left,
                right,
                kind,
                on,
                right_prefix,
                ..
            } => {
                let join = JoinInputs {
                    left,
                    right,
                    kind: *kind,
                    on,
                    right_prefix,
                };
                Ok((self.join(node, &join)?, "join"))
            }
            Operation::Deduplicate {
                input,
                group,
                identity,
            } => Ok((
                self.deduplicate(node, input, group, identity)?,
                "deduplicate",
            )),
            Operation::SemiJoin { .. } => Ok((self.semi_join(node)?, "semijoin")),
            _ => Err(error(InvariantViolation::NodeOutsideSlice, node)),
        }
    }

    fn filter(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        predicate: &Expression,
    ) -> Result<SelectBody, GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let items = pass_through(node, input.schema())?;
        let condition = self.expression(node, Columns::Input(input.schema()), predicate)?;
        Ok(SelectBody {
            items,
            from: written.name,
            clauses: vec![format!("WHERE {condition}")],
        })
    }

    /// Keys, then components cast to their fixed result type (E6-4.5), grouped by the keys.
    fn aggregate(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        keys: &[NamedExpression],
        components: &[AggregateComponent],
    ) -> Result<SelectBody, GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let columns = Columns::Input(input.schema());
        let mut items = Vec::new();
        for key in keys {
            let value = self.expression(node, columns, &key.expression)?;
            items.push(format!("{value} AS {}", quote(key.name.as_str())));
        }
        for component in components {
            let call = self
                .writer(node, columns)
                .aggregate(&component.call, component.filter.as_ref())?;
            let result_type = column_type(node, component.name.as_str())?;
            items.push(format!(
                "CAST({call} AS {result_type}) AS {}",
                quote(component.name.as_str())
            ));
        }
        let clauses = if keys.is_empty() {
            Vec::new()
        } else {
            let ordinals: Vec<String> = (1..=keys.len())
                .map(|ordinal| ordinal.to_string())
                .collect();
            vec![format!("GROUP BY {}", ordinals.join(", "))]
        };
        Ok(SelectBody {
            items,
            from: written.name,
            clauses,
        })
    }

    /// Each output column; a computed one is cast to its type (E6-4.5), and the divisions of a
    /// derived formula follow its `on_division_by_zero` (E6-4.6a).
    fn project(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        expressions: &'input [NamedExpression],
    ) -> Result<SelectBody, GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let mut items = Vec::new();
        for named in expressions {
            let writer = self.writer(node, Columns::Input(input.schema()));
            let mut writer = match &named.derived {
                Some(formula) => {
                    writer.in_derived_metric(&formula.metric, formula.on_division_by_zero)
                }
                None => writer,
            };
            let value = writer.write(&named.expression)?;
            let value = match named.expression {
                Expression::Reference(_) => value,
                _ => format!(
                    "CAST({value} AS {})",
                    column_type(node, named.name.as_str())?
                ),
            };
            items.push(format!("{value} AS {}", quote(named.name.as_str())));
        }
        Ok(SelectBody {
            items,
            from: written.name,
            clauses: Vec::new(),
        })
    }

    fn sort(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        items: &[SortItem],
    ) -> Result<(SelectBody, String), GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let terms = items
            .iter()
            .map(|item| sort_term(node, input.schema(), item))
            .collect::<Result<Vec<_>, _>>()?;
        let ordering = format!("ORDER BY {}", terms.join(", "));
        let body = SelectBody {
            items: pass_through(node, input.schema())?,
            from: written.name,
            clauses: vec![ordering.clone()],
        };
        Ok((body, ordering))
    }

    /// `LIMIT n`, after the order of the `Sort` below it, restated (E6-4.11: `n` is a literal).
    fn limit(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        rows: u64,
    ) -> Result<(SelectBody, Option<String>), GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let mut clauses: Vec<String> = written.ordering.iter().cloned().collect();
        clauses.push(format!("LIMIT {rows}"));
        let body = SelectBody {
            items: pass_through(node, input.schema())?,
            from: written.name,
            clauses,
        };
        Ok((body, written.ordering))
    }

    fn writer<'step>(
        &'step mut self,
        node: &'input PlanNode,
        columns: Columns<'input>,
    ) -> ExpressionWriter<'input, 'step> {
        ExpressionWriter::new(
            self.environment,
            columns,
            node.origin(),
            &mut self.parameters,
        )
    }

    pub(crate) fn expression(
        &mut self,
        node: &'input PlanNode,
        columns: Columns<'input>,
        expression: &Expression,
    ) -> Result<String, GenerateError> {
        self.writer(node, columns).write(expression)
    }
}

/// The columns of `node`, each read unchanged from `input`.
pub(crate) fn pass_through(
    node: &PlanNode,
    input: &OutputSchema,
) -> Result<Vec<String>, GenerateError> {
    node.schema()
        .columns()
        .iter()
        .map(|column| {
            let known = input
                .columns()
                .iter()
                .any(|candidate| candidate.name == column.name);
            if known {
                Ok(quote(column.name.as_str()))
            } else {
                Err(error(InvariantViolation::UnknownColumn, node))
            }
        })
        .collect()
}

/// `"column" ASC NULLS FIRST`: the NULL order is always written (E6-4.8).
fn sort_term(
    node: &PlanNode,
    input: &OutputSchema,
    item: &SortItem,
) -> Result<String, GenerateError> {
    if !input
        .columns()
        .iter()
        .any(|column| column.name == item.column)
    {
        return Err(error(InvariantViolation::UnknownColumn, node));
    }
    let direction = match item.direction {
        Direction::Asc => "ASC",
        Direction::Desc => "DESC",
    };
    let nulls = match item.nulls {
        NullsOrder::First => "NULLS FIRST",
        NullsOrder::Last => "NULLS LAST",
    };
    Ok(format!(
        "{} {direction} {nulls}",
        quote(item.column.as_str())
    ))
}

/// The `DuckDB` type of the column `name` of `node`.
fn column_type(node: &PlanNode, name: &str) -> Result<String, GenerateError> {
    let column = node
        .schema()
        .columns()
        .iter()
        .find(|column| column.name.as_str() == name)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
    physical_type(&column.logical_type)
        .ok_or_else(|| error(InvariantViolation::UnsupportedType, node))
}

/// The error `violation` in `node`.
pub(crate) fn error(violation: InvariantViolation, node: &PlanNode) -> GenerateError {
    GenerateError::new(violation, node.origin())
}
