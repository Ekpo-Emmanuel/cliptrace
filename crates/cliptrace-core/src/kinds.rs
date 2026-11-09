//! Display kind of a clip, worked out from its text when it is read.
//!
//! The kind is never stored (the `kind` column only records `url` or `other`), so these
//! rules can change without a migration. Every rule is conservative: text that is only
//! similar to a type stays `text`, because a wrong badge is worse than no badge.

/// One of `link`, `email`, `color`, `path`, `phone`, `number`, `json`, `code`, `text`.
pub fn detect(text: &str) -> &'static str {
    let t = text.trim();
    if is_link(t) {
        "link"
    } else if is_email(t) {
        "email"
    } else if is_color(t) {
        "color"
    } else if is_path(t) {
        "path"
    } else if is_phone(t) {
        "phone"
    } else if is_number(t) {
        "number"
    } else if looks_like_json(t) {
        "json"
    } else if looks_like_code(text) {
        "code"
    } else {
        "text"
    }
}

fn is_single_line(t: &str) -> bool {
    !t.is_empty() && !t.contains('\n') && !t.contains('\r')
}

fn is_link(t: &str) -> bool {
    let lower = t.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn is_email(t: &str) -> bool {
    if !is_single_line(t) || t.len() > 254 || t.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = t.split_once('@') else {
        return false;
    };
    if local.is_empty() || local.len() > 64 || domain.contains('@') {
        return false;
    }
    let local_ok = local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._%+-".contains(c));
    let domain_ok = domain
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    if !local_ok || !domain_ok || domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    match domain.rsplit_once('.') {
        Some((host, tld)) => {
            !host.is_empty()
                && !host.contains("..")
                && tld.len() >= 2
                && tld.chars().all(|c| c.is_ascii_alphabetic())
        }
        None => false,
    }
}

fn is_color(t: &str) -> bool {
    if !is_single_line(t) || t.len() > 40 {
        return false;
    }
    if let Some(hex) = t.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    let lower = t.to_ascii_lowercase();
    let inner = ["rgba(", "rgb(", "hsla(", "hsl("]
        .iter()
        .find_map(|p| lower.strip_prefix(p))
        .and_then(|rest| rest.strip_suffix(')'));
    match inner {
        Some(args) => {
            !args.is_empty()
                && args.chars().any(|c| c.is_ascii_digit())
                && args
                    .chars()
                    .all(|c| c.is_ascii_digit() || " ,.%/".contains(c))
        }
        None => false,
    }
}

/// A Windows path: `C:\dir\file` or a UNC share `\\server\share`.
fn is_path(t: &str) -> bool {
    if !is_single_line(t) || t.len() > 1000 || t.contains(['<', '>', '"', '|', '?', '*']) {
        return false;
    }
    let b = t.as_bytes();
    let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/');
    let unc = t.starts_with("\\\\") && t.len() > 3 && !t[2..].starts_with('\\');
    drive || unc
}

fn is_ipv4(t: &str) -> bool {
    let parts: Vec<&str> = t.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()))
}

/// A phone number: an international `+` number (8 to 15 digits), or 10 to 15 digits written
/// with phone punctuation. Plain digits are a `number`, and dates and IPs are not phones.
fn is_phone(t: &str) -> bool {
    if !is_single_line(t) || t.len() > 30 || is_ipv4(t) {
        return false;
    }
    if !t.chars().all(|c| c.is_ascii_digit() || "+-(). ".contains(c)) {
        return false;
    }
    let digits = t.chars().filter(char::is_ascii_digit).count();
    let plus = t.starts_with('+');
    if t[1..].contains('+') {
        return false;
    }
    if plus {
        return (8..=15).contains(&digits);
    }
    (10..=15).contains(&digits) && t.contains(['-', '(', ')', '.'])
}

