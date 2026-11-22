//! Small text helpers shared by every detector.

pub(crate) fn prev_char(text: &str, i: usize) -> Option<char> {
    text[..i].chars().next_back()
}

pub(crate) fn next_char(text: &str, i: usize) -> Option<char> {
    text[i..].chars().next()
}

pub(crate) fn is_word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// Placeholders already in a text, as byte ranges: `[`, capitals, `_`, digits, `]`. A redaction pass
/// never matches inside one, which is what makes redaction idempotent.
pub fn token_spans(text: &str) -> Vec<(usize, usize)> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'[' {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_uppercase() {
                j += 1;
            }
            if j > i + 1 && j < b.len() && b[j] == b'_' {
                let mut k = j + 1;
                while k < b.len() && b[k].is_ascii_digit() {
                    k += 1;
                }
                if k > j + 1 && k < b.len() && b[k] == b']' {
                    out.push((i, k + 1));
                    i = k + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// The lowercase words in the few characters before `start` — what makes "account 12345678" an
/// identifier and "population 12345678" a number.
pub(crate) fn words_before(text: &str, start: usize, chars: usize) -> Vec<String> {
    let before: String = {
        let head = &text[..start];
        let n = head.chars().count();
        head.chars().skip(n.saturating_sub(chars)).collect()
    };
    before
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '#')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn keyword_before(text: &str, start: usize, words: &[&str]) -> bool {
    let near = words_before(text, start, 28);
    near.iter().any(|w| words.contains(&w.as_str()))
        || (words.contains(&"#") && text[..start].trim_end().ends_with('#'))
}

pub(crate) const PHONE_WORDS: &[&str] = &[
    "phone",
    "tel",
    "telephone",
    "call",
    "cell",
    "mobile",
    "fax",
    "text",
    "whatsapp",
    "sms",
];
pub(crate) const ID_WORDS: &[&str] = &[
    "account",
    "acct",
    "policy",
    "member",
    "claim",
    "case",
    "ticket",
    "reference",
    "ref",
    "order",
    "invoice",
    "employee",
    "badge",
    "licence",
    "license",
    "passport",
    "routing",
    "id",
    "mrn",
    "patient",
    "plate",
    "vin",
    "serial",
    "#",
];
pub(crate) const SSN_WORDS: &[&str] = &["ssn", "social", "sin", "nino", "tin", "itin"];
