//! The steps of a `SemiJoin` node (E4-3.4, E4-6.5a, E6-4.14): keep a row of the input only if
//! at least one related row exists along the path and every related row is visible.
//!
//! The steps come after those of the input (E6-4.12): one `Scan` per dataset of the path, in
//! path order, then one grouped step per path step, from the last dataset back to the first,
//! then the `SemiJoin` itself. A grouped step holds, per key, the number of related rows and
//! the number of visible ones: a row of the last dataset is visible when the predicate is
//! TRUE; a row of an earlier dataset is visible when its own related rows are all visible and
//! there is at least one. Keys are compared with `=`, so a NULL key never matches.

use menura_contracts::types::catalog::RelationSchema;
use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, NormalizedName, Relationship, ValueType};
use menura_representations::logical_plan::{OutputSchema, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;
use menura_representations::paths::JoinStep;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::{Columns, ExpressionWriter};
use crate::identifiers::{classify_source, quote, source_sql};
use crate::steps::{StepWriter, error};

/// The alias of the row whose related rows are counted.
const ROW: &str = "\"menura_row\"";
/// The alias of the grouped step of the related rows.
const RELATED: &str = "\"menura_related\"";
/// The number of related rows of a key.
const ROWS: &str = "\"menura.rows\"";
/// The number of visible related rows of a key.
const VISIBLE_ROWS: &str = "\"menura.visible_rows\"";

/// One step of the path, resolved against the model.
struct ResolvedStep<'input> {
    relationship: &'input Relationship,
    dataset: &'input Dataset,
    relation: &'input RelationSchema,
}

impl<'input> StepWriter<'input> {
    /// The steps of `input`, then those of `path`, then the body of the `SemiJoin` `node`.
    pub(crate) fn semi_join(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        path: &'input [JoinStep],
        predicate: &Expression,
    ) -> Result<SelectBody, GenerateError> {
        let resolved = self.resolve(node, path)?;
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let mut scans = Vec::new();
        for step in &resolved {
            let body = self.path_scan(node, step)?;
            scans.push(self.push_step("scan", body));
        }
        let (last, earlier) = resolved
            .split_last()
            .ok_or_else(|| error(InvariantViolation::EmptyPath, node))?;
        let condition = self.visible(node, last, predicate)?;
        let mut related = self.push_step(
            "related",
            related_rows(
                last,
                scan_name(node, &scans, earlier.len())?,
                &format!("COUNT(*) FILTER (WHERE {condition})"),
                None,
            ),
        );
        let mut next = last;
        for (index, step) in earlier.iter().enumerate().rev() {
            let join = format!(
                "LEFT JOIN {related} AS {RELATED} ON {}",
                key_condition(&next.relationship.from_columns)
            );
            let visible =
                format!("COUNT(*) FILTER (WHERE {RELATED}.{ROWS} = {RELATED}.{VISIBLE_ROWS})");
            let body = related_rows(step, scan_name(node, &scans, index)?, &visible, Some(join));
            related = self.push_step("related", body);
            next = step;
        }
        let first = resolved
            .first()
            .ok_or_else(|| error(InvariantViolation::EmptyPath, node))?;
        kept_rows(node, input, &written.name, first, &related)
    }

    /// The relationship, dataset and catalog relation of every step of `path`.
    fn resolve(
        &self,
        node: &PlanNode,
        path: &[JoinStep],
    ) -> Result<Vec<ResolvedStep<'input>>, GenerateError> {
        if path.is_empty() {
            return Err(error(InvariantViolation::EmptyPath, node));
        }
        let model = self.environment.model;
        path.iter()
            .map(|step| {
                let relationship = model
                    .parts()
                    .relationships
                    .iter()
                    .find(|relationship| {
                        relationship.name == step.relationship
                            && relationship.from == step.from
                            && relationship.to == step.to
                    })
                    .ok_or_else(|| error(InvariantViolation::UnknownRelationship, node))?;
                let dataset = model
                    .dataset(&step.to)
                    .ok_or_else(|| error(InvariantViolation::UnknownDataset, node))?;
                let relation = self
                    .environment
                    .catalog
                    .relations
                    .iter()
                    .find(|relation| relation.source == dataset.source)
                    .ok_or_else(|| error(InvariantViolation::UnknownRelation, node))?;
                Ok(ResolvedStep {
                    relationship,
                    dataset,
                    relation,
                })
            })
            .collect()
    }

    /// The `Scan` of the dataset a step reaches: every field of it that has an expression.
    fn path_scan(
        &mut self,
        node: &'input PlanNode,
        step: &ResolvedStep<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let source = classify_source(&step.dataset.source)
            .ok_or_else(|| error(InvariantViolation::MalformedSource, node))?;
        let items = scanned_fields(step.dataset)
            .map(|name| self.field_item(node, step.dataset, step.relation, name))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SelectBody {
            items,
            from: source_sql(&source),
            clauses: Vec::new(),
        })
    }

    /// `predicate`, over the scanned fields of the last dataset of the path.
    fn visible(
        &mut self,
        node: &'input PlanNode,
        step: &ResolvedStep<'input>,
        predicate: &Expression,
    ) -> Result<String, GenerateError> {
        let columns = scanned_fields(step.dataset)
            .map(|name| scanned_column(node, step.dataset, name))
            .collect::<Result<Vec<_>, _>>()?;
        let schema = OutputSchema::new(columns);
        ExpressionWriter::new(
            self.environment,
            Columns::Input(&schema),
            node.origin(),
            &mut self.parameters,
        )
        .write(predicate)
    }

    /// Adds the step `body` of `kind` and returns its quoted name.
    fn push_step(&mut self, kind: &str, body: SelectBody) -> String {
        let name = quote(&format!("menura_{}_{kind}", self.steps.len() + 1));
        self.steps.push((name.clone(), body));
        name
    }
}

