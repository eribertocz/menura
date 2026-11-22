//! The base side of a summary (CHK-5.1, CHK-5.5): the base rows in the bound period range,
//! grouped by the summary's dimensions and aggregated by its measures. Its build query and its
//! check both start from this plan, so they never differ in how they group, truncate or
//! aggregate.

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::Grain;
use menura_model::expression::{DateUnit, Expression};
use menura_model::internal_model::{
    Dataset, Field, InternalModel, NormalizedName, Summary, SummaryDimension, SummaryMeasure,
    ValueType,
};
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::field_references::over_columns;
use super::key_columns::STAGE;
use super::plan_nodes::{project, reference};
use super::typed_columns::typed_column;
use crate::compile::InternalError;

/// One field of a summary, in the summary's field order: a dimension or a measure (SUM-6.1.2).
#[derive(Debug, Clone, Copy)]
pub(super) enum SummaryField<'model> {
    /// A dimension.
    Dimension(&'model SummaryDimension),
    /// A measure.
    Measure(&'model SummaryMeasure),
}

impl SummaryField<'_> {
    /// The summary field's normalized name.
    pub(super) fn name(&self) -> &NormalizedName {
        match self {
            Self::Dimension(dimension) => &dimension.field,
            Self::Measure(measure) => &measure.field,
        }
    }
}

/// A summary with the datasets it names.
#[derive(Debug, Clone, Copy)]
pub(super) struct SummaryParts<'model> {
    /// The summary.
    pub(super) summary: &'model Summary,
    /// Its base dataset (`of`).
    pub(super) base: &'model Dataset,
    /// The summary dataset.
    pub(super) dataset: &'model Dataset,
}

impl<'model> SummaryParts<'model> {
    /// The summary `summary` of `model` with its datasets.
    pub(super) fn of(
        summary: &'model Summary,
        model: &'model InternalModel,
    ) -> Result<Self, InternalError> {
        let dataset = |name: &NormalizedName| {
            model.dataset(name).ok_or_else(|| {
                defect(
                    summary,
                    &format!("names the unknown dataset {}", name.as_str()),
                )
            })
        };
        Ok(Self {
            summary,
            base: dataset(&summary.of)?,
            dataset: dataset(&summary.dataset)?,
        })
    }

    /// Where the summary is declared: the origin of every node of its plans.
    pub(super) fn origin(&self) -> Origin {
        Origin::Model(self.summary.location.clone())
    }

    /// The summary's fields in its field order, each a dimension or a measure.
    pub(super) fn fields(&self) -> Result<Vec<SummaryField<'model>>, InternalError> {
        let summary = self.summary;
        self.dataset
            .fields
            .iter()
            .map(|field| {
                if let Some(dimension) = summary.dimensions.iter().find(|d| d.field == field.name) {
                    return Ok(SummaryField::Dimension(dimension));
                }
                summary
                    .measures
                    .iter()
                    .find(|measure| measure.field == field.name)
                    .map(SummaryField::Measure)
                    .ok_or_else(|| {
                        defect(
                            summary,
                            &format!("has the field {} of no dimension", field.name.as_str()),
                        )
                    })
            })
            .collect()
    }

    /// The coverage dimension (`coverage_field`).
    pub(super) fn coverage(&self) -> Result<&'model SummaryDimension, InternalError> {
        self.summary
            .dimensions
            .iter()
            .find(|dimension| dimension.field == self.summary.coverage_field)
            .ok_or_else(|| defect(self.summary, "has no coverage dimension"))
    }
}

/// The base aggregate of `parts` (CHK-5.5): the `Scan` of the base, its rows in the bound
/// period range of the coverage dimension's `base` field (CHK-7.4), grouped by each dimension's
/// `base` truncated to its `grain` and aggregated by each measure's `base_aggregate`. Its
/// columns are the dimensions, then the measures, each in field order and named by its summary
/// field.
pub(super) fn base_aggregate(parts: &SummaryParts<'_>) -> Result<PlanNode, InternalError> {
    let origin = parts.origin();
    let fields = parts.fields()?;
    let mut keys = Vec::new();
    let mut components = Vec::new();
    let mut read: Vec<NormalizedName> = Vec::new();
    for field in &fields {
        match field {
            SummaryField::Dimension(dimension) => {
                keys.push((dimension.field.clone(), key_expression(dimension)?));
                add_once(&mut read, &dimension.base);
            }
            SummaryField::Measure(measure) => {
                let (call, reads) = measure_expression(parts.summary, measure)?;
                components.push((measure.field.clone(), call));
                for name in &reads {
                    add_once(&mut read, name);
                }
            }
        }
    }
    let scan = base_scan(parts, &read, &origin)?;
    let in_range = PlanNode::new(
        Operation::PeriodRange {
            coverage: ColumnName::new(parts.coverage()?.base.as_str()),
            input: Box::new(scan.clone()),
        },
        scan.schema().clone(),
        origin.clone(),
    );
    aggregate(in_range, &keys, &components, &origin)
}

/// The build query of `parts` (CHK-5.1, CHK-5.5): its base aggregate with each column renamed
/// to the catalog column of its summary field, in the summary's field order. Returns the plan
/// and the names of its columns.
pub(super) fn build_plan(
    parts: &SummaryParts<'_>,
    catalog: &CatalogSchemaDocument,
) -> Result<(PlanNode, Vec<String>), InternalError> {
    let aggregate = base_aggregate(parts)?;
    let columns = parts
        .fields()?
        .iter()
        .map(|field| {
            let column = catalog_column(parts, field.name(), catalog)?;
            Ok((ColumnName::new(field.name().as_str()), column))
        })
        .collect::<Result<Vec<_>, InternalError>>()?;
    let names = columns.iter().map(|(_, name)| name.clone()).collect();
    Ok((project(aggregate, &columns, &parts.origin())?, names))
}

