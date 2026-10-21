#[path = "support/production.rs"]
mod production;
mod support;

use production::*;
use vexel_scene::*;

fn decode(source: &str) -> Result<SceneGraph> {
    run_production(source, &mut ProductionFixture::default()).map(|(graph, _)| graph)
}

#[test]
fn use_can_reference_use_and_rejects_cycles_and_excessive_depth() {
    let nested = "<svg width=\"8\" height=\"8\"><defs><use id=\"outer\" href=\"#inner\"/><use id=\"inner\" href=\"#shape\"/><rect id=\"shape\" width=\"2\" height=\"2\"/></defs><use href=\"#outer\"/></svg>";
    let graph = decode(nested).expect("nested use should expand");
    assert_eq!(graph.root().shapes.len(), 1);

    let cycle = "<svg width=\"8\" height=\"8\"><defs><use id=\"a\" href=\"#b\"/><use id=\"b\" href=\"#a\"/></defs><use href=\"#a\"/></svg>";
    assert_eq!(decode(cycle).unwrap_err().kind, ErrorKind::Unsupported);

    let mut deep = String::from("<svg width=\"8\" height=\"8\"><defs>");
    for i in 0..17 {
        deep.push_str(&format!("<use id=\"u{i}\" href=\"#u{}\"/>", i + 1));
    }
    deep.push_str("<rect id=\"u17\" width=\"1\" height=\"1\"/></defs><use href=\"#u0\"/></svg>");
    assert_eq!(
        decode(&deep).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::References)
    );
}
