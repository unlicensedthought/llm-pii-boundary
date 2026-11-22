//! A deterministic end-to-end turn for the leak check. No network and no model: a stand-in "model"
//! echoes what it was sent, which is the worst case for a boundary (everything it saw comes back).

use serde_json::{json, Value};

use crate::boundary::Boundary;
use crate::kind::Kind;
use crate::leak::{self, Outcome, Planted};
use crate::redactor::parse_terms;

#[derive(Debug, Clone)]
pub struct Fixture {
    pub id: String,
    pub group: String,
    pub question: String,
    /// The user's own identity terms (name, employer, town).
    pub terms: Vec<String>,
    /// Names that only an outside detector can find. The fixture hands them to the boundary the way
    /// a detector's output would arrive.
    pub names: Vec<String>,
    pub planted: Vec<Planted>,
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn load(raw: &str) -> Result<Vec<Fixture>, String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("fixtures are not JSON: {e}"))?;
    let list = v["fixtures"]
        .as_array()
        .ok_or("fixtures: no \"fixtures\" array")?;
    list.iter()
        .map(|f| {
            let get = |k: &str| {
                f[k].as_str()
                    .map(str::to_string)
                    .ok_or(format!("fixture without \"{k}\""))
            };
            Ok(Fixture {
                id: get("id")?,
                group: get("group")?,
                question: get("question")?,
                terms: strings(&f["terms"]),
                names: strings(&f["names"]),
                planted: f["planted"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|p| Planted {
                                value: p["value"].as_str().unwrap_or_default().to_string(),
                                variants: strings(&p["variants"]),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// The stand-in model: it repeats its whole prompt, legend included, which is what a careless
/// model does and what the boundary has to survive.
fn stand_in(prompt: &str) -> String {
    format!("Draft answer. You wrote: {prompt}")
}

fn chat_body(user: &str) -> Value {
    json!({
        "model": "stand-in",
        "messages": [
            {"role": "system", "content": "You are a careful assistant."},
            {"role": "user", "content": user}
        ]
    })
}

fn last_user(body: &Value) -> String {
    body["messages"]
        .as_array()
        .and_then(|m| m.iter().rev().find(|m| m["role"] == "user"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or_default()
        .to_string()
}

/// One turn with three outbound calls (answer, rewrite, search query) through `boundary`, then the
/// leak check against what the same turn says with no boundary.
pub fn run_turn(fx: &Fixture, boundary: Boundary) -> Outcome {
    let boundary = boundary.capturing();
    for n in &fx.names {
        boundary.add_literal(n, Kind::Person, None);
    }

    let mut first = chat_body(&fx.question);
    boundary.apply_json(&mut first);
    let draft = stand_in(&last_user(&first));

    let mut second = chat_body(&draft);
    boundary.apply_json(&mut second);
    let rewritten = stand_in(&last_user(&second));

    let mut search = json!({ "q": boundary.search_query(&fx.question) });
    boundary.apply_json(&mut search);

    let final_answer = boundary.restore(&rewritten);
    let base_answer = stand_in(&stand_in(&fx.question));
    leak::check(
        &fx.planted,
        &fx.question,
        &base_answer,
        &boundary.captured(),
        &final_answer,
    )
}

/// The boundary a fixture's user would have.
pub fn boundary_for(fx: &Fixture) -> Boundary {
    Boundary::on(&parse_terms(&fx.terms))
}
