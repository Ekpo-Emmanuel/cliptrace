use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Weekday};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedQuery {
    pub text: String,
    pub range: Option<(i64, i64)>,
}

/// Parse a time phrase from `query`. Uses `now`; never reads the clock.
pub fn parse_time_phrase(query: &str, now: DateTime<Local>) -> ParsedQuery {
    let lower = query.to_ascii_lowercase();
    if let Some((start, end, span)) = match_phrase(&lower, now) {
        let stripped = drop_from_on(&strip_span(query, span));
        return ParsedQuery {
            text: stripped,
            range: Some((to_ms(start), to_ms(end) - 1)),
        };
    }
    ParsedQuery {
        text: collapse_ws(query),
        range: None,
    }
}

fn match_phrase(lower: &str, now: DateTime<Local>) -> Option<(DateTime<Local>, DateTime<Local>, (usize, usize))> {
    let day = start_of_day(now);
    let tomorrow = day + Duration::days(1);

    let candidates: Vec<(&str, (DateTime<Local>, DateTime<Local>))> = vec![
        ("this morning", (day, day + Duration::hours(12))),
        ("last month", {
            let this_month = day.with_day(1).unwrap_or(day);
            let last_month = prev_month_start(this_month);
            (last_month, this_month)
        }),
        ("last week", {
            let this_week = start_of_week_sunday(day);
            (this_week - Duration::days(7), this_week)
        }),
        ("this week", {
            let this_week = start_of_week_sunday(day);
            (this_week, this_week + Duration::days(7))
        }),
        ("last night", (day - Duration::hours(6), day + Duration::hours(6))),
        ("tonight", (day + Duration::hours(18), tomorrow)),
        ("yesterday", (day - Duration::days(1), day)),
        ("today", (day, tomorrow)),
    ];

    for (phrase, range) in &candidates {
        if let Some(span) = find_words(lower, phrase) {
            return Some((range.0, range.1, span));
        }
    }

    if let Some((n, unit, span)) = find_ago(lower) {
        let (start, end) = match unit {
            AgoUnit::Minutes => (now - Duration::minutes(n), now),
            AgoUnit::Hours => (now - Duration::hours(n), now),
            AgoUnit::Days => {
                let d = start_of_day(now - Duration::days(n));
                (d, d + Duration::days(1))
            }
        };
        return Some((start, end, span));
    }

    for (prefix, weekday) in weekday_prefixed(lower) {
        if let Some(span) = find_words(lower, &prefix) {
            let (start, end) = weekday_range(now, weekday);
            return Some((start, end, span));
        }
    }

    for (name, weekday) in WEEKDAYS {
        if let Some(span) = find_words(lower, name) {
            let (start, end) = weekday_range(now, *weekday);
            return Some((start, end, span));
        }
    }

    None
}

const WEEKDAYS: &[(&str, Weekday)] = &[
    ("monday", Weekday::Mon),
    ("tuesday", Weekday::Tue),
    ("wednesday", Weekday::Wed),
    ("thursday", Weekday::Thu),
    ("friday", Weekday::Fri),
    ("saturday", Weekday::Sat),
    ("sunday", Weekday::Sun),
];

fn weekday_prefixed(_lower: &str) -> Vec<(String, Weekday)> {
    let mut out = Vec::new();
    for (name, day) in WEEKDAYS {
        out.push((format!("last {name}"), *day));
        out.push((format!("from {name}"), *day));
        out.push((format!("on {name}"), *day));
    }
    out
}

/// Most recent past occurrence. If `now` is that weekday, use today.
fn weekday_range(now: DateTime<Local>, weekday: Weekday) -> (DateTime<Local>, DateTime<Local>) {
    let today = start_of_day(now);
    let mut d = today;
    for _ in 0..7 {
        if d.weekday() == weekday {
            return (d, d + Duration::days(1));
        }
        d -= Duration::days(1);
    }
    (today, today + Duration::days(1))
}

enum AgoUnit {
    Minutes,
    Hours,
    Days,
}

fn find_ago(lower: &str) -> Option<(i64, AgoUnit, (usize, usize))> {
    let bytes = lower.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let n: i64 = lower[start..i].parse().ok()?;
            let rest = lower[i..].trim_start();
            let gap = lower[i..].len() - rest.len();
            let (unit, word_len) = if rest.starts_with("minutes ago") || rest.starts_with("minute ago") {
                let w = if rest.starts_with("minutes") { "minutes ago".len() } else { "minute ago".len() };
                (AgoUnit::Minutes, w)
            } else if rest.starts_with("hours ago") || rest.starts_with("hour ago") {
                let w = if rest.starts_with("hours") { "hours ago".len() } else { "hour ago".len() };
                (AgoUnit::Hours, w)
            } else if rest.starts_with("days ago") || rest.starts_with("day ago") {
                let w = if rest.starts_with("days") { "days ago".len() } else { "day ago".len() };
                (AgoUnit::Days, w)
            } else {
                i = start + 1;
                continue;
            };
            let end = i + gap + word_len;
            return Some((n, unit, (start, end)));
        }
        i += 1;
    }
    None
}

