/// Turn a user query into an FTS5 MATCH expression.
/// Each whitespace token is double-quoted so `-`, `*`, `:`, and `"` cannot be operators.
pub fn escape_fts_query(query: &str) -> Option<String> {
    let mut parts = Vec::new();
    for token in query.split_whitespace() {
        if token.is_empty() {
            continue;
        }
        let escaped = token.replace('"', "\"\"");
        parts.push(format!("\"{escaped}\""));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Format a past timestamp relative to `now_ms`. Never reads the clock.
pub fn relative_time(then_ms: i64, now_ms: i64) -> String {
    let delta_ms = (now_ms - then_ms).max(0);
    let secs = delta_ms / 1000;
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_quotes_tokens_and_specials() {
        assert_eq!(escape_fts_query("stripe"), Some(r#""stripe""#.to_string()));
        assert_eq!(
            escape_fts_query("stripe key"),
            Some(r#""stripe" "key""#.to_string())
        );
        assert_eq!(
            escape_fts_query("foo -bar"),
            Some(r#""foo" "-bar""#.to_string())
        );
        assert_eq!(escape_fts_query("a:b"), Some(r#""a:b""#.to_string()));
        assert_eq!(escape_fts_query("*"), Some(r#""*""#.to_string()));
        assert_eq!(
            escape_fts_query(r#"say "hi""#),
            Some("\"say\" \"\"\"hi\"\"\"".to_string())
        );
        assert_eq!(escape_fts_query("   \t  "), None);
    }

    #[test]
    fn relative_time_uses_injected_now() {
        let now = 10_000_000;
        assert_eq!(relative_time(now - 5_000, now), "5s ago");
        assert_eq!(relative_time(now - 2 * 60 * 1000, now), "2m ago");
        assert_eq!(relative_time(now - 2 * 3600 * 1000, now), "2h ago");
        assert_eq!(relative_time(now - 3 * 86400 * 1000, now), "3d ago");
        assert_eq!(relative_time(now + 1000, now), "0s ago");
    }
}
