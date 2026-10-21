//! A native property track whose geometry attribute does not exist on its
//! target element (here `cy` on a path) used to make `render_png` and the GIF
//! export fail with generic text. It is now rejected when created over MCP,
//! and a document that already has such a track reports the cause.

mod common;

use common::{Dir, Rig, element_id, policy_for};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 24,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    }
}

/// Document with a rect (id returned) and a path (id returned).
fn scene(rig: &mut Rig) -> (String, String) {
    let rect = rig.add_rect(10, 10, "#2A7FFF");
    let path = element_id(
        rig.doc_tool(
            "element_add",
            r##""parentId":"root","template":"path","attributes":{"d":"M20 100 Q60 60 100 100 L100 140 Z","fill":"#FF3300"}"##,
        )
        .ok(),
    );
    (rect, path)
}

fn add_track(rig: &mut Rig, id: &str, target: &str, property: &str) -> common::Reply {
    rig.tool(
        "native_timeline_add_property_track",
        &format!(
            r#"{{"documentId":"preview-document","trackId":"{id}","targetId":"{target}","property":"{property}","valueType":"scalar","interpolation":"linear","value":1}}"#
        ),
    )
}

#[test]
fn geometry_track_on_a_path_is_rejected_when_created() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let (rect, path) = scene(&mut rig);
    let reply = add_track(&mut rig, "bob", &path, "cy");
    assert!(reply.is_error);
    assert!(
        reply.error().contains("geometry attribute") && reply.error().contains("transform track"),
        "{}",
        reply.error()
    );
    // cy does not exist on a rect either; x does.
    assert!(add_track(&mut rig, "bad", &rect, "cy").is_error);
    assert!(!add_track(&mut rig, "ok", &rect, "x").is_error);
    // Opacity applies to any element, path included.
    assert!(!add_track(&mut rig, "fade", &path, "opacity").is_error);
}

#[test]
fn existing_geometry_track_on_a_path_names_the_cause_in_png_and_gif() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let (_, path) = scene(&mut rig);
    // Bypass the MCP check, as a file saved by an older build would.
    rig.app
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.add_property_track(
                b"bob",
                path.as_bytes(),
                b"cy",
                b"scalar",
                b"linear",
                b"1",
                b"",
            )
        })
        .unwrap();
    let png = rig.doc_tool("render_png", &format!(r#""path":"{}""#, dir.text("a.png")));
    assert!(
        png.error().starts_with("export png render failed: ")
            && png.error().contains("cy on a path"),
        "{}",
        png.error()
    );
    let gif = rig.doc_tool(
        "export_animation",
        &format!(
            r#""path":"{}","format":"gif","fps":12,"duration":1"#,
            dir.text("a.gif")
        ),
    );
    assert!(
        gif.error()
            .starts_with("export could not render the animation: ")
            && gif.error().contains("cy on a path"),
        "{}",
        gif.error()
    );
    // Removing the offending track makes the same scene render.
    rig.doc_tool("native_timeline_remove_track", r#""trackId":"bob""#)
        .ok();
    rig.doc_tool("render_png", &format!(r#""path":"{}""#, dir.text("b.png")))
        .ok();
}
