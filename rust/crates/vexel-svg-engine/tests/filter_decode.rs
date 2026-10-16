//! WO-RUST-SVG-FILTER-01: a referenced filter decodes end to end through the
//! engine and real Decoder; markers stay deferred; unsupported primitives keep
//! their typed diagnostics.
mod common;
use common::*;
use vexel_scene::*;

const HEAD: &str = "<svg width='16' height='16'>";

#[test]
fn referenced_filter_on_group_decodes_and_publishes_the_owner_group() {
    let svg = format!(
        "{HEAD}<defs><filter id='f'><feGaussianBlur stdDeviation='1'/></filter></defs>\
<g filter='url(#f)'><rect width='8' height='8'/></g></svg>"
    );
    let graph = decode_ok(&svg);
    let scene = graph.root();
    assert_eq!(scene.filters.len(), 1);
    assert_eq!(scene.groups.len(), 1);
    assert!(scene.groups[0].filter.is_some());
    assert_eq!(scene.groups[0].shapes.count, 1);
    assert_eq!(scene.shapes.len(), 1);
}

#[test]
fn referenced_filter_on_shape_decodes() {
    let svg = format!(
        "{HEAD}<defs><filter id='f'><feOffset dx='1'/></filter></defs>\
<rect width='8' height='8' filter='url(#f)'/></svg>"
    );
    let graph = decode_ok(&svg);
    assert_eq!(graph.root().filters.len(), 1);
    assert!(graph.root().groups.iter().any(|g| g.filter.is_some()));
}

#[test]
fn unreferenced_filter_is_preserved_untouched() {
    let svg = format!(
        "{HEAD}<defs><filter id='f'><feGaussianBlur stdDeviation='1'/></filter></defs>\
<rect width='8' height='8'/></svg>"
    );
    let graph = decode_ok(&svg);
    assert_eq!(graph.source(), svg.as_bytes());
    assert_eq!(graph.root().shapes.len(), 1);
    assert!(graph.root().filters.is_empty() && graph.root().groups.is_empty());
}

// Superseded by WO-RUST-CORPUS-UNSUPPORTED-01: referenced markers now decode.
#[test]
fn referenced_marker_decodes_with_one_instance() {
    let svg = format!(
        "{HEAD}<defs><marker id='m' markerWidth='2' markerHeight='2'><rect width='1' height='1'/></marker></defs>\
<path d='M0 0L8 8' stroke='red' marker-end='url(#m)'/></svg>"
    );
    assert_eq!(decode_ok(&svg).root().shapes.len(), 2);
}

#[test]
fn unsupported_primitives_keep_typed_errors() {
    for (name, kind) in [
        ("feTurbulence", ErrorKind::UnsupportedFeTurbulence),
        ("feDisplacementMap", ErrorKind::UnsupportedFeDisplacementMap),
        ("feConvolveMatrix", ErrorKind::UnsupportedFeConvolveMatrix),
        ("feImage", ErrorKind::UnsupportedFeImage),
        ("feDiffuseLighting", ErrorKind::UnsupportedFeLighting),
        ("feSpecularLighting", ErrorKind::UnsupportedFeLighting),
        ("feMorphology", ErrorKind::UnsupportedFeMorphology),
    ] {
        let svg = format!(
            "{HEAD}<defs><filter id='f'><{name}/></filter></defs>\
<g filter='url(#f)'><rect width='8' height='8'/></g></svg>"
        );
        let error = decode_err(&svg);
        assert_eq!(error.kind, kind, "{name}");
        assert_eq!(
            error.offset,
            svg.find(&format!("<{name}")).unwrap() + 1,
            "{name}"
        );
    }
}
