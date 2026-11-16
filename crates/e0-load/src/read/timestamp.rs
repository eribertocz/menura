//! YAML 1.1 dates and timestamps, and their ISO 8601 text (E0A-2.5, E0A-2.8).
//!
//! The text form is the one the reference loader's values print as ISO 8601: `2026-11-16` for a
//! date, `2026-11-16T10:20:30` for a timestamp without time zone, followed by `.ffffff` when the
//! microseconds are not zero (digits beyond the sixth are dropped) and `+HH:MM` or `-HH:MM` when
//! the timestamp has a time zone (`Z` is `+00:00`).
//!
//! The specification does not fix the timestamp form yet (#61); this one is provisional.

/// The parts of a date or timestamp, as written.
struct Parts {
    year: u32,
    month: u32,
    day: u32,
    /// Whether the month and the day are written with two digits each.
    two_digit_date: bool,
    time: Option<Time>,
}

struct Time {
    hour: u32,
    minute: u32,
    second: u32,
    microsecond: u32,
    /// The time zone offset in minutes east of UTC, when there is one.
    offset_minutes: Option<i32>,
}

/// Whether an untagged plain scalar resolves to a timestamp: the date alone needs two-digit
/// months and days, a date with a time does not.
pub(crate) fn is_implicit_timestamp(text: &str) -> bool {
    parse(text).is_some_and(|parts| parts.time.is_some() || parts.two_digit_date)
}

/// Constructs a timestamp as its ISO 8601 text.
///
/// # Errors
///
/// A description when the text is not a YAML 1.1 timestamp, or names a date, time or time zone
/// that does not exist.
pub(crate) fn timestamp_text(text: &str) -> Result<String, String> {
    let parts = parse(text).ok_or_else(|| format!("'{text}' is not a timestamp"))?;
    validate(&parts).map_err(|problem| format!("'{text}' is not a valid timestamp: {problem}"))?;
    let date = format!("{:04}-{:02}-{:02}", parts.year, parts.month, parts.day);
    let Some(time) = &parts.time else {
        return Ok(date);
    };
    let fraction = if time.microsecond == 0 {
        String::new()
    } else {
        format!(".{:06}", time.microsecond)
    };
    let offset = time.offset_minutes.map(offset_text).unwrap_or_default();
    Ok(format!(
        "{date}T{:02}:{:02}:{:02}{fraction}{offset}",
        time.hour, time.minute, time.second
    ))
}

/// `+HH:MM` or `-HH:MM`; a zero offset is `+00:00`.
fn offset_text(offset_minutes: i32) -> String {
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let magnitude = offset_minutes.unsigned_abs();
    format!("{sign}{:02}:{:02}", magnitude / 60, magnitude % 60)
}

fn validate(parts: &Parts) -> Result<(), &'static str> {
    if !(1..=9999).contains(&parts.year) {
        return Err("the year is outside 1 to 9999");
    }
    if !(1..=12).contains(&parts.month) {
        return Err("the month is outside 1 to 12");
    }
    if parts.day == 0 || parts.day > days_in_month(parts.year, parts.month) {
        return Err("the day is out of range for the month");
    }
    let Some(time) = &parts.time else {
        return Ok(());
    };
    if time.hour > 23 || time.minute > 59 || time.second > 59 {
        return Err("the time of day does not exist");
    }
    if time
        .offset_minutes
        .is_some_and(|offset| offset.unsigned_abs() >= 24 * 60)
    {
        return Err("the time zone offset is 24 hours or more");
    }
    Ok(())
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Reads the YAML 1.1 timestamp expression:
/// `YYYY-M?M-D?D([Tt]|[ \t]+)H?H:MM:SS(.F*)?([ \t]*(Z|[-+]H?H(:MM)?))?`, the time part optional.
fn parse(text: &str) -> Option<Parts> {
    let mut reader = Reader::new(text);
    let year = reader.digits(4, 4)?;
    reader.require(b'-')?;
    let (month, month_digits) = reader.counted_digits(1, 2)?;
    reader.require(b'-')?;
    let (day, day_digits) = reader.counted_digits(1, 2)?;
    let time = if reader.at_end() {
        None
    } else {
        Some(parse_time(&mut reader)?)
    };
    reader.at_end().then_some(Parts {
        year,
        month,
        day,
        two_digit_date: month_digits == 2 && day_digits == 2,
        time,
    })
}

