use crate::kind::Kind;
use crate::span::Span;
use crate::text::*;

pub(crate) const PUBLIC_CODE_PREFIXES: &[&str] = &["PMC", "NCT"];

/// Codes with a letter prefix and a long number — case numbers, member ids: "E9900012345".
pub(crate) fn alnum_ids(text: &str, out: &mut Vec<Span>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let ok = |c: u8| c.is_ascii_alphanumeric() || c == b'-';
        if !b[i].is_ascii_alphanumeric() || is_word(prev_char(text, i)) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < b.len() && ok(b[j]) {
            j += 1;
        }
        while j > i && b[j - 1] == b'-' {
            j -= 1;
        }
        if is_word(next_char(text, j)) {
            i = j.max(i + 1);
            continue;
        }
        let w = &text[i..j];
        let digits = w.bytes().filter(u8::is_ascii_digit).count();
        let letters = w.bytes().filter(u8::is_ascii_alphabetic).count();
        let prefix = w.bytes().take_while(u8::is_ascii_uppercase).count();
        let coded = (1..=3).contains(&prefix)
            && digits >= 7
            && w[prefix..].bytes().all(|c| c.is_ascii_digit() || c == b'-')
            // Public registry ids are citations, not identifiers of a person: PubMed Central
            // ("PMC4347870", once redacted out of a source by mistake) and
            // ClinicalTrials.gov ("NCT01234567").
            && !PUBLIC_CODE_PREFIXES.contains(&&w[..prefix]);
        if (6..=24).contains(&w.len())
            && letters >= 1
            && digits >= 5
            && (coded || keyword_before(text, i, ID_WORDS))
        {
            out.push(Span::new(i, j, Kind::Id));
        }
        i = j.max(i + 1);
    }
}
