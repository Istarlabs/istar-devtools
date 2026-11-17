// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conversion between Unix timestamps and calendar dates.

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Zones offered in the UI. Index 0 is UTC, index 1 the local zone; the rest are IANA names.
pub const ZONES: [&str; 12] = [
    "UTC",
    "Local",
    "Europe/Berlin",
    "Europe/London",
    "Europe/Moscow",
    "America/New_York",
    "America/Chicago",
    "America/Los_Angeles",
    "Asia/Kolkata",
    "Asia/Shanghai",
    "Asia/Tokyo",
    "Australia/Sydney",
];

#[derive(Debug, Clone, Copy)]
pub enum Zone {
    Utc,
    Local,
    Named(Tz),
}

impl Zone {
    pub fn from_index(index: i32) -> Self {
        match ZONES.get(index.max(0) as usize).copied() {
            Some("Local") => Zone::Local,
            Some(name) => name.parse::<Tz>().map(Zone::Named).unwrap_or(Zone::Utc),
            None => Zone::Utc,
        }
    }

    fn rfc3339(self, instant: DateTime<Utc>) -> String {
        match self {
            Zone::Utc => instant.to_rfc3339(),
            Zone::Local => instant.with_timezone(&Local).to_rfc3339(),
            Zone::Named(tz) => instant.with_timezone(&tz).to_rfc3339(),
        }
    }

    fn rfc2822(self, instant: DateTime<Utc>) -> String {
        match self {
            Zone::Utc => instant.to_rfc2822(),
            Zone::Local => instant.with_timezone(&Local).to_rfc2822(),
            Zone::Named(tz) => instant.with_timezone(&tz).to_rfc2822(),
        }
    }

    /// Interpret a wall-clock time in this zone. `None` for a time that does not exist (DST gap).
    fn resolve(self, naive: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self {
            Zone::Utc => Some(naive.and_utc()),
            Zone::Local => Local
                .from_local_datetime(&naive)
                .earliest()
                .map(|t| t.with_timezone(&Utc)),
            Zone::Named(tz) => tz
                .from_local_datetime(&naive)
                .earliest()
                .map(|t| t.with_timezone(&Utc)),
        }
    }
}

/// Parse a Unix timestamp. The unit is guessed from the magnitude:
/// seconds below 1e11, milliseconds below 1e14, microseconds below 1e17, else nanoseconds.
pub fn parse_unix(text: &str) -> Option<DateTime<Utc>> {
    let text = text.trim();
    if let Ok(value) = text.parse::<i64>() {
        let magnitude = value.unsigned_abs();
        return match magnitude {
            0..100_000_000_000 => DateTime::from_timestamp(value, 0),
            100_000_000_000..100_000_000_000_000 => DateTime::from_timestamp_millis(value),
            100_000_000_000_000..100_000_000_000_000_000 => DateTime::from_timestamp_micros(value),
            _ => Some(DateTime::from_timestamp_nanos(value)),
        };
    }
    // Fractional seconds, e.g. "1700000000.25".
    let seconds: f64 = text.parse().ok()?;
    if !seconds.is_finite() || seconds.abs() >= 1e11 {
        return None;
    }
    let whole = seconds.floor();
    let nanos = ((seconds - whole) * 1e9).round() as u32;
    DateTime::from_timestamp(whole as i64, nanos.min(999_999_999))
}

/// Parse a date. Offsets in the text win; a bare date/time is read in `zone`.
pub fn parse_date(text: &str, zone: Zone) -> Option<DateTime<Utc>> {
    let text = text.trim();
    if let Ok(parsed) = DateTime::parse_from_rfc3339(text) {
        return Some(parsed.with_timezone(&Utc));
    }
    if let Ok(parsed) = DateTime::parse_from_rfc2822(text) {
        return Some(parsed.with_timezone(&Utc));
    }
    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M",
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(text, format) {
            return zone.resolve(naive);
        }
    }
    for format in ["%Y-%m-%d", "%d.%m.%Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(text, format) {
            return zone.resolve(date.and_hms_opt(0, 0, 0)?);
        }
    }
    None
}

