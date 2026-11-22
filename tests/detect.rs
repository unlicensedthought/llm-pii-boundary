use llm_pii_boundary::{detect, Kind};

fn kinds(text: &str) -> Vec<(String, Kind)> {
    detect(text)
        .into_iter()
        .map(|s| (text[s.start..s.end].to_string(), s.kind))
        .collect()
}

fn found(text: &str, want: &str, kind: Kind) {
    let k = kinds(text);
    assert!(
        k.iter().any(|(v, kk)| v == want && *kk == kind),
        "expected {kind:?} {want:?} in {text:?}, got {k:?}"
    );
}

fn nothing(text: &str) {
    let k = kinds(text);
    assert!(k.is_empty(), "false positive in {text:?}: {k:?}");
}

#[test]
fn emails_are_found_and_trailing_punctuation_is_not_part_of_them() {
    found(
        "write to jane.roe+x@example.org.",
        "jane.roe+x@example.org",
        Kind::Email,
    );
    found("<bob@mail.co.test>", "bob@mail.co.test", Kind::Email);
    nothing("the @media rule and user@localhost are not addresses");
}

#[test]
fn shapes_that_once_slipped_through() {
    // An exchange starting with 0 or 1 is not dialable in NANP, but it is still someone's number
    // as written; the boundary over-redacts rather than argue.
    found(
        "The adjuster ([x], (555) 014-7731). Reach", // gate:allow-line: test fixture
        "(555) 014-7731",                            // gate:allow-line: test fixture
        Kind::Phone,
    );
    found("at (555) 014-6610. How", "(555) 014-6610", Kind::Phone); // gate:allow-line: test fixture
    found(
        "From: Jo, (555) 014-2297\nRe:", // gate:allow-line: test fixture
        "(555) 014-2297",                // gate:allow-line: test fixture
        Kind::Phone,
    );
    // An SSN in parentheses.
    found(
        "Do not include your SSN (123-45-6789) in the letter", // gate:allow-line: test fixture
        "123-45-6789",                                         // gate:allow-line: test fixture
        Kind::Ssn,
    );
}

#[test]
fn phones_national_and_international() {
    found("call (303) 555-0147 today", "(303) 555-0147", Kind::Phone); // gate:allow-line: test fixture
    found("303-555-0147", "303-555-0147", Kind::Phone); // gate:allow-line: test fixture
    found("cell 303.555.0147", "303.555.0147", Kind::Phone); // gate:allow-line: test fixture
    found("+44 20 7946 0958", "+44 20 7946 0958", Kind::Phone);
    found("+49 30 901820", "+49 30 901820", Kind::Phone);
    found("ring 020 7946 0958", "020 7946 0958", Kind::Phone);
    found("phone: 5550147", "5550147", Kind::Phone);
    // A longer run still yields the phone inside it.
    found("call (303) 555-0147 2 times", "(303) 555-0147", Kind::Phone); // gate:allow-line: test fixture
}

#[test]
fn government_card_and_bank_numbers() {
    found("SSN 123-45-6789", "123-45-6789", Kind::Ssn); // gate:allow-line: test fixture
    found("my ssn is 123456789", "123456789", Kind::Ssn);
    nothing("000-12-3456 is not a valid SSN"); // gate:allow-line: test fixture
    found(
        "card 4111 1111 1111 1111",
        "4111 1111 1111 1111",
        Kind::Card,
    );
    found("4111111111111111", "4111111111111111", Kind::Card);
    // Luhn fails → not a card.
    nothing("4111 1111 1111 1112");
    found(
        "IBAN DE89 3704 0044 0532 0130 00 please",
        "DE89 3704 0044 0532 0130 00",
        Kind::Iban,
    );
    found(
        "GB82WEST12345698765432",
        "GB82WEST12345698765432",
        Kind::Iban,
    );
    nothing("DE89 3704 0044 0532 0130 01"); // mod-97 fails
}

