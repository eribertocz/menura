//! A context declaration of the `context` extension (CTX-2), as data types only.

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;

use crate::extensions::additivity::FieldReference;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The `context` of one metric whose keys and values hold in E0.c (CTX-5.1.1, CTX-5.1.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextDeclaration {
    /// The JSON Pointer of the metric that declares it.
    pub metric: JsonPointer,
    /// The JSON Pointer of the `data` member of the metric's entry of ours, where every
    /// diagnostic about the declaration is located (E0C-2.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// The value of `base` as written: the name of the metric evaluated in the modified
    /// context, which E0.d resolves (CTX-5.1.3).
    pub base: String,
    /// The modifier.
    pub modifier: Modifier,
}

/// The value of `modifier.type`, with the keys of that type (CTX-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modifier {
    /// `period_to_date`: the base from the start of the `grain` period (CTX-3a.1).
    PeriodToDate(TimeModifier),
    /// `prior_period`: the base for the period shifted back `offset` units of `grain`
    /// (CTX-3a.2).
    PriorPeriod {
        /// The time dimension and grain.
        time: TimeModifier,
        /// The number of `grain` units to shift back; always positive (`E0C-62`).
        offset: NonZeroU64,
    },
    /// `share_of_total`: the base divided by the base with `remove` taken out of the grouping
    /// (CTX-3a.3).
    ShareOfTotal(Removed),
}

impl Modifier {
    /// The time dimension and grain of a `period_to_date` or `prior_period` modifier.
    #[must_use]
    pub fn time(&self) -> Option<&TimeModifier> {
        match self {
            Self::PeriodToDate(time) | Self::PriorPeriod { time, .. } => Some(time),
            Self::ShareOfTotal(_) => None,
        }
    }

    /// The fields a `share_of_total` modifier removes, when it lists them.
    #[must_use]
    pub fn removed_fields(&self) -> &[FieldReference] {
        match self {
            Self::ShareOfTotal(Removed::Fields(fields)) => fields,
            _ => &[],
        }
    }
}

/// The keys `time_dimension` and `grain` of the time modifiers (CTX-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeModifier {
    /// The time field, a reference `dataset.field` (CTX-5.1.2).
    pub time_dimension: FieldReference,
    /// The grain of the period.
    pub grain: Grain,
}

/// The value of `remove` of a `share_of_total` modifier (CTX-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removed {
    /// The string `all`: every dimension is taken out of the grouping.
    All,
    /// A non-empty list of fields without repeats (E0D-1.3).
    Fields(Vec<FieldReference>),
}
