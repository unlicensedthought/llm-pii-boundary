use llm_pii_boundary::{parse_terms, Boundary, Kind};
fn main() {
    let fx = llm_pii_boundary::eval::load(include_str!("../tests/fixtures.json")).unwrap();
    for f in fx.iter().filter(|f| f.group == "pii") {
        let b = Boundary::on(&parse_terms(&f.terms));
        for n in &f.names {
            b.add_literal(n, Kind::Person, None);
        }
        println!("{}\n{}\n{}\n", f.id, b.redact(&f.question), b.meta());
    }
}