#[test]
fn addresses_ips_and_postcodes() {
    found(
        "I live at 1428 Elm Street, Springfield",
        "1428 Elm Street",
        Kind::Address,
    );
    found("221B Baker St. London", "221B Baker St.", Kind::Address);
    found(
        "500 N Main St, Apt 4",
        "500 N Main St, Apt 4",
        Kind::Address,
    );
    found("PO Box 1234", "PO Box 1234", Kind::Address);
    found("from 203.0.113.42 yesterday", "203.0.113.42", Kind::Ip);
    found(
        "addr 2001:db8::8a2e:370:7334",
        "2001:db8::8a2e:370:7334",
        Kind::Ip,
    );
    found("Denver, CO 80202", "80202", Kind::Postcode);
    found("zip code 80202-1234", "80202-1234", Kind::Postcode);
    found("London SW1A 1AA", "SW1A 1AA", Kind::Postcode);
    found("Ottawa K1A 0B1", "K1A 0B1", Kind::Postcode);
}

#[test]
fn exact_dates_become_their_year_and_vaguer_dates_do_not() {
    let d = detect("fired on March 3, 2026 after review");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].kind, Kind::Date);
    assert_eq!(d[0].general.as_deref(), Some("2026"));
    found("on 3 March 2026", "3 March 2026", Kind::Date);
    found(
        "on the 3rd of March, 2026",
        "3rd of March, 2026",
        Kind::Date,
    );
    found("filed 2026-03-03", "2026-03-03", Kind::Date);
    found("filed 03/03/2026", "03/03/2026", Kind::Date);
    found("on 3/3/26", "3/3/26", Kind::Date);
    found("since July 17", "July 17", Kind::Date);
    // Vaguer than a day is not an exact date.
    nothing("In March 2026 the policy changed");
    nothing("the evolution of greetings between 1600-1900");
    nothing("in 1984, and again in 2026");
    nothing("you may 3 times try"); // lowercase "may" is a verb
}

#[test]
fn urls_only_when_the_path_is_a_person() {
    let s = "see https://www.linkedin.com/in/janeroe and https://docs.rs/serde/latest/serde/";
    let d = detect(s);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(
        &s[d[0].start..d[0].end],
        "https://www.linkedin.com/in/janeroe"
    );
    assert_eq!(d[0].general.as_deref(), Some("linkedin.com"));
    found(
        "https://example.com/users/jroe",
        "https://example.com/users/jroe",
        Kind::Url,
    );
    found(
        "https://github.com/janeroe",
        "https://github.com/janeroe",
        Kind::Url,
    );
    nothing("https://github.com/rust-lang/rust is the repo");
    found(
        "reset at https://app.example.com/reset?token=abc123",
        "https://app.example.com/reset?token=abc123",
        Kind::Url,
    );
}

#[test]
fn identifier_numbers_need_a_word_that_says_so() {
    found("claim 99887766", "99887766", Kind::Id);
    found("account #12345678", "12345678", Kind::Id);
    found("case E9900012345 at the agency", "E9900012345", Kind::Id);
    nothing("a population of 12345678 people");
    nothing("commit d726668 landed");
    // Public registry ids are citations.
    nothing("see PMC4347870 for the trial");
    nothing("registered as NCT01234567");
}

#[test]
fn version_numbers_years_and_citations_are_left_alone() {
    nothing("upgrade from 1.2.3 to v10.0.1.4 and wasm-bindgen 0.2.130");
    nothing("Rust 1.79.0 shipped in 2024");
    nothing("as shown [3] and [12], see also [1][2]");
    nothing("pi is 3.14159265 and e is 2.71828");
    nothing("between 2010 2011 2012 2013 the trend held");
    nothing("std::io::Write and a::b and 12:30:45");
    nothing("$1,250,000.00 in damages, 40 hours a week");
    nothing("chapter 3 of the book, page 214");
}
