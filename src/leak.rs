//! The outbound-leak property, as a check instead of a claim.
//!
//! A fixture plants known identifiers in the user's text. After a turn, every string that went out
//! is searched for every planted value (and its spelling variants, case-insensitively). One hit is
//! a leak. The check is deterministic: no judge, no model.

/// One planted identifier and the other ways it may be written.
#[derive(Debug, Clone, PartialEq)]
pub struct Planted {
    pub value: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Leak {
    pub value: String,
    pub form: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub leaks: Vec<Leak>,
    /// Planted values that were in the text the turn sent and did not leave.
    pub protected: usize,
    /// Planted values the unredacted answer carries that the final answer carries too.
    pub restored: usize,
    pub restorable: usize,
    /// Placeholders still in the final answer, which is a restore failure.
    pub placeholders_left: Vec<String>,
}

fn contains(hay: &str, needle: &str) -> bool {
    !needle.is_empty() && hay.to_lowercase().contains(&needle.to_lowercase())
}

/// Check one turn.
///
/// - `source`: the text the user wrote (what the turn was built from).
/// - `base_answer`: the answer the same turn gives with no boundary, i.e. what restoration must reproduce.
/// - `outbound`: every string that left, captured at the hook.
/// - `final_answer`: what the user sees after restoration.
pub fn check(
    planted: &[Planted],
    source: &str,
    base_answer: &str,
    outbound: &[String],
    final_answer: &str,
) -> Outcome {
    let mut leaks = Vec::new();
    let (mut protected, mut restored, mut restorable) = (0, 0, 0);
    for p in planted {
        let forms: Vec<&String> = std::iter::once(&p.value).chain(p.variants.iter()).collect();
        let hit = outbound
            .iter()
            .find_map(|o| forms.iter().find(|f| contains(o, f)));
        if let Some(form) = hit {
            leaks.push(Leak {
                value: p.value.clone(),
                form: (*form).clone(),
            });
        } else if !outbound.is_empty() && forms.iter().any(|f| contains(source, f)) {
            protected += 1;
        }
        if contains(base_answer, &p.value) {
            restorable += 1;
            if contains(final_answer, &p.value) {
                restored += 1;
            }
        }
    }
    Outcome {
        leaks,
        protected,
        restored,
        restorable,
        placeholders_left: placeholders_in(final_answer),
    }
}

/// `[NAME_3]`-shaped tokens in a text, de-duplicated and sorted.
pub fn placeholders_in(text: &str) -> Vec<String> {
    let mut v: Vec<String> = crate::token_spans(text)
        .into_iter()
        .map(|(a, b)| text[a..b].to_string())
        .collect();
    v.sort();
    v.dedup();
    v
}
