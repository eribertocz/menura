use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};

use super::{MappedType, map_clickhouse_type};
use crate::clickhouse::tables::column_type;

fn logical(physical_type: &str) -> LogicalType {
    map_clickhouse_type(physical_type).logical_type
}

fn integer(bits: IntegerBits, signed: bool) -> LogicalType {
    LogicalType::Integer { bits, signed }
}

fn instant(precision: TimestampPrecision) -> LogicalType {
    LogicalType::Timestamp {
        precision,
        with_time_zone: true,
    }
}

const INTEGER_WIDTHS: [(u16, IntegerBits); 5] = [
    (8, IntegerBits::Eight),
    (16, IntegerBits::Sixteen),
    (32, IntegerBits::ThirtyTwo),
    (64, IntegerBits::SixtyFour),
    (128, IntegerBits::OneHundredTwentyEight),
];

#[test]
fn e6_7_13_maps_bool_to_boolean() {
    assert_eq!(logical("Bool"), LogicalType::Boolean);
}

#[test]
fn e6_7_13_maps_signed_integers_with_their_width() {
    for (width, bits) in INTEGER_WIDTHS {
        assert_eq!(
            logical(&format!("Int{width}")),
            integer(bits, true),
            "{width}"
        );
    }
}

#[test]
fn e6_7_13_maps_unsigned_integers_with_their_width() {
    for (width, bits) in INTEGER_WIDTHS {
        assert_eq!(
            logical(&format!("UInt{width}")),
            integer(bits, false),
            "{width}"
        );
    }
}

#[test]
fn e6_7_13_maps_a_decimal_of_precision_at_most_38() {
    assert_eq!(
        logical("Decimal(10, 2)"),
        LogicalType::Decimal {
            precision: 10,
            scale: 2
        }
    );
    assert_eq!(
        logical("Decimal(38, 0)"),
        LogicalType::Decimal {
            precision: 38,
            scale: 0
        }
    );
    assert_eq!(
        logical("Decimal(1,1)"),
        LogicalType::Decimal {
            precision: 1,
            scale: 1
        }
    );
}

#[test]
fn e6_7_13_maps_a_decimal_of_precision_above_38_to_unsupported() {
    for name in ["Decimal(39, 2)", "Decimal(76, 10)"] {
        assert_eq!(logical(name), LogicalType::Unsupported, "{name}");
    }
}

#[test]
fn e6_7_13_maps_floats_with_their_width() {
    assert_eq!(
        logical("Float32"),
        LogicalType::Float {
            bits: FloatBits::ThirtyTwo
        }
    );
    assert_eq!(
        logical("Float64"),
        LogicalType::Float {
            bits: FloatBits::SixtyFour
        }
    );
}

#[test]
fn e6_7_13_maps_string_to_text() {
    assert_eq!(logical("String"), LogicalType::Text);
}

#[test]
fn e6_7_13_maps_fixed_string_to_binary() {
    assert_eq!(logical("FixedString(16)"), LogicalType::Binary);
    assert_eq!(logical("FixedString(1)"), LogicalType::Binary);
}

#[test]
fn e6_7_13_maps_uuid_to_uuid() {
    assert_eq!(logical("UUID"), LogicalType::Uuid);
}

#[test]
fn e6_7_13_maps_date_and_date32_to_date() {
    assert_eq!(logical("Date"), LogicalType::Date);
    assert_eq!(logical("Date32"), LogicalType::Date);
}

#[test]
fn e6_7_13_maps_date_time_with_a_zone_to_an_instant_of_seconds() {
    assert_eq!(logical("DateTime('UTC')"), instant(TimestampPrecision::S));
    assert_eq!(
        logical("DateTime('America/La_Paz')"),
        instant(TimestampPrecision::S)
    );
}

#[test]
fn e6_7_13_maps_date_time64_with_a_zone_to_an_instant_of_its_precision() {
    for (digits, precision) in [
        (0, TimestampPrecision::S),
        (3, TimestampPrecision::Ms),
        (6, TimestampPrecision::Us),
        (9, TimestampPrecision::Ns),
    ] {
        assert_eq!(
            logical(&format!("DateTime64({digits}, 'UTC')")),
            instant(precision),
            "{digits}"
        );
    }
}

#[test]
fn e6_7_13_maps_a_date_time_without_a_time_zone_to_unsupported() {
    for name in ["DateTime", "DateTime64(3)", "DateTime64(6)", "DateTime('')"] {
        assert_eq!(logical(name), LogicalType::Unsupported, "{name}");
    }
}

#[test]
fn e6_7_13_maps_a_date_time64_of_another_precision_to_unsupported() {
    for name in [
        "DateTime64(1, 'UTC')",
        "DateTime64(2, 'UTC')",
        "DateTime64(7, 'UTC')",
    ] {
        assert_eq!(logical(name), LogicalType::Unsupported, "{name}");
    }
}

