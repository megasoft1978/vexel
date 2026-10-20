//! Native drawing-cel walkthrough over the live JSON-RPC MCP surface: asset
//! import, drawing track, cels, move/remove, undo/redo, save/reopen and native
//! GIF export, plus the specific message of every documented failure mode.
//!
//! `export_animation` is served through `McpHost` and `ExportJob` directly:
//! the shared rig starts every export as a sampled export, while the shell's
//! pump (`mcp_pump.rs`) starts a packed native export when the host returns a
//! `VEXEL_NATIVE_GIF` source, which is what is reproduced here.

mod common;

use common::{Dir, Rig, element_id, policy_for};
use std::sync::mpsc::sync_channel;
use vexel_app::animation::job::ExportJob;
use vexel_app::preview::Operation;
use vexel_app::session::McpOutcome;

const GENERIC: &str = "invalid preview tool arguments";

/// A drawing-asset SVG matching the 512x512 preview document, JSON-escaped.
fn svg(cy: u32) -> String {
    format!(
        r##"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\"><circle cx=\"256\" cy=\"{cy}\" r=\"32\" fill=\"#e74c3c\"/></svg>"##
    )
}

fn ok(rig: &mut Rig, tool: &str, args: &str) -> String {
    rig.doc_tool(&format!("native_timeline_{tool}"), args)
        .ok()
        .to_owned()
}

fn fails(rig: &mut Rig, tool: &str, args: &str, fragment: &str) {
    let reply = rig.doc_tool(&format!("native_timeline_{tool}"), args);
    let error = reply.error().to_owned();
    assert_ne!(error, GENERIC, "{tool} {args}");
    assert!(
        error.contains(fragment),
        "{tool} {args}: `{error}` lacks `{fragment}`"
    );
}

fn add_ball(rig: &mut Rig) -> String {
    let reply = rig.doc_tool(
        "element_add",
        r##""parentId":"root","template":"ellipse","attributes":{"cx":"256","cy":"140","rx":"32","ry":"32","fill":"#e74c3c"}"##,
    );
    element_id(reply.ok())
}

