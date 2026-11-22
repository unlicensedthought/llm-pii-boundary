use crate::kind::Kind;
use crate::span::Span;
use crate::text::*;

pub(crate) const MONTHS: &[(&str, &str)] = &[
    ("january", "jan"),
    ("february", "feb"),
    ("march", "mar"),
    ("april", "apr"),
    ("may", "may"),
    ("june", "jun"),
    ("july", "jul"),
    ("august", "aug"),
    ("september", "sep"),
    ("october", "oct"),
    ("november", "nov"),
    ("december", "dec"),
];

/// A capitalised month name or abbreviation. Capitalised, because "may" is a verb.
pub(crate) fn is_month(w: &str) -> bool {
    let Some(first) = w.chars().next() else {
        return false;
    };
    if !first.is_uppercase() {
        return false;
    }
    let l = w.to_lowercase();
    MONTHS
        .iter()
        .any(|(full, abbr)| l == *full || l == *abbr || (l == "sept" && *abbr == "sep"))
}

pub(crate) fn day_of(w: &str) -> Option<u32> {
    let core = w
        .strip_suffix("st")
        .or_else(|| w.strip_suffix("nd"))
        .or_else(|| w.strip_suffix("rd"))
        .or_else(|| w.strip_suffix("th"))
        .unwrap_or(w);
    if core.is_empty() || core.len() > 2 || !core.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    core.parse::<u32>().ok().filter(|d| (1..=31).contains(d))
}

pub(crate) fn year_of(w: &str) -> Option<u32> {
    if w.len() != 4 || !w.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    w.parse::<u32>().ok().filter(|y| (1000..=2999).contains(y))
}

/// Alphanumeric words with their byte ranges.
pub(crate) fn words(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            out.push((s, i));
        }
    }
    if let Some(s) = start {
        out.push((s, text.len()));
    }
    out
}

pub(crate) fn date_span(start: usize, end: usize, year: Option<u32>) -> Span {
    let s = Span::new(start, end, Kind::Date);
    match year {
        Some(y) => s.with_general(y.to_string()),
        None => s,
    }
}

/// Exact dates (day precision) → `[DATE_n]`, generalised to the year (HIPAA Safe Harbor #3). A bare
/// year, a month-and-year, or a range of years is NOT an exact date and is left alone.
pub(crate) fn dates(text: &str, out: &mut Vec<Span>) {
    numeric_dates(text, out);
    let w = words(text);
    let gap = |a: usize, b: usize| &text[w[a].1..w[b].0];
    let word = |k: usize| &text[w[k].0..w[k].1];
    for i in 0..w.len() {
        // Month Day[, Year]: "March 3, 2026", "Mar. 3rd 2026", "March 3".
        if is_month(word(i)) && i + 1 < w.len() && matches!(gap(i, i + 1), " " | ". ") {
            if let Some(_d) = day_of(word(i + 1)) {
                let mut end = w[i + 1].1;
                let mut year = None;
                if i + 2 < w.len() && matches!(gap(i + 1, i + 2), " " | ", ") {
                    if let Some(y) = year_of(word(i + 2)) {
                        end = w[i + 2].1;
                        year = Some(y);
                    }
                }
                if !next_char(text, end).is_some_and(|c| c.is_ascii_digit()) {
                    out.push(date_span(w[i].0, end, year));
                }
            }
        }
        // Day [of] Month[, Year]: "3 March 2026", "3rd of March, 2026".
        if day_of(word(i)).is_some() && i + 1 < w.len() && gap(i, i + 1) == " " {
            let (m, ok) = if word(i + 1) == "of" && i + 2 < w.len() && gap(i + 1, i + 2) == " " {
                (i + 2, true)
            } else {
                (i + 1, true)
            };
            if ok && is_month(word(m)) && !is_word(prev_char(text, w[i].0)) {
                let mut end = w[m].1;
                if text[end..].starts_with('.') && word(m).len() == 3 {
                    end += 1;
                }
                let mut year = None;
                if m + 1 < w.len() && matches!(gap(m, m + 1), " " | ", " | ". ") {
                    if let Some(y) = year_of(word(m + 1)) {
                        end = w[m + 1].1;
                        year = Some(y);
                    }
                }
                out.push(date_span(w[i].0, end, year));
            }
        }
    }
}

pub(crate) fn numeric_dates(text: &str, out: &mut Vec<Span>) {
    let b = text.as_bytes();
    let digits_at = |i: usize| {
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        j
    };
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() || is_word(prev_char(text, i)) {
            i += 1;
            continue;
        }
        let a_end = digits_at(i);
        let sep = b.get(a_end).copied();
        let Some(sep) = sep.filter(|c| b"-/.".contains(c)) else {
            i = a_end.max(i + 1);
            continue;
        };
        let m_start = a_end + 1;
        let m_end = digits_at(m_start);
        if m_end == m_start || b.get(m_end) != Some(&sep) {
            i = a_end.max(i + 1);
            continue;
        }
        let c_start = m_end + 1;
        let c_end = digits_at(c_start);
        if c_end == c_start || is_word(next_char(text, c_end)) {
            i = a_end.max(i + 1);
            continue;
        }
        // A fourth part ("1.2.3.4", "2026-03-03-01") is not a date.
        if b.get(c_end).is_some_and(|c| *c == sep)
            && b.get(c_end + 1).is_some_and(u8::is_ascii_digit)
        {
            i = c_end;
            continue;
        }
        let pa: u32 = text[i..a_end].parse().unwrap_or(0);
        let pb: u32 = text[m_start..m_end].parse().unwrap_or(0);
        let pc: u32 = text[c_start..c_end].parse().unwrap_or(0);
        let (la, lb, lc) = (a_end - i, m_end - m_start, c_end - c_start);
        let year = if la == 4 && lb <= 2 && lc <= 2 {
            // ISO: yyyy-mm-dd
            ((1..=12).contains(&pb) && (1..=31).contains(&pc) && sep != b'.').then_some(pa)
        } else if la <= 2 && lb <= 2 && (lc == 4 || (lc == 2 && sep != b'.')) {
            let plausible =
                (1..=31).contains(&pa) && (1..=31).contains(&pb) && (pa <= 12 || pb <= 12);
            plausible.then(|| {
                if lc == 4 {
                    pc
                } else if pc <= 50 {
                    2000 + pc
                } else {
                    1900 + pc
                }
            })
        } else {
            None
        };
        if let Some(y) = year.filter(|y| (1000..=2999).contains(y)) {
            out.push(date_span(i, c_end, Some(y)));
        }
        i = c_end.max(i + 1);
    }
}
