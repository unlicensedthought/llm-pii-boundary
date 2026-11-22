//! `pii-eval [fixtures.json]`: run every fixture with the boundary on, and again with it off.
//!
//! Exit 1 if any planted identifier leaks with the boundary on, or if the boundary-off run leaks
//! nothing (a leak check that cannot fail is not a check). Last stdout line is a one-line summary.

use llm_pii_boundary::boundary::Boundary;
use llm_pii_boundary::eval::{boundary_for, load, run_turn};

const BUNDLED: &str = include_str!("../../tests/fixtures.json");

fn main() {
    let raw = match std::env::args().nth(1) {
        Some(p) => std::fs::read_to_string(&p).unwrap_or_else(|e| {
            eprintln!("pii-eval: cannot read {p}: {e}");
            std::process::exit(2);
        }),
        None => BUNDLED.to_string(),
    };
    let fixtures = load(&raw).unwrap_or_else(|e| {
        eprintln!("pii-eval: {e}");
        std::process::exit(2);
    });

    println!("| config | fixture | planted | leaks | protected | restored | placeholders left |");
    println!("|---|---|---|---|---|---|---|");
    let (mut on_leaks, mut off_leaks, mut planted, mut restored, mut restorable) = (0, 0, 0, 0, 0);
    for (config, is_on) in [("boundary-on", true), ("boundary-off", false)] {
        for fx in &fixtures {
            let b = if is_on {
                boundary_for(fx)
            } else {
                Boundary::off()
            };
            let o = run_turn(fx, b);
            println!(
                "| {config} | {} | {} | {} | {} | {}/{} | {} |",
                fx.id,
                fx.planted.len(),
                o.leaks.len(),
                o.protected,
                o.restored,
                o.restorable,
                o.placeholders_left.join(" ")
            );
            if is_on {
                on_leaks += o.leaks.len();
                planted += fx.planted.len();
                restored += o.restored;
                restorable += o.restorable;
                for l in &o.leaks {
                    eprintln!(
                        "LEAK {}: planted value reached outbound as {:?}",
                        fx.id, l.form
                    );
                }
            } else {
                off_leaks += o.leaks.len();
            }
        }
    }
    let ok = on_leaks == 0 && off_leaks > 0;
    println!(
        "PII-EVAL {{\"planted\":{planted},\"leaks_on\":{on_leaks},\"leaks_off\":{off_leaks},\"restored\":{restored},\"restorable\":{restorable},\"verdict\":\"{}\"}}",
        if ok { "pass" } else { "fail" }
    );
    std::process::exit(if ok { 0 } else { 1 });
}