/// "3 hours ago" / "in 2 days".
pub fn relative(now: DateTime<Utc>, instant: DateTime<Utc>) -> String {
    let seconds = (instant - now).num_seconds();
    let magnitude = seconds.unsigned_abs();
    let (value, unit) = match magnitude {
        0..5 => return "just now".to_string(),
        5..60 => (magnitude, "second"),
        60..3_600 => (magnitude / 60, "minute"),
        3_600..86_400 => (magnitude / 3_600, "hour"),
        86_400..2_592_000 => (magnitude / 86_400, "day"),
        2_592_000..31_536_000 => (magnitude / 2_592_000, "month"),
        _ => (magnitude / 31_536_000, "year"),
    };
    let plural = if value == 1 { "" } else { "s" };
    if seconds < 0 {
        format!("{value} {unit}{plural} ago")
    } else {
        format!("in {value} {unit}{plural}")
    }
}

/// Convert whatever the user typed (a Unix timestamp or a date) into labelled rows.
pub fn convert(
    input: &str,
    zone: Zone,
    zone_label: &str,
    now: DateTime<Utc>,
) -> Result<Vec<(String, String)>, String> {
    let instant = parse_unix(input)
        .or_else(|| parse_date(input, zone))
        .ok_or_else(|| "Not a Unix timestamp or a recognised date".to_string())?;

    Ok(vec![
        (
            "Unix (seconds)".to_string(),
            instant.timestamp().to_string(),
        ),
        (
            "Unix (milliseconds)".to_string(),
            instant.timestamp_millis().to_string(),
        ),
        ("UTC (ISO 8601)".to_string(), instant.to_rfc3339()),
        (format!("{zone_label} (ISO 8601)"), zone.rfc3339(instant)),
        (format!("{zone_label} (RFC 2822)"), zone.rfc2822(instant)),
        ("Relative".to_string(), relative(now, instant)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }

    #[test]
    fn unix_unit_is_guessed_from_magnitude() {
        let expected = at(1_700_000_000);
        assert_eq!(parse_unix("1700000000"), Some(expected));
        assert_eq!(parse_unix("1700000000000"), Some(expected));
        assert_eq!(parse_unix("1700000000000000"), Some(expected));
        assert_eq!(parse_unix("1700000000000000000"), Some(expected));
        assert_eq!(parse_unix("-86400"), Some(at(-86_400)));
        assert_eq!(
            parse_unix("1700000000.5")
                .unwrap()
                .timestamp_subsec_millis(),
            500
        );
        assert_eq!(parse_unix("abc"), None);
    }

    #[test]
    fn dates_are_read_in_the_selected_zone() {
        let berlin = Zone::from_index(2);
        // CET (UTC+1) in winter, CEST (UTC+2) in summer.
        assert_eq!(
            parse_date("2024-01-15 12:00:00", berlin),
            Some(at(1_705_316_400))
        );
        assert_eq!(
            parse_date("2024-07-15 12:00", berlin),
            Some(at(1_721_037_600))
        );
        // An explicit offset wins over the zone.
        assert_eq!(
            parse_date("2024-01-15T12:00:00+00:00", berlin),
            Some(at(1_705_320_000))
        );
        assert_eq!(parse_date("15.01.2024", Zone::Utc), Some(at(1_705_276_800)));
        assert_eq!(parse_date("nonsense", Zone::Utc), None);
    }

    #[test]
    fn relative_phrases() {
        let now = at(1_700_000_000);
        assert_eq!(relative(now, at(1_700_000_000)), "just now");
        assert_eq!(relative(now, at(1_700_000_000 - 3 * 3600)), "3 hours ago");
        assert_eq!(relative(now, at(1_700_000_000 + 86_400)), "in 1 day");
    }

    #[test]
    fn convert_produces_all_rows() {
        let rows = convert(
            "1700000000",
            Zone::from_index(2),
            "Europe/Berlin",
            at(1_700_000_000),
        )
        .unwrap();
        assert_eq!(rows[0].1, "1700000000");
        assert_eq!(rows[2].1, "2023-11-14T22:13:20+00:00");
        assert_eq!(rows[3].1, "2023-11-14T23:13:20+01:00");
        assert!(convert("x", Zone::Utc, "UTC", at(0)).is_err());
    }
}
