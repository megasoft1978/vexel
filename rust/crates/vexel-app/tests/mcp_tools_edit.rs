//! Routed editing tools driven through the real JSON-RPC stdio layer against
//! the one shared `AppDocument` (AC 1-3, 7).

mod common;

use common::{Reply, Rig, State, no_grants};
use vexel_app::animation::{driver::Driver, playback::MonotonicClock};
use vexel_app::session::AppDocument;
use vexel_v1_document::V1Document;

const ROUTED: [&str; 6] = [
    "document_open",
    "attribute_set",
    "element_delete",
    "save",
    "save_as",
    "export_svg",
];
const ORIGINAL: [&str; 9] = [
    "document_list",
    "document_info",
    "element_add",
    "undo",
    "redo",
    "animation_list",
    "animation_add",
    "animation_control",
    "export_animation",
];
const ID1: &str = r#""elementId":"vexel-object-1""#;
const GENERIC: &str = "invalid preview tool arguments";
const BAD_COLOR: &str = "invalid preview fill color";
const LINE_OK: &str = r##""x1":"10","y1":"10","x2":"100","y2":"90","stroke":"#ff0000""##;

fn rig() -> Rig {
    Rig::new(no_grants())
}

fn set(name: &str, value: &str) -> String {
    format!(r#"{ID1},"name":"{name}","value":"{value}""#)
}

fn line_args(attributes: &str) -> String {
    format!(r#""parentId":"root","template":"line","attributes":{{{attributes}}}"#)
}

fn path_args(attributes: &str) -> String {
    format!(r#""parentId":"root","template":"path","attributes":{{{attributes}}}"#)
}

/// Runs `tool`, checks the exact bytes against an independent document that
/// applied `expected`, then undo and redo (full source bytes each time).
fn check_edit(
    rig: &mut Rig,
    tool: &str,
    extra: &str,
    expected: impl FnOnce(&mut V1Document),
) -> Reply {
    let before = rig.state();
    let mut model = V1Document::from_source(&before.source).unwrap();
    expected(&mut model);
    let want = model.source().unwrap();
    assert_ne!(want, before.source, "{tool} must change the bytes");

    let reply = rig.doc_tool(tool, extra);
    assert!(!reply.is_error, "{tool}: {}", reply.body);
    let after = rig.state();
    assert_eq!(after.source, want, "{tool} bytes");
    assert!(after.dirty);
    assert_eq!(after.revision, before.revision + 1, "{tool} revision");
    assert_eq!(after.history, before.history + 1, "{tool} one undo entry");

    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, before.source, "{tool} undo restores");
    rig.doc_tool("redo", "").ok();
    assert_eq!(rig.state().source, after.source, "{tool} redo reapplies");
    reply
}

/// A rejected call returns exactly `error` and changes nothing.
fn rejects(rig: &mut Rig, tool: &str, extra: &str, error: &str) {
    let before: State = rig.state();
    let reply = rig.doc_tool(tool, extra);
    assert_eq!(reply.error(), error, "{tool} {extra}");
    assert_eq!(rig.state(), before, "{tool} {extra} must not change state");
}

#[test]
fn library_target_links_and_lists_routed_tools() {
    let app = AppDocument::new(10, 10).unwrap();
    assert_eq!(app.revision(), 0);
    let mut rig = rig();
    let names = rig.list_tools();
    for name in ROUTED.iter().chain(&ORIGINAL) {
        assert!(names.iter().any(|n| n == name), "missing {name}");
    }
    // render_png is routed since WO-RUST-V1-PNG-01.
    // Eleven native_timeline_* tools are routed since the V1 native MCP slice.
    // Twenty-three document, paint, query and timeline-edit tools follow the V1 advanced-animation work.
    // Plus native_timeline_set_duration, element_reparent and batch, plus
    // element_add_fragment and use_add.
    assert_eq!(
        names.len(),
        ROUTED.len() + ORIGINAL.len() + 1 + 11 + 23 + 1 + 1 + 1 + 2
    );
    assert!(names.iter().any(|n| n == "render_png"));
}

#[test]
fn render_png_without_required_arguments_is_a_typed_error() {
    let mut rig = rig();
    let before = rig.state();
    let reply = rig.doc_tool("render_png", "");
    assert!(reply.is_error);
    assert_eq!(
        reply.body,
        r#"{"error":{"code":"invalid_arguments","message":"render_png requires a path"}}"#
    );
    assert_eq!(rig.state(), before);
}

#[test]
fn attribute_set_fill_stroke_width_and_translate() {
    let mut rig = rig();
    assert_eq!(rig.add_rect(10, 20, "#2A7FFF"), "vexel-object-1");

    let reply = check_edit(&mut rig, "attribute_set", &set("fill", "#112233"), |d| {
        d.set_fill("vexel-object-1", "#112233").unwrap();
    });
    assert_eq!(
        reply.body,
        r#"{"ok":true,"operation":"attribute_set","elementId":"vexel-object-1","dirty":true}"#
    );
    // check_edit leaves the edit applied (redone), so the next steps build on it.

    // Stroke keeps the current width (none yet: the SVG default of 1).
    check_edit(&mut rig, "attribute_set", &set("stroke", "#445566"), |d| {
        d.set_stroke("vexel-object-1", "#445566", 1.0).unwrap();
    });
    check_edit(&mut rig, "attribute_set", &set("stroke-width", "5"), |d| {
        d.set_stroke("vexel-object-1", "#445566", 5.0).unwrap();
    });
    // A later stroke change keeps the width given above.
    check_edit(&mut rig, "attribute_set", &set("stroke", "#778899"), |d| {
        d.set_stroke("vexel-object-1", "#778899", 5.0).unwrap();
    });
}

#[test]
fn attribute_set_translate_both_spellings() {
    for value in [
        "translate(10 20)",
        "translate(10,20)",
        "translate( 10 , 20 )",
    ] {
        let mut rig = rig();
        rig.add_rect(10, 20, "#2A7FFF");
        check_edit(&mut rig, "attribute_set", &set("transform", value), |d| {
            d.translate("vexel-object-1", 10.0, 20.0).unwrap();
        });
    }
}

#[test]
fn attribute_set_changes_only_the_named_element() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    rig.add_rect(100, 100, "#222222");
    let before = rig.state().source;
    let args = format!(
        r#""elementId":"vexel-object-2","name":"fill","value":"{}""#,
        "#abcdef"
    );
    rig.doc_tool("attribute_set", &args).ok();
    let after = rig.state().source;
    let parts = |s: &[u8]| {
        String::from_utf8(s.to_vec())
            .unwrap()
            .split("<rect")
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let (old, new) = (parts(&before), parts(&after));
    assert_eq!(old.len(), 3);
    assert_eq!(old[0], new[0]);
    assert_eq!(old[1], new[1]);
    assert_eq!(old[2].replace("#222222", "#abcdef"), new[2]);
}

#[test]
fn attribute_set_rejections() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    let long = "x".repeat(65);
    let cases: Vec<(&str, String, &str)> = vec![
        ("fill", "#12".into(), BAD_COLOR),
        ("fill", "red".into(), BAD_COLOR),
        ("stroke", "red".into(), BAD_COLOR),
        ("fill", format!("#{long}"), GENERIC),
        // opacity is accepted now (widened attribute_set); out of range is not.
        ("opacity", "2".into(), "unsupported attribute"),
        ("id", "other".into(), "unsupported attribute"),
        ("stroke-width", "257".into(), GENERIC),
        ("stroke-width", "-1".into(), GENERIC),
        ("stroke-width", "NaN".into(), GENERIC),
        // No stroke colour exists yet, so a width alone has nothing to widen.
        ("stroke-width", "2".into(), GENERIC),
        ("transform", "rotate(5)".into(), "unsupported attribute"),
        ("transform", "translate(1)".into(), "unsupported attribute"),
        (
            "transform",
            "translate(1 2 3)".into(),
            "unsupported attribute",
        ),
        (
            "transform",
            "translate(1,2,3)".into(),
            "unsupported attribute",
        ),
        (
            "transform",
            "translate(1e999 0)".into(),
            "unsupported attribute",
        ),
        (
            "transform",
            "translate(NaN 0)".into(),
            "unsupported attribute",
        ),
        (
            "transform",
            "translate(100001 0)".into(),
            "unsupported attribute",
        ),
        (
            "transform",
            "translate(1 2) rotate(3)".into(),
            "unsupported attribute",
        ),
        ("transform", long.clone(), GENERIC),
    ];
    for (name, value, error) in &cases {
        rejects(&mut rig, "attribute_set", &set(name, value), error);
    }
    let unknown = format!(
        r#""elementId":"vexel-object-99","name":"fill","value":"{}""#,
        "#112233"
    );
    rejects(&mut rig, "attribute_set", &unknown, "unknown element");
    let long_id = format!(
        r#""elementId":"{}","name":"fill","value":"{}""#,
        "e".repeat(129),
        "#112233"
    );
    rejects(&mut rig, "attribute_set", &long_id, GENERIC);
    // Wrong documentId.
    let before = rig.state();
    let reply = rig.tool(
        "attribute_set",
        &format!(r#"{{"documentId":"nope",{}}}"#, set("fill", "#000000")),
    );
    assert_eq!(reply.error(), "unknown preview document");
    assert_eq!(rig.state(), before);
}

#[test]
fn element_delete_removes_one_element() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    rig.add_rect(100, 100, "#222222");
    let reply = check_edit(&mut rig, "element_delete", ID1, |d| {
        d.delete("vexel-object-1").unwrap();
    });
    assert_eq!(
        reply.body,
        r#"{"ok":true,"operation":"element_delete","elementId":"vexel-object-1","dirty":true}"#
    );
}

#[test]
fn element_delete_rejections() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    rejects(
        &mut rig,
        "element_delete",
        r#""elementId":"vexel-object-7""#,
        "unknown element",
    );
    let long = format!(r#""elementId":"{}""#, "e".repeat(129));
    rejects(&mut rig, "element_delete", &long, GENERIC);
    // The root carries no stable id, so it cannot be named at all.
    rejects(
        &mut rig,
        "element_delete",
        r#""elementId":"svg""#,
        "unknown element",
    );
    let before = rig.state();
    let reply = rig.tool(
        "element_delete",
        r#"{"documentId":"nope","elementId":"vexel-object-1"}"#,
    );
    assert_eq!(reply.error(), "unknown preview document");
    assert_eq!(rig.state(), before);
}

#[test]
fn element_add_line_and_path_use_stable_ids() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    let line = format!(r#"{LINE_OK},"stroke-width":"3""#);
    let reply = check_edit(&mut rig, "element_add", &line_args(&line), |d| {
        d.add_line_with_id(10.0, 10.0, 100.0, 90.0, "#ff0000", 3.0, "vexel-object-2")
            .unwrap();
    });
    assert!(
        reply.body.contains(r#""operation":"element_add""#),
        "{}",
        reply.body
    );
    assert!(reply.body.contains(r#""elementId":"vexel-object-2""#));
    assert!(reply.body.contains(r#""dirty":true"#));

    let path = r##""d":"M 10 10 L 50 10 L 50 50 Z","fill":"#00ff00","stroke":"#000000","stroke-width":"2""##;
    let reply = check_edit(&mut rig, "element_add", &path_args(path), |d| {
        d.add_path_with_id(
            "M 10 10 L 50 10 L 50 50 Z",
            "#00ff00",
            "#000000",
            2.0,
            "vexel-object-3",
        )
        .unwrap();
    });
    assert!(
        reply.body.contains(r#""elementId":"vexel-object-3""#),
        "{}",
        reply.body
    );

    // Defaults: stroke width 1, fill and stroke none.
    check_edit(
        &mut rig,
        "element_add",
        &path_args(r#""d":"M 5 5 L 20 20""#),
        |d| {
            d.add_path_with_id("M 5 5 L 20 20", "none", "none", 1.0, "vexel-object-4")
                .unwrap();
        },
    );
}

#[test]
fn element_add_line_and_path_rejections() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    let big_d = format!(
        "M 0 0 {}",
        "L 1 1 ".repeat(vexel_v1_document::MAX_PATH_DATA_BYTES / 6 + 1)
    );
    assert!(big_d.len() > vexel_v1_document::MAX_PATH_DATA_BYTES);
    let lines: Vec<String> = vec![
        r##""x1":"NaN","y1":"1","x2":"5","y2":"5","stroke":"#ff0000""##.into(),
        r##""x1":"inf","y1":"1","x2":"5","y2":"5","stroke":"#ff0000""##.into(),
        r##""x1":"1","y1":"1","x2":"5","y2":"5","stroke":"#ff0000","stroke-width":"300""##.into(),
        r##""x1":"1","y1":"1","x2":"5","y2":"5","stroke":"#ff0000","fill":"#000000""##.into(),
        r#""x1":"1","y1":"1","x2":"5","y2":"5""#.into(),
        // Outside the canvas and a zero-length line fail document validation.
        r##""x1":"1","y1":"1","x2":"9999","y2":"5","stroke":"#ff0000""##.into(),
        r##""x1":"4","y1":"4","x2":"4","y2":"4","stroke":"#ff0000""##.into(),
        format!(
            r##""x1":"{}","y1":"1","x2":"5","y2":"5","stroke":"#ff0000""##,
            "9".repeat(40)
        ),
    ];
    for attributes in &lines {
        // An unknown attribute key now gets an actionable error naming the key.
        let want = if attributes.contains(r#""fill""#) {
            r#"element_add line does not accept attribute \"fill\"; allowed: x1, y1, x2, y2, stroke, stroke-width"#
        } else {
            GENERIC
        };
        rejects(&mut rig, "element_add", &line_args(attributes), want);
    }
    rejects(
        &mut rig,
        "element_add",
        &line_args(r#""x1":"1","y1":"1","x2":"5","y2":"5","stroke":"red""#),
        BAD_COLOR,
    );
    let paths: Vec<(String, &str)> = vec![
        (format!(r#""d":"{big_d}""#), GENERIC),
        (r#""d":"""#.into(), GENERIC),
        (r#""d":"M 0 0 A 1 1 0 0 0 2 2""#.into(), GENERIC),
        (r#""d":"M 0 0 L""#.into(), GENERIC),
        (r#""d":"M 0 0 L 5 5","fill":"red""#.into(), BAD_COLOR),
        (
            format!(r#""d":"M 0 0 L 5 5","stroke":"{}""#, "#12"),
            BAD_COLOR,
        ),
        (r#""d":"M 0 0 L 5 5","stroke-width":"-3""#.into(), GENERIC),
        (
            r#""d":"M 0 0 L 5 5","x":"1""#.into(),
            r#"element_add path does not accept attribute \"x\"; allowed: d, fill, stroke, stroke-width, stroke-linecap, stroke-linejoin, fill-rule"#,
        ),
        (r#""fill":"none""#.into(), GENERIC),
    ];
    for (attributes, error) in &paths {
        rejects(&mut rig, "element_add", &path_args(attributes), error);
    }
    // circle, g and text stay rejected.
    for template in ["circle", "g", "text"] {
        let before = rig.state();
        let reply = rig.doc_tool(
            "element_add",
            &format!(r#""parentId":"root","template":"{template}","attributes":{{}}"#),
        );
        assert!(reply.is_error, "{template}");
        assert_eq!(rig.state(), before);
    }
    // A parent that is not a group is refused with an actionable message.
    let extra =
        format!(r#""parentId":"vexel-object-1","template":"line","attributes":{{{LINE_OK}}}"#);
    rejects(
        &mut rig,
        "element_add",
        &extra,
        r#"element_add parentId must be \"root\" or a group id; create a group with element_group"#,
    );
    // Wrong documentId.
    let before = rig.state();
    let reply = rig.tool(
        "element_add",
        &format!(r#"{{"documentId":"nope",{}}}"#, line_args(LINE_OK)),
    );
    assert_eq!(reply.error(), "unknown preview document");
    assert_eq!(rig.state(), before);
}

#[test]
fn wrong_document_id_is_rejected_by_save_tools_and_export() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    let before = rig.state();
    for (tool, args) in [
        ("save", r#"{"documentId":"nope"}"#),
        ("save_as", r#"{"documentId":"nope","path":"/tmp/x.vexel"}"#),
        ("export_svg", r#"{"documentId":"nope"}"#),
    ] {
        let reply = rig.tool(tool, args);
        assert_eq!(reply.error(), "unknown preview document", "{tool}");
    }
    assert_eq!(rig.state(), before);
}

#[test]
fn export_svg_returns_source_and_changes_nothing() {
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    rig.doc_tool("element_add", &line_args(LINE_OK)).ok();
    let before = rig.state();
    let reply = rig.doc_tool("export_svg", "");
    let body = reply.ok().to_owned();
    assert!(body.starts_with(r#"{"svg":""#), "{body}");
    assert_eq!(rig.state(), before);
    // The reply text decodes (JSON unescape) to the exact source bytes.
    let doc = vexel_json::JsonDocument::parse(body.as_bytes()).unwrap();
    let svg = doc.object_get(doc.root(), b"svg").unwrap();
    assert_eq!(doc.decoded_string(svg).unwrap(), before.source);
}

#[test]
fn export_svg_too_large_is_typed() {
    let mut rig = rig();
    // 4 KiB path data per element until the source passes 512 KiB.
    let d = format!("M 0 0 {}", "L 1 1 ".repeat(600));
    assert!(d.len() < 4096);
    while rig.app.source_bytes().unwrap().len() <= 512 * 1024 {
        rig.doc_tool("element_add", &path_args(&format!(r#""d":"{d}""#)))
            .ok();
    }
    rejects(&mut rig, "export_svg", "", "export_svg too large");
}

#[test]
fn every_routed_edit_resyncs_the_shared_driver() {
    let mut rig = rig();
    let mut driver = Driver::new(MonotonicClock::new());
    let rect = r##""parentId":"root","template":"rect","attributes":{"x":"1","y":"1","width":"9","height":"9","fill":"#123456"}"##;
    let steps: Vec<(&str, String)> = vec![
        ("element_add", rect.to_owned()),
        ("element_add", line_args(LINE_OK)),
        ("element_add", path_args(r#""d":"M 5 5 L 20 20""#)),
        ("attribute_set", set("fill", "#654321")),
        ("attribute_set", set("transform", "translate(5 5)")),
        (
            "element_delete",
            r#""elementId":"vexel-object-2""#.to_owned(),
        ),
        ("undo", String::new()),
        ("redo", String::new()),
    ];
    for (tool, extra) in &steps {
        let before = rig.app.revision();
        rig.app.request_frame(&mut driver, 0.0).unwrap();
        assert!(driver.knows(before), "{tool}");
        rig.doc_tool(tool, extra).ok();
        let after = rig.app.revision();
        assert!(after > before, "{tool} must move the revision");
        // The shell's driver notices the new revision and the session
        // re-syncs it through `request_frame`.
        assert!(!driver.knows(after), "{tool}");
        rig.app.request_frame(&mut driver, 0.0).unwrap();
        assert!(driver.knows(after), "{tool}");
    }
}

#[test]
fn host_driver_follows_routed_edits() {
    // The host's playback driver is the shell's: a routed edit followed by
    // animation_control must see the new revision.
    let mut rig = rig();
    rig.add_rect(10, 10, "#111111");
    rig.doc_tool(
        "animation_add",
        r#""targetId":"vexel-object-1","xml":"<animate attributeName=\"x\" values=\"10;60\" dur=\"2s\"/>""#,
    )
    .ok();
    let reply = rig.doc_tool("animation_control", r#""action":"seek","timeMs":500"#);
    assert!(
        reply.ok().contains("\"timeSeconds\":0.500"),
        "{}",
        reply.body
    );
    assert!(rig.driver.knows(rig.app.revision()));
    rig.doc_tool("attribute_set", &set("fill", "#445566")).ok();
    assert!(!rig.driver.knows(rig.app.revision()));
    let reply = rig.doc_tool("animation_control", r#""action":"seek","timeMs":1000"#);
    assert!(
        reply.ok().contains("\"timeSeconds\":1.000"),
        "{}",
        reply.body
    );
    assert!(rig.driver.knows(rig.app.revision()));
}

#[test]
fn undo_and_redo_without_history_stay_typed() {
    let mut rig = rig();
    rejects(&mut rig, "undo", "", "nothing to undo");
    rejects(&mut rig, "redo", "", "nothing to redo");
}
