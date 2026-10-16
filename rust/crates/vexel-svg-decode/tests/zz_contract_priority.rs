//! Preserve original traversal/animation ordering as failing acceptance predicates
//! while the coordinator owns the missing SceneBuilder initialization contracts.
mod support;
use vexel_scene::ErrorKind;
#[test]
fn original_hidden_root_bypasses_authored_transform_and_viewport() {
    // traverse.c sets root_seen, computes display, and exits before either parse.
    let graph = support::decode(
        "<svg display=\"none\" transform=\"broken\" width=\"bad\"><path d=\"broken\"/></svg>",
    )
    .expect("original hidden root succeeds before transform/viewport parsing");
    assert!(graph.root().shapes.is_empty());
}
#[test]
fn original_traversal_error_precedes_animation_track_exhaustion() {
    // decode.c performs the complete traversal before svg_anim_parse.
    let tracks = "<animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/>".repeat(129);
    let source = format!(
        "<svg width=\"8\" height=\"8\"><path d=\"Mbad\"/><rect width=\"1\" height=\"1\">{tracks}</rect></svg>"
    );
    let error = support::decode(&source).unwrap_err();
    assert_eq!(
        error.kind,
        ErrorKind::Path,
        "animation must not replace an earlier traversal error"
    );
    assert_eq!(error.offset, source.find("Mbad").unwrap() + 1);
}
