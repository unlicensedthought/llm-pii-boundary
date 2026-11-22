use crate::kind::Kind;
use crate::span::Span;
use crate::text::*;

pub(crate) fn luhn(digits: &str) -> bool {
    let mut sum = 0u32;
    for (i, c) in digits.chars().rev().enumerate() {
        let mut d = c as u32 - '0' as u32;
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    sum.is_multiple_of(10)
}

/// Runs of digits joined by single separators, as byte ranges with their digit groups.
/// A run of digits: its start, its end, and the byte range of each digit group in it.
pub(crate) type NumberRun = (usize, usize, Vec<(usize, usize)>);

pub(crate) fn number_runs(text: &str) -> Vec<NumberRun> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let lead = b[i] == b'+' || b[i] == b'(';
        let starts =
            b[i].is_ascii_digit() || (lead && i + 1 < b.len() && b[i + 1].is_ascii_digit());
        // A digit glued to a word ("E9900012345", "sha256") is not the start of a number; the
        // alphanumeric-identifier scanner owns those.
        if !starts || is_word(prev_char(text, i)) {
            i += 1;
            continue;
        }
        let mut j = if lead { i + 1 } else { i };
        let mut groups = Vec::new();
        loop {
            let g0 = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            groups.push((g0, j));
            let mut k = j;
            let mut seps = 0;
            while k < b.len() && seps < 2 && b" -.()".contains(&b[k]) {
                k += 1;
                seps += 1;
            }
            if seps > 0 && k < b.len() && b[k].is_ascii_digit() && groups.len() < 12 {
                j = k;
                continue;
            }
            break;
        }
        if is_word(next_char(text, j)) {
            // "12abc": a number glued to a word is not an identifier we can classify.
            i = j.max(i + 1);
            continue;
        }
        out.push((i, j, groups));
        i = j.max(i + 1);
    }
    out
}

pub(crate) fn is_ipv4(raw: &str) -> bool {
    let parts: Vec<&str> = raw.split('.').collect();
    parts.len() == 4
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.len() <= 3
                && p.bytes().all(|c| c.is_ascii_digit())
                && p.parse::<u16>().is_ok_and(|v| v <= 255)
        })
}

pub(crate) fn valid_ssn(d: &str) -> bool {
    let area: u32 = d[0..3].parse().unwrap_or(0);
    let group: u32 = d[3..5].parse().unwrap_or(0);
    let serial: u32 = d[5..9].parse().unwrap_or(0);
    area != 0 && area != 666 && area < 900 && group != 0 && serial != 0
}

/// A list of years ("2010 2011 2012 2013") is prose, however many digits it adds up to.
pub(crate) fn looks_like_years(groups: &[&str]) -> bool {
    groups.len() >= 2
        && groups
            .iter()
            .all(|g| g.len() == 4 && g.parse::<u32>().is_ok_and(|y| (1000..=2999).contains(&y)))
}

