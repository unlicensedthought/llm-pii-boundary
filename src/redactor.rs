//! The redactor: placeholders in, placeholders out.

use std::collections::{BTreeMap, HashMap};

use crate::detect::detect_all;
use crate::kind::Kind;
use crate::span::{select, Span};
use crate::text::*;

/// One of the caller's own identity terms. `"Acme Turbines = a wind-turbine maker"` carries a
/// generalisation, which is what the search query and the model's legend get instead of nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub text: String,
    pub general: Option<String>,
}

/// Raw strings → terms. Trimmed, case-insensitively de-duplicated, one-character terms dropped
/// (they would redact every "I" and "a").
pub fn parse_terms(raw: &[String]) -> Vec<Term> {
    let mut out: Vec<Term> = Vec::new();
    for r in raw {
        let (text, general) = match r.split_once(" = ") {
            Some((t, g)) => (t.trim(), Some(g.trim()).filter(|g| !g.is_empty())),
            None => (r.trim(), None),
        };
        if text.chars().count() < 2 {
            continue;
        }
        if out.iter().any(|t| t.text.eq_ignore_ascii_case(text)) {
            continue;
        }
        out.push(Term {
            text: text.to_string(),
            general: general.map(str::to_string),
        });
    }
    out
}

#[derive(Debug, Clone)]
pub(crate) struct Matcher {
    pub(crate) lower: String,
    kind: Kind,
    general: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) token: String,
    pub(crate) value: String,
    pub(crate) kind: Kind,
    pub(crate) general: Option<String>,
}

impl Entry {
    /// The words that may stand in for this placeholder in a SEARCH QUERY: a generalisation keeps the
    /// topic searchable ("wrongful termination at a wind-turbine maker"); a person, a number or an
    /// address is simply dropped.
    pub(crate) fn search_word(&self) -> Option<String> {
        match self.kind {
            Kind::Date | Kind::Url | Kind::Org | Kind::Place | Kind::Term => self.general.clone(),
            _ => None,
        }
    }

    pub(crate) fn describe(&self) -> String {
        match (self.kind, &self.general) {
            (Kind::Date, Some(y)) => format!("a date in {y}"),
            (Kind::Url, Some(h)) => format!("a personal link on {h}"),
            (Kind::Person | Kind::Org | Kind::Place | Kind::Id | Kind::Term, Some(g)) => g.clone(),
            _ => self.kind.noun().to_string(),
        }
    }
}

/// The per-turn map. Same value → same placeholder, everywhere in the turn; numbered by first
/// appearance, in the order texts are redacted.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    pub(crate) matchers: Vec<Matcher>,
    pub(crate) entries: Vec<Entry>,
    index: HashMap<String, usize>,
    next: BTreeMap<Kind, usize>,
    occurrences: usize,
}

/// A generalisation must not smuggle the value back in, nor a placeholder.
fn clean_general(g: Option<String>) -> Option<String> {
    g.map(|g| g.trim().chars().take(60).collect::<String>())
        .filter(|g| !g.is_empty() && token_spans(g).is_empty() && !g.contains('@'))
}

impl Redactor {
    pub fn new(terms: &[Term]) -> Self {
        let mut r = Redactor::default();
        for t in terms {
            r.add_matcher(&t.text, Kind::Term, t.general.clone());
        }
        r
    }

    /// A literal to redact wherever it appears (case-insensitive, whole words).
    pub fn add_matcher(&mut self, text: &str, kind: Kind, general: Option<String>) -> bool {
        let lower = text.trim().to_lowercase();
        if lower.chars().count() < 2 || self.matchers.iter().any(|m| m.lower == lower) {
            return false;
        }
        self.matchers.push(Matcher {
            lower,
            kind,
            general: clean_general(general),
        });
        // Longest first, so "Jane Roe" is matched before "Jane".
        self.matchers
            .sort_by_key(|m| std::cmp::Reverse(m.lower.chars().count()));
        true
    }

