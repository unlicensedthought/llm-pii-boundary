use llm_pii_boundary::{parse_terms, strip_legend, Kind, Redactor};

#[test]
fn a_generalisation_that_names_a_protected_value_never_leaves() {
    let mut r = Redactor::new(&[]);
    r.add_matcher("Jane Roe", Kind::Person, Some("Jane Roe's sister".into()));
    r.add_matcher(
        "Acme Turbines",
        Kind::Org,
        Some("acme turbines, a wind-turbine maker".into()),
    );
    // Cross-entry: one value's generalisation names ANOTHER protected value.
    r.add_matcher("Globex", Kind::Org, Some("a rival of Acme Turbines".into()));
    let red = r.redact("Jane Roe sued Acme Turbines and Globex");
    let legend = r.legend().to_lowercase();
    assert!(
        !legend.contains("jane roe"),
        "legend leaked a name: {legend}"
    );
    assert!(!legend.contains("acme"), "legend leaked an org: {legend}");
    let q = r.search_safe(&red).to_lowercase();
    assert!(
        !q.contains("jane") && !q.contains("acme"),
        "query leaked: {q}"
    );
    // Positive control: an honest generalisation still goes out.
    let mut ok = Redactor::new(&[]);
    ok.add_matcher("Initech", Kind::Org, Some("a software company".into()));
    let red = ok.redact("Initech layoffs");
    assert!(ok.legend().contains("a software company"));
    assert!(ok.search_safe(&red).contains("a software company"));
}

#[test]
fn a_legend_the_model_copied_never_reaches_the_reader() {
    let legend =
        "[Privacy note: square-bracket placeholders such as [PERSON_1] stand for personal \
                  details. [ID_1] is an identifying number.]";
    let reply = format!("Sourdough is sour [1].\n\n{legend}");
    assert_eq!(strip_legend(&reply), "Sourdough is sour [1].");
    let mid = format!("Para one.\n\n{legend}\n\nPara two.");
    assert_eq!(strip_legend(&mid), "Para one.\n\nPara two.");
    // A note cut off mid-way is dropped to the end.
    assert_eq!(
        strip_legend("Answer.\n\n[Privacy note: [PERSON_1] is"),
        "Answer."
    );
    // Ordinary brackets and placeholders are untouched.
    assert_eq!(strip_legend("[PERSON_1] said [2]."), "[PERSON_1] said [2].");
}

#[test]
fn redaction_round_trips_through_placeholders() {
    let terms = parse_terms(&[
        "Jane Roe".into(),
        "Acme Turbines = a wind-turbine maker".into(),
    ]);
    let mut r = Redactor::new(&terms);
    let original = concat!(
        "I'm Jane Roe (jane.roe@example.org, (303) 555-0147). ", // gate:allow-line: test fixture
        "Acme Turbines fired me on March 3, 2026. jane roe again; JANE ROE shouting."
    );
    let red = r.redact(original);
    for leak in [
        "Jane Roe",
        "jane roe",
        "JANE ROE",
        "jane.roe@example.org",
        "555-0147",
        "Acme",
        "March 3",
    ] {
        assert!(!red.contains(leak), "{leak:?} leaked: {red}");
    }
    assert!(
        red.contains("[TERM_1]") && red.contains("[EMAIL_1]") && red.contains("[DATE_1]"),
        "{red}"
    );
    // Same value, same placeholder — case-insensitively for terms.
    assert_eq!(red.matches("[TERM_1]").count(), 3, "{red}");
    // Idempotent.
    assert_eq!(r.redact(&red), red);
    // Round trip: every placeholder comes back. Case-folded repeats restore to the first form.
    let (back, unknown) = r.restore(&red);
    assert_eq!(unknown, 0);
    assert!(
        back.contains("I'm Jane Roe (jane.roe@example.org, (303) 555-0147)"), // gate:allow-line: test fixture
        "{back}"
    );
    assert!(
        back.contains("Acme Turbines fired me on March 3, 2026"),
        "{back}"
    );
    // An invented placeholder is left and counted.
    let (_, unknown) = r.restore("[PERSON_9] said so");
    assert_eq!(unknown, 1);
}

#[test]
fn the_search_query_keeps_its_topic_and_loses_its_identifiers() {
    let terms = parse_terms(&[
        "Jane Roe".into(),
        "Acme Turbines = a wind-turbine maker".into(),
    ]);
    let mut r = Redactor::new(&terms);
    let q = r.redact("Jane Roe wrongful termination Acme Turbines March 3, 2026 jane@x.test");
    let s = r.search_safe(&q);
    assert_eq!(s, "wrongful termination a wind-turbine maker 2026");
    // Only identifiers: search for the kind of thing, never for nothing or for brackets.
    let q2 = r.redact("jane@x.test");
    assert_eq!(r.search_safe(&q2), "email address");
    let q3 = r.redact("[call (303) 555-0147]"); // gate:allow-line: test fixture
    assert!(!r.search_safe(&q3).contains("[PHONE"));
}

#[test]
fn the_legend_describes_without_identifying() {
    let mut r = Redactor::new(&parse_terms(&["Acme = a wind-turbine maker".into()]));
    r.redact("Acme on 2026-03-03 via https://www.linkedin.com/in/jroe");
    let l = r.legend();
    assert!(l.contains("[TERM_1] is a wind-turbine maker"), "{l}");
    assert!(l.contains("[DATE_1] is a date in 2026"), "{l}");
    assert!(
        l.contains("[URL_1] is a personal link on linkedin.com"),
        "{l}"
    );
    assert!(!l.contains("Acme") && !l.contains("jroe"), "{l}");
    assert!(Redactor::default().legend().is_empty());
}

#[test]
fn a_term_matches_whole_words_only() {
    let mut r = Redactor::new(&parse_terms(&["Ann".into()]));
    let out = r.redact("Ann met Annabel and Joanne; ann's car");
    assert_eq!(out, "[TERM_1] met Annabel and Joanne; [TERM_1]'s car");
}

#[test]
fn terms_parse_dedup_and_drop_one_character_terms() {
    let t = parse_terms(&[" Jane ".into(), "jane".into(), "x".into(), "Acme = ".into()]);
    assert_eq!(t.len(), 2);
    assert_eq!(t[0].text, "Jane");
    assert_eq!(t[1].general, None);
}
