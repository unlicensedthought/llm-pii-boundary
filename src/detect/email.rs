use crate::kind::Kind;
use crate::span::Span;

pub(crate) fn emails(text: &str, out: &mut Vec<Span>) {
    let b = text.as_bytes();
    let local_ok = |c: u8| c.is_ascii_alphanumeric() || b"._%+-".contains(&c);
    let dom_ok = |c: u8| c.is_ascii_alphanumeric() || c == b'.' || c == b'-';
    for (at, _) in text.match_indices('@') {
        let mut s = at;
        while s > 0 && local_ok(b[s - 1]) {
            s -= 1;
        }
        while s < at && b[s] == b'.' {
            s += 1;
        }
        if s == at {
            continue;
        }
        let mut e = at + 1;
        while e < b.len() && dom_ok(b[e]) {
            e += 1;
        }
        while e > at + 1 && (b[e - 1] == b'.' || b[e - 1] == b'-') {
            e -= 1;
        }
        let domain = &text[at + 1..e];
        let Some(dot) = domain.rfind('.') else {
            continue;
        };
        let tld = &domain[dot + 1..];
        if dot == 0 || tld.len() < 2 || !tld.bytes().all(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        out.push(Span::new(s, e, Kind::Email));
    }
}
