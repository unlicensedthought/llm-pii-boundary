# llm-pii-boundary

Take personal details out of text before it goes to a language model, put them back in the answer, and test that nothing slipped out in between.

Direct identifiers (emails, phone numbers, card and bank numbers, SSNs, street addresses, personal links, exact dates, a few more) become placeholders like `[EMAIL_1]` and `[PERSON_2]`. The placeholder map lives in your process for one turn. The model sees placeholders. You restore them once, on the way out.

The part most privacy layers skip is the test. Here a fixture plants known identifiers in the user's text. After a turn, every string that went out is searched for every planted value. One hit is a leak, and the eval exits 1.

Write-up: [Redact before the model, restore after](https://unlicensedthought.com/writing/redact-before-the-model-restore-after/)

## Quick start

```bash
cargo run --bin pii-eval          # 8 synthetic fixtures, boundary on and off
cargo run --example show          # see what the redacted text looks like
scripts/verify.sh                 # install, build, fmt, clippy, tests, eval
```

In code:

```rust
use llm_pii_boundary::{parse_terms, Boundary, Kind};
use serde_json::json;

// Terms are the user's own details that no pattern can know: name, employer, town.
let b = Boundary::on(&parse_terms(&["Mara Okonkwo-Lind".into(), "Halden Freight = a freight firm".into()]));
b.add_literal("Tomas Verhoeven", Kind::Person, None); // e.g. from a name detector you run

let mut body = json!({"messages": [{"role": "user", "content": "Mail tverhoeven@claims.example for Mara Okonkwo-Lind"}]});
b.apply_json(&mut body);                 // every string leaf is redacted; a legend is added
// ... send `body` to the model ...
let shown = b.restore(&reply);           // once, on the way out
```

## How it works

1. **Detect, deterministic first.** Patterns for emails, phones, SSNs, IPv4/IPv6, dates, street addresses, ZIP and postcodes. Checksums where a shape alone would cry wolf: Luhn for cards, mod-97 for IBANs. Host rules for personal URLs (a profile or a private document is personal; a docs page isn't). Keyword context for ID numbers: `claim 99887766` is an identifier, `population 12345678` isn't.
2. **Terms.** The user's own names and places are matched case-insensitively on word boundaries. `"Acme = a wind-turbine maker"` gives the model something to work with instead of nothing.
3. **Placeholders.** Same value, same placeholder, numbered by first appearance. Redaction is idempotent: a placeholder is never matched again.
4. **Legend.** The last user message gets a short note saying what each placeholder present in that body stands for. A generalisation that echoes a protected value is swapped for the plain noun.
5. **Restore once.** A copied legend is stripped. A placeholder this turn never issued is left alone and counted.
6. **Search queries.** `search_query` keeps the topic and drops the identifiers: a date becomes its year, a link its host.

The eval (`src/eval.rs`, `src/leak.rs`) runs three outbound calls per fixture through the hook, uses a stand-in model that echoes everything it sees, and checks the captured outbound strings against the planted values. It also runs with the boundary off, and fails if *that* run doesn't leak: a check that can't fail isn't a check.

## What it doesn't do

- **Names need help.** A name has no shape. Give it as a term, or feed it in with `add_literal` from a detector you run. This crate ships no model pass.
- **It isn't anonymisation.** It's pseudonymisation. "Water damage on a Tuesday in a town of 900" can still point at someone. Only direct identifiers and the terms you list are removed.
- **Patterns lean US, UK and Canada.** Phones, postcodes and addresses outside those shapes may pass through.
- **Case-folding is ASCII only** for terms. A non-ASCII name has to match exactly.
- **JSON keys aren't redacted**, only string values.
- **Repeats come back in the first form seen.** `jane roe` and `JANE ROE` both restore as the form that came first.
- **The bundled eval uses a stand-in model.** It proves the boundary holds on the outbound side and round-trips on the way back. It says nothing about how a real model answers when it sees placeholders. Measure that yourself.
- **Eight fixtures, 30 planted values, all written by one person.** They show the property holds. They don't show it holds on your text.

## Tests

```bash
cargo test            # detectors, redactor, boundary, leak check
cargo run --bin pii-eval
```

`tests/detect.rs` has the false-positive controls (version numbers, years in prose, citation markers) next to the detections, because a filter that eats the answer is its own failure. `tests/leak.rs` has the known-bad and known-good cases for the leak check itself, and a test that drops one term from a fixture and expects the leak to be caught.

## License

Apache-2.0.
