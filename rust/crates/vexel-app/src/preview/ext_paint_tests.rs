use super::*;
use crate::compose::compose_svg;

fn run(app: &mut AppDocument, tool: &str, args: &str) -> Result<String, String> {
    let sep = if args.is_empty() { "" } else { "," };
    let json = format!("{{\"documentId\":\"preview-document\"{sep}{args}}}");
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    let op = match parse(&doc, doc.root(), tool).expect("tool in family") {
        Ok(op) => op,
        Err(PreviewArgumentError::Actionable(e)) => return Err(e),
        Err(PreviewArgumentError::Generic(e)) => return Err(e.to_owned()),
    };
    match apply(app, op) {
        Ok(McpOutcome::Reply(json)) => Ok(json),
        Ok(other) => panic!("unexpected {other:?}"),
        Err(e) => Err(e.to_owned()),
    }
}

fn ok(app: &mut AppDocument, tool: &str, args: &str) -> String {
    run(app, tool, args).unwrap_or_else(|e| panic!("{tool}: {e}"))
}

fn err(app: &mut AppDocument, tool: &str, args: &str) -> String {
    let before = app.source_bytes().unwrap();
    let e = run(app, tool, args).expect_err("must fail");
    assert!(e.starts_with(tool), "{e}");
    assert_eq!(app.source_bytes().unwrap(), before, "{tool} changed source");
    e
}

fn app_with_rect() -> AppDocument {
    let mut app = AppDocument::new(64, 64).unwrap();
    app.edit(|d| d.add_rectangle_with_id(8.0, 8.0, 40.0, 40.0, "#00ff00", "vexel-object-1"))
        .unwrap();
    app
}

fn field(json: &str, key: &str) -> String {
    let at = json.find(&format!("\"{key}\":\"")).expect(key) + key.len() + 4;
    json[at..at + json[at..].find('"').unwrap()].to_owned()
}

fn src(app: &AppDocument) -> String {
    String::from_utf8(app.source_bytes().unwrap()).unwrap()
}

/// Asserts that `edit` is one undo step that redo restores exactly.
fn undo_redo(app: &mut AppDocument, edit: impl FnOnce(&mut AppDocument)) {
    let before = app.source_bytes().unwrap();
    edit(app);
    let after = app.source_bytes().unwrap();
    assert_ne!(before, after);
    assert!(app.undo().unwrap());
    assert_eq!(app.source_bytes().unwrap(), before);
    assert!(app.redo().unwrap());
    assert_eq!(app.source_bytes().unwrap(), after);
}

const STOPS: &str = "\"stops\":\"0:#ff0000:1,1:#0000ff:0.5\"";

#[test]
fn gradient_add_linear_and_radial_reply_and_undo() {
    let mut app = app_with_rect();
    undo_redo(&mut app, |app| {
        let reply = ok(app, "gradient_add", &format!("\"kind\":\"linear\",{STOPS}"));
        assert_eq!(field(&reply, "gradientId"), "vexel-gradient-1");
        assert!(
            reply.contains("\"stopIds\":[\"vexel-gradient-1-stop-0\",\"vexel-gradient-1-stop-1\"]"),
            "{reply}"
        );
        assert!(reply.contains("\"operation\":\"gradient_add\""));
    });
    let reply = ok(
        &mut app,
        "gradient_add",
        &format!(
            "\"kind\":\"radial\",\"cx\":0.4,\"cy\":0.4,\"r\":0.6,\"units\":\"userSpace\",\"spread\":\"reflect\",{STOPS}"
        ),
    );
    assert_eq!(field(&reply, "gradientId"), "vexel-gradient-2");
    let s = src(&app);
    assert!(
        s.contains("<radialGradient id=\"vexel-gradient-2\" cx=\"0.4\""),
        "{s}"
    );
    assert!(s.contains("spreadMethod=\"reflect\""));
    assert!(s.contains("gradientUnits=\"userSpaceOnUse\""));
    assert!(s.contains("stop-opacity=\"0.5\""));
}