#[test]
fn e6_7_13_maps_an_array_to_a_list_of_the_element_mapping() {
    assert_eq!(
        logical("Array(Int64)"),
        LogicalType::List {
            element: Box::new(integer(IntegerBits::SixtyFour, true))
        }
    );
    assert_eq!(
        logical("Array(Nullable(String))"),
        LogicalType::List {
            element: Box::new(LogicalType::Text)
        }
    );
    assert_eq!(
        logical("Array(Array(Date32))"),
        LogicalType::List {
            element: Box::new(LogicalType::List {
                element: Box::new(LogicalType::Date)
            })
        }
    );
}

#[test]
fn e6_7_13_maps_an_array_of_an_unsupported_element_to_unsupported() {
    assert_eq!(logical("Array(IPv4)"), LogicalType::Unsupported);
}

#[test]
fn e6_7_13_reads_through_nullable_into_nullable() {
    assert_eq!(
        map_clickhouse_type("Nullable(Int32)"),
        MappedType {
            logical_type: integer(IntegerBits::ThirtyTwo, true),
            nullable: true,
        }
    );
    assert_eq!(
        map_clickhouse_type("Int32"),
        MappedType {
            logical_type: integer(IntegerBits::ThirtyTwo, true),
            nullable: false,
        }
    );
}

#[test]
fn e6_7_13_reads_through_low_cardinality() {
    assert_eq!(
        map_clickhouse_type("LowCardinality(String)"),
        MappedType {
            logical_type: LogicalType::Text,
            nullable: false,
        }
    );
}

#[test]
fn e6_7_13_reads_through_low_cardinality_then_nullable() {
    assert_eq!(
        map_clickhouse_type("LowCardinality(Nullable(String))"),
        MappedType {
            logical_type: LogicalType::Text,
            nullable: true,
        }
    );
}

#[test]
fn e6_7_13_does_not_read_the_wrappers_in_the_other_nesting_order() {
    assert_eq!(
        logical("Nullable(LowCardinality(String))"),
        LogicalType::Unsupported
    );
}

#[test]
fn e6_7_13_maps_every_type_the_table_does_not_list_to_unsupported() {
    for name in [
        "Int256",
        "UInt256",
        "Decimal32(2)",
        "Decimal256(10)",
        "BFloat16",
        "Enum8('a' = 1)",
        "IPv4",
        "IPv6",
        "Map(String, Int64)",
        "Tuple(Int64, String)",
        "JSON",
        "Nothing",
        "Time",
        "FixedString(0)",
        "Nullable(IPv4)",
        "",
    ] {
        assert_eq!(logical(name), LogicalType::Unsupported, "{name}");
    }
}

/// Every scalar logical type E6-7.2 writes a `ClickHouse` type for.
fn e6_7_2_scalar_types() -> Vec<LogicalType> {
    let mut types = vec![
        LogicalType::Boolean,
        LogicalType::Decimal {
            precision: 1,
            scale: 0,
        },
        LogicalType::Decimal {
            precision: 38,
            scale: 10,
        },
        LogicalType::Float {
            bits: FloatBits::ThirtyTwo,
        },
        LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
        LogicalType::Text,
        LogicalType::Uuid,
        LogicalType::Date,
    ];
    for (_, bits) in INTEGER_WIDTHS {
        types.push(integer(bits, true));
        types.push(integer(bits, false));
    }
    for precision in [
        TimestampPrecision::S,
        TimestampPrecision::Ms,
        TimestampPrecision::Us,
        TimestampPrecision::Ns,
    ] {
        types.push(instant(precision));
    }
    types
}

#[test]
fn e6_7_13_inverts_every_e6_7_2_scalar_type() {
    for logical_type in e6_7_2_scalar_types() {
        for nullable in [false, true] {
            let written = column_type(&logical_type, nullable).unwrap();
            assert_eq!(
                map_clickhouse_type(&written),
                MappedType {
                    logical_type: logical_type.clone(),
                    nullable,
                },
                "{written}"
            );
        }
    }
}

#[test]
fn e6_7_13_reads_binary_written_as_string_back_as_text() {
    for nullable in [false, true] {
        let written = column_type(&LogicalType::Binary, nullable).unwrap();
        assert_eq!(
            map_clickhouse_type(&written),
            MappedType {
                logical_type: LogicalType::Text,
                nullable,
            },
            "{written}"
        );
    }
}

#[test]
fn e6_7_13_inverts_the_e6_7_2_list_types() {
    let list = LogicalType::List {
        element: Box::new(integer(IntegerBits::SixtyFour, true)),
    };
    for written in ["Array(Int64)", "Array(Nullable(Int64))"] {
        assert_eq!(logical(written), list, "{written}");
    }
}
