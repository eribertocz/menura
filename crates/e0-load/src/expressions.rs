//! Substage E0.f, parsing only: turns the expression text of a model element into its
//! normalized tree through the crate `semantic-compiler-expressions`, the only expression
//! parser (S10), and turns a failure into the element's E0 diagnostic (E0-4.1, E0F-8).

use semantic_compiler_expressions::{ExpressionCode, ExpressionError, ExpressionErrorKind, parse};
use semantic_compiler_model::expression::Expression;

/// Where an expression appears in the model; it decides the code of a window function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpressionPlace {
    /// A field expression over `source` columns.
    Field,
    /// A metric expression.
    Metric,
    /// An expression inside an extension: population condition, row policy or summary
    /// definition (E0F-1).
    Extension,
}

/// The query error code of an element whose expression cannot be used (E0-4 table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpressionDiagnosticCode {
    /// `E0F-01`.
    Unparsable,
    /// `E0F-02`.
    OutsideSubset,
    /// `E_DEFERRED_WINDOWED_METRIC`: a window function in a metric (E0F-2.4, E0G-2.2).
    DeferredWindowedMetric,
}

impl ExpressionDiagnosticCode {
    /// The code as the specification writes it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unparsable => "E0F-01",
            Self::OutsideSubset => "E0F-02",
            Self::DeferredWindowedMetric => "E_DEFERRED_WINDOWED_METRIC",
        }
    }
}

/// The diagnostic of an element whose expression cannot be used: the element is unusable and
/// the load reports `W0F-01` (E0-4.1, E0F-2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpressionDiagnostic {
    /// The query error code of the element.
    pub(crate) code: ExpressionDiagnosticCode,
    /// The parser's error, with its location in the expression text.
    pub(crate) error: ExpressionError,
}

impl ExpressionDiagnostic {
    /// The load warning reported for every expression problem.
    pub(crate) const WARNING: &'static str = "W0F-01";
}

/// Parses the expression text of an element found at `place`.
pub(crate) fn parse_element_expression(
    text: &str,
    place: ExpressionPlace,
) -> Result<Expression, ExpressionDiagnostic> {
    parse(text).map_err(|error| diagnostic(error, place))
}

fn diagnostic(error: ExpressionError, place: ExpressionPlace) -> ExpressionDiagnostic {
    let code = match (error.kind(), place) {
        (ExpressionErrorKind::WindowFunction, ExpressionPlace::Metric) => {
            ExpressionDiagnosticCode::DeferredWindowedMetric
        }
        _ => match error.code() {
            ExpressionCode::Unparsable => ExpressionDiagnosticCode::Unparsable,
            ExpressionCode::OutsideSubset => ExpressionDiagnosticCode::OutsideSubset,
        },
    };
    ExpressionDiagnostic { code, error }
}

#[cfg(test)]
mod tests {
    use super::{
        ExpressionDiagnostic, ExpressionDiagnosticCode, ExpressionPlace, parse_element_expression,
    };

    fn code_of(text: &str, place: ExpressionPlace) -> ExpressionDiagnosticCode {
        parse_element_expression(text, place)
            .expect_err("the expression must fail")
            .code
    }

    #[test]
    fn s10_e0_parses_through_the_expressions_crate() {
        let tree = parse_element_expression("sum((orders.amount))", ExpressionPlace::Metric)
            .expect("the expression parses");
        assert_eq!(
            Ok(tree),
            semantic_compiler_expressions::parse("SUM(orders.amount)")
        );
    }

    #[test]
    fn e0f_01_and_e0f_02_become_element_diagnostics() {
        assert_eq!(code_of("SUM(", ExpressionPlace::Metric).as_str(), "E0F-01");
        assert_eq!(
            code_of("RANDOM()", ExpressionPlace::Field).as_str(),
            "E0F-02"
        );
        assert_eq!(ExpressionDiagnostic::WARNING, "W0F-01");
    }

    #[test]
    fn e0f_2_4_window_functions_are_deferred_only_in_metrics() {
        let window = "SUM(orders.amount) OVER (ORDER BY orders.order_id)";
        assert_eq!(
            code_of(window, ExpressionPlace::Metric).as_str(),
            "E_DEFERRED_WINDOWED_METRIC"
        );
        assert_eq!(code_of(window, ExpressionPlace::Field).as_str(), "E0F-02");
        assert_eq!(
            code_of(window, ExpressionPlace::Extension).as_str(),
            "E0F-02"
        );
    }
}