#[test]
fn gradient_add_errors() {
    let mut app = app_with_rect();
    for args in [
        "\"kind\":\"conic\",\"stops\":\"0:#ff0000,1:#0000ff\"".to_owned(),
        "\"stops\":\"0:#ff0000,1:#0000ff\"".to_owned(),
        "\"kind\":\"linear\"".to_owned(),
        "\"kind\":\"linear\",\"stops\":\"0:#ff0000\"".to_owned(),
        "\"kind\":\"linear\",\"stops\":\"0:red,1:blue\"".to_owned(),
        "\"kind\":\"linear\",\"stops\":\"0.8:#ff0000,0.2:#0000ff\"".to_owned(),
        "\"kind\":\"linear\",\"stops\":\"0:#ff0000,2:#0000ff\"".to_owned(),
        "\"kind\":\"linear\",\"x1\":\"0\",\"stops\":\"0:#ff0000,1:#0000ff\"".to_owned(),
        "\"kind\":\"radial\",\"r\":0,\"stops\":\"0:#ff0000,1:#0000ff\"".to_owned(),
        "\"kind\":\"linear\",\"units\":\"em\",\"stops\":\"0:#ff0000,1:#0000ff\"".to_owned(),
        "\"kind\":\"linear\",\"spread\":\"wrap\",\"stops\":\"0:#ff0000,1:#0000ff\"".to_owned(),
        format!(
            "\"kind\":\"linear\",\"stops\":\"{}\"",
            vec!["0:#000000"; 17].join(",")
        ),
    ] {
        let e = err(&mut app, "gradient_add", &args);
        assert!(e.len() < 400, "{e}");
    }
    let e = err(
        &mut app,
        "gradient_add",
        "\"kind\":\"conic\",\"stops\":\"0:#ff0000,1:#0000ff\"",
    );
    assert!(e.contains("kind") && e.contains("`linear`"), "{e}");
}

#[test]
fn gradient_set_stop_roundtrip_and_errors() {
    let mut app = app_with_rect();
    ok(
        &mut app,
        "gradient_add",
        &format!("\"kind\":\"linear\",{STOPS}"),
    );
    undo_redo(&mut app, |app| {
        let reply = ok(
            app,
            "gradient_set_stop",
            "\"gradientId\":\"vexel-gradient-1\",\"index\":1,\"offset\":0.75,\"color\":\"#00ff00\",\"opacity\":0.25",
        );
        assert!(reply.contains("\"index\":1"), "{reply}");
    });
    let s = src(&app);
    assert!(
        s.contains("offset=\"0.75\" stop-color=\"#00ff00\" stop-opacity=\"0.25\""),
        "{s}"
    );
    err(
        &mut app,
        "gradient_set_stop",
        "\"gradientId\":\"vexel-gradient-1\",\"index\":1",
    );
    err(
        &mut app,
        "gradient_set_stop",
        "\"gradientId\":\"vexel-gradient-1\",\"index\":2,\"opacity\":1",
    );
    err(
        &mut app,
        "gradient_set_stop",
        "\"gradientId\":\"vexel-gradient-1\",\"index\":99,\"opacity\":1",
    );
    err(
        &mut app,
        "gradient_set_stop",
        "\"gradientId\":\"vexel-gradient-1\",\"index\":1,\"color\":\"green\"",
    );
    err(
        &mut app,
        "gradient_set_stop",
        "\"gradientId\":\"vexel-gradient-1\",\"index\":0,\"offset\":0.9",
    );
    err(
        &mut app,
        "gradient_set_stop",
        "\"gradientId\":\"vexel-gradient-9\",\"index\":0,\"opacity\":1",
    );
    err(&mut app, "gradient_set_stop", "\"index\":0,\"opacity\":1");
}

#[test]
fn element_set_paint_fill_stroke_and_errors() {
    let mut app = app_with_rect();
    ok(
        &mut app,
        "gradient_add",
        &format!("\"kind\":\"linear\",{STOPS}"),
    );
    undo_redo(&mut app, |app| {
        let reply = ok(
            app,
            "element_set_paint",
            "\"elementId\":\"vexel-object-1\",\"property\":\"fill\",\"gradientId\":\"vexel-gradient-1\"",
        );
        assert!(reply.contains("\"property\":\"fill\""), "{reply}");
    });
    ok(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"vexel-object-1\",\"property\":\"stroke\",\"gradientId\":\"vexel-gradient-1\"",
    );
    let s = src(&app);
    assert!(s.contains("fill=\"url(#vexel-gradient-1)\""), "{s}");
    assert!(s.contains("stroke=\"url(#vexel-gradient-1)\""), "{s}");
    err(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"vexel-object-1\",\"property\":\"color\",\"gradientId\":\"vexel-gradient-1\"",
    );
    err(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"nope\",\"property\":\"fill\",\"gradientId\":\"vexel-gradient-1\"",
    );
    err(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"vexel-object-1\",\"property\":\"fill\",\"gradientId\":\"vexel-gradient-9\"",
    );
    err(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"vexel-object-1\",\"property\":\"fill\"",
    );
    err(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"vexel-gradient-1\",\"property\":\"fill\",\"gradientId\":\"vexel-gradient-1\"",
    );
}

