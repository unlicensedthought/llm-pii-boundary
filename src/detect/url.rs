use crate::kind::Kind;
use crate::span::Span;
use crate::text::*;

/// Hosts where any path is a person: a profile, a handle, a payment link.
pub(crate) const PROFILE_HOSTS: &[&str] = &[
    "linkedin.com",
    "facebook.com",
    "fb.com",
    "instagram.com",
    "twitter.com",
    "x.com",
    "tiktok.com",
    "threads.net",
    "bsky.app",
    "mastodon.social",
    "venmo.com",
    "cash.app",
    "paypal.me",
    "calendly.com",
    "t.me",
    "wa.me",
    "patreon.com",
    "youtube.com",
    "reddit.com",
    "medium.com",
    "about.me",
    "linktr.ee",
    "onlyfans.com",
];
/// Hosts where a path is somebody's private document.
pub(crate) const DOC_HOSTS: &[&str] = &[
    "docs.google.com",
    "drive.google.com",
    "dropbox.com",
    "1drv.ms",
    "onedrive.live.com",
    "box.com",
    "icloud.com",
];
/// Code hosts: one path segment is a profile; deeper is a project (public, and usually the topic).
pub(crate) const CODE_HOSTS: &[&str] =
    &["github.com", "gitlab.com", "bitbucket.org", "codeberg.org"];
pub(crate) const PERSONAL_SEGMENTS: &[&str] = &[
    "in", "u", "user", "users", "profile", "profiles", "people", "member", "members", "author",
    "account", "accounts", "me",
];
pub(crate) const PERSONAL_QUERY_KEYS: &[&str] = &[
    "email",
    "e-mail",
    "mail",
    "name",
    "user",
    "username",
    "uid",
    "userid",
    "user_id",
    "account",
    "phone",
    "token",
    "sig",
    "signature",
    "session",
    "sid",
    "code",
    "key",
];

pub(crate) fn host_is(host: &str, list: &[&str]) -> bool {
    list.iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
}

/// The host, when a URL identifies a person (HIPAA Safe Harbor #14, narrowed to the URLs that do).
/// A documentation link is not personal and is left alone — it is usually the topic.
pub fn personal_url(url: &str) -> Option<String> {
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    let cut = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(cut);
    let userinfo = authority.contains('@');
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = host.split(':').next().unwrap_or(host);
    let host = host.strip_prefix("www.").unwrap_or(host).to_string();
    if host.is_empty() {
        return None;
    }
    let (path, query) = match tail.find('?') {
        Some(q) => (&tail[..q], &tail[q + 1..]),
        None => (tail, ""),
    };
    let path = path.split('#').next().unwrap_or(path);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let personal = userinfo
        || tail.contains('@')
        || (host_is(&host, PROFILE_HOSTS) && !segments.is_empty())
        || (host_is(&host, DOC_HOSTS) && !segments.is_empty())
        || (host_is(&host, CODE_HOSTS) && segments.len() == 1)
        || segments
            .iter()
            .any(|s| PERSONAL_SEGMENTS.contains(s) || s.starts_with('~') || s.starts_with("%7e"))
        || query.split('&').any(|kv| {
            let k = kv.split('=').next().unwrap_or("");
            PERSONAL_QUERY_KEYS.contains(&k)
        });
    personal.then_some(host)
}

pub(crate) fn urls(text: &str, out: &mut Vec<Span>) {
    let lower = text.to_ascii_lowercase();
    let lb = lower.as_bytes();
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let starts = lb[i..].starts_with(b"https://")
            || lb[i..].starts_with(b"http://")
            || lb[i..].starts_with(b"www.");
        if !starts || is_word(prev_char(text, i)) {
            i += 1;
            continue;
        }
        let mut e = i;
        while e < b.len() && !b[e].is_ascii_whitespace() && !b"<>\"`".contains(&b[e]) {
            e += 1;
        }
        while e > i && b".,;:!?)]}'".contains(&b[e - 1]) {
            e -= 1;
        }
        if let Some(host) = personal_url(&text[i..e]) {
            out.push(Span::new(i, e, Kind::Url).with_general(host));
        }
        i = e.max(i + 1);
    }
}
