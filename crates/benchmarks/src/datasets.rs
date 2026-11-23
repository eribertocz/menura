//! The datasets of a generated model, before they are written as documents (R5).

use crate::model_shape::{ModelShape, OTHER_DATASETS};

/// The logical kind of a generated column, which gives its catalog type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnKind {
    /// An integer key or measure.
    Integer,
    /// A date, declared as a time dimension in the model.
    Date,
    /// A text attribute.
    Text,
}

/// One field of a generated dataset, named after its column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) kind: ColumnKind,
}

/// One generated dataset: its name, key, fields and Menura extension object (JSON text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dataset {
    pub(crate) name: String,
    pub(crate) primary_key: Vec<String>,
    pub(crate) fields: Vec<Field>,
    pub(crate) extension: String,
}

/// The name of the star dataset at `index` (counted from 1).
pub(crate) fn star_name(index: usize) -> String {
    format!("star_{index:03}")
}

/// The name of the chain dataset at `index` (counted from 1).
pub(crate) fn chain_name(index: usize) -> String {
    format!("chain_{index:03}")
}

/// The name of the attribute at `index` (counted from 1) of the star or chain dataset
/// `dataset`. It starts with the dataset name, so that no two output columns share a name.
pub(crate) fn attribute_name(dataset: &str, index: usize) -> String {
    format!("{dataset}_attribute_{index:02}")
}

fn field(name: &str, kind: ColumnKind) -> Field {
    Field {
        name: name.to_owned(),
        kind,
    }
}

fn key_field(dataset: &str) -> Field {
    field(&format!("{dataset}_id"), ColumnKind::Integer)
}

/// Every dataset of the model of `shape`: `sales`, the star, the chain, `stock` and the two
/// summaries, in that order.
pub(crate) fn datasets(shape: &ModelShape) -> Vec<Dataset> {
    let mut attributes = attribute_counts(shape).into_iter();
    let mut all = vec![sales(shape)];
    for index in 1..=shape.star_datasets {
        let count = attributes.next().unwrap_or(1);
        all.push(star(index, count, index <= shape.row_policy_datasets));
    }
    for index in 1..=shape.chain_datasets {
        let count = attributes.next().unwrap_or(1);
        all.push(chain(index, count, index < shape.chain_datasets));
    }
    all.push(stock());
    all.extend(summaries());
    all
}

/// Fields of `sales`, `stock` and the summaries, and the keys of the star and the chain.
fn fixed_fields(shape: &ModelShape) -> usize {
    let sales = 6 + shape.star_datasets;
    let others = 3 + 3 + 4;
    let keys = shape.star_datasets + 2 * shape.chain_datasets - 1;
    sales + others + keys
}

/// The attributes of the sweep dataset, the last star dataset: one per dimension of the
/// largest R3 sweep query, so that every sweep query reads the same joined dataset.
pub(crate) const SWEEP_ATTRIBUTES: usize = 64;

/// The attribute count of each star dataset, then of each chain dataset, so that the model
/// has exactly `total_fields` fields: the sweep dataset has [`SWEEP_ATTRIBUTES`], the others
/// share the rest, and the first of them take the remainder.
fn attribute_counts(shape: &ModelShape) -> Vec<usize> {
    let others = shape.datasets() - OTHER_DATASETS - 1;
    let attributes = shape
        .total_fields
        .saturating_sub(fixed_fields(shape) + SWEEP_ATTRIBUTES);
    let each = attributes / others;
    let remainder = attributes % others;
    let mut counts: Vec<usize> = (0..others)
        .map(|index| each + usize::from(index < remainder))
        .collect();
    counts.insert(shape.star_datasets - 1, SWEEP_ATTRIBUTES);
    counts
}

fn roles_access() -> &'static str {
    r#""access": {"mode": "roles", "roles": ["analyst"]}"#
}

fn extension(members: &str) -> String {
    format!(r#"{{"namespace": "menura", "version": 1, {members}}}"#)
}

fn sales(shape: &ModelShape) -> Dataset {
    let mut fields = vec![
        field("sales_id", ColumnKind::Integer),
        field("sale_date", ColumnKind::Date),
        field("amount", ColumnKind::Integer),
        field("quantity", ColumnKind::Integer),
        field("cost", ColumnKind::Integer),
    ];
    fields.extend((1..=shape.star_datasets).map(|index| key_field(&star_name(index))));
    fields.push(key_field(&chain_name(1)));
    Dataset {
        name: "sales".to_owned(),
        primary_key: vec!["sales_id".to_owned()],
        fields,
        extension: extension(roles_access()),
    }
}

fn with_attributes(dataset: &str, mut fields: Vec<Field>, count: usize) -> Vec<Field> {
    fields
        .extend((1..=count).map(|index| field(&attribute_name(dataset, index), ColumnKind::Text)));
    fields
}

fn star(index: usize, attributes: usize, row_policy: bool) -> Dataset {
    let name = star_name(index);
    let members = if row_policy {
        format!(
            r#"{}, "row_access": [{{"name": "{name}_scope", "condition": "{name}_attribute_01 IN (USER_ATTRIBUTE('regions'))", "roles": ["analyst"]}}]"#,
            roles_access()
        )
    } else {
        roles_access().to_owned()
    };
    Dataset {
        primary_key: vec![format!("{name}_id")],
        fields: with_attributes(&name, vec![key_field(&name)], attributes),
        extension: extension(&members),
        name,
    }
}

fn chain(index: usize, attributes: usize, has_next: bool) -> Dataset {
    let name = chain_name(index);
    let mut keys = vec![key_field(&name)];
    if has_next {
        keys.push(key_field(&chain_name(index + 1)));
    }
    Dataset {
        primary_key: vec![format!("{name}_id")],
        fields: with_attributes(&name, keys, attributes),
        extension: extension(roles_access()),
        name,
    }
}

fn stock() -> Dataset {
    Dataset {
        name: "stock".to_owned(),
        primary_key: vec!["warehouse".to_owned(), "product".to_owned()],
        fields: vec![
            field("warehouse", ColumnKind::Text),
            field("product", ColumnKind::Text),
            field("quantity", ColumnKind::Integer),
        ],
        extension: extension(roles_access()),
    }
}

fn summaries() -> [Dataset; 2] {
    let measures = r#""measures": [{"field": "amount_sum", "base_aggregate": "SUM(sales.amount)"}, {"field": "sale_count", "base_aggregate": "COUNT(sales.sales_id)"}], "coverage_field": "sale_month""#;
    let month = r#"{"field": "sale_month", "base": "sale_date", "grain": "month"}"#;
    let summary = |name: &str, dimensions: &str, extra: Vec<Field>| {
        let mut fields = vec![field("sale_month", ColumnKind::Date)];
        fields.extend(extra);
        fields.push(field("amount_sum", ColumnKind::Integer));
        fields.push(field("sale_count", ColumnKind::Integer));
        Dataset {
            name: name.to_owned(),
            primary_key: fields
                .iter()
                .take(fields.len() - 2)
                .map(|field| field.name.clone())
                .collect(),
            fields,
            extension: extension(&format!(
                r#""access": {{"mode": "internal"}}, "aggregate": {{"of": "sales", "dimensions": [{dimensions}], {measures}}}"#
            )),
        }
    };
    [
        summary("sales_by_month", month, Vec::new()),
        summary(
            "sales_by_month_and_star",
            &format!(r#"{month}, {{"field": "star_001_id", "base": "star_001_id"}}"#),
            vec![key_field(&star_name(1))],
        ),
    ]
}
