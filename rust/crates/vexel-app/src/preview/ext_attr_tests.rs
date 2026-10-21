use crate::animation::driver::Driver;
use crate::animation::playback::ManualClock;
use crate::preview::{parse_operation, PreviewArgumentError};
use crate::session::{AppDocument, McpHost, McpOutcome};
use vexel_json::JsonDocument;
use vexel_v1_document::{TextAnchor, TextSpec};

fn run_full(app: &mut AppDocument, tool: &str, args: &str) -> Result<String, String> {
    let sep = if args.is_empty() { "" } else { "," };
    let json = format!("{{\"documentId\":\"preview-document\"{sep}{args}}}");
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    let operation = match parse_operation(&doc, doc.root(), tool) {
        Ok(op) => op,
        Err(PreviewArgumentError::Generic(m)) => return Err(m.to_owned()),
        Err(PreviewArgumentError::Actionable(m)) => return Err(m),
    };
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    match host.handle_with(app, operation, &mut driver) {
        Ok(McpOutcome::Reply(json)) => Ok(json),
        Ok(other) => panic!("unexpected {other:?}"),
        Err(m) => Err(m.to_owned()),
    }
}

fn run(app: &mut AppDocument, tool: &str, args: &str) -> String {
    run_full(app, tool, args).unwrap()
}

fn source(app: &AppDocument) -> String {
    String::from_utf8(app.source_bytes().unwrap()).unwrap()
}

fn rect(app: &mut AppDocument) {
    run(
        app,
        "element_add",
        "\"template\":\"rect\",\"attributes\":{\"x\":\"1\",\"y\":\"1\",\"width\":\"5\",\"height\":\"5\",\"fill\":\"#112233\"}",
    );
}

fn text(app: &mut AppDocument) -> String {
    app.edit(|doc| {
        doc.add_text(&TextSpec {
            x: 2.0,
            y: 20.0,
            content: "Hi".to_owned(),
            font_size: 12.0,
            anchor: TextAnchor::Start,
            fill: "#000000".to_owned(),
        })
    })
    .unwrap()
}

fn set(app: &mut AppDocument, name: &str, value: &str) -> Result<String, String> {
    run_full(
        app,
        "attribute_set",
        &format!("\"elementId\":\"vexel-object-1\",\"name\":\"{name}\",\"value\":\"{value}\""),
    )
}

#[test]
fn attribute_set_widened_names_roundtrip() {
    for (name, value) in [
        ("opacity", "0.5"),
        ("fill-opacity", "0.25"),
        ("stroke-opacity", "1"),
        ("stroke-linecap", "round"),
        ("stroke-linejoin", "bevel"),
        ("stroke-dasharray", "4 2"),
        ("stroke-dashoffset", "3"),
        ("fill-rule", "evenodd"),
        ("visibility", "hidden"),
        ("display", "none"),
        ("fill", "none"),
        ("stroke", "none"),
    ] {
        let mut app = AppDocument::new(64, 64).unwrap();
        rect(&mut app);
        let before = source(&app);
        set(&mut app, name, value).unwrap_or_else(|e| panic!("{name}: {e}"));
        let after = source(&app);
        assert!(after.contains(&format!("{name}=\"{value}\"")), "{after}");
        assert!(app.undo().unwrap());
        assert_eq!(source(&app), before, "{name}");
        assert!(app.redo().unwrap());
        assert_eq!(source(&app), after, "{name}");
    }
}

#[test]
fn attribute_set_fill_url_needs_existing_target() {
    let mut app = AppDocument::new(64, 64).unwrap();
    rect(&mut app);
    let before = source(&app);
    assert_eq!(
        set(&mut app, "fill", "url(#nope)").unwrap_err(),
        "unknown element"
    );
    assert_eq!(source(&app), before);
}

#[test]
fn attribute_set_widened_rejections() {
    let mut app = AppDocument::new(64, 64).unwrap();
    rect(&mut app);
    let before = source(&app);
    for (name, value) in [
        ("opacity", "2"),
        ("opacity", "abc"),
        ("stroke-linecap", "wavy"),
        ("display", "block"),
        ("id", "x"),
        ("stroke", "red"),
    ] {
        assert!(set(&mut app, name, value).is_err(), "{name}={value}");
    }
    assert_eq!(source(&app), before);
}

#[test]
fn attribute_remove_roundtrip_and_errors() {
    let mut app = AppDocument::new(64, 64).unwrap();
    rect(&mut app);
    set(&mut app, "opacity", "0.5").unwrap();
    let with = source(&app);
    let reply = run(
        &mut app,
        "attribute_remove",
        "\"elementId\":\"vexel-object-1\",\"name\":\"opacity\"",
    );
    assert!(reply.starts_with("{\"ok\":true"), "{reply}");
    let without = source(&app);
    assert!(!without.contains("opacity"), "{without}");
    assert!(app.undo().unwrap());
    assert_eq!(source(&app), with);
    assert!(app.redo().unwrap());
    assert_eq!(source(&app), without);

    for (args, expected) in [
        (
            "\"elementId\":\"vexel-object-9\",\"name\":\"opacity\"",
            "attribute_remove: element not found",
        ),
        (
            "\"elementId\":\"vexel-object-1\",\"name\":\"id\"",
            "attribute_remove: name must be one of",
        ),
        (
            "\"elementId\":\"vexel-object-1\"",
            "attribute_remove: name must be one of",
        ),
        (
            "\"name\":\"opacity\"",
            "attribute_remove: elementId must be",
        ),
    ] {
        let error = run_full(&mut app, "attribute_remove", args).unwrap_err();
        assert!(error.starts_with(expected), "{error}");
    }
    assert_eq!(source(&app), without);
    let json = "{\"documentId\":\"nope\",\"elementId\":\"a\",\"name\":\"opacity\"}";
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    assert!(parse_operation(&doc, doc.root(), "attribute_remove").is_err());
}