fn parse_time(reader: &mut Reader<'_>) -> Option<Time> {
    if !reader.eat_one(|byte| byte == b'T' || byte == b't') && reader.skip_blanks() == 0 {
        return None;
    }
    let hour = reader.digits(1, 2)?;
    reader.require(b':')?;
    let minute = reader.digits(2, 2)?;
    reader.require(b':')?;
    let second = reader.digits(2, 2)?;
    let microsecond = if reader.eat_one(|byte| byte == b'.') {
        reader.microseconds()
    } else {
        0
    };
    let offset_minutes = if reader.at_end() {
        None
    } else {
        reader.skip_blanks();
        Some(parse_offset(reader)?)
    };
    Some(Time {
        hour,
        minute,
        second,
        microsecond,
        offset_minutes,
    })
}

/// `Z` or `[-+]H?H(:MM)?`, in minutes east of UTC.
fn parse_offset(reader: &mut Reader<'_>) -> Option<i32> {
    if reader.eat_one(|byte| byte == b'Z') {
        return Some(0);
    }
    let negative = if reader.eat_one(|byte| byte == b'-') {
        true
    } else {
        reader.require(b'+')?;
        false
    };
    let hours = reader.digits(1, 2)?;
    let minutes = if reader.eat_one(|byte| byte == b':') {
        reader.digits(2, 2)?
    } else {
        0
    };
    let total = i32::try_from(hours * 60 + minutes).ok()?;
    Some(if negative { -total } else { total })
}

/// A position in the bytes of a timestamp.
struct Reader<'text> {
    bytes: &'text [u8],
    position: usize,
}

impl<'text> Reader<'text> {
    fn new(text: &'text str) -> Self {
        Self {
            bytes: text.as_bytes(),
            position: 0,
        }
    }

    fn at_end(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn eat_one(&mut self, accept: impl Fn(u8) -> bool) -> bool {
        match self.bytes.get(self.position) {
            Some(&byte) if accept(byte) => {
                self.position += 1;
                true
            }
            _ => false,
        }
    }

    /// Consumes `expected`, or fails when another byte or the end comes next.
    fn require(&mut self, expected: u8) -> Option<()> {
        self.eat_one(|byte| byte == expected).then_some(())
    }

    /// Skips spaces and tabs, and returns how many there were.
    fn skip_blanks(&mut self) -> usize {
        let start = self.position;
        while self.eat_one(|byte| byte == b' ' || byte == b'\t') {}
        self.position - start
    }

    /// Reads between `minimum` and `maximum` decimal digits as a number.
    fn digits(&mut self, minimum: usize, maximum: usize) -> Option<u32> {
        self.counted_digits(minimum, maximum)
            .map(|(value, _)| value)
    }

    /// Reads between `minimum` and `maximum` decimal digits, and returns the number and how many
    /// digits it had.
    fn counted_digits(&mut self, minimum: usize, maximum: usize) -> Option<(u32, usize)> {
        let mut value = 0;
        let mut count = 0;
        while count < maximum {
            let Some(digit) = self.next_digit() else {
                break;
            };
            value = value * 10 + digit;
            count += 1;
        }
        (count >= minimum).then_some((value, count))
    }

    /// Reads the fraction digits: the first six are the microseconds, padded with zeros.
    fn microseconds(&mut self) -> u32 {
        let mut value = 0;
        let mut count = 0;
        while let Some(digit) = self.next_digit() {
            if count < 6 {
                value = value * 10 + digit;
                count += 1;
            }
        }
        value * 10_u32.pow(6 - count)
    }

    fn next_digit(&mut self) -> Option<u32> {
        let byte = *self.bytes.get(self.position)?;
        let digit = char::from(byte).to_digit(10)?;
        self.position += 1;
        Some(digit)
    }
}