#[test]
fn clip_add_and_set_clip_with_removal() {
    let mut app = app_with_rect();
    undo_redo(&mut app, |app| {
        let reply = ok(
            app,
            "clip_add",
            "\"shape\":\"rect\",\"a\":10,\"b\":10,\"c\":20,\"d\":20",
        );
        assert_eq!(field(&reply, "clipId"), "vexel-clip-1");
    });
    let reply = ok(
        &mut app,
        "clip_add",
        "\"shape\":\"ellipse\",\"a\":20,\"b\":20,\"c\":10,\"d\":5",
    );
    assert_eq!(field(&reply, "clipId"), "vexel-clip-2");
    assert!(src(&app).contains("<ellipse cx=\"20\" cy=\"20\" rx=\"10\" ry=\"5\"/>"));

    let plain = src(&app);
    undo_redo(&mut app, |app| {
        let reply = ok(
            app,
            "element_set_clip",
            "\"elementId\":\"vexel-object-1\",\"clipId\":\"vexel-clip-1\"",
        );
        assert!(reply.contains("\"clipId\":\"vexel-clip-1\""), "{reply}");
    });
    let clipped = src(&app);
    assert!(
        clipped.contains("clip-path=\"url(#vexel-clip-1)\""),
        "{clipped}"
    );
    let reply = ok(
        &mut app,
        "element_set_clip",
        "\"elementId\":\"vexel-object-1\"",
    );
    assert!(reply.contains("\"clipId\":null"), "{reply}");
    assert_eq!(src(&app), plain);

    err(
        &mut app,
        "clip_add",
        "\"shape\":\"star\",\"a\":1,\"b\":1,\"c\":1,\"d\":1",
    );
    err(
        &mut app,
        "clip_add",
        "\"shape\":\"rect\",\"a\":1,\"b\":1,\"c\":0,\"d\":5",
    );
    err(
        &mut app,
        "clip_add",
        "\"shape\":\"rect\",\"a\":1,\"b\":1,\"c\":5",
    );
    err(
        &mut app,
        "clip_add",
        "\"shape\":\"ellipse\",\"a\":1,\"b\":1,\"c\":-5,\"d\":5",
    );
    err(
        &mut app,
        "element_set_clip",
        "\"elementId\":\"vexel-object-1\",\"clipId\":\"vexel-clip-9\"",
    );
    err(
        &mut app,
        "element_set_clip",
        "\"elementId\":\"missing\",\"clipId\":\"vexel-clip-1\"",
    );
    err(&mut app, "element_set_clip", "\"clipId\":\"vexel-clip-1\"");
}

#[test]
fn element_set_transform_roundtrip_identity_and_errors() {
    let mut app = app_with_rect();
    let plain = src(&app);
    undo_redo(&mut app, |app| {
        ok(
            app,
            "element_set_transform",
            "\"elementId\":\"vexel-object-1\",\"translateX\":10,\"translateY\":5,\"rotate\":90,\"pivotX\":24,\"pivotY\":24,\"scaleX\":0.5,\"scaleY\":0.5,\"skewX\":10",
        );
    });
    assert!(src(&app).contains("transform=\"matrix("));
    let reply = ok(
        &mut app,
        "element_set_transform",
        "\"elementId\":\"vexel-object-1\"",
    );
    assert!(reply.contains("\"elementId\":\"vexel-object-1\""));
    // Removing an attribute leaves the whitespace before `/>` (document-level
    // behaviour); everything else must equal the original bytes.
    assert_eq!(
        src(&app).replace(" />", "/>"),
        plain,
        "identity removes the transform"
    );

    ok(
        &mut app,
        "element_set_transform",
        "\"elementId\":\"vexel-object-1\",\"rotate\":0,\"translateX\":3",
    );
    assert!(
        src(&app).contains("matrix(1e0 0e0 0e0 1e0 3e0 0e0)"),
        "{}",
        src(&app)
    );
    err(
        &mut app,
        "element_set_transform",
        "\"elementId\":\"nope\",\"rotate\":5",
    );
    err(
        &mut app,
        "element_set_transform",
        "\"elementId\":\"vexel-object-1\",\"rotate\":\"5\"",
    );
    err(
        &mut app,
        "element_set_transform",
        "\"elementId\":\"vexel-object-1\",\"scaleX\":1e999",
    );
    err(&mut app, "element_set_transform", "\"rotate\":5");
}

