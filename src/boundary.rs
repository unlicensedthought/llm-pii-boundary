//! The boundary for one turn: every outbound string goes through [`Boundary::redact`] or
//! [`Boundary::apply_json`], and the one reply that comes back goes through [`Boundary::restore`].

use std::cell::RefCell;
use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::kind::Kind;
use crate::legend::{strip_legend, LEGEND_HEAD};
use crate::redactor::{Redactor, Term};
use crate::text::token_spans;

/// A turn's PII boundary. Pass it by reference to every outbound call of the turn.
///
/// It is an explicit value, not a global, so two turns running side by side can't read each other's
/// placeholder maps. `Boundary::off()` changes nothing, so call sites take one unconditionally and
/// the on/off decision lives in one place.
#[derive(Debug)]
pub struct Boundary {
    on: bool,
    r: RefCell<Redactor>,
    terms: usize,
    capture: bool,
    captured: RefCell<Vec<String>>,
}

/// A restored reply and how many placeholders in it this turn never issued.
#[derive(Debug, Clone, PartialEq)]
pub struct Restored {
    pub text: String,
    pub unknown: usize,
}

impl Boundary {
    pub fn off() -> Self {
        Boundary {
            on: false,
            r: RefCell::new(Redactor::default()),
            terms: 0,
            capture: false,
            captured: RefCell::new(Vec::new()),
        }
    }

    pub fn on(terms: &[Term]) -> Self {
        Boundary {
            on: true,
            r: RefCell::new(Redactor::new(terms)),
            terms: terms.len(),
            ..Boundary::off()
        }
    }

    /// Record every string [`Boundary::apply_json`] sends. Works with the boundary off too, so an
    /// eval can see the unredacted outbound of a boundary-off run.
    pub fn capturing(mut self) -> Self {
        self.capture = true;
        self
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// Every string the guarded calls sent, in order.
    pub fn captured(&self) -> Vec<String> {
        self.captured.borrow().clone()
    }

    /// Teach the boundary a literal it must hide wherever it appears: a name an outside detector
    /// found, say. Returns false when it was already known or too short to be safe.
    pub fn add_literal(&self, text: &str, kind: Kind, general: Option<String>) -> bool {
        self.on && self.r.borrow_mut().add_matcher(text, kind, general)
    }

    pub fn redact(&self, s: &str) -> String {
        if self.on {
            self.r.borrow_mut().redact(s)
        } else {
            s.to_string()
        }
    }

    /// The outbound hook: redact every string leaf of a JSON request body, give a chat-completions
    /// body's last user message the legend for the placeholders it carries, and capture what is sent.
    pub fn apply_json(&self, v: &mut Value) {
        if self.on {
            let mut r = self.r.borrow_mut();
            walk_strings(v, &mut |s| {
                let red = r.redact(s);
                *s = red;
            });
            let mut present = BTreeSet::new();
            walk_strings(v, &mut |s| {
                for (a, b) in token_spans(s) {
                    present.insert(s[a..b].to_string());
                }
            });
            let legend = r.legend_for(&present);
            if !legend.is_empty() {
                if let Some(last) = v
                    .get_mut("messages")
                    .and_then(|m| m.as_array_mut())
                    .and_then(|m| m.iter_mut().rev().find(|m| m["role"] == "user"))
                {
                    if let Some(c) = last["content"].as_str() {
                        if !c.contains(LEGEND_HEAD) {
                            last["content"] = json!(format!("{c}\n\n{legend}"));
                        }
                    }
                }
            }
        }
        if self.capture {
            let mut out = self.captured.borrow_mut();
            walk_strings(v, &mut |s| out.push(s.clone()));
        }
    }

    /// Put the values back, once, on the way out. A copied legend is stripped first. A placeholder
    /// this turn did not issue is left in place and counted.
    pub fn restore_counted(&self, s: &str) -> Restored {
        if !self.on {
            return Restored {
                text: s.to_string(),
                unknown: 0,
            };
        }
        let (text, unknown) = self.r.borrow().restore(&strip_legend(s));
        Restored { text, unknown }
    }

    pub fn restore(&self, s: &str) -> String {
        self.restore_counted(s).text
    }

    /// A query for a search engine: redacted, then its placeholders turned into searchable words
    /// (a year, a host, a generalisation) or dropped.
    pub fn search_query(&self, q: &str) -> String {
        if !self.on {
            return q.to_string();
        }
        let red = self.r.borrow_mut().redact(q);
        self.r.borrow().search_safe(&red)
    }

    /// The placeholders this turn has issued: the only ones a reply may use.
    pub fn issued_tokens(&self) -> Vec<String> {
        self.r.borrow().tokens().map(str::to_string).collect()
    }

    /// Every value this boundary has redacted (tests and leak checks only).
    pub fn values(&self) -> Vec<String> {
        self.r.borrow().values().map(str::to_string).collect()
    }

    /// Counts by kind, never the values. Safe to log.
    pub fn meta(&self) -> Value {
        if !self.on {
            return json!({ "on": false });
        }
        let (distinct, occurrences, kinds) = self.r.borrow().counts();
        json!({
            "on": true,
            "redacted_count": distinct,
            "occurrences": occurrences,
            "kinds": kinds,
            "terms": self.terms,
        })
    }
}

fn walk_strings(v: &mut Value, f: &mut dyn FnMut(&mut String)) {
    match v {
        Value::String(s) => f(s),
        Value::Array(a) => a.iter_mut().for_each(|x| walk_strings(x, f)),
        Value::Object(o) => o.values_mut().for_each(|x| walk_strings(x, f)),
        _ => {}
    }
}
