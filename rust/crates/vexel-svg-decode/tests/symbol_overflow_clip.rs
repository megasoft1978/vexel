//! Symbol viewport clipping: `<symbol>` defaults to `overflow: hidden`, so content
//! outside the `<use>` viewport is clipped; `overflow="visible"` disables the clip.
mod support;
use support::*;
use vexel_scene::Point;

/// Returns the clip rectangle `[min_x, min_y, max_x, max_y]` for the first shape's clip.
fn first_shape_clip_bounds(source: &str) -> Option<[f64; 4]> {
    let graph = decode(source).unwrap();
    let root = graph.root();
    let clip_id = root.shapes[0].clip?;
    let clip = root.clip(clip_id).unwrap();
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for Point { x, y } in clip.geometry.edges.iter().flat_map(|e| [e.a, e.b]) {
        b[0] = b[0].min(x);
        b[1] = b[1].min(y);
        b[2] = b[2].max(x);
        b[3] = b[3].max(y);
    }
    Some(b)
}

#[test]
fn symbol_content_outside_viewport_is_clipped_when_overflow_unset() {
    let source = r##"<svg width="8" height="8"><defs><symbol id="s" viewBox="0 0 2 2"><rect width="4" height="4"/></symbol></defs><use href="#s" width="2" height="2"/></svg>"##;
    assert_eq!(first_shape_clip_bounds(source), Some([0.0, 0.0, 2.0, 2.0]));
}

#[test]
fn symbol_overflow_hidden_clips_to_use_viewport_at_offset() {
    let source = r##"<svg width="8" height="8"><defs><symbol id="s" overflow="hidden" viewBox="0 0 2 2"><rect width="4" height="4"/></symbol></defs><use href="#s" x="1" y="1" width="2" height="2"/></svg>"##;
    assert_eq!(first_shape_clip_bounds(source), Some([1.0, 1.0, 3.0, 3.0]));
}

#[test]
fn symbol_overflow_visible_disables_clip() {
    let source = r##"<svg width="8" height="8"><defs><symbol id="s" overflow="visible" viewBox="0 0 2 2"><rect width="4" height="4"/></symbol></defs><use href="#s" width="2" height="2"/></svg>"##;
    assert_eq!(first_shape_clip_bounds(source), None);
}

#[test]
fn symbol_overflow_auto_disables_clip() {
    let source = r##"<svg width="8" height="8"><defs><symbol id="s" overflow="auto" viewBox="0 0 2 2"><rect width="4" height="4"/></symbol></defs><use href="#s" width="2" height="2"/></svg>"##;
    assert_eq!(first_shape_clip_bounds(source), None);
}
