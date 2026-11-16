//! A small deterministic generator of valid trees, for the property test of E0H-1.3.
//!
//! A failure always reproduces from its seed, and no property-testing dependency is needed.

use semantic_compiler_model::expression::{
    AggregateCall, ApproximateNumber, BinaryOperator, CaseBranch, CastTarget, DateValue,
    DecimalTarget, DistinctAggregate, ExactNumber, Expression, Identifier, LikeCase, Literal,
    PercentileFunction, Reference, ScalarFunction, SimpleAggregate, SortDirection, TimeOfDay,
    TimestampValue, UnaryOperator,
};

/// A xorshift generator: deterministic and dependency free.
pub struct Generator(u64);

impl Generator {
    /// A generator whose sequence depends only on `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).max(1))
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    fn flag(&mut self) -> bool {
        self.next().is_multiple_of(2)
    }
}

const BINARY_OPERATORS: [BinaryOperator; 14] = [
    BinaryOperator::Add,
    BinaryOperator::Subtract,
    BinaryOperator::Multiply,
    BinaryOperator::Divide,
    BinaryOperator::Modulo,
    BinaryOperator::Concatenate,
    BinaryOperator::Equal,
    BinaryOperator::NotEqual,
    BinaryOperator::Less,
    BinaryOperator::LessOrEqual,
    BinaryOperator::Greater,
    BinaryOperator::GreaterOrEqual,
    BinaryOperator::And,
    BinaryOperator::Or,
];

const SIMPLE_AGGREGATES: [SimpleAggregate; 12] = [
    SimpleAggregate::Sum,
    SimpleAggregate::Count,
    SimpleAggregate::Average,
    SimpleAggregate::Minimum,
    SimpleAggregate::Maximum,
    SimpleAggregate::StandardDeviation,
    SimpleAggregate::PopulationStandardDeviation,
    SimpleAggregate::SampleStandardDeviation,
    SimpleAggregate::Variance,
    SimpleAggregate::PopulationVariance,
    SimpleAggregate::SampleVariance,
    SimpleAggregate::Median,
];

fn identifier(generator: &mut Generator) -> Identifier {
    match generator.below(4) {
        0 => Identifier::regular("amount").unwrap(),
        1 => Identifier::regular("_Status2").unwrap(),
        2 => Identifier::regular("naïve").unwrap(),
        _ => Identifier::delimited("Order \"Total\" -- x").unwrap(),
    }
}

fn reference(generator: &mut Generator) -> Reference {
    Reference {
        qualifier: generator.flag().then(|| identifier(generator)),
        name: identifier(generator),
    }
}

fn exact(generator: &mut Generator) -> ExactNumber {
    let (integer, fraction) = *generator.pick(&[("0", ""), ("12", ""), ("3", "50"), ("0", "001")]);
    ExactNumber::new(integer, fraction).unwrap()
}

fn literal(generator: &mut Generator) -> Literal {
    match generator.below(7) {
        0 => Literal::Null,
        1 => Literal::Boolean(generator.flag()),
        2 => Literal::Text((*generator.pick(&["", "it's", "a\nb", "-- not a comment"])).to_owned()),
        3 => Literal::Exact(exact(generator)),
        4 => Literal::Approximate(ApproximateNumber {
            mantissa: exact(generator),
            exponent: *generator.pick(&[-12, 0, 7]),
        }),
        5 => Literal::Date(DateValue::new(2024, 2, 29).unwrap()),
        _ => Literal::Timestamp(TimestampValue {
            date: DateValue::new(1, 1, 1).unwrap(),
            time: TimeOfDay::new(23, 59, 59, generator.pick(&["", "5", "123456789"])).unwrap(),
        }),
    }
}

fn cast_target(generator: &mut Generator) -> CastTarget {
    *generator.pick(&[
        CastTarget::Boolean,
        CastTarget::SmallInteger,
        CastTarget::Integer,
        CastTarget::BigInteger,
        CastTarget::Decimal(DecimalTarget::new(38, 4).unwrap()),
        CastTarget::Real,
        CastTarget::DoublePrecision,
        CastTarget::Varchar,
        CastTarget::Date,
        CastTarget::Time,
        CastTarget::Timestamp,
    ])
}

