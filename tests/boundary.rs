use llm_pii_boundary::{parse_terms, Boundary, Kind};
use serde_json::json;

#[test]
fn the_boundary_off_changes_nothing() {
    let b = Boundary::off();
    assert_eq!(b.redact("jane@x.test"), "jane@x.test");
    assert_eq!(b.restore("[EMAIL_1]"), "[EMAIL_1]");
    assert_eq!(b.meta(), json!({"on": false}));
    assert!(b.captured().is_empty());
}

#[test]
fn meta_carries_counts_and_never_values() {
    let b = Boundary::on(&parse_terms(&["Jane Roe".into()]));
    b.redact("Jane Roe, jane@x.test, bob@y.test, (303) 555-0147"); // gate:allow-line: test fixture
    let m = b.meta();
    assert_eq!(m["on"], true);
    assert_eq!(m["redacted_count"], 4);
    assert_eq!(m["kinds"]["email"], 2);
    assert_eq!(m["kinds"]["term"], 1);
    let s = m.to_string();
    for v in ["Jane", "jane@", "555"] {
        assert!(!s.contains(v), "meta carries a value: {s}");
    }
}

#[test]
fn every_string_leaf_of_a_request_body_is_redacted() {
    let b = Boundary::on(&parse_terms(&["Jane Roe".into()])).capturing();
    let mut body = json!({
        "model": "m",
        "messages": [
            {"role": "system", "content": "Be brief. The user is Jane Roe."},
            {"role": "user", "content": "Mail bob@y.test about it."}
        ],
        "metadata": {"tags": ["from jane@x.test"]}
    });
    b.apply_json(&mut body);
    let sent = body.to_string();
    for v in ["Jane", "bob@y.test", "jane@x.test"] {
        assert!(!sent.contains(v), "{v} left in {sent}");
    }
    assert!(b.captured().iter().all(|s| !s.contains("bob@y.test")));
}

#[test]
fn the_legend_rides_on_the_last_user_message_and_names_only_what_is_there() {
    let b = Boundary::on(&parse_terms(&["Jane Roe".into()]));
    b.redact("Jane Roe and bob@y.test were seen earlier");
    let mut body = json!({"messages": [
        {"role": "user", "content": "Is bob@y.test a real address?"}
    ]});
    b.apply_json(&mut body);
    let c = body["messages"][0]["content"].as_str().unwrap();
    assert!(
        c.contains("[Privacy note:") && c.contains("[EMAIL_1] is an email address"),
        "{c}"
    );
    assert!(
        !c.contains("[TERM_1] is"),
        "legend names a placeholder this body doesn't carry: {c}"
    );
    // A second pass doesn't stack a second legend.
    b.apply_json(&mut body);
    assert_eq!(
        body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .matches("[Privacy note:")
            .count(),
        1
    );
}

#[test]
fn restore_strips_a_copied_legend_and_counts_invented_placeholders() {
    let b = Boundary::on(&parse_terms(&["Jane Roe".into()]));
    let red = b.redact("Hello Jane Roe");
    let mut body = json!({"messages": [{"role": "user", "content": red.clone()}]});
    b.apply_json(&mut body);
    let reply = format!(
        "{} And [PERSON_9] too.",
        body["messages"][0]["content"].as_str().unwrap()
    );
    let r = b.restore_counted(&reply);
    assert!(r.text.starts_with("Hello Jane Roe"), "{}", r.text);
    assert!(!r.text.contains("Privacy note"), "{}", r.text);
    assert_eq!(r.unknown, 1);
}

#[test]
fn a_literal_added_later_applies_to_everything_after_it() {
    let b = Boundary::on(&[]);
    assert!(b.redact("Ask Bob Smith.").contains("Bob Smith"));
    assert!(b.add_literal("Bob Smith", Kind::Person, None));
    assert!(!b.redact("Ask Bob Smith.").contains("Bob"));
    assert!(
        !b.add_literal("Bob Smith", Kind::Person, None),
        "second add is a no-op"
    );
}

#[test]
fn a_generalisation_that_echoes_the_value_is_replaced_by_the_plain_noun() {
    let b = Boundary::on(&[]);
    b.add_literal("Jane Roe", Kind::Person, Some("Jane Roe's sister".into()));
    let mut body = json!({"messages": [{"role": "user", "content": "Jane Roe wrote"}]});
    b.apply_json(&mut body);
    let c = body["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_lowercase();
    assert!(!c.contains("jane"), "{c}");
    assert!(c.contains("[person_1] is a person"), "{c}");
}
