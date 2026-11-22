//! Types shared by several contract documents.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::contract::ContractVersion;

/// An opaque data version or document identifier (CAT-6): compared by equality of both fields,
/// never ordered or parsed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    /// Identifies the source.
    pub origin: String,
    /// Identifies an immutable state of the source.
    pub snapshot: String,
}

/// A neutral logical type (CAT-4.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogicalType {
    /// `BOOLEAN`.
    Boolean,
    /// Integer of a fixed width.
    Integer {
        /// Width in bits.
        bits: IntegerBits,
        /// Whether negative values exist.
        signed: bool,
    },
    /// Exact decimal.
    Decimal {
        /// Total digits, 1–38.
        precision: u8,
        /// Digits after the point, 0–`precision`.
        scale: u8,
    },
    /// IEEE-754 binary float.
    Float {
        /// Width in bits.
        bits: FloatBits,
    },
    /// Text.
    Text,
    /// Calendar date.
    Date,
    /// Time of day.
    Time,
    /// Timestamp.
    Timestamp {
        /// Stored fractional-second precision.
        precision: TimestampPrecision,
        /// Whether it is an instant (with time zone) or a wall-clock value.
        with_time_zone: bool,
    },
    /// Interval.
    Interval,
    /// Binary data.
    Binary,
    /// UUID.
    Uuid,
    /// List of one element type.
    List {
        /// Type of every element.
        element: Box<LogicalType>,
    },
    /// Any type without a neutral equivalent.
    Unsupported,
}

/// Width of an integer type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub enum IntegerBits {
    /// 8 bits.
    Eight,
    /// 16 bits.
    Sixteen,
    /// 32 bits.
    ThirtyTwo,
    /// 64 bits.
    SixtyFour,
    /// 128 bits.
    OneHundredTwentyEight,
}

impl TryFrom<u16> for IntegerBits {
    type Error = String;

    fn try_from(bits: u16) -> Result<Self, Self::Error> {
        match bits {
            8 => Ok(Self::Eight),
            16 => Ok(Self::Sixteen),
            32 => Ok(Self::ThirtyTwo),
            64 => Ok(Self::SixtyFour),
            128 => Ok(Self::OneHundredTwentyEight),
            other => Err(format!("{other} is not an integer width")),
        }
    }
}

impl From<IntegerBits> for u16 {
    fn from(bits: IntegerBits) -> Self {
        match bits {
            IntegerBits::Eight => 8,
            IntegerBits::Sixteen => 16,
            IntegerBits::ThirtyTwo => 32,
            IntegerBits::SixtyFour => 64,
            IntegerBits::OneHundredTwentyEight => 128,
        }
    }
}

/// Width of a float type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub enum FloatBits {
    /// 32 bits.
    ThirtyTwo,
    /// 64 bits.
    SixtyFour,
}

impl TryFrom<u16> for FloatBits {
    type Error = String;

    fn try_from(bits: u16) -> Result<Self, Self::Error> {
        match bits {
            32 => Ok(Self::ThirtyTwo),
            64 => Ok(Self::SixtyFour),
            other => Err(format!("{other} is not a float width")),
        }
    }
}

impl From<FloatBits> for u16 {
    fn from(bits: FloatBits) -> Self {
        match bits {
            FloatBits::ThirtyTwo => 32,
            FloatBits::SixtyFour => 64,
        }
    }
}

/// Fractional-second precision of a timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampPrecision {
    /// Seconds.
    S,
    /// Milliseconds.
    Ms,
    /// Microseconds.
    Us,
    /// Nanoseconds.
    Ns,
}

/// A time grain (CAT-5.1, QRY-4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grain {
    /// Day.
    Day,
    /// ISO week, starting Monday.
    Week,
    /// Month.
    Month,
    /// Quarter.
    Quarter,
    /// Year.
    Year,
}

/// Target SQL dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dialect {
    /// `DuckDB`.
    Duckdb,
}

/// A compiler stage that takes a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Step {
    /// Load.
    E0,
    /// Parse.
    E1,
    /// Bind.
    E2,
    /// Paths.
    E3,
    /// Plan.
    E4,
    /// Optimize.
    E5,
    /// Generate.
    E6,
}

/// One decision of the explanation (P7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// The rule ID that decides it (PLN-7), stable across releases; absent before contract
    /// version 1.2.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Stage that decided.
    pub step: Step,
    /// The object decided about, as `<kind>:<name>` (PLN-7.2); absent before contract version
    /// 1.2.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// What was decided.
    pub decision: String,
    /// Why.
    pub reason: String,
}

/// Versions of the compiler and of the specifications it implements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    /// Compiler version.
    pub compiler_version: ContractVersion,
    /// Normative Apache Ossie release.
    pub ossie_normative: String,
    /// Tracked Apache Ossie development commit (7–40 lowercase hex digits).
    pub ossie_tracked_commit: String,
    /// Time zone database version.
    pub tzdata_version: String,
}

/// A bound parameter (PLN-1), with its value encoded as CON-4 requires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    /// Position, from 1 (`$1`).
    pub position: u64,
    /// Logical type of the value.
    pub logical_type: LogicalType,
    /// The value, in CON-4 encoding.
    pub value: Value,
}

/// Result of a check run by a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// The check query returned zero rows.
    Passed,
    /// The check query returned violations.
    Failed,
}

/// Deserializes a member whose presence matters even when its value is `null`: a present member
/// becomes `Some`, so `null` round-trips as `null` instead of disappearing.
pub(crate) fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
