//! One detected identifier, and the rule for choosing between overlapping ones.

use crate::kind::Kind;

/// One detected identifier: a byte range of the text it was found in.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
    /// The generalisation that survives where the value cannot: a date's year, a link's host.
    pub general: Option<String>,
    /// Found by a literal term (the caller's own, or a detector's) rather than a pattern. Wins a tie.
    pub(crate) term: bool,
}

impl Span {
    pub(crate) fn new(start: usize, end: usize, kind: Kind) -> Self {
        Span {
            start,
            end,
            kind,
            general: None,
            term: false,
        }
    }
    pub(crate) fn with_general(mut self, g: impl Into<String>) -> Self {
        self.general = Some(g.into());
        self
    }
}

/// Longest first at each start; a literal term wins a tie; nothing inside an existing placeholder.
pub(crate) fn select(mut spans: Vec<Span>, protected: &[(usize, usize)]) -> Vec<Span> {
    spans.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then((b.end - b.start).cmp(&(a.end - a.start)))
            .then(b.term.cmp(&a.term))
    });
    let mut kept: Vec<Span> = Vec::new();
    for s in spans {
        if s.end <= s.start {
            continue;
        }
        let overlaps = |a: usize, b: usize| s.start < b && a < s.end;
        if protected.iter().any(|&(a, b)| overlaps(a, b)) {
            continue;
        }
        if kept.iter().any(|k| overlaps(k.start, k.end)) {
            continue;
        }
        kept.push(s);
    }
    kept.sort_by_key(|s| s.start);
    kept
}