#[test]
fn text_set_roundtrip_and_errors() {
    let mut app = AppDocument::new(64, 64).unwrap();
    rect(&mut app);
    let id = text(&mut app);
    let before = source(&app);
    let args = format!("\"elementId\":\"{id}\",\"text\":\"Bye\"");
    let reply = run(&mut app, "text_set", &args);
    assert!(reply.starts_with("{\"ok\":true"), "{reply}");
    let after = source(&app);
    assert!(
        after.contains(">Bye<") && !after.contains(">Hi<"),
        "{after}"
    );
    assert!(app.undo().unwrap());
    assert_eq!(source(&app), before);
    assert!(app.redo().unwrap());
    assert_eq!(source(&app), after);

    let on_rect = run_full(
        &mut app,
        "text_set",
        "\"elementId\":\"vexel-object-1\",\"text\":\"x\"",
    );
    assert_eq!(
        on_rect.unwrap_err(),
        "text_set: element is not a text element"
    );
    let missing = run_full(
        &mut app,
        "text_set",
        "\"elementId\":\"nope\",\"text\":\"x\"",
    );
    assert_eq!(missing.unwrap_err(), "text_set: element not found");
    for bad in ["", "a\\nb"] {
        let args = format!("\"elementId\":\"{id}\",\"text\":\"{bad}\"");
        let error = run_full(&mut app, "text_set", &args).unwrap_err();
        assert!(error.starts_with("text_set: text must be"), "{error}");
    }
    let long = "x".repeat(257);
    let args = format!("\"elementId\":\"{id}\",\"text\":\"{long}\"");
    assert!(run_full(&mut app, "text_set", &args).is_err());
    assert_eq!(source(&app), after);
}

#[test]
fn element_add_accepts_fill_none() {
    for (template, attrs, tag) in [
        (
            "rect",
            "\"x\":\"1\",\"y\":\"2\",\"width\":\"10\",\"height\":\"8\"",
            "<rect",
        ),
        (
            "ellipse",
            "\"cx\":\"20\",\"cy\":\"20\",\"rx\":\"5\",\"ry\":\"4\"",
            "<ellipse",
        ),
    ] {
        let mut app = AppDocument::new(64, 64).unwrap();
        let before = source(&app);
        let args =
            format!("\"template\":\"{template}\",\"attributes\":{{{attrs},\"fill\":\"none\"}}");
        let reply = run(&mut app, "element_add", &args);
        assert!(reply.contains("vexel-object-1"), "{reply}");
        let after = source(&app);
        assert!(
            after.contains(tag) && after.contains("fill=\"none\""),
            "{after}"
        );
        assert!(app.undo().unwrap());
        assert_eq!(source(&app), before);
        assert!(app.redo().unwrap());
        assert_eq!(source(&app), after);
        // Outline workflow: a stroke can be added afterwards.
        set_on(&mut app, "stroke", "#ff0000");
    }
    let mut app = AppDocument::new(64, 64).unwrap();
    let error = run_full(
        &mut app,
        "element_add",
        "\"template\":\"rect\",\"attributes\":{\"x\":\"1\",\"y\":\"1\",\"width\":\"0\",\"height\":\"5\",\"fill\":\"none\"}",
    );
    assert!(error.is_err());
    let error = run_full(
        &mut app,
        "element_add",
        "\"template\":\"rect\",\"attributes\":{\"x\":\"1\",\"y\":\"1\",\"width\":\"5\",\"height\":\"5\",\"fill\":\"red\"}",
    );
    assert_eq!(error.unwrap_err(), "invalid preview fill color");
}

fn set_on(app: &mut AppDocument, name: &str, value: &str) {
    run(
        app,
        "attribute_set",
        &format!("\"elementId\":\"vexel-object-1\",\"name\":\"{name}\",\"value\":\"{value}\""),
    );
}

#[test]
fn animation_control_documented_calls_work_or_say_why() {
    let mut app = AppDocument::new(64, 64).unwrap();
    rect(&mut app);
    // Without a SMIL animation there is nothing to play; the message is static.
    assert_eq!(
        run_full(&mut app, "animation_control", "\"action\":\"play\"").unwrap_err(),
        "no preview animation to play"
    );
    assert!(
        run(&mut app, "animation_control", "\"action\":\"pause\"").contains("\"playing\":false")
    );
    assert!(run(
        &mut app,
        "animation_control",
        "\"action\":\"seek\",\"timeMs\":500"
    )
    .contains("\"timeSeconds\":0.500"));
    for (args, expected) in [
        (
            "\"action\":\"seek\"",
            "animation_control: seek requires timeMs",
        ),
        (
            "\"action\":\"seek\",\"timeMs\":20000",
            "animation_control: seek requires timeMs",
        ),
        (
            "\"action\":\"seek\",\"timeMs\":\"abc\"",
            "animation_control: seek requires timeMs",
        ),
        (
            "\"action\":\"rewind\"",
            "animation_control: action is required",
        ),
        ("\"timeMs\":5", "animation_control: action is required"),
    ] {
        let error = run_full(&mut app, "animation_control", args).unwrap_err();
        assert!(error.starts_with(expected), "{args}: {error}");
    }
    // Case and whitespace in the action are tolerated.
    run(&mut app, "animation_control", "\"action\":\" Pause \"");
}
