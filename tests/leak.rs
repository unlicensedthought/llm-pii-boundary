use llm_pii_boundary::eval::{boundary_for, load, run_turn};
use llm_pii_boundary::leak::{check, Planted};
use llm_pii_boundary::Boundary;

const FIXTURES: &str = include_str!("fixtures.json");

fn p(v: &str, variants: &[&str]) -> Planted {
    Planted {
        value: v.into(),
        variants: variants.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn known_bad_a_planted_value_in_an_outbound_string_is_a_leak() {
    let planted = [p("Mara Okonkwo-Lind", &["mara okonkwo"])];
    let out = check(
        &planted,
        "I'm Mara Okonkwo-Lind",
        "ok",
        &["hello MARA OKONKWO here".to_string()],
        "ok",
    );
    assert_eq!(out.leaks.len(), 1, "{out:?}");
    assert_eq!(out.leaks[0].form, "mara okonkwo");
}

#[test]
fn known_good_a_clean_outbound_is_protected_not_leaked() {
    let planted = [p("Mara Okonkwo-Lind", &[])];
    let out = check(
        &planted,
        "I'm Mara Okonkwo-Lind",
        "hi Mara Okonkwo-Lind",
        &["hello [TERM_1]".to_string()],
        "hi Mara Okonkwo-Lind",
    );
    assert!(out.leaks.is_empty(), "{out:?}");
    assert_eq!((out.protected, out.restored, out.restorable), (1, 1, 1));
}

#[test]
fn a_placeholder_left_in_the_final_answer_is_reported() {
    let out = check(&[], "", "", &[], "dear [PERSON_2]");
    assert_eq!(out.placeholders_left, vec!["[PERSON_2]".to_string()]);
}

#[test]
fn nothing_planted_can_be_checked_without_any_outbound() {
    let planted = [p("Mara Okonkwo-Lind", &[])];
    let out = check(&planted, "I'm Mara Okonkwo-Lind", "", &[], "");
    assert_eq!(
        out.protected, 0,
        "no call went out, so nothing was shown to be protected"
    );
}

#[test]
fn boundary_on_leaks_nothing_and_restores_everything_across_the_fixtures() {
    let fixtures = load(FIXTURES).unwrap();
    assert_eq!(fixtures.iter().filter(|f| f.group == "pii").count(), 4);
    let mut planted = 0;
    for fx in &fixtures {
        let o = run_turn(fx, boundary_for(fx));
        assert!(o.leaks.is_empty(), "{}: {:?}", fx.id, o.leaks);
        assert_eq!(o.restored, o.restorable, "{}: {o:?}", fx.id);
        assert!(o.placeholders_left.is_empty(), "{}: {o:?}", fx.id);
        assert_eq!(
            o.protected,
            fx.planted.len(),
            "{}: every planted value should be shown protected",
            fx.id
        );
        planted += fx.planted.len();
    }
    assert_eq!(planted, 30);
}

#[test]
fn negative_proof_boundary_off_leaks_every_planted_value() {
    let fixtures = load(FIXTURES).unwrap();
    let mut leaks = 0;
    for fx in &fixtures {
        leaks += run_turn(fx, Boundary::off()).leaks.len();
    }
    assert_eq!(leaks, 30, "the check must be able to fail");
}

#[test]
fn negative_proof_a_boundary_missing_one_term_is_caught() {
    let fixtures = load(FIXTURES).unwrap();
    let fx = fixtures.iter().find(|f| f.id == "pii-employer-hr").unwrap();
    let mut weak = fx.clone();
    weak.terms.retain(|t| t != "Halden Freight");
    let o = run_turn(&weak, boundary_for(&weak));
    assert!(o.leaks.iter().any(|l| l.value == "Halden Freight"), "{o:?}");
}

#[test]
fn controls_are_left_untouched() {
    let fixtures = load(FIXTURES).unwrap();
    for fx in fixtures.iter().filter(|f| f.group == "control") {
        let b = boundary_for(fx);
        assert_eq!(b.redact(&fx.question), fx.question, "{} was altered", fx.id);
        assert_eq!(b.meta()["redacted_count"], 0);
    }
}
