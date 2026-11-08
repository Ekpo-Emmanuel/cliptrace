use thiserror::Error;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};

const ENTROPY_THRESHOLD: f64 = 4.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKind {
    Stripe,
    GitHub,
    Aws,
    Slack,
    Google,
    Jwt,
    PrivateKey,
    UrlPassword,
    HighEntropy,
}

impl SecretKind {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Stripe => "stripe",
            Self::GitHub => "github",
            Self::Aws => "aws",
            Self::Slack => "slack",
            Self::Google => "google",
            Self::Jwt => "jwt",
            Self::PrivateKey => "private-key",
            Self::UrlPassword => "url-password",
            Self::HighEntropy => "token",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Stripe => "stripe key",
            Self::GitHub => "github token",
            Self::Aws => "aws access key",
            Self::Slack => "slack token",
            Self::Google => "google api key",
            Self::Jwt => "jwt",
            Self::PrivateKey => "private key",
            Self::UrlPassword => "url password",
            Self::HighEntropy => "secret token",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "stripe" => Some(Self::Stripe),
            "github" => Some(Self::GitHub),
            "aws" => Some(Self::Aws),
            "slack" => Some(Self::Slack),
            "google" => Some(Self::Google),
            "jwt" => Some(Self::Jwt),
            "private-key" => Some(Self::PrivateKey),
            "url-password" => Some(Self::UrlPassword),
            "token" => Some(Self::HighEntropy),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("dpapi protect failed")]
    Protect,
    #[error("dpapi unprotect failed")]
    Unprotect,
}

/// First 4 characters of the secret, for masked display only.
pub fn secret_hint(plain: &str) -> String {
    plain.chars().take(4).collect()
}

pub fn mask_secret(kind: SecretKind, hint: &str) -> String {
    let prefix: String = hint.chars().take(4).collect();
    format!("[{} {prefix}…]", kind.label())
}

pub fn secret_search_text(kind: SecretKind, source_exe: &str, window_title: &str) -> String {
    let title = redact_secrets(window_title);
    format!(
        "{} copied from {source_exe}, \"{title}\"",
        kind.label()
    )
}

pub fn detect(text: &str) -> Option<SecretKind> {
    detect_span(text).map(|(kind, _, _)| kind)
}

pub fn detect_span(text: &str) -> Option<(SecretKind, usize, usize)> {
    if let Some(span) = find_prefix(text, "sk_live_", SecretKind::Stripe) {
        return Some(span);
    }
    if let Some(span) = find_prefix(text, "sk_test_", SecretKind::Stripe) {
        return Some(span);
    }
    if let Some(span) = find_prefix(text, "rk_live_", SecretKind::Stripe) {
        return Some(span);
    }
    if let Some(span) = find_prefix(text, "ghp_", SecretKind::GitHub) {
        return Some(span);
    }
    if let Some(span) = find_prefix(text, "github_pat_", SecretKind::GitHub) {
        return Some(span);
    }
    if let Some(span) = find_akia(text) {
        return Some(span);
    }
    if let Some(span) = find_xox(text) {
        return Some(span);
    }
    if let Some(span) = find_prefix(text, "AIza", SecretKind::Google) {
        return Some(span);
    }
    if text.contains("-----BEGIN") && text.contains("PRIVATE KEY-----") {
        return Some((SecretKind::PrivateKey, 0, text.len()));
    }
    if let Some(span) = find_url_password(text) {
        return Some(span);
    }
    if let Some(span) = find_jwt(text) {
        return Some(span);
    }
    if is_high_entropy_token(text) {
        return Some((SecretKind::HighEntropy, 0, text.len()));
    }
    None
}

