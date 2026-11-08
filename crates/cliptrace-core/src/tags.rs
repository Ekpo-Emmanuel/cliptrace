pub const MAX_TAG_CHARS: usize = 32;
pub const MAX_TAGS_PER_CLIP: usize = 10;

/// Trim, collapse whitespace, cap at 32 chars. Empty after trim is None.
pub fn normalize_tag(raw: &str) -> Option<String> {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    let capped: String = collapsed.chars().take(MAX_TAG_CHARS).collect();
    let capped = capped.trim().to_string();
    if capped.is_empty() {
        None
    } else {
        Some(capped)
    }
}

/// Pull `#tag` tokens out of a query. Remaining text is for FTS/timeparse.
pub fn parse_hash_tags(query: &str) -> (String, Vec<String>) {
    let mut rest = Vec::new();
    let mut tags = Vec::new();
    for token in query.split_whitespace() {
        if let Some(raw) = token.strip_prefix('#') {
            if let Some(tag) = normalize_tag(raw) {
                tags.push(tag);
                continue;
            }
        }
        rest.push(token);
    }
    (rest.join(" "), tags)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowseFilter {
    pub pinned_only: bool,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_normalization() {
        assert_eq!(normalize_tag("  RBC   app  "), Some("RBC app".into()));
        assert_eq!(normalize_tag("   "), None);
        let long = "x".repeat(40);
        assert_eq!(normalize_tag(&long).unwrap().chars().count(), 32);
    }

    #[test]
    fn hash_tag_parsing() {
        let (text, tags) = parse_hash_tags("wifi #work login #rbc");
        assert_eq!(text, "wifi login");
        assert_eq!(tags, vec!["work".to_string(), "rbc".to_string()]);
        let (only, tags2) = parse_hash_tags("#rbc");
        assert_eq!(only, "");
        assert_eq!(tags2, vec!["rbc".to_string()]);
        let (none, tags3) = parse_hash_tags("no tags here");
        assert_eq!(none, "no tags here");
        assert!(tags3.is_empty());
    }
}