/// A single figure such as `42`, `-3.5`, `1,234.56`, `$99.00`, `12%` or `7421 0985 3367`.
fn is_number(t: &str) -> bool {
    if !is_single_line(t) || t.len() > 40 {
        return false;
    }
    let body = t.trim_start_matches(['+', '-', '$', '€', '£', '¥']);
    let body = body.strip_suffix('%').unwrap_or(body);
    if body.is_empty() || !body.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    if !body
        .chars()
        .all(|c| c.is_ascii_digit() || ",._ ".contains(c))
    {
        return false;
    }
    body.matches('.').count() <= 1 && body.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '.')
}

/// Shaped like JSON. The UI parses it for real and falls back to a code view if it fails.
fn looks_like_json(t: &str) -> bool {
    let object = t.starts_with('{') && t.ends_with('}') && (t.contains("\":") || t.contains("\" :"));
    let array = t.starts_with('[')
        && t.ends_with(']')
        && t.len() > 2
        && (t.contains(',') || t[1..].trim_start().starts_with(['{', '"']));
    object || array
}

fn looks_like_code(text: &str) -> bool {
    let lines = text.lines().count();
    let has_block = text.contains('{') && text.contains('}');
    let indented = text
        .lines()
        .filter(|l| l.starts_with("    ") || l.starts_with('\t'))
        .count();
    has_block || (lines >= 3 && indented >= 2)
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn all(kind: &str, samples: &[&str]) {
        for s in samples {
            assert_eq!(detect(s), kind, "expected {kind}: {s:?}");
        }
    }

    #[test]
    fn links() {
        all("link", &["https://docs.acme.dev/api", "HTTP://x.io", "  http://a.b  "]);
    }

    #[test]
    fn emails() {
        all("email", &["maya.lee@acme.dev", "a+tag@sub.example.co.uk", " b_c-d@x.io "]);
        all("text", &["maya@", "@acme.dev", "a@b", "two@@x.io", "me @ home.com", "hi maya.lee@acme.dev"]);
    }

    #[test]
    fn colors() {
        all("color", &["#ff8800", "#FFF", "#ff880080", "rgb(255, 136, 0)", "hsl(30 100% 50%)", "rgba(0,0,0,0.5)"]);
        all("text", &["#ff880", "#gggggg", "#", "rgb()", "rgb(a, b, c)", "color: #ff8800"]);
    }

    #[test]
    fn paths() {
        all("path", &["C:\\Users\\jtabo\\file.txt", "c:/dev/app", "\\\\nas\\share\\docs"]);
        all("text", &["C:", "C:file", "\\\\", "see C:\\x\\y\nand more", "C:\\a\\b?.txt"]);
    }

    #[test]
    fn phones() {
        all("phone", &["+1 (555) 123-4567", "(555) 123-4567", "555-123-4567", "+44 20 7946 0958", "555.123.4567"]);
        // Dates, IPs, short numbers and plain digits are not phones.
        assert_ne!(detect("2026-11-08"), "phone");
        assert_ne!(detect("192.168.100.200"), "phone");
        assert_ne!(detect("555-1234"), "phone");
        assert_ne!(detect("5551234567"), "phone");
    }

    #[test]
    fn numbers() {
        all("number", &["42", "-3.5", "1,234.56", "$99.00", "12%", "7421 0985 3367", "5551234567"]);
        all("text", &["1.2.3", "192.168.1.1", "$", "12abc", "1,2,3 apples"]);
    }

    #[test]
    fn json() {
        all("json", &["{\"a\": 1}", "{ \"name\": \"x\", \"tags\": [1, 2] }", "[1, 2, 3]", "[{\"id\": 1}]", "[\"a\"]"]);
        // Object literals without quoted keys are code, and empty or lone brackets are text.
        assert_eq!(detect("{ a: 1 }"), "code");
        assert_eq!(detect("[]"), "text");
        assert_eq!(detect("[x]"), "text");
    }

    #[test]
    fn code_and_text_keep_working() {
        all("code", &["fn main() { println!(\"hi\"); }", "a\n    b\n    c\nd"]);
        all("text", &["Hi Maya, thanks for the contract.", "Tracking number 7421 0985 3367", "221B Baker Street, London NW1 6XE"]);
    }
}