pub fn redact_secrets(text: &str) -> String {
    let mut spans = Vec::new();
    let mut idx = 0;
    while idx < text.len() {
        match detect_span(&text[idx..]) {
            Some((_, start, end)) if end > start => {
                spans.push((idx + start, idx + end));
                idx += end;
            }
            _ => break,
        }
    }
    if spans.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut last = 0;
    for (start, end) in spans {
        out.push_str(&text[last..start]);
        out.push_str("***");
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

fn find_prefix(text: &str, prefix: &str, kind: SecretKind) -> Option<(SecretKind, usize, usize)> {
    let start = text.find(prefix)?;
    let rest = &text[start + prefix.len()..];
    let extra = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .count();
    if extra < 4 {
        return None;
    }
    let end = start + prefix.len() + extra;
    Some((kind, start, end))
}

fn find_akia(text: &str) -> Option<(SecretKind, usize, usize)> {
    let start = text.find("AKIA")?;
    let rest = &text[start + 4..];
    let extra: String = rest.chars().take(16).collect();
    if extra.len() != 16 || !extra.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some((SecretKind::Aws, start, start + 20))
}

fn find_xox(text: &str) -> Option<(SecretKind, usize, usize)> {
    for prefix in ["xoxb-", "xoxp-"] {
        if let Some(start) = text.find(prefix) {
            let rest = &text[start + prefix.len()..];
            let extra = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .count();
            if extra >= 8 {
                return Some((SecretKind::Slack, start, start + prefix.len() + extra));
            }
        }
    }
    None
}

fn find_url_password(text: &str) -> Option<(SecretKind, usize, usize)> {
    let scheme = text.find("://")?;
    let after = scheme + 3;
    let rest = &text[after..];
    let at = rest.find('@')?;
    let creds = &rest[..at];
    if let Some(colon) = creds.find(':') {
        if colon > 0 && colon + 1 < creds.len() && !creds[colon + 1..].is_empty() {
            return Some((SecretKind::UrlPassword, after, after + at));
        }
    }
    None
}

fn find_jwt(text: &str) -> Option<(SecretKind, usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_b64url(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_b64url(bytes[i]) {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                let mid = i + 1;
                i = mid;
                while i < bytes.len() && is_b64url(bytes[i]) {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'.' {
                    let last = i + 1;
                    i = last;
                    while i < bytes.len() && is_b64url(bytes[i]) {
                        i += 1;
                    }
                    let a = mid - 1 - start;
                    let b = last - 1 - mid;
                    let c = i - last;
                    if a >= 8 && b >= 8 && c >= 8 {
                        return Some((SecretKind::Jwt, start, i));
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

fn is_b64url(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

fn is_high_entropy_token(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.chars().any(char::is_whitespace) || trimmed.chars().count() < 20 {
        return false;
    }
    if looks_like_url(trimmed) || looks_like_path(trimmed) || looks_like_uuid(trimmed) || looks_like_hex_hash(trimmed)
    {
        return false;
    }
    if class_count(trimmed) < 3 {
        return false;
    }
    shannon(trimmed) >= ENTROPY_THRESHOLD
}

fn looks_like_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.contains("://")
}

fn looks_like_path(s: &str) -> bool {
    let bytes = s.as_bytes();
    (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/'))
        || s.starts_with('\\')
        || s.starts_with('/')
}

fn looks_like_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 36 {
        return false;
    }
    let dash = |i| b[i] == b'-';
    dash(8)
        && dash(13)
        && dash(18)
        && dash(23)
        && b.iter().enumerate().all(|(i, c)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                true
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

fn looks_like_hex_hash(s: &str) -> bool {
    let n = s.len();
    (n == 32 || n == 40 || n == 64) && s.bytes().all(|c| c.is_ascii_hexdigit())
}

fn class_count(s: &str) -> u32 {
    let mut lower = false;
    let mut upper = false;
    let mut digit = false;
    let mut other = false;
    for c in s.chars() {
        if c.is_ascii_lowercase() {
            lower = true;
        } else if c.is_ascii_uppercase() {
            upper = true;
        } else if c.is_ascii_digit() {
            digit = true;
        } else {
            other = true;
        }
    }
    u32::from(lower) + u32::from(upper) + u32::from(digit) + u32::from(other)
}

fn shannon(s: &str) -> f64 {
    let mut freq = [0u32; 256];
    let bytes = s.as_bytes();
    for b in bytes {
        freq[*b as usize] += 1;
    }
    let n = bytes.len() as f64;
    freq.iter()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = f64::from(*c) / n;
            -p * p.log2()
        })
        .sum()
}

pub fn dpapi_protect(plain: &[u8]) -> Result<Vec<u8>, SecretError> {
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // Safety: input buffer lives for the call; output is LocalAlloc'd by DPAPI.
    let ok = unsafe {
        CryptProtectData(
            &input,
            windows::core::PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok.is_err() {
        return Err(SecretError::Protect);
    }
    let copy = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    // Safety: pbData came from CryptProtectData LocalAlloc.
    unsafe {
        let _ = LocalFree(HLOCAL(output.pbData.cast()));
    }
    let _ = &mut input;
    Ok(copy)
}

pub fn dpapi_unprotect(blob: &[u8]) -> Result<Vec<u8>, SecretError> {
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: blob.len() as u32,
        pbData: blob.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // Safety: input buffer lives for the call; output is LocalAlloc'd by DPAPI.
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok.is_err() {
        return Err(SecretError::Unprotect);
    }
    let copy = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        let _ = LocalFree(HLOCAL(output.pbData.cast()));
    }
    let _ = &mut input;
    Ok(copy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_table_positive_and_negative() {
        let positives = [
            ("sk_test_4eC39HqLyjWDarjtT1zdp7dc", SecretKind::Stripe),
            ("sk_live_abcdefghijklmnop", SecretKind::Stripe),
            ("rk_live_abcdefghijklmnop", SecretKind::Stripe),
            ("ghp_abcdefghijklmnopqrstuvwx", SecretKind::GitHub),
            ("github_pat_abcdefghijklmnopqr", SecretKind::GitHub),
            ("AKIAIOSFODNN7EXAMPLE", SecretKind::Aws),
            ("xoxb-123456789012-ab", SecretKind::Slack),
            ("AIzaSyDummyGoogleApiKey000001", SecretKind::Google),
            (
                "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
                SecretKind::Jwt,
            ),
            (
                "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAK8=\n-----END RSA PRIVATE KEY-----",
                SecretKind::PrivateKey,
            ),
            ("postgres://user:s3cretpass@localhost/db", SecretKind::UrlPassword),
            ("Please rotate sk_test_abc123xyz999 before share", SecretKind::Stripe),
        ];
        for (sample, kind) in positives {
            assert_eq!(detect(sample), Some(kind), "positive: {sample}");
        }

        let negatives = [
            "https://example.com/very/long/path/to/resource?query=1&other=2",
            r"C:\Users\foo\projects\cliptrace\src\main.rs",
            "0123456789abcdef0123456789abcdef01234567",
            "550e8400-e29b-41d4-a716-446655440000",
            "The quick brown fox jumps over the lazy dog today",
            "pay with stripe once on the dashboard",
            "abcdefghijklmnopqrstuvwxyz",
        ];
        for sample in negatives {
            assert_eq!(detect(sample), None, "negative: {sample}");
        }

        let token = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEX";
        assert_eq!(detect(token), Some(SecretKind::HighEntropy));
    }

    #[test]
    fn dpapi_roundtrip() {
        let plain = b"sk_test_roundtrip_value";
        let enc = dpapi_protect(plain).expect("protect");
        assert_ne!(enc, plain);
        let back = dpapi_unprotect(&enc).expect("unprotect");
        assert_eq!(back, plain);
    }
}
