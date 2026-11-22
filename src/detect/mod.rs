//! Deterministic detection: patterns, checksums and keyword context. No model, no network.

pub(crate) mod address;
pub(crate) mod alnum_id;
pub(crate) mod date;
pub(crate) mod email;
pub(crate) mod iban;
pub(crate) mod ipv6;
pub(crate) mod number;
pub(crate) mod url;

pub use url::personal_url;

use crate::span::{select, Span};
use crate::text::token_spans;

/// Every deterministic identifier in `text`, non-overlapping and in order.
///
/// Pure. The specification is its tests, including the false-positive controls (version numbers,
/// years in prose, citation markers) that separate a privacy layer from a filter that eats the answer.
pub fn detect(text: &str) -> Vec<Span> {
    select(detect_all(text), &token_spans(text))
}

pub(crate) fn detect_all(text: &str) -> Vec<Span> {
    let mut all = Vec::new();
    url::urls(text, &mut all);
    email::emails(text, &mut all);
    iban::ibans(text, &mut all);
    number::numbers(text, &mut all);
    alnum_id::alnum_ids(text, &mut all);
    ipv6::ipv6(text, &mut all);
    date::dates(text, &mut all);
    address::addresses(text, &mut all);
    address::postcodes(text, &mut all);
    all
}
