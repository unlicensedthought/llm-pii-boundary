use crate::kind::Kind;
use crate::span::Span;
use crate::text::*;

pub(crate) fn ipv6(text: &str, out: &mut Vec<Span>) {
    let b = text.as_bytes();
    let ok = |c: u8| c.is_ascii_hexdigit() || c == b':';
    let mut i = 0;
    while i < b.len() {
        if !ok(b[i]) || is_word(prev_char(text, i)) || (i > 0 && b[i - 1] == b':') {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < b.len() && ok(b[j]) {
            j += 1;
        }
        let raw = &text[i..j];
        let colons = raw.matches(':').count();
        let groups_ok = raw.split(':').all(|g| g.len() <= 4);
        let has_digit = raw.bytes().any(|c| c.is_ascii_digit());
        if !is_word(next_char(text, j))
            && colons >= 2
            && (raw.contains("::") || colons == 7)
            && groups_ok
            && has_digit
            && raw.len() >= 6
            && !raw.contains(":::")
        {
            out.push(Span::new(i, j, Kind::Ip));
        }
        i = j.max(i + 1);
    }
}