fn add_asset(rig: &mut Rig, name: &str, cy: u32) {
    ok(
        rig,
        "add_drawing_asset",
        &format!(r#""assetName":"{name}","svg":"{}""#, svg(cy)),
    );
}

#[test]
fn v1_cel_walkthrough_draw_cels_save_export() {
    let grant = Dir::new();
    let project = grant.text("ball.vexel");
    let gif = grant.text("ball.gif");
    let mut rig = Rig::new(policy_for(&[&grant.0]));

    let ball = add_ball(&mut rig);
    assert_eq!(ball, "vexel-object-1");
    for (name, cy) in [("ball-high", 140), ("ball-mid", 256), ("ball-low", 380)] {
        add_asset(&mut rig, name, cy);
    }
    ok(
        &mut rig,
        "add_track",
        &format!(r#""trackId":"ball","targetId":"{ball}","assetName":"ball-high","name":"Ball""#),
    );
    ok(
        &mut rig,
        "set_cel",
        r#""trackId":"ball","tick":8,"assetName":"ball-mid","label":"mid""#,
    );
    ok(
        &mut rig,
        "set_cel",
        r#""trackId":"ball","tick":16,"assetName":"ball-low""#,
    );
    let cels = ok(&mut rig, "get", "");
    for needle in [
        r#""id":"ball""#,
        r#""kind":"drawing""#,
        r#""tick":0"#,
        r#""tick":8"#,
        r#""tick":16"#,
        r#""asset":"ball-high""#,
        r#""asset":"ball-mid""#,
        r#""asset":"ball-low""#,
        r#""durationTicks":48"#,
    ] {
        assert!(cels.contains(needle), "{needle} missing: {cels}");
    }

    // Move the last cel, then remove and re-add the middle one.
    ok(
        &mut rig,
        "move_item",
        r#""trackId":"ball","fromTick":16,"toTick":20"#,
    );
    let moved = ok(&mut rig, "get", "");
    assert!(moved.contains(r#""tick":20"#) && !moved.contains(r#""tick":16"#));
    ok(&mut rig, "remove_cel", r#""trackId":"ball","tick":8"#);
    assert!(!ok(&mut rig, "get", "").contains(r#""tick":8"#));
    ok(
        &mut rig,
        "set_cel",
        r#""trackId":"ball","tick":8,"assetName":"ball-mid""#,
    );
    let readded = ok(&mut rig, "get", "");
    assert!(readded.contains(r#""tick":8"#));

    // Undo the re-add, then redo it; the timeline round-trips exactly.
    ok(&mut rig, "undo", "");
    assert!(!ok(&mut rig, "get", "").contains(r#""tick":8"#));
    ok(&mut rig, "redo", "");
    assert_eq!(ok(&mut rig, "get", ""), readded);

    // Save as, reopen in a fresh host: assets and cels survive.
    assert!(
        rig.doc_tool("save_as", &format!(r#""path":"{project}""#))
            .ok()
            .contains(r#""operation":"save_as""#)
    );
    let mut fresh = Rig::new(policy_for(&[&grant.0]));
    fresh
        .tool("document_open", &format!(r#"{{"path":"{project}"}}"#))
        .ok();
    assert_eq!(ok(&mut fresh, "get", ""), readded, "reopened timeline");

    // Native GIF export: one frame per tick of the 48-tick loop at 4 cs.
    let outcome = rig
        .host
        .handle_with(
            &mut rig.app,
            Operation::Export {
                path: gif.clone(),
                fps: 12,
                duration: 2.0,
            },
            &mut rig.driver,
        )
        .unwrap();
    let McpOutcome::Export { source, fps: 0, .. } = outcome else {
        panic!("a native timeline selects the native export");
    };
    let (tx, rx) = sync_channel(1);
    let mut job =
        ExportJob::spawn_native_packed(source, rig.host.policy().clone(), gif.clone().into(), tx)
            .unwrap();
    for _ in 0..4000 {
        if job.poll().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let result = rx.recv().unwrap().unwrap();
    assert!(
        result.contains("\"frames\":48") && result.contains("\"delayCs\":4"),
        "{result}"
    );
    let bytes = std::fs::read(&gif).unwrap();
    assert_eq!(&bytes[..6], b"GIF89a");
    let frames = bytes
        .windows(3)
        .filter(|w| *w == [0x21, 0xF9, 0x04])
        .count();
    assert_eq!(frames, 48);
}

#[test]
fn v1_cel_failure_modes_have_specific_messages() {
    let grant = Dir::new();
    let mut rig = Rig::new(policy_for(&[&grant.0]));
    let ball = add_ball(&mut rig);
    add_asset(&mut rig, "ball-high", 140);
    ok(
        &mut rig,
        "add_track",
        &format!(r#""trackId":"ball","targetId":"{ball}","assetName":"ball-high""#),
    );
    let before = rig.state();

    // Drawing-asset SVG rejections.
    let asset = |svg: &str| format!(r#""assetName":"bad","svg":"{svg}""#);
    for (svg, fragment) in [
        (
            r#"<circle cx=\"1\" cy=\"1\" r=\"1\"/>"#,
            "root must be <svg>",
        ),
        (
            r#"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\"><circle r=\"1\"/></svg>"#,
            "width, height and viewBox",
        ),
        (
            r#"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 512 512\"><circle r=\"1\"/></svg>"#,
            "width, height and viewBox",
        ),
        (
            r#"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\" viewBox=\"0 0 100 100\"><circle r=\"1\"/></svg>"#,
            "must equal the document size",
        ),
        (
            r#"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\"><script>1</script></svg>"#,
            "<script>",
        ),
        (
            r#"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\"><image xlink:href=\"http://example.com/a.png\"/></svg>"#,
            "external resources",
        ),
        (r#"<svg width=\"512\""#, "not well-formed XML"),
    ] {
        fails(&mut rig, "add_drawing_asset", &asset(svg), fragment);
    }
    fails(
        &mut rig,
        "add_drawing_asset",
        &format!(r#""assetName":"ball-high","svg":"{}""#, svg(1)),
        "assetName already exists",
    );
    fails(
        &mut rig,
        "add_drawing_asset",
        &format!(r#""assetName":"bad name","svg":"{}""#, svg(1)),
        "assetName must be 1-128 ASCII letters",
    );

    // Track and cel rejections.
    fails(
        &mut rig,
        "add_track",
        &format!(r#""trackId":"t2","targetId":"{ball}","assetName":"missing""#),
        "assetName is not an existing asset",
    );
    fails(
        &mut rig,
        "add_track",
        r#""trackId":"t2","targetId":"nope","assetName":"ball-high""#,
        "targetId is not an element id",
    );
    fails(
        &mut rig,
        "add_track",
        &format!(r#""trackId":"ball","targetId":"{ball}","assetName":"ball-high""#),
        "trackId already exists",
    );
    fails(
        &mut rig,
        "add_track",
        &format!(r#""trackId":"bad id","targetId":"{ball}","assetName":"ball-high""#),
        "trackId must be 1-128 ASCII letters",
    );
    fails(
        &mut rig,
        "set_cel",
        r#""trackId":"ball","tick":48,"assetName":"ball-high""#,
        "tick must be an integer from 0 through durationTicks-1",
    );
    fails(
        &mut rig,
        "set_cel",
        r#""trackId":"ball","tick":1,"assetName":"missing""#,
        "assetName is not an existing asset",
    );
    fails(
        &mut rig,
        "set_cel",
        r#""trackId":"ghost","tick":1,"assetName":"ball-high""#,
        "trackId is not an existing track",
    );
    fails(
        &mut rig,
        "remove_cel",
        r#""trackId":"ball","tick":3"#,
        "tick has no cel",
    );
    fails(
        &mut rig,
        "remove_cel",
        r#""trackId":"ball","tick":0"#,
        "cannot remove the last cel",
    );
    ok(
        &mut rig,
        "set_cel",
        r#""trackId":"ball","tick":4,"assetName":"ball-high""#,
    );
    fails(
        &mut rig,
        "move_item",
        r#""trackId":"ball","fromTick":0,"toTick":9"#,
        "fromTick 0 cannot be moved",
    );
    fails(
        &mut rig,
        "move_item",
        r#""trackId":"ball","fromTick":4,"toTick":48"#,
        "toTick must be an integer from 0 through durationTicks-1",
    );
    fails(
        &mut rig,
        "move_item",
        r#""trackId":"ball","fromTick":5,"toTick":9"#,
        "fromTick has no item",
    );
    fails(
        &mut rig,
        "move_item",
        r#""trackId":"ball","fromTick":4,"toTick":0"#,
        "toTick is occupied",
    );

    // Argument failures name the tool, the argument and the accepted form.
    fails(
        &mut rig,
        "set_cel",
        r#""trackId":"ball","tick":4294967296,"assetName":"ball-high""#,
        "native_timeline_set_cel argument tick is missing or invalid; use an integer",
    );
    fails(
        &mut rig,
        "set_cel",
        r#""trackId":"","tick":1,"assetName":"ball-high""#,
        "argument trackId is missing or invalid",
    );
    fails(
        &mut rig,
        "add_drawing_asset",
        r#""assetName":"x","svg":"""#,
        "argument svg is missing or invalid; use a complete SVG string",
    );
    fails(
        &mut rig,
        "set_property_key",
        r#""trackId":"ball","tick":1,"value":1"#,
        "trackId must be a property track",
    );

    // Caller text is echoed only as [A-Za-z0-9_.:-], at most 32 characters.
    let hostile = "x<>\\\"\\n&y".repeat(40);
    let reply = rig.doc_tool(
        "native_timeline_set_cel",
        &format!(r#""trackId":"ball","tick":1,"assetName":"{hostile}""#),
    );
    let error = reply.error();
    assert!(error.contains("argument assetName `"), "{error}");
    let echoed = error.split('`').nth(1).unwrap();
    assert!(echoed.len() <= 32 && echoed.bytes().all(|b| b.is_ascii_alphanumeric()));
    assert!(!error.contains('<') && !error.contains('&'), "{error}");

    // Rejected calls changed nothing beyond the one accepted set_cel.
    assert_eq!(rig.state().source, before.source);
}
