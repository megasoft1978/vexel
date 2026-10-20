use super::*;
use crate::animation::driver::Driver;
use crate::animation::playback::ManualClock;
use crate::preview::parse_operation;
use crate::session::McpHost;

fn run(app: &mut AppDocument, tool: &str, args: &str) -> Result<String, &'static str> {
    let sep = if args.is_empty() { "" } else { "," };
    let json = format!("{{\"documentId\":\"preview-document\"{sep}{args}}}");
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    let operation =
        parse_operation(&doc, doc.root(), tool).map_err(|_| "invalid preview tool arguments")?;
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
fn default_timeline_is_a_short_loop() {
    let mut app = AppDocument::new(64, 64).unwrap();
    let timeline = run(&mut app, "native_timeline_get", "").unwrap();
    assert!(timeline.contains("\"ticksPerSecond\":24"), "{timeline}");
    assert!(timeline.contains("\"durationTicks\":48"), "{timeline}");
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

#[test]
fn mcp_export_animation_uses_native_timeline() {
    use crate::animation::job::ExportJob;
    use std::ffi::OsString;
    use std::sync::mpsc::sync_channel;
    use vexel_mcp_access::AccessPolicy;

    let mut app = AppDocument::new(64, 64).unwrap();
    rect(&mut app);
    run(
        &mut app,
        "native_timeline_add_property_track",
        "\"trackId\":\"opacity\",\"targetId\":\"vexel-object-1\",\"property\":\"opacity\",\"valueType\":\"scalar\",\"interpolation\":\"linear\",\"value\":0",
    )
    .unwrap();
    run(
        &mut app,
        "native_timeline_set_property_key",
        "\"trackId\":\"opacity\",\"tick\":20,\"value\":1",
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!("vexel-mcp-native-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(dir).unwrap();
    let policy = AccessPolicy::new(Some(OsString::from(&dir)), None);
    let path = dir.join("native.gif");
    let mut host = McpHost::with_policy(policy.clone());
    let mut driver = Driver::new(ManualClock::default());
    let outcome = host
        .handle_with(
            &mut app,
            crate::preview::Operation::Export {
                path: path.to_string_lossy().into_owned(),
                fps: 12,
                duration: 2.0,
            },
            &mut driver,
        )
        .unwrap();
    let McpOutcome::Export { source, fps: 0, .. } = outcome else {
        panic!("native timeline should select native export");
    };
    let (tx, rx) = sync_channel(1);
    let mut job = ExportJob::spawn_native_packed(source, policy, path.clone().into(), tx).unwrap();
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
    let gif = std::fs::read(path).unwrap();
    assert_eq!(&gif[..6], b"GIF89a");
    assert_eq!(
        gif.windows(8)
            .filter(|w| w[..3] == [0x21, 0xF9, 0x04])
            .count(),
        48
    );
}
