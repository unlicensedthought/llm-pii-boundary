use crate::detect::date::day_of;
use crate::kind::Kind;
use crate::span::Span;
use crate::text::*;

pub(crate) const STREET_SUFFIXES: &[&str] = &[
    "street",
    "st",
    "avenue",
    "ave",
    "road",
    "rd",
    "boulevard",
    "blvd",
    "lane",
    "ln",
    "drive",
    "dr",
    "court",
    "ct",
    "way",
    "place",
    "pl",
    "terrace",
    "ter",
    "circle",
    "cir",
    "parkway",
    "pkwy",
    "highway",
    "hwy",
    "trail",
    "trl",
    "square",
    "sq",
    "crescent",
    "close",
    "row",
    "alley",
    "loop",
];
pub(crate) const UNIT_WORDS: &[&str] = &["apt", "apartment", "unit", "suite", "ste", "flat", "#"];
pub(crate) const DIRECTIONS: &[&str] = &[
    "N", "S", "E", "W", "NE", "NW", "SE", "SW", "N.", "S.", "E.", "W.",
];

/// Whitespace-separated tokens with their byte ranges.
pub(crate) fn tokens(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push((s, i));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, text.len()));
    }
    out
}

pub(crate) fn trim_punct(s: &str) -> &str {
    s.trim_end_matches([',', '.', ';', ':', '!', '?', ')'])
}

/// "1428 Elm Street", "221B Baker St.", "500 N. Main St, Apt 4", "PO Box 1234". A heuristic: a house
/// number, up to four capitalised words, and a street suffix.
pub(crate) fn addresses(text: &str, out: &mut Vec<Span>) {
    let t = tokens(text);
    let tok = |k: usize| &text[t[k].0..t[k].1];
    for i in 0..t.len() {
        let core = trim_punct(tok(i));
        // PO Box
        if matches!(core.to_ascii_lowercase().as_str(), "po" | "p.o" | "p.o.")
            && i + 2 < t.len()
            && trim_punct(tok(i + 1)).eq_ignore_ascii_case("box")
            && trim_punct(tok(i + 2)).bytes().all(|c| c.is_ascii_digit())
        {
            let e = t[i + 2].0 + trim_punct(tok(i + 2)).len();
            out.push(Span::new(t[i].0, e, Kind::Address));
            continue;
        }
        let digits = core.bytes().take_while(u8::is_ascii_digit).count();
        let rest = &core[digits..];
        let house = (1..=6).contains(&digits)
            && (rest.is_empty() || (rest.len() == 1 && rest.as_bytes()[0].is_ascii_alphabetic()))
            && core.len() == tok(i).len();
        if !house {
            continue;
        }
        for j in (i + 2)..(i + 7).min(t.len()) {
            let w = trim_punct(tok(j));
            let middle_ok = (i + 1..j).all(|k| {
                let m = tok(k);
                let first = m.chars().next().unwrap_or(' ');
                !m.ends_with(',')
                    && (first.is_uppercase()
                        || DIRECTIONS.contains(&m)
                        || (m.chars().next().is_some_and(|c| c.is_ascii_digit())
                            && day_of(m).is_some()))
            });
            if !middle_ok {
                break;
            }
            let first_upper = w.chars().next().is_some_and(char::is_uppercase);
            if first_upper && STREET_SUFFIXES.contains(&w.to_ascii_lowercase().as_str()) {
                let mut end = t[j].0 + w.len();
                if tok(j).len() > w.len() && tok(j)[w.len()..].starts_with('.') {
                    end += 1;
                }
                // An optional unit: ", Apt 4", " Suite 200", " #12".
                let mut k = j + 1;
                if k < t.len() {
                    let u = trim_punct(tok(k)).to_ascii_lowercase();
                    if u.starts_with('#') && u.len() > 1 {
                        end = t[k].0 + trim_punct(tok(k)).len();
                    } else if UNIT_WORDS.contains(&u.as_str()) && k + 1 < t.len() {
                        k += 1;
                        end = t[k].0 + trim_punct(tok(k)).len();
                    }
                }
                out.push(Span::new(t[i].0, end, Kind::Address));
                break;
            }
        }
    }
}

pub(crate) const US_STATES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC", "PR",
];

pub(crate) fn postcodes(text: &str, out: &mut Vec<Span>) {
    let t = tokens(text);
    let tok = |k: usize| trim_punct(&text[t[k].0..t[k].1]);
    for i in 0..t.len() {
        let w = tok(i);
        // US ZIP / ZIP+4 after a state abbreviation or the word "zip".
        let zip = (w.len() == 5 && w.bytes().all(|c| c.is_ascii_digit()))
            || (w.len() == 10
                && w.as_bytes()[5] == b'-'
                && w[..5].bytes().all(|c| c.is_ascii_digit())
                && w[6..].bytes().all(|c| c.is_ascii_digit()));
        if zip && i > 0 {
            let prev = tok(i - 1);
            if US_STATES.contains(&prev)
                || keyword_before(text, t[i].0, &["zip", "postal", "postcode", "zipcode"])
            {
                out.push(Span::new(t[i].0, t[i].0 + w.len(), Kind::Postcode));
                continue;
            }
        }
        if i + 1 >= t.len() {
            continue;
        }
        let a = &text[t[i].0..t[i].1];
        let bw = tok(i + 1);
        let up = |s: &str, pat: &str| {
            s.len() == pat.len()
                && s.bytes().zip(pat.bytes()).all(|(c, p)| match p {
                    b'A' => c.is_ascii_uppercase(),
                    b'9' => c.is_ascii_digit(),
                    b'X' => c.is_ascii_uppercase() || c.is_ascii_digit(),
                    _ => false,
                })
        };
        // UK: outward A9 / A99 / AA9 / AA99 / A9A / AA9A, inward 9AA.
        let uk_out = ["A9", "A99", "AA9", "AA99", "A9A", "AA9A"]
            .iter()
            .any(|p| up(a, p));
        let uk_in = up(bw, "9AA");
        // Canada: A9A 9A9.
        let ca = up(a, "A9A") && up(bw, "9A9");
        if (uk_out && uk_in) || ca {
            out.push(Span::new(t[i].0, t[i + 1].0 + bw.len(), Kind::Postcode));
        }
    }
}
