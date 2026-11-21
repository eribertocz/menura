//! One common table expression per plan node, in plan order (E6-4.4).

use menura_contracts::types::catalog::RelationSchema;
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, NormalizedName};
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, SortItem,
};

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::{Columns, Environment, ExpressionWriter};
use crate::identifiers::{Source, classify_source, quote, source_sql};
use crate::parameters::Parameters;
use crate::source_columns::{
    empty_scan_item, source_column_item, source_column_name, source_prefix,
};
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
}

impl<'input> StepWriter<'input> {
    /// A writer with no step yet.
    pub(crate) fn new(environment: &'input Environment<'input>) -> Self {
        Self {
            environment,
            parameters: Parameters::default(),
            steps: Vec::new(),
        }
    }

    /// Writes the steps of `node`'s inputs, then the step of `node`.
    pub(crate) fn write(&mut self, node: &'input PlanNode) -> Result<WrittenNode, GenerateError> {
        let (body, ordering, kind) = match node.operation() {
            Operation::Scan { dataset } => (self.scan(node, dataset)?, None, "scan"),
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
            Operation::Deduplicate {
                input,
                group,
                identity,
            } => (
                self.deduplicate(node, input, group, identity)?,
                None,
                "deduplicate",
            ),
            Operation::SemiJoin { .. } => (self.semi_join(node)?, None, "semijoin"),
            Operation::Join { .. } | Operation::Stitch { .. } => {
                return Err(error(InvariantViolation::NodeOutsideSlice, node));
            }
        };
        let name = quote(&format!("menura_{}_{kind}", self.steps.len() + 1));
        self.steps.push((name.clone(), body));
        Ok(WrittenNode { name, ordering })
    }

    /// Each schema column is the expression of the field of the same name, over the columns of
    /// the catalog relation whose `source` is the dataset's, character for character (CAT-4),
    /// a query `source` included (E6-4.3); a schema column that names a catalog column is that
    /// column, in the catalog's spelling (PLN-2.2). A `Scan` with no column selects one
    /// constant, so that the rows can still be counted.
    fn scan(
        &mut self,
        node: &'input PlanNode,
        dataset: &NormalizedName,
    ) -> Result<SelectBody, GenerateError> {
        let model = self.environment.model;
        let dataset = model
            .dataset(dataset)
            .ok_or_else(|| error(InvariantViolation::UnknownDataset, node))?;
        let source: Source<'_> = classify_source(&dataset.source)
            .ok_or_else(|| error(InvariantViolation::MalformedSource, node))?;
        let relation = self
            .environment
            .catalog
            .relations
            .iter()
            .find(|relation| relation.source == dataset.source)
            .ok_or_else(|| error(InvariantViolation::UnknownRelation, node))?;
        self.parameters.start_node(node.origin());
        let prefix = source_prefix(dataset);
        let mut items = Vec::new();
        for column in node.schema().columns() {
            let name = column.name.as_str();
            let item = match source_column_name(&prefix, name) {
                Some(catalog_name) => source_column_item(relation, catalog_name, name)
                    .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?,
                None => self.field_item(node, dataset, relation, name)?,
            };
            items.push(item);
        }
        if items.is_empty() {
            items.push(empty_scan_item(&prefix));
        }
        Ok(SelectBody {
            items,
            from: source_sql(&source),
            clauses: Vec::new(),
        })
    }

    /// The select item of the `Scan` column `name` of `dataset`: the expression of the field of
    /// that name over the columns of `relation`.
    fn field_item(
        &mut self,
        node: &'input PlanNode,
        dataset: &Dataset,
        relation: &'input RelationSchema,
        name: &str,
    ) -> Result<String, GenerateError> {
        let field = dataset
            .fields
            .iter()
            .find(|field| field.name.as_str() == name)
            .ok_or_else(|| error(InvariantViolation::UnknownField, node))?;
        let expression = field
            .expression
            .as_ref()
            .ok_or_else(|| error(InvariantViolation::FieldWithoutExpression, node))?;
        let value = self.expression(node, Columns::Catalog(relation), expression)?;
        Ok(format!("{value} AS {}", quote(name)))
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

    /// Each output column; a computed one is cast to its type (E6-4.5).
    fn project(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        expressions: &[NamedExpression],
    ) -> Result<SelectBody, GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let mut items = Vec::new();
        for named in expressions {
            let value = self.expression(node, Columns::Input(input.schema()), &named.expression)?;
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

    fn expression(
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