fn find_words(hay: &str, needle: &str) -> Option<(usize, usize)> {
    let mut start = 0;
    while start + needle.len() <= hay.len() {
        if hay[start..].starts_with(needle) {
            let end = start + needle.len();
            let before_ok = start == 0 || !hay.as_bytes()[start - 1].is_ascii_alphanumeric();
            let after_ok = end == hay.len() || !hay.as_bytes()[end].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return Some((start, end));
            }
        }
        start += hay[start..].chars().next()?.len_utf8();
    }
    None
}

fn strip_span(query: &str, (start, end): (usize, usize)) -> String {
    let mut s = String::new();
    s.push_str(&query[..start]);
    s.push(' ');
    s.push_str(&query[end..]);
    collapse_ws(&s)
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn drop_from_on(s: &str) -> String {
    let mut tokens: Vec<&str> = s.split_whitespace().collect();
    while tokens
        .last()
        .is_some_and(|t| t.eq_ignore_ascii_case("from") || t.eq_ignore_ascii_case("on"))
    {
        tokens.pop();
    }
    while tokens
        .first()
        .is_some_and(|t| t.eq_ignore_ascii_case("from") || t.eq_ignore_ascii_case("on"))
    {
        tokens.remove(0);
    }
    tokens.join(" ")
}

fn start_of_day(now: DateTime<Local>) -> DateTime<Local> {
    let naive = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    now.timezone()
        .from_local_datetime(&naive)
        .single()
        .unwrap_or(now)
}

fn start_of_week_sunday(day: DateTime<Local>) -> DateTime<Local> {
    let back = day.weekday().num_days_from_sunday() as i64;
    day - Duration::days(back)
}

fn prev_month_start(this_month: DateTime<Local>) -> DateTime<Local> {
    let d = this_month.date_naive();
    let (y, m) = if d.month() == 1 {
        (d.year() - 1, 12)
    } else {
        (d.year(), d.month() - 1)
    };
    let naive = chrono::NaiveDate::from_ymd_opt(y, m, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    this_month
        .timezone()
        .from_local_datetime(&naive)
        .single()
        .unwrap_or(this_month)
}

fn to_ms(dt: DateTime<Local>) -> i64 {
    dt.timestamp_millis()
}

pub fn local_from_unix_ms(ms: i64) -> DateTime<Local> {
    let utc = DateTime::from_timestamp_millis(ms)
        .or_else(|| DateTime::from_timestamp(0, 0))
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH);
    utc.with_timezone(&Local)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sat_afternoon() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 7, 15, 30, 0).unwrap()
    }

    fn day_ms(y: i32, m: u32, d: u32) -> (i64, i64) {
        let start = Local.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap();
        let end = start + Duration::days(1);
        (to_ms(start), to_ms(end) - 1)
    }

    #[test]
    fn saturday_now_resolves_weekdays() {
        let now = sat_afternoon();
        let tue = parse_time_phrase("stripe from tuesday", now);
        assert_eq!(tue.text, "stripe");
        assert_eq!(tue.range, Some(day_ms(2026, 11, 3)));

        let yest = parse_time_phrase("yesterday", now);
        assert_eq!(yest.text, "");
        assert_eq!(yest.range, Some(day_ms(2026, 11, 6)));

        let sat = parse_time_phrase("saturday", now);
        assert_eq!(sat.range, Some(day_ms(2026, 11, 7)));

        let last_tue = parse_time_phrase("last tuesday", now);
        assert_eq!(last_tue.range, Some(day_ms(2026, 11, 3)));

        let today = parse_time_phrase("that stripe key thing from today", now);
        assert_eq!(today.text, "that stripe key thing");
        assert_eq!(today.range, Some(day_ms(2026, 11, 7)));
    }

    #[test]
    fn relative_ago_and_no_match() {
        let now = sat_afternoon();
        let ago = parse_time_phrase("wifi 3 hours ago", now);
        assert_eq!(ago.text, "wifi");
        let (start, end) = ago.range.expect("range");
        assert_eq!(end, to_ms(now) - 1);
        assert_eq!(start, to_ms(now - Duration::hours(3)));

        let none = parse_time_phrase("stripe key", now);
        assert_eq!(none.text, "stripe key");
        assert_eq!(none.range, None);
    }
}
