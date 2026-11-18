use crate::kind::Kind;
use crate::span::Span;
use crate::text::*;

/// ISO 13616 mod-97 check: the difference between an IBAN and a run of capitals and digits.
pub(crate) fn iban_valid(s: &str) -> bool {
    if s.len() < 15 || s.len() > 34 {
        return false;
    }
    let rearranged = format!("{}{}", &s[4..], &s[..4]);
    let mut rem: u32 = 0;
    for c in rearranged.chars() {
        let v = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            _ => return false,
        };
        let digits = if v >= 10 { 100 } else { 10 };
        rem = (rem * digits + v) % 97;
    }
    rem == 1
}

pub(crate) fn ibans(text: &str, out: &mut Vec<Span>) {
    let b = text.as_bytes();
    let up_or_digit = |c: u8| c.is_ascii_uppercase() || c.is_ascii_digit();
    let mut i = 0;
    while i + 4 <= b.len() {
        let head = b[i].is_ascii_uppercase()
            && b[i + 1].is_ascii_uppercase()
            && b[i + 2].is_ascii_digit()
            && b[i + 3].is_ascii_digit();
        if !head || is_word(prev_char(text, i)) {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut clean = String::new();
        let mut group = 0;
        loop {
            if j < b.len() && up_or_digit(b[j]) {
                clean.push(b[j] as char);
                group += 1;
                j += 1;
            } else if j + 1 < b.len() && b[j] == b' ' && group == 4 && up_or_digit(b[j + 1]) {
                group = 0;
                j += 1;
            } else {
                break;
            }
        }
        if !is_word(next_char(text, j)) && iban_valid(&clean) {
            out.push(Span::new(i, j, Kind::Iban));
            i = j;
            continue;
        }
        i += 1;
    }
}
