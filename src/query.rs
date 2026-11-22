//! Turning a redacted search query into one a search engine can use.

use crate::kind::Kind;
use crate::redactor::Redactor;
use crate::text::token_spans;

impl Redactor {
    /// A search query with its placeholders turned into something a search engine can use.
    ///
    /// THE CRITICAL PART OF THE BOUNDARY. A search engine given `[ORG_1] wrongful termination` searches
    /// for the literal brackets, and the turn's sources are about nothing. Identifiers go; the topic
    /// stays: a date becomes its year, a link its host, an organisation or place its generalisation
    /// when one is known, and everything else is dropped.
    pub fn search_safe(&self, q: &str) -> String {
        let mut out = String::new();
        let mut removed: Vec<Kind> = Vec::new();
        let mut last = 0;
        for (s, e) in token_spans(q) {
            out.push_str(&q[last..s]);
            match self
                .by_token(&q[s..e])
                .and_then(|en| en.search_word())
                .filter(|w| !self.reveals(w))
            {
                Some(w) => out.push_str(&w),
                None => {
                    if let Some(en) = self.by_token(&q[s..e]) {
                        removed.push(en.kind);
                    }
                }
            }
            last = e;
        }
        out.push_str(&q[last..]);
        let tidy = tidy_query(&out);
        if tidy.chars().any(char::is_alphanumeric) {
            return tidy;
        }
        // Nothing left but the identifiers: search for what KIND of thing it was, not for nothing.
        removed.dedup();
        removed
            .iter()
            .map(|k| k.noun().trim_start_matches("a ").trim_start_matches("an "))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Whitespace collapsed, empty brackets and orphaned possessives removed, stray punctuation trimmed.
fn tidy_query(q: &str) -> String {
    let mut s = q.replace("()", " ").replace("[]", " ").replace("\"\"", " ");
    // "[PERSON_1]'s claim" with the person dropped leaves "'s claim".
    for orphan in [" 's ", " ’s "] {
        s = s.replace(orphan, " ");
    }
    let s = s.trim_start_matches("'s ").trim_start_matches("’s ");
    let joined = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let joined = joined.replace(" ,", ",").replace(" .", ".");
    joined
        .trim_matches(|c: char| c.is_whitespace() || ",.;:-'’".contains(c))
        .to_string()
}

// =================================================================================================
// The boundary: one per turn
