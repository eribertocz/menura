//! The values a `ToDate` node carries (E4-3.15, E4-8.5).

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;

/// The query `date_range` a `ToDate` node keeps periods by, when it is on the time dimension:
/// only the query-grain periods that overlap it are kept (E4-8.5). Its bounds are query values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToDateRange {
    /// The first day of the range, inclusive.
    pub from: DateValue,
    /// The last day of the range, inclusive.
    pub to: DateValue,
}

/// The grains of a `ToDate` node (E4-8.5, E4-8.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToDateGrains {
    /// The query grain of the time dimension: the grain of the periods a row is repeated for.
    pub query: Grain,
    /// The `grain` of the modifier: a row is repeated only within its period of this grain,
    /// the ISO week-numbering year for week periods and `year`.
    pub to_date: Grain,
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{Grain, LogicalType};

    use super::ToDateGrains;
    use crate::logical_plan::{Operation, OutputSchema, PlanNode, SchemaColumn};
    use crate::names::ColumnName;
    use crate::test_support::{in_model, name};

    fn date_column(text: &str) -> SchemaColumn {
        SchemaColumn {
            name: ColumnName::new(text),
            logical_type: LogicalType::Date,
            nullable: false,
        }
    }

    #[test]
    fn e4_3_15_a_to_date_reads_its_input_and_adds_its_period() {
        let origin = in_model("/semantic_model/0/datasets/0");
        let input = PlanNode::new(
            Operation::Scan {
                dataset: name("ORDERS"),
                row_identity: None,
            },
            OutputSchema::new(vec![date_column("ORDER_DATE")]),
            origin.clone(),
        );
        let to_date = PlanNode::new(
            Operation::ToDate {
                input: Box::new(input),
                dimension: ColumnName::new("ORDER_DATE"),
                grains: ToDateGrains {
                    query: Grain::Week,
                    to_date: Grain::Year,
                },
                range: None,
                period: ColumnName::new("menura.period"),
            },
            OutputSchema::new(vec![
                date_column("ORDER_DATE"),
                date_column("menura.period"),
            ]),
            origin,
        );
        let inputs = to_date.inputs();
        assert_eq!(inputs.len(), 1);
        assert!(matches!(
            inputs.first().map(|input| input.operation()),
            Some(Operation::Scan { .. })
        ));
        assert!(to_date.is_simple_plan());
    }
}
