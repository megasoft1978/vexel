//! WO-RUST-SVG-COMPAT-01: namespaced attributes, separator-free transforms,
//! symbol overflow, symbol without viewBox, and use without width/height.
mod support;
use support::*;
use vexel_scene::{ErrorKind, IgnoredKind};
use vexel_svg_decode::parse_transform;
use vexel_xml::XmlSpan;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const SODIPODI_NS: &str = "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd";

fn span(text: &str) -> XmlSpan {
    XmlSpan {
        offset: 0,
        length: text.len(),
    }
}

fn transform_ok(text: &str) -> bool {
    parse_transform(text.as_bytes(), span(text)).is_ok()
}

fn same_geometry(a: &str, b: &str) {
    let a = decode(a).unwrap();
    let b = decode(b).unwrap();
    assert_eq!(a.root().edges, b.root().edges);
    assert_eq!(a.root().edge_colors, b.root().edge_colors);
    assert_eq!(a.root().shapes.len(), b.root().shapes.len());
}

// Fix 1: namespaced attributes.

#[test]
fn foreign_prefixed_shape_attributes_are_ignored_like_plain_ones() {
    let with = format!(
        r##"<svg xmlns:sodipodi="{SODIPODI_NS}" width="8" height="8"><circle sodipodi:cx="5" sodipodi:r="2" sodipodi:fill="red" cx="2" cy="2" r="1" fill="blue"/></svg>"##
    );
    let without = r##"<svg width="8" height="8"><circle cx="2" cy="2" r="1" fill="blue"/></svg>"##;
    same_geometry(&with, without);
    let g = decode(&with).unwrap();
    assert_eq!(g.root().ignored(IgnoredKind::UnknownAttribute).count, 0);
}

#[test]
fn foreign_prefixed_common_attribute_is_ignored_on_group_and_path() {
    let with = format!(
        r##"<svg xmlns:i="{SODIPODI_NS}" width="8" height="8"><g i:transform="scale(9)"><path i:fill="red" d="M1 1 L4 1 L4 4 Z"/></g></svg>"##
    );
    let without = r##"<svg width="8" height="8"><g><path d="M1 1 L4 1 L4 4 Z"/></g></svg>"##;
    same_geometry(&with, without);
}

#[test]
fn foreign_binding_declared_on_ancestor_is_ignored() {
    let with = format!(
        r##"<svg xmlns:s="{SODIPODI_NS}" width="8" height="8"><g><circle s:r="9" cx="2" cy="2" r="1"/></g></svg>"##
    );
    let without = r##"<svg width="8" height="8"><g><circle cx="2" cy="2" r="1"/></g></svg>"##;
    same_geometry(&with, without);
}

#[test]
fn nearest_foreign_binding_wins_over_outer_svg_binding() {
    let with = format!(
        r##"<svg xmlns:s="{SVG_NS}" width="8" height="8"><g xmlns:s="{SODIPODI_NS}"><circle s:r="9" cx="2" cy="2" r="1"/></g></svg>"##
    );
    let without = r##"<svg width="8" height="8"><g><circle cx="2" cy="2" r="1"/></g></svg>"##;
    same_geometry(&with, without);
}

#[test]
fn svg_bound_prefix_still_rejects_shape_attribute() {
    let source = format!(
        r##"<svg xmlns:sodipodi="{SVG_NS}" width="8" height="8"><circle sodipodi:r="2" cx="2" cy="2" r="1"/></svg>"##
    );
    assert_eq!(decode(&source).unwrap_err().kind, ErrorKind::Unsupported);
}

#[test]
fn nearest_svg_rebinding_still_rejects_shape_attribute() {
    let source = format!(
        r##"<svg xmlns:s="{SODIPODI_NS}" width="8" height="8"><g xmlns:s="{SVG_NS}"><circle s:r="2" cx="2" cy="2" r="1"/></g></svg>"##
    );
    assert_eq!(decode(&source).unwrap_err().kind, ErrorKind::Unsupported);
}

#[test]
fn unbound_prefix_still_rejects_shape_and_common_attributes() {
    for source in [
        r##"<svg width="8" height="8"><circle svg:r="2" cx="2" cy="2" r="1"/></svg>"##,
        r##"<svg width="8" height="8"><circle sodipodi:cx="2" cx="2" cy="2" r="1"/></svg>"##,
        r##"<svg width="8" height="8"><circle sodipodi:fill="red" cx="2" cy="2" r="1"/></svg>"##,
    ] {
        assert_eq!(
            decode(source).unwrap_err().kind,
            ErrorKind::Unsupported,
            "{source}"
        );
    }
}

#[test]
fn foreign_xlink_href_on_shape_still_rejects() {
    let source = r##"<svg xmlns:xlink="urn:x" width="8" height="8"><rect xlink:href="#a" width="2" height="2"/></svg>"##;
    assert_eq!(decode(source).unwrap_err().kind, ErrorKind::Unsupported);
}

#[test]
fn foreign_prefix_with_dotted_local_name_is_ignored() {
    let with = format!(
        r##"<svg xmlns:s="{SODIPODI_NS}" width="8" height="8"><circle s:cx.x="1" cx="2" cy="2" r="1"/></svg>"##
    );
    let without = r##"<svg width="8" height="8"><circle cx="2" cy="2" r="1"/></svg>"##;
    same_geometry(&with, without);
}

