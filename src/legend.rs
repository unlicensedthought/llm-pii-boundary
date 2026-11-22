//! The note the model reads about placeholders, and its removal from a reply.

use std::collections::BTreeSet;

use crate::redactor::{Entry, Redactor};

impl Redactor {
    /// The note the model reads: what the placeholders are, and how to treat them. Every entry.
    pub fn legend(&self) -> String {
        let all: BTreeSet<String> = self.entries.iter().map(|e| e.token.clone()).collect();
        self.legend_for(&all)
    }

    /// The legend for the placeholders in `present` only — what one outbound body carries.
    pub fn legend_for(&self, present: &BTreeSet<String>) -> String {
        let shown: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|e| present.contains(&e.token))
            .collect();
        if shown.is_empty() {
            return String::new();
        }
        let mut s = format!(
            "{LEGEND_HEAD} square-bracket placeholders such as [PERSON_1] stand for personal \
             details removed before this text left the user's own server. Copy a placeholder \
             exactly when you need to refer to it; never guess, expand, alter or invent one."
        );
        let parts: Vec<String> = shown
            .iter()
            .take(40)
            .map(|e| {
                // A generalisation that names a value it stands for (or any other known value) is
                // not a generalisation: the plain noun goes out instead.
                let d = match e.general.as_deref() {
                    Some(g) if self.reveals(g) => e.kind.noun().to_string(),
                    _ => e.describe(),
                };
                format!("{} is {d}", e.token)
            })
            .collect();
        s.push(' ');
        s.push_str(&parts.join("; "));
        s.push_str(".]");
        s
    }
}

/// How the legend starts; also how the HTTP hook knows a body already carries one.
pub const LEGEND_HEAD: &str = "[Privacy note:";

/// Remove every privacy legend from a model's reply.
///
/// The legend rides on the last user message of an outbound body, and for a rewrite step that message IS
/// the answer being edited, so a rewrite step can copy the legend into its output. In testing, the
/// note reached the reader on control questions that way.
/// The legend is for the model, never for the reader. Brackets are matched by depth, because the
/// legend itself contains placeholders.
pub fn strip_legend(text: &str) -> String {
    let mut out = text.to_string();
    while let Some(start) = out.find(LEGEND_HEAD) {
        let mut depth = 0usize;
        let mut end = None;
        for (i, c) in out[start..].char_indices() {
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        // An unclosed legend (a reply cut mid-note) is dropped to the end: none of it is the answer.
        let end = end.unwrap_or(out.len());
        let head = out[..start].trim_end().to_string();
        let tail = out[end..].trim_start();
        out = if tail.is_empty() {
            head
        } else if head.is_empty() {
            tail.to_string()
        } else {
            format!("{head}\n\n{tail}")
        };
    }
    out
}