/// The `Aggregate` of `input` by `keys` with the components `components`, each column typed
/// by the typer of E0.f.
fn aggregate(
    input: PlanNode,
    keys: &[(NormalizedName, Expression)],
    components: &[(NormalizedName, Expression)],
    origin: &Origin,
) -> Result<PlanNode, InternalError> {
    let mut schema = Vec::new();
    let mut named_keys = Vec::new();
    for (name, expression) in keys {
        let column = ColumnName::new(name.as_str());
        schema.push(typed_column(&column, expression, input.schema())?);
        named_keys.push(NamedExpression {
            name: column,
            expression: expression.clone(),
            derived: None,
        });
    }
    let mut named_components = Vec::new();
    for (name, expression) in components {
        let column = ColumnName::new(name.as_str());
        schema.push(typed_column(&column, expression, input.schema())?);
        let Expression::Aggregate { call, filter } = expression else {
            return Err(InternalError::new(
                STAGE,
                "a measure is not an aggregate call",
            ));
        };
        named_components.push(AggregateComponent {
            name: column,
            call: call.clone(),
            filter: filter.as_deref().cloned(),
            recomposition: None,
        });
    }
    Ok(PlanNode::new(
        Operation::Aggregate {
            input: Box::new(input),
            keys: named_keys,
            components: named_components,
        },
        OutputSchema::new(schema),
        origin.clone(),
    ))
}

/// The key of `dimension`: its `base` field, truncated to its `grain` when it has one (E6-5.2).
fn key_expression(dimension: &SummaryDimension) -> Result<Expression, InternalError> {
    let base = reference(&ColumnName::new(dimension.base.as_str()))?;
    Ok(match dimension.grain {
        Some(grain) => Expression::DateTrunc {
            unit: date_unit(grain),
            operand: Box::new(base),
        },
        None => base,
    })
}

/// The `base_aggregate` of `measure` over the columns of the base `Scan`, and the fields it
/// reads.
fn measure_expression(
    summary: &Summary,
    measure: &SummaryMeasure,
) -> Result<(Expression, Vec<NormalizedName>), InternalError> {
    let tree = measure.base_aggregate.as_ref().ok_or_else(|| {
        defect(
            summary,
            &format!(
                "has the measure {} without base_aggregate",
                measure.field.as_str()
            ),
        )
    })?;
    Ok(over_columns(tree))
}

/// The `Scan` of the base that reads the fields `read`, in order.
fn base_scan(
    parts: &SummaryParts<'_>,
    read: &[NormalizedName],
    origin: &Origin,
) -> Result<PlanNode, InternalError> {
    let schema = read
        .iter()
        .map(|name| {
            let field = parts.base.field(name).ok_or_else(|| {
                defect(
                    parts.summary,
                    &format!("reads the unknown base field {}", name.as_str()),
                )
            })?;
            field_column(parts.summary, field)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlanNode::new(
        Operation::Scan {
            dataset: parts.base.name.clone(),
            row_identity: None,
        },
        OutputSchema::new(schema),
        origin.clone(),
    ))
}

/// The `Scan` column of `field`: its normalized name, type and nullability (E0H-2.2).
pub(super) fn field_column(
    summary: &Summary,
    field: &Field,
) -> Result<SchemaColumn, InternalError> {
    let typed = field.typed.as_ref().ok_or_else(|| {
        defect(
            summary,
            &format!("reads the untyped field {}", field.name.as_str()),
        )
    })?;
    let ValueType::Logical(logical_type) = &typed.value_type else {
        return Err(defect(
            summary,
            &format!("reads the field {} of type NULL", field.name.as_str()),
        ));
    };
    Ok(SchemaColumn {
        name: ColumnName::new(field.name.as_str()),
        logical_type: logical_type.clone(),
        nullable: typed.nullable,
    })
}

/// The catalog column the expression of the summary field `name` names, as the catalog writes
/// it (CHK-5.5, E0D-1.5).
fn catalog_column(
    parts: &SummaryParts<'_>,
    name: &NormalizedName,
    catalog: &CatalogSchemaDocument,
) -> Result<String, InternalError> {
    let not_a_column = || {
        defect(
            parts.summary,
            &format!(
                "has the field {}, which names no catalog column",
                name.as_str()
            ),
        )
    };
    let field = parts.dataset.field(name).ok_or_else(not_a_column)?;
    let Some(Expression::Reference(reference)) = &field.expression else {
        return Err(not_a_column());
    };
    if reference.qualifier.is_some() {
        return Err(not_a_column());
    }
    // The generator resolves a field's column the same way (CAT-8.2 makes it unique).
    let wanted = normalize_unquoted_identifier(reference.name.text());
    let matches = |column: &str| normalize_unquoted_identifier(column) == wanted;
    catalog
        .relations
        .iter()
        .find(|relation| relation.source == parts.dataset.source)
        .and_then(|relation| relation.columns.iter().find(|column| matches(&column.name)))
        .map(|column| column.name.clone())
        .ok_or_else(not_a_column)
}

fn date_unit(grain: Grain) -> DateUnit {
    match grain {
        Grain::Day => DateUnit::Day,
        Grain::Week => DateUnit::Week,
        Grain::Month => DateUnit::Month,
        Grain::Quarter => DateUnit::Quarter,
        Grain::Year => DateUnit::Year,
    }
}

fn add_once(names: &mut Vec<NormalizedName>, name: &NormalizedName) {
    if !names.contains(name) {
        names.push(name.clone());
    }
}

pub(super) fn defect(summary: &Summary, what: &str) -> InternalError {
    InternalError::new(
        STAGE,
        format!("the summary {} {what}", summary.dataset.as_str()),
    )
}