/// The names of the fields a path `Scan` reads: those with an expression, in model order.
fn scanned_fields(dataset: &Dataset) -> impl Iterator<Item = &str> {
    dataset
        .fields
        .iter()
        .filter(|field| field.expression.is_some())
        .map(|field| field.name.as_str())
}

/// The plan column of the scanned field `name` of `dataset`.
fn scanned_column(
    node: &PlanNode,
    dataset: &Dataset,
    name: &str,
) -> Result<SchemaColumn, GenerateError> {
    let typed = dataset
        .fields
        .iter()
        .find(|field| field.name.as_str() == name)
        .and_then(|field| field.typed.as_ref())
        .ok_or_else(|| error(InvariantViolation::UntypedOperand, node))?;
    match &typed.value_type {
        ValueType::Logical(logical_type) => Ok(SchemaColumn {
            name: ColumnName::new(name),
            logical_type: logical_type.clone(),
            nullable: typed.nullable,
        }),
        ValueType::Null => Err(error(InvariantViolation::UnsupportedType, node)),
    }
}

/// The quoted name of the path `Scan` at `index`.
fn scan_name<'name>(
    node: &PlanNode,
    scans: &'name [String],
    index: usize,
) -> Result<&'name str, GenerateError> {
    scans
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| error(InvariantViolation::EmptyPath, node))
}

/// The grouped step of `step`, over its `scan`: per key of the relationship, the number of
/// related rows and the number `visible` of visible ones; `join`, when given, joins the
/// grouped step of the next step, and the columns of `scan` are then qualified.
fn related_rows(
    step: &ResolvedStep<'_>,
    scan: &str,
    visible: &str,
    join: Option<String>,
) -> SelectBody {
    let qualifier = if join.is_some() {
        format!("{ROW}.")
    } else {
        String::new()
    };
    let mut items: Vec<String> = step
        .relationship
        .to_columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            format!(
                "{qualifier}{} AS {}",
                quote(column.as_str()),
                key_name(index)
            )
        })
        .collect();
    let groups: Vec<String> = (1..=items.len())
        .map(|ordinal| ordinal.to_string())
        .collect();
    items.push(format!("COUNT(*) AS {ROWS}"));
    items.push(format!("{visible} AS {VISIBLE_ROWS}"));
    let from = if join.is_some() {
        format!("{scan} AS {ROW}")
    } else {
        scan.to_owned()
    };
    let mut clauses: Vec<String> = join.into_iter().collect();
    clauses.push(format!("GROUP BY {}", groups.join(", ")));
    SelectBody {
        items,
        from,
        clauses,
    }
}

/// The body of the `SemiJoin`: the rows of `input` whose key finds a group of `related` in
/// which every related row is visible.
fn kept_rows(
    node: &PlanNode,
    input: &PlanNode,
    input_name: &str,
    first: &ResolvedStep<'_>,
    related: &str,
) -> Result<SelectBody, GenerateError> {
    let items = node
        .schema()
        .columns()
        .iter()
        .map(|column| {
            let known = input
                .schema()
                .columns()
                .iter()
                .any(|candidate| candidate.name == column.name);
            if known {
                Ok(format!("{ROW}.{}", quote(column.name.as_str())))
            } else {
                Err(error(InvariantViolation::UnknownColumn, node))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let on = key_condition(&first.relationship.from_columns);
    Ok(SelectBody {
        items,
        from: format!("{input_name} AS {ROW}"),
        clauses: vec![
            format!("JOIN {related} AS {RELATED} ON {on}"),
            format!("WHERE {RELATED}.{ROWS} = {RELATED}.{VISIBLE_ROWS}"),
        ],
    })
}

/// `row.from_1 = related.key_1 AND …`, one equality per column pair, in the order written.
fn key_condition(from_columns: &[NormalizedName]) -> String {
    from_columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            format!(
                "{ROW}.{} = {RELATED}.{}",
                quote(column.as_str()),
                key_name(index)
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// The name of the key column at `index`, from 0, of a grouped step.
fn key_name(index: usize) -> String {
    quote(&format!("menura.key_{}", index + 1))
}
