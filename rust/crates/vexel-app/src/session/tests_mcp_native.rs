use super::*;
use crate::animation::driver::Driver;
use crate::animation::playback::ManualClock;
use crate::preview::parse_operation;
use crate::session::McpHost;

fn run(app: &mut AppDocument, tool: &str, args: &str) -> Result<String, &'static str> {
    let sep = if args.is_empty() { "" } else { "," };
    let json = format!("{{\"documentId\":\"preview-document\"{sep}{args}}}");
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    let operation = parse_operation(&doc, doc.root(), tool)?;
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    match host.handle_with(app, operation, &mut driver)? {
        McpOutcome::Reply(json) => Ok(json),
        other => panic!("unexpected {other:?}"),
    }
}

fn rect(app: &mut AppDocument) {
    let json = run(
        app,
        "element_add",
        "\"template\":\"rect\",\"attributes\":{\"x\":\"1\",\"y\":\"1\",\"width\":\"5\",\"height\":\"5\",\"fill\":\"#112233\"}",
    )
    .unwrap();
    assert!(json.contains("vexel-object-1"));
}

#[test]
fn mcp_native_timeline_set_key_roundtrip() {
    let mut app = AppDocument::new(64, 64).unwrap();
    rect(&mut app);
    let state = app.state_revision();
    run(
        &mut app,
        "native_timeline_add_property_track",
        "\"trackId\":\"t1\",\"targetId\":\"vexel-object-1\",\"property\":\"opacity\",\"valueType\":\"scalar\",\"interpolation\":\"linear\",\"value\":0",
    )
    .unwrap();
    assert!(app.state_revision() > state);
    run(
        &mut app,
        "native_timeline_set_property_key",
        "\"trackId\":\"t1\",\"tick\":12,\"value\":1",
    )
    .unwrap();
    let with_key = run(&mut app, "native_timeline_get", "").unwrap();
    assert!(with_key.contains("\"id\":\"t1\""), "{with_key}");
    assert!(with_key.contains("\"tick\":12"), "{with_key}");

    run(&mut app, "native_timeline_undo", "").unwrap();
    let undone = run(&mut app, "native_timeline_get", "").unwrap();
    assert!(!undone.contains("\"tick\":12"), "{undone}");
    assert!(undone.contains("\"id\":\"t1\""));

    run(&mut app, "native_timeline_redo", "").unwrap();
    assert_eq!(run(&mut app, "native_timeline_get", "").unwrap(), with_key);

    run(
        &mut app,
        "native_timeline_set_loop",
        "\"mode\":\"ping_pong\"",
    )
    .unwrap();
    assert!(
        run(&mut app, "native_timeline_get", "")
            .unwrap()
            .contains("\"loop\":\"ping_pong\"")
    );
}

#[test]
fn mcp_native_rejects_bad_arguments() {
    let mut app = AppDocument::new(64, 64).unwrap();
    for (tool, args) in [
        ("native_timeline_set_loop", "\"mode\":\"bogus\""),
        (
            "native_timeline_set_property_key",
            "\"trackId\":\"t\",\"tick\":-1,\"value\":1",
        ),
        (
            "native_timeline_set_property_key",
            "\"trackId\":\"t\",\"tick\":1,\"value\":\"x\"",
        ),
        (
            "native_timeline_set_property_key",
            "\"trackId\":\"missing\",\"tick\":1,\"value\":1",
        ),
    ] {
        assert!(run(&mut app, tool, args).is_err(), "{tool} {args}");
    }
    assert!(run(&mut app, "native_timeline_undo", "").is_err());
}
