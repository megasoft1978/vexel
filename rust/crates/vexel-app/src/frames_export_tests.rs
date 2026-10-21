use super::*;
use crate::preview::parse_operation;
use crate::session::McpOutcome;
use std::fs;
use std::path::Path;
use vexel_json::JsonDocument;

fn call(app: &mut AppDocument, tool: &str, args: &str) {
    let json = format!("{{\"documentId\":\"preview-document\",{args}}}");
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    let operation = parse_operation(&doc, doc.root(), tool).unwrap();
    let mut driver = Driver::new(MonotonicClock::new());
    let outcome = McpHost::new().handle_with(app, operation, &mut driver);
    assert!(matches!(outcome, Ok(McpOutcome::Reply(_))), "{outcome:?}");
}

fn animated() -> AppDocument {
    let mut app = AppDocument::new(16, 16).unwrap();
    call(
        &mut app,
        "element_add",
        "\"template\":\"rect\",\"attributes\":{\"x\":\"1\",\"y\":\"1\",\"width\":\"5\",\"height\":\"5\",\"fill\":\"#112233\"}",
    );
    call(
        &mut app,
        "native_timeline_add_property_track",
        "\"trackId\":\"t1\",\"targetId\":\"vexel-object-1\",\"property\":\"opacity\",\"valueType\":\"scalar\",\"interpolation\":\"linear\",\"value\":0",
    );
    app
}

fn temp(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vexel-ui-frames-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn apng_is_written_atomically_with_no_staging_left() {
    let dir = temp("apng");
    let mut app = animated();
    let out = dir.join("clip.apng");
    assert_eq!(write_apng(&mut app, out.clone()), Ok("Exported APNG"));
    let bytes = fs::read(&out).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert!(bytes.windows(4).any(|w| w == b"acTL"));
    assert_eq!(names(&dir), ["clip.apng"]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn frames_use_the_base_name_or_pattern_and_refuse_to_overwrite() {
    let dir = temp("frames");
    let mut app = animated();
    assert_eq!(
        write_frames(&mut app, dir.join("walk.png")),
        Ok("Exported PNG frames")
    );
    let first = names(&dir);
    assert!(first.len() > 1 && first[0] == "walk_0000.png", "{first:?}");
    assert!(
        first
            .iter()
            .all(|n| n.starts_with("walk_") && n.ends_with(".png"))
    );
    assert!(
        write_frames(&mut app, dir.join("walk"))
            .unwrap_err()
            .contains("overwrite")
    );
    assert_eq!(names(&dir), first, "refusal writes nothing");
    assert_eq!(
        write_frames(&mut app, dir.join("alt_#####.png")),
        Ok("Exported PNG frames")
    );
    assert!(names(&dir).contains(&"alt_00000.png".to_owned()));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn exports_without_a_native_timeline_fail_with_a_message_and_leave_nothing() {
    let dir = temp("none");
    let mut app = AppDocument::new(16, 16).unwrap();
    let message = write_apng(&mut app, dir.join("x.apng")).unwrap_err();
    assert!(message.contains("native timeline"), "{message}");
    assert!(write_frames(&mut app, dir.join("x.png")).is_err());
    assert!(names(&dir).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn hevc_alpha_writes_a_quicktime_movie() {
    let dir = temp("hevc");
    let app = animated();
    let out = dir.join("clip.mov");
    assert_eq!(
        crate::video_export::write_video_alpha(&app, out.clone()),
        Ok("Exported HEVC video with alpha")
    );
    let bytes = fs::read(&out).unwrap();
    assert!(bytes.len() > 100 && bytes.windows(4).any(|w| w == b"hvc1" || w == b"hev1"));
    assert_eq!(names(&dir), ["clip.mov"]);
    let _ = fs::remove_dir_all(&dir);
}