#[test]
fn text_add_defaults_options_undo_and_errors() {
    let mut app = app_with_rect();
    undo_redo(&mut app, |app| {
        let reply = ok(
            app,
            "text_add",
            "\"x\":4,\"y\":20,\"content\":\"Hi & <bye>\"",
        );
        assert!(reply.contains("\"operation\":\"text_add\""));
        assert!(!field(&reply, "elementId").is_empty());
    });
    let reply = ok(
        &mut app,
        "text_add",
        "\"x\":32,\"y\":50,\"content\":\"Mid\",\"fontSize\":24,\"anchor\":\"middle\",\"fill\":\"#336699\"",
    );
    let id = field(&reply, "elementId");
    let s = src(&app);
    assert!(
        s.contains(&id) && s.contains("text-anchor=\"middle\"") && s.contains("#336699"),
        "{s}"
    );
    assert!(s.contains("Hi &amp; &lt;bye&gt;"), "{s}");

    err(&mut app, "text_add", "\"x\":4,\"y\":20");
    err(&mut app, "text_add", "\"x\":4,\"y\":20,\"content\":\"\"");
    err(
        &mut app,
        "text_add",
        "\"x\":4,\"y\":20,\"content\":\"a\\nb\"",
    );
    err(
        &mut app,
        "text_add",
        "\"x\":4,\"y\":20,\"content\":\"x\",\"anchor\":\"left\"",
    );
    err(
        &mut app,
        "text_add",
        "\"x\":4,\"y\":20,\"content\":\"x\",\"fontSize\":0",
    );
    err(
        &mut app,
        "text_add",
        "\"x\":4,\"y\":20,\"content\":\"x\",\"fill\":\"blue\"",
    );
    err(&mut app, "text_add", "\"y\":20,\"content\":\"x\"");
    err(
        &mut app,
        "text_add",
        &format!("\"x\":1,\"y\":1,\"content\":\"{}\"", "a".repeat(257)),
    );
}

#[test]
fn unknown_tool_is_not_in_family() {
    let doc = JsonDocument::parse(br#"{"documentId":"preview-document"}"#).unwrap();
    assert!(parse(&doc, doc.root(), "element_group").is_none());
    assert!(parse(&doc, doc.root(), "gradient_remove").is_none());
}

#[test]
fn errors_never_echo_caller_text() {
    let mut app = app_with_rect();
    let e = run(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"vexel-object-1\",\"property\":\"EVIL-PAYLOAD\",\"gradientId\":\"g\"",
    )
    .unwrap_err();
    assert!(!e.contains("EVIL"), "{e}");
    let e = run(
        &mut app,
        "element_set_clip",
        "\"elementId\":\"EVIL-PAYLOAD\",\"clipId\":\"vexel-clip-1\"",
    )
    .unwrap_err();
    assert!(!e.contains("EVIL"), "{e}");
}

fn pixel(frame: &crate::compose::Frame, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * frame.width + x) * 4) as usize;
    frame.rgba[at..at + 4].try_into().unwrap()
}

#[test]
fn render_proof_gradient_and_transform() {
    let mut app = AppDocument::new(64, 64).unwrap();
    app.edit(|d| d.add_rectangle_with_id(0.0, 0.0, 32.0, 16.0, "#00ff00", "vexel-object-1"))
        .unwrap();
    ok(
        &mut app,
        "gradient_add",
        "\"kind\":\"linear\",\"stops\":\"0:#ff0000:1,1:#0000ff:1\"",
    );
    ok(
        &mut app,
        "element_set_paint",
        "\"elementId\":\"vexel-object-1\",\"property\":\"fill\",\"gradientId\":\"vexel-gradient-1\"",
    );
    let flat = compose_svg(&app.source_bytes().unwrap(), 64, 64).unwrap();
    let left = pixel(&flat, 2, 8);
    let right = pixel(&flat, 29, 8);
    assert!(left[0] > 200 && left[2] < 60, "left {left:?}");
    assert!(right[2] > 200 && right[0] < 60, "right {right:?}");
    assert_eq!(pixel(&flat, 40, 8)[3], 0, "outside the rect is transparent");
    assert_eq!(pixel(&flat, 8, 40)[3], 0);

    ok(
        &mut app,
        "element_set_transform",
        "\"elementId\":\"vexel-object-1\",\"translateY\":32,\"translateX\":16",
    );
    let moved = compose_svg(&app.source_bytes().unwrap(), 64, 64).unwrap();
    assert_eq!(pixel(&moved, 2, 8)[3], 0, "old position cleared");
    let at = pixel(&moved, 18, 40);
    assert!(at[0] > 200 && at[2] < 60, "moved left edge {at:?}");
    let at = pixel(&moved, 45, 40);
    assert!(at[2] > 200 && at[0] < 60, "moved right edge {at:?}");

    assert!(app.undo().unwrap());
    let back = compose_svg(&app.source_bytes().unwrap(), 64, 64).unwrap();
    assert_eq!(pixel(&back, 2, 8), left);
}