    fn term_spans(&self, text: &str) -> Vec<Span> {
        let mut out = Vec::new();
        if self.matchers.is_empty() {
            return out;
        }
        // ASCII lowercasing keeps byte offsets identical, so a match in `hay` is a match in `text`.
        // Non-ASCII letters must then match exactly — a documented limit, not a silent one.
        let hay = text.to_ascii_lowercase();
        for m in &self.matchers {
            let needle = m.lower.as_str();
            for (s, _) in hay.match_indices(needle) {
                let e = s + needle.len();
                if !text.is_char_boundary(s) || !text.is_char_boundary(e) {
                    continue;
                }
                let starts_alnum = needle.chars().next().is_some_and(char::is_alphanumeric);
                let ends_alnum = needle
                    .chars()
                    .next_back()
                    .is_some_and(char::is_alphanumeric);
                if (starts_alnum && is_word(prev_char(text, s)))
                    || (ends_alnum && is_word(next_char(text, e)))
                {
                    continue;
                }
                let mut sp = Span::new(s, e, m.kind);
                sp.general = m.general.clone();
                sp.term = true;
                out.push(sp);
            }
        }
        out
    }

    fn token_for(&mut self, value: &str, kind: Kind, general: Option<String>) -> String {
        let key = value.to_lowercase();
        if let Some(&i) = self.index.get(&key) {
            return self.entries[i].token.clone();
        }
        let n = self.next.entry(kind).or_insert(0);
        *n += 1;
        let token = format!("[{}_{}]", kind.tag(), n);
        self.entries.push(Entry {
            token: token.clone(),
            value: value.to_string(),
            kind,
            general: clean_general(general),
        });
        self.index.insert(key, self.entries.len() - 1);
        token
    }

    /// Replace every identifier in `text` with its placeholder, learning new ones as it goes.
    /// Idempotent: a placeholder is never matched again.
    pub fn redact(&mut self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        let mut spans = detect_all(text);
        spans.extend(self.term_spans(text));
        let chosen = select(spans, &token_spans(text));
        if chosen.is_empty() {
            return text.to_string();
        }
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for s in chosen {
            out.push_str(&text[last..s.start]);
            let token = self.token_for(&text[s.start..s.end], s.kind, s.general);
            out.push_str(&token);
            self.occurrences += 1;
            last = s.end;
        }
        out.push_str(&text[last..]);
        out
    }

    pub(crate) fn by_token(&self, token: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.token == token)
    }

    /// Put the values back. A placeholder this turn did not issue (a model inventing `[PERSON_9]`) is
    /// left as it is and counted, so the caller can say so.
    pub fn restore(&self, text: &str) -> (String, usize) {
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        let mut unknown = 0;
        for (s, e) in token_spans(text) {
            out.push_str(&text[last..s]);
            match self.by_token(&text[s..e]) {
                Some(en) => out.push_str(&en.value),
                None => {
                    unknown += 1;
                    out.push_str(&text[s..e]);
                }
            }
            last = e;
        }
        out.push_str(&text[last..]);
        (out, unknown)
    }

    /// Whether `text` carries any value this redactor protects: one of the caller's terms, a value it has
    /// replaced, or an identifier its patterns detect. Checked on every generalisation at the two
    /// points one leaves the process (the legend and the search-safe query), because a generalisation
    /// can come from the outside name detector, which may echo the value back ("Jane Roe's employer").
    pub(crate) fn reveals(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        !detect_all(text).is_empty()
            || self
                .matchers
                .iter()
                .any(|m| lower.contains(m.lower.as_str()))
            || self.entries.iter().any(|e| {
                let v = e.value.trim().to_lowercase();
                v.chars().count() >= 2 && lower.contains(v.as_str())
            })
    }

    /// Counts only — never a value.
    pub fn counts(&self) -> (usize, usize, BTreeMap<&'static str, usize>) {
        let mut kinds = BTreeMap::new();
        for e in &self.entries {
            *kinds.entry(e.kind.key()).or_insert(0) += 1;
        }
        (self.entries.len(), self.occurrences, kinds)
    }

    /// Every value this turn redacted, for the property tests and the outbound leak check.
    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.value.as_str())
    }

    /// The tokens issued, for checking a reply did not invent one.
    pub fn tokens(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.token.as_str())
    }
}
