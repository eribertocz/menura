//! The time zone and reference time of a query (E1-5 "Time zone, reference time").

use menura_model::expression::{DateValue, TimestampValue};

/// A time-zone name: the exact spelling of a zone or link of the bundled IANA database
/// (E1-2.3). The caller has checked it is known.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimeZoneName(String);

impl TimeZoneName {
    /// The time zone named `text`, such as `Europe/Madrid`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The offset from UTC of an RFC 3339 instant, in whole minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UtcOffset(i16);

impl UtcOffset {
    /// The largest offset RFC 3339 can write, `23:59`, in minutes.
    pub const MAXIMUM_MINUTES: i16 = 23 * 60 + 59;

    /// The offset of `minutes` east of UTC. Returns `None` beyond ±23:59.
    ///
    /// ```
    /// use menura_representations::time::UtcOffset;
    /// assert!(UtcOffset::from_minutes(-300).is_some());
    /// assert!(UtcOffset::from_minutes(24 * 60).is_none());
    /// ```
    #[must_use]
    pub fn from_minutes(minutes: i16) -> Option<Self> {
        (minutes.abs() <= Self::MAXIMUM_MINUTES).then_some(Self(minutes))
    }

    /// The offset in minutes east of UTC.
    #[must_use]
    pub fn minutes(self) -> i16 {
        self.0
    }
}

/// The `reference_time` of a query: an RFC 3339 instant, as its local date and time with its
/// offset from UTC.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceTime {
    /// The date and time at `offset`.
    pub local: TimestampValue,
    /// The offset from UTC.
    pub offset: UtcOffset,
}

/// The time settings of a query, which E2 uses to resolve relative ranges (QRY-4.2) and E4
/// and E6 to truncate time grains (E6-5.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryTime {
    /// The `time_zone`.
    pub time_zone: TimeZoneName,
    /// The `reference_time`, when present.
    pub reference_time: Option<ReferenceTime>,
    /// The date of `reference_time` in `time_zone`, which relative ranges count from
    /// (QRY-4.2); `None` without a reference time, or when that date falls outside the years
    /// 0001–9999.
    pub reference_date: Option<DateValue>,
}