/// What a sub-run of digits is, if anything. `start` is its offset in `text`, for the keyword check.
pub(crate) fn classify_number(text: &str, start: usize, raw: &str, run_head: bool) -> Option<Span> {
    let end = start + raw.len();
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let d = digits.len();
    let groups: Vec<&str> = raw
        .split(|c: char| !c.is_ascii_digit())
        .filter(|g| !g.is_empty())
        .collect();
    let lens: Vec<usize> = groups.iter().map(|g| g.len()).collect();
    let seps: Vec<char> = raw.chars().filter(|c| !c.is_ascii_digit()).collect();
    let only = |allowed: &[char]| seps.iter().all(|c| allowed.contains(c));
    let span = |k: Kind| Some(Span::new(start, end, k));

    if raw.contains('.') && is_ipv4(raw) {
        let v = prev_char(text, start);
        return if matches!(v, Some('v') | Some('V')) {
            None
        } else {
            span(Kind::Ip)
        };
    }
    if lens == [3, 2, 4] && only(&['-', ' ']) && valid_ssn(&digits) {
        return span(Kind::Ssn);
    }
    // "SSN (123-45-6789)": a run that opened on a parenthesis. The span starts after it, so the // gate:allow-line: test fixture
    // redacted text keeps its brackets balanced. It leaked before this case existed.
    if let Some(inner) = raw.strip_prefix('(') {
        if lens == [3, 2, 4]
            && inner
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
            && valid_ssn(&digits)
        {
            return Some(Span::new(start + 1, end, Kind::Ssn));
        }
    }
    if d == 9 && lens.len() == 1 && keyword_before(text, start, SSN_WORDS) {
        return span(Kind::Ssn);
    }
    if looks_like_years(&groups) {
        return None;
    }
    let card_shape =
        lens.len() == 1 || (lens[0] == 4 && lens.iter().all(|&g| g >= 3) && only(&[' ', '-']));
    let all_same = digits.chars().all(|c| digits.starts_with(c));
    if (13..=19).contains(&d) && card_shape && !all_same && luhn(&digits) {
        return span(Kind::Card);
    }
    if raw.starts_with('+') && (8..=15).contains(&d) && (lens.len() >= 2 || d >= 10) {
        return span(Kind::Phone);
    }
    // North American: [1-]NXX-XXX-XXXX, with separators (a bare ten-digit run needs a keyword). The
    // exchange is NOT checked: "(555) 014-7731" is undialable but is still someone's number as // gate:allow-line: test fixture
    // written. Checking the exchange let real numbers through.
    let nanp_lens = if lens.first() == Some(&1) && digits.starts_with('1') {
        &lens[1..]
    } else {
        &lens[..]
    };
    let nanp_digits = if d == 11 && digits.starts_with('1') {
        &digits[1..]
    } else {
        &digits[..]
    };
    if nanp_lens == [3, 3, 4]
        && nanp_digits.len() == 10
        && only(&['-', '.', ' ', '(', ')'])
        && nanp_digits.as_bytes()[0] >= b'2'
    {
        return span(Kind::Phone);
    }
    // National numbers with a trunk 0 (UK, EU, AU …): "020 7946 0958", "07700 900123". The weakest
    // shape here, so only at the HEAD of a run: the tail of a longer digit run ("DE89 3704 0044
    // 0532 0130 01", an IBAN that failed its check) is not a phone number.
    if run_head
        && digits.starts_with('0')
        && (10..=11).contains(&d)
        && lens.len() >= 2
        && lens.iter().all(|&g| g >= 2)
        && only(&[' ', '-'])
    {
        return span(Kind::Phone);
    }
    // A keyword makes a number an identifier only when the number itself has no telling shape (one or
    // two groups); otherwise "call (303) 555-0147 2 times" would swallow the "2". // gate:allow-line: test fixture
    if lens.len() <= 2 && (7..=15).contains(&d) && keyword_before(text, start, PHONE_WORDS) {
        return span(Kind::Phone);
    }
    if lens.len() <= 2 && d >= 6 && keyword_before(text, start, ID_WORDS) {
        return span(Kind::Id);
    }
    None
}

pub(crate) fn numbers(text: &str, out: &mut Vec<Span>) {
    let b = text.as_bytes();
    for (run_start, _run_end, groups) in number_runs(text) {
        let n = groups.len();
        let sub_start = |a: usize| {
            if a == 0 {
                run_start
            } else if groups[a].0 > 0 && b[groups[a].0 - 1] == b'(' {
                groups[a].0 - 1
            } else {
                groups[a].0
            }
        };
        // Greedy over sub-runs: the longest classifiable stretch starting at each group, so "call
        // (303) 555-0147 2 times" still finds the phone inside the longer run. // gate:allow-line: test fixture
        let mut a = 0;
        while a < n {
            let mut found = None;
            for z in (a..n).rev() {
                let s = sub_start(a);
                let e = groups[z].1;
                if let Some(span) = classify_number(text, s, &text[s..e], a == 0) {
                    found = Some((z, span));
                    break;
                }
            }
            match found {
                Some((z, span)) => {
                    out.push(span);
                    a = z + 1;
                }
                None => a += 1,
            }
        }
    }
}