#[test]
fn nested_colon_names_are_malformed_xml_and_refused() {
    let source = format!(
        r##"<svg xmlns:s="{SODIPODI_NS}" width="8" height="8"><circle s:a:b="1" cx="2" cy="2" r="1"/></svg>"##
    );
    assert!(decode(&source).is_err());
}

#[test]
fn foreign_binding_with_entity_reference_fails_closed() {
    let source = format!(
        r##"<svg xmlns:s="http://www.w3.org/&#50;000/svg" width="8" height="8"><circle s:r="2" cx="2" cy="2" r="1"/></svg>"##
    );
    assert_eq!(decode(&source).unwrap_err().kind, ErrorKind::Unsupported);
}

// Fix 2: separator-free transform lists.

#[test]
fn separator_free_transform_list_equals_spaced_composition() {
    let with = r##"<svg width="8" height="8"><g transform="translate(1 2)scale(3 4)"><rect width="1" height="1"/></g></svg>"##;
    let spaced = r##"<svg width="8" height="8"><g transform="translate(1 2) scale(3 4)"><rect width="1" height="1"/></g></svg>"##;
    same_geometry(with, spaced);
}

#[test]
fn separator_free_matplotlib_style_transform_parses() {
    assert!(transform_ok("translate(1 2)scale(3 4)rotate(10)"));
    assert!(transform_ok("matrix(1 0 0 1 0 0)translate(5 5)"));
    assert!(transform_ok("translate(1)scale(2)"));
}

#[test]
fn separator_free_transform_adversarial_inputs_are_rejected() {
    for text in [
        "translate(1)5",
        "translate(1)-",
        "translate(1)(",
        "translate(1)xyz(",
        "translate(1)scale",
        "translate(1)scale(2",
        "translate(1)scale(2))",
        "translate(1)@scale(2)",
        "translate(1)scale(2)5",
        "translate(1).scale(2)",
    ] {
        assert!(!transform_ok(text), "accepted {text}");
    }
}

#[test]
fn separator_free_chain_beyond_operation_cap_is_refused() {
    let chain = "scale(1)".repeat(65);
    assert!(!transform_ok(&chain));
    assert!(transform_ok(&"scale(1)".repeat(64)));
}

// Fix 3: symbol overflow, symbol without viewBox, use without width/height.

#[test]
fn symbol_overflow_visible_and_hidden_render() {
    for value in ["visible", "hidden", "scroll", "auto"] {
        let source = format!(
            r##"<svg width="8" height="8"><symbol id="s" overflow="{value}" viewBox="0 0 2 2"><rect width="1" height="1"/></symbol><use href="#s" width="4" height="4"/></svg>"##
        );
        let g = decode(&source).unwrap_or_else(|e| panic!("{value}: {e:?}"));
        assert_eq!(g.root().shapes.len(), 1, "{value}");
    }
}

#[test]
fn symbol_overflow_invalid_values_are_rejected() {
    for value in ["foo", "", "inherit", "VISIBLE x"] {
        let source = format!(
            r##"<svg width="8" height="8"><symbol id="s" overflow="{value}" viewBox="0 0 2 2"><rect width="1" height="1"/></symbol><use href="#s" width="4" height="4"/></svg>"##
        );
        assert_eq!(
            decode(&source).unwrap_err().kind,
            ErrorKind::Unsupported,
            "{value}"
        );
    }
}

#[test]
fn overflow_on_non_symbol_elements_is_unchanged() {
    for source in [
        r##"<svg width="8" height="8"><rect width="1" height="1" overflow="visible"/></svg>"##,
        r##"<svg width="8" height="8"><g overflow="hidden"><rect width="1" height="1"/></g></svg>"##,
        r##"<svg width="8" height="8" overflow="visible"><rect width="1" height="1"/></svg>"##,
    ] {
        assert_eq!(
            decode(source).unwrap_err().kind,
            ErrorKind::Unsupported,
            "{source}"
        );
    }
}

#[test]
fn symbol_without_viewbox_maps_by_identity_at_use_origin() {
    let with = r##"<svg width="8" height="8"><defs><symbol id="s"><rect width="1" height="1"/></symbol></defs><use href="#s" x="2" y="3" width="4" height="4"/></svg>"##;
    let without = r##"<svg width="8" height="8"><g transform="translate(2 3)"><rect width="1" height="1"/></g></svg>"##;
    same_geometry(with, without);
}

#[test]
fn use_without_width_or_height_defaults_to_full_viewport() {
    let with = r##"<svg width="8" height="8"><defs><symbol id="s" viewBox="0 0 2 2"><rect width="1" height="1"/></symbol></defs><use href="#s"/></svg>"##;
    let explicit = r##"<svg width="8" height="8"><defs><symbol id="s" viewBox="0 0 2 2"><rect width="1" height="1"/></symbol></defs><use href="#s" width="8" height="8"/></svg>"##;
    same_geometry(with, explicit);
}

#[test]
fn huge_use_width_is_refused_by_validity_check() {
    let source = r##"<svg width="8" height="8"><defs><symbol id="s" viewBox="0 0 2 2"><rect width="1" height="1"/></symbol></defs><use href="#s" width="1e308" height="4"/></svg>"##;
    assert_eq!(decode(source).unwrap_err().kind, ErrorKind::Unsupported);
}

#[test]
fn symbol_without_viewbox_and_without_use_size_renders() {
    let source = r##"<svg width="8" height="8"><defs><symbol id="s"><rect width="1" height="1"/></symbol></defs><use href="#s"/></svg>"##;
    assert_eq!(decode(source).unwrap().root().shapes.len(), 1);
}
