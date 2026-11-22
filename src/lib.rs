//! A reversible PII boundary for LLM calls.
//!
//! Redact direct identifiers to placeholders (`[EMAIL_1]`) before any text leaves your process,
//! keep the map in memory for the turn, restore the placeholders once on the way back, and prove the
//! outbound-leak property with planted identifiers (see [`leak`]).

mod detect;
mod kind;
mod span;
mod text;

pub mod boundary;
pub mod eval;
pub mod leak;
pub mod legend;
mod query;
pub mod redactor;

pub use boundary::Boundary;
pub use detect::{detect, personal_url};
pub use kind::Kind;
pub use legend::{strip_legend, LEGEND_HEAD};
pub use redactor::{parse_terms, Redactor, Term};
pub use span::Span;
pub use text::token_spans;
