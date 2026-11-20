//! The codes of typing problems (E0F-8), as data types only.

use std::fmt;

/// The code of a typing diagnostic (E0F-8), and `E0F-02` for `AGG(…)` where no metric may be
/// read (DER-5.1.2). In E0, every one makes its element unusable and is reported as `W0F-01`
/// (E0-4 table).
///
/// The order of the variants is the order of the codes, so the lowest code of an unusable
/// element is the first that E2-5 row 12a raises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypingCode {
    /// `E0F-02`: `AGG(…)` in a field or a dataset metric, outside the accepted subset
    /// (DER-5.1.2).
    OutsideSubset,
    /// `E0F-03`: incompatible types without an explicit `CAST`.
    IncompatibleTypes,
    /// `E0F-04`: use of an `unsupported` column (CAT-4.3).
    UnsupportedColumn,
    /// `E0F-05`: a relationship key pair with incompatible types (E0F-4.1).
    IncompatibleKeyPair,
    /// `E0F-06`: an expression that must be boolean is not.
    NotBoolean,
    /// `E0F-07`: an aggregate where none is allowed, or a nested aggregate.
    MisplacedAggregate,
    /// `E0F-08`: a decimal multiplication whose scale exceeds 38.
    DecimalScaleOverflow,
}

impl TypingCode {
    /// The code as the specification writes it, such as `E0F-03`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OutsideSubset => "E0F-02",
            Self::IncompatibleTypes => "E0F-03",
            Self::UnsupportedColumn => "E0F-04",
            Self::IncompatibleKeyPair => "E0F-05",
            Self::NotBoolean => "E0F-06",
            Self::MisplacedAggregate => "E0F-07",
            Self::DecimalScaleOverflow => "E0F-08",
        }
    }
}

impl fmt::Display for TypingCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
