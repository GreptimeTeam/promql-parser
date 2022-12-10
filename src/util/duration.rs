use lazy_static::lazy_static;
use regex::Regex;
use std::time::Duration;

/// 290 years
const MAX_DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 365 * 290);

lazy_static! {
    static ref DURATION_RE: Regex = Regex::new(
        r"^((?P<year>[0-9]+)y)?((?P<week>[0-9]+)w)?((?P<day>[0-9]+)d)?((?P<hour>[0-9]+)h)?((?P<minute>[0-9]+)m)?((?P<second>[0-9]+)s)?((?P<milli>[0-9]+)ms)?$",
    )
    .unwrap();
}

/// parses a string into a Duration, assuming that a year
/// always has 365d, a week always has 7d, and a day always has 24h.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::time::Duration;
/// use promql_parser::util;
///
/// assert_eq!(util::parse_duration("1h").unwrap(), Duration::from_secs(3600));
/// assert_eq!(util::parse_duration("4d").unwrap(), Duration::from_secs(3600 * 24 * 4));
/// assert_eq!(util::parse_duration("4d1h").unwrap(), Duration::from_secs(3600 * 97));
/// ```
pub fn parse_duration(ds: &str) -> Result<Duration, String> {
    if ds == "" {
        return Err("empty duration string".into());
    } else if ds == "0" {
        // Allow 0 without a unit.
        return Ok(Duration::ZERO);
    }
    if !DURATION_RE.is_match(ds) {
        return Err(format!("not a valid duration string: {}", ds));
    }

    let caps = DURATION_RE.captures(ds).unwrap();
    let mut result = Duration::ZERO;

    let mut checked_add = |title: &str, millis: u64| {
        if let Some(cap) = caps.name(title) {
            let v = cap.as_str().parse::<u64>().unwrap();
            result = result + Duration::from_millis(v * millis);
        };
    };

    checked_add("year", 1000 * 60 * 60 * 24 * 365); // y
    checked_add("week", 1000 * 60 * 60 * 24 * 7); // w
    checked_add("day", 1000 * 60 * 60 * 24); // d
    checked_add("hour", 1000 * 60 * 60); // h
    checked_add("minute", 1000 * 60); // m
    checked_add("second", 1000); // s
    checked_add("milli", 1); // ms

    if result.as_secs() > MAX_DURATION.as_secs() {
        return Err("duration out of range".into());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_re() {
        // valid regex
        let res = vec![
            "1y", "2w", "3d", "4h", "5m", "6s", "7ms", "1y2w3d", "4h30m", "3600ms",
        ];
        for re in res {
            assert!(DURATION_RE.is_match(re), "{} failed.", re)
        }

        // invalid regex
        let res = vec!["1", "1y1m1d", "-1w", "1.5d", "d"];
        for re in res {
            assert!(!DURATION_RE.is_match(re), "{} failed.", re)
        }
    }

    #[test]
    fn test_valid_duration() {
        let ds = vec![
            ("0", Duration::ZERO),
            ("0w", Duration::ZERO),
            ("0s", Duration::ZERO),
            ("324ms", Duration::from_millis(324)),
            ("3s", Duration::from_secs(3)),
            ("5m", Duration::from_secs(300)),
            ("1h", Duration::from_secs(3600)),
            ("4d", Duration::from_secs(3600 * 24 * 4)),
            ("4d1h", Duration::from_secs(3600 * 97)),
            ("14d", Duration::from_secs(3600 * 24 * 14)),
            ("3w", Duration::from_secs(3600 * 24 * 21)),
            ("3w2d1h", Duration::from_secs(3600 * (23 * 24 + 1))),
            ("10y", Duration::from_secs(3600 * 24 * 365 * 10)),
        ];

        for (s, expect) in ds {
            let d = parse_duration(s);
            assert!(d.is_ok());
            assert_eq!(
                expect.as_secs(),
                d.unwrap().as_secs(),
                "{} and {:?} not matched",
                s,
                expect
            );
        }
    }

    #[test]
    fn test_invalid_duration() {
        let ds = vec![
            "1",
            "1y1m1d",
            "-1w",
            "1.5d",
            "d",
            "294y",
            "200y10400w",
            "107675d",
            "2584200h",
            "",
        ];
        for d in ds {
            assert!(parse_duration(d).is_err(), "{} is invalid duration!", d);
        }
    }
}