fn leaf(generator: &mut Generator) -> Expression {
    match generator.below(6) {
        0 => Expression::Reference(reference(generator)),
        1 => Expression::MetricAggregate(reference(generator)),
        2 => Expression::UserAttribute("it's".to_owned()),
        3 => Expression::CurrentDate,
        4 => Expression::CurrentTimestamp,
        _ => Expression::Literal(literal(generator)),
    }
}

fn boxed(generator: &mut Generator, depth: u32) -> Box<Expression> {
    Box::new(expression(generator, depth))
}

pub fn expression(generator: &mut Generator, depth: u32) -> Expression {
    if depth == 0 {
        return leaf(generator);
    }
    let depth = depth - 1;
    match generator.below(12) {
        0 => Expression::Unary {
            operator: *generator.pick(&[
                UnaryOperator::Plus,
                UnaryOperator::Minus,
                UnaryOperator::Not,
            ]),
            operand: boxed(generator, depth),
        },
        1 => Expression::Binary {
            operator: *generator.pick(&BINARY_OPERATORS),
            left: boxed(generator, depth),
            right: boxed(generator, depth),
        },
        2..=6 => predicate(generator, depth),
        7 => case_expression(generator, depth),
        8 => Expression::Cast {
            operand: boxed(generator, depth),
            target: cast_target(generator),
        },
        9 => function(generator, depth),
        10 => aggregate(generator, depth),
        _ => leaf(generator),
    }
}

/// One of the predicates: `IS NULL`, `IS DISTINCT FROM`, `IN`, `BETWEEN`, `LIKE`.
fn predicate(generator: &mut Generator, depth: u32) -> Expression {
    match generator.below(5) {
        0 => Expression::IsNull {
            operand: boxed(generator, depth),
            negated: generator.flag(),
        },
        1 => Expression::IsDistinctFrom {
            left: boxed(generator, depth),
            right: boxed(generator, depth),
            negated: generator.flag(),
        },
        2 => Expression::InList {
            operand: boxed(generator, depth),
            list: (0..=generator.below(3))
                .map(|_| expression(generator, depth))
                .collect(),
            negated: generator.flag(),
        },
        3 => Expression::Between {
            operand: boxed(generator, depth),
            low: boxed(generator, depth),
            high: boxed(generator, depth),
            negated: generator.flag(),
        },
        _ => Expression::Like {
            operand: boxed(generator, depth),
            pattern: boxed(generator, depth),
            case: *generator.pick(&[LikeCase::Sensitive, LikeCase::Insensitive]),
            negated: generator.flag(),
        },
    }
}

fn case_expression(generator: &mut Generator, depth: u32) -> Expression {
    Expression::Case {
        operand: generator.flag().then(|| boxed(generator, depth)),
        branches: (0..=generator.below(2))
            .map(|_| CaseBranch {
                condition: expression(generator, depth),
                result: expression(generator, depth),
            })
            .collect(),
        otherwise: generator.flag().then(|| boxed(generator, depth)),
    }
}

fn function(generator: &mut Generator, depth: u32) -> Expression {
    let function = *generator.pick(&[
        ScalarFunction::Coalesce,
        ScalarFunction::Nullif,
        ScalarFunction::Round,
    ]);
    let count = match function {
        ScalarFunction::Coalesce => 2 + generator.below(2),
        ScalarFunction::Nullif | ScalarFunction::Round => 2,
    };
    Expression::Function {
        function,
        arguments: (0..count).map(|_| expression(generator, depth)).collect(),
    }
}

fn aggregate(generator: &mut Generator, depth: u32) -> Expression {
    let call = match generator.below(4) {
        0 => AggregateCall::CountRows,
        1 => AggregateCall::Simple {
            function: *generator.pick(&SIMPLE_AGGREGATES),
            argument: boxed(generator, depth),
        },
        2 => AggregateCall::Distinct {
            function: *generator.pick(&[DistinctAggregate::Sum, DistinctAggregate::Count]),
            argument: boxed(generator, depth),
        },
        _ => AggregateCall::Percentile {
            function: *generator
                .pick(&[PercentileFunction::Continuous, PercentileFunction::Discrete]),
            fraction: boxed(generator, depth),
            ordering: boxed(generator, depth),
            direction: *generator.pick(&[SortDirection::Ascending, SortDirection::Descending]),
        },
    };
    Expression::Aggregate {
        call,
        filter: generator.flag().then(|| boxed(generator, depth)),
    }
}
