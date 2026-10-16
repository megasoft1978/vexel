//! ADR-0029 V1 deferred features through the engine and real Decoder (AC4).
//! Test names state the outcome: `*_fail_closed_*` is a typed `Unsupported`
//! diagnostic at a source offset; `*_preserved_untouched_*` publishes the subset
//! shapes while the unreferenced definition stays source-only; `*_pending_*`
//! is suspended work that is never a success.
mod common;
use common::*;
use vexel_scene::*;

const HEAD: &str = "<svg width='16' height='16'>";

fn assert_fail_closed_at_element(svg: &str, element: &str) {
    let error = decode_err(svg);
    assert_eq!(error.kind, ErrorKind::Unsupported, "{element}: {error:?}");
    // Diagnostics point at the element start `<` or its name (start + 1).
    let start = element_offset(svg, element);
    assert!(
        error.offset == start || error.offset == start + 1,
        "{element}: offset {} not at element start {start}",
        error.offset
    );
    assert!(
        svg.as_bytes()[error.offset..].starts_with(element.as_bytes())
            || svg.as_bytes()[error.offset..].starts_with(format!("<{element}").as_bytes())
    );
}

fn assert_preserved_untouched(svg: &str) {
    let graph = decode_ok(svg);
    let scene = graph.root();
    assert_eq!(scene.shapes.len(), 1, "subset shape still published");
    assert_eq!(graph.source(), svg.as_bytes(), "source bytes untouched");
    assert!(scene.filters.is_empty() && scene.masks.is_empty() && scene.patterns.is_empty());
    assert!(scene.groups.is_empty());
}

#[test]
fn filter_referenced_fail_closed_unsupported_at_filter_element() {
    let svg = format!(
        "{HEAD}<defs><filter id='f'><feGaussianBlur stdDeviation='1'/></filter></defs>\
<rect width='8' height='8' filter='url(#f)'/></svg>"
    );
    assert_fail_closed_at_element(&svg, "filter");
    let svg_group = format!(
        "{HEAD}<defs><filter id='f'><feOffset dx='1'/></filter></defs>\
<g filter='url(#f)'><rect width='8' height='8'/></g></svg>"
    );
    assert_fail_closed_at_element(&svg_group, "filter");
}

#[test]
fn filter_unreferenced_preserved_untouched_with_shapes_published() {
    assert_preserved_untouched(&format!(
        "{HEAD}<defs><filter id='f'><feGaussianBlur stdDeviation='1'/></filter></defs>\
<rect width='8' height='8'/></svg>"
    ));
}

#[test]
fn mask_referenced_fail_closed_unsupported_at_mask_element() {
    let on_rect = format!(
        "{HEAD}<defs><mask id='m'><rect width='4' height='4' fill='white'/></mask></defs>\
<rect width='8' height='8' mask='url(#m)'/></svg>"
    );
    assert_fail_closed_at_element(&on_rect, "mask");
    let on_group = format!(
        "{HEAD}<defs><mask id='m' x='0' y='0' width='16' height='16' maskUnits='userSpaceOnUse'>\
<rect width='4' height='4' fill='white'/></mask></defs>\
<g mask='url(#m)'><rect width='8' height='8'/></g></svg>"
    );
    assert_fail_closed_at_element(&on_group, "mask");
}

#[test]
fn mask_unreferenced_preserved_untouched_with_shapes_published() {
    assert_preserved_untouched(&format!(
        "{HEAD}<defs><mask id='m'><rect width='4' height='4' fill='white'/></mask></defs>\
<rect width='8' height='8'/></svg>"
    ));
}

#[test]
fn marker_referenced_fail_closed_unsupported_at_marker_element() {
    let svg = format!(
        "{HEAD}<defs><marker id='m' markerWidth='2' markerHeight='2'><rect width='1' height='1'/></marker></defs>\
<path d='M0 0L8 8' stroke='red' marker-end='url(#m)'/></svg>"
    );
    assert_fail_closed_at_element(&svg, "marker");
}

#[test]
fn marker_unreferenced_preserved_untouched_with_shapes_published() {
    assert_preserved_untouched(&format!(
        "{HEAD}<defs><marker id='m'><rect width='1' height='1'/></marker></defs>\
<path d='M0 0L8 8' stroke='red'/></svg>"
    ));
}

#[test]
fn text_fail_closed_unsupported_at_text_element() {
    let svg = format!("{HEAD}<rect width='8' height='8'/><text x='1' y='2'>hi</text></svg>");
    assert_fail_closed_at_element(&svg, "text");
    let tspan = format!("{HEAD}<text x='1' y='2'><tspan>hi</tspan></text></svg>");
    assert_fail_closed_at_element(&tspan, "text");
}

#[test]
fn pattern_referenced_pending_never_success_until_resumed() {
    let svg = format!(
        "{HEAD}<defs><pattern id='p' width='4' height='4' patternUnits='userSpaceOnUse'>\
<rect width='2' height='2'/></pattern></defs><rect width='8' height='8' fill='url(#p)'/></svg>"
    );
    let outcome = try_decode_outcome(svg.as_bytes(), &mut Control::default()).unwrap();
    assert!(
        matches!(outcome, Outcome::NeedPattern),
        "referenced pattern must suspend, not complete"
    );
}

#[test]
fn pattern_unreferenced_preserved_untouched_with_shapes_published() {
    assert_preserved_untouched(&format!(
        "{HEAD}<defs><pattern id='p' width='4' height='4' patternUnits='userSpaceOnUse'>\
<rect width='2' height='2'/></pattern></defs><rect width='8' height='8'/></svg>"
    ));
}

#[test]
fn unknown_rendered_element_fail_closed_unsupported_at_element() {
    assert_fail_closed_at_element(
        &format!("{HEAD}<baz a='1'><qux/></baz><rect width='8' height='8'/></svg>"),
        "baz",
    );
}

#[test]
fn external_reference_fail_closed_unsupported_at_value_inside_host_element() {
    let svg = format!("{HEAD}<rect width='8' height='8' fill='url(http://x/y#g)'/></svg>");
    let error = decode_err(&svg);
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(error.offset, svg.find("url(http").unwrap());
}

#[test]
fn style_selector_and_smil_animation_decode_without_panic_and_preserve_source() {
    let style = format!("{HEAD}<style>rect{{fill:red}}</style><rect width='8' height='8'/></svg>");
    let graph = decode_ok(&style);
    assert_eq!(graph.source(), style.as_bytes());
    assert_eq!(graph.root().shapes.len(), 1);
    let anim = format!(
        "{HEAD}<rect width='8' height='8'><animate attributeName='x' from='0' to='4' dur='1s'/></rect></svg>"
    );
    let graph = decode_ok(&anim);
    assert_eq!(graph.source(), anim.as_bytes());
    assert_eq!(graph.root().shapes.len(), 1);
}
