//! Open path, launch parsing, worker supersession and the shell view state.
use super::tests::{fixture, fixture_path};
use super::*;
use crate::session::AppDocument;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use vexel_ui::Rect;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "vexel-compose-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn wait_for(view: &mut DocumentView, done: impl Fn(&DocumentView) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done(view) {
        assert!(
            Instant::now() < deadline,
            "timed out; status {:?}",
            view.status()
        );
        view.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
}

// ---- open_path --------------------------------------------------------------

#[test]
fn open_path_reads_a_file_and_names_it() {
    let dir = TempDir::new();
    let svg = fixture("compose-v1.svg");
    let path = dir.file("drawing.svg", &svg);
    let opened = open_path(&path).unwrap();
    assert_eq!(opened.bytes, svg);
    assert_eq!(opened.display_name(), "drawing.svg");
    // The opened bytes compose to the same frame as the fixture.
    assert_eq!(
        compose_svg(&opened.bytes, 160, 120),
        compose_svg(&svg, 160, 120)
    );
}

#[test]
fn open_path_rejects_missing_directory_oversize_and_bad_paths() {
    let dir = TempDir::new();
    assert_eq!(
        open_path(&dir.0.join("absent.svg")),
        Err(OpenError::Unreadable(std::io::ErrorKind::NotFound))
    );
    assert_eq!(open_path(&dir.0), Err(OpenError::NotAFile));
    let big = dir.0.join("big.svg");
    let file = std::fs::File::create(&big).unwrap();
    file.set_len(MAX_SVG_BYTES as u64 + 1).unwrap();
    assert_eq!(open_path(&big), Err(OpenError::TooLarge));
    let at_cap = dir.0.join("cap.svg");
    std::fs::File::create(&at_cap)
        .unwrap()
        .set_len(MAX_SVG_BYTES as u64)
        .unwrap();
    assert_eq!(open_path(&at_cap).unwrap().bytes.len(), MAX_SVG_BYTES);
    assert_eq!(open_path(Path::new("")), Err(OpenError::InvalidPath));
    let nul = PathBuf::from(OsString::from_vec(b"/tmp/a\0b.svg".to_vec()));
    assert_eq!(open_path(&nul), Err(OpenError::InvalidPath));
    let not_utf8 = PathBuf::from(OsString::from_vec(vec![b'/', b't', 0xff, 0xfe]));
    assert_eq!(open_path(&not_utf8), Err(OpenError::InvalidPath));
    let long = PathBuf::from(format!("/{}", "a".repeat(5000)));
    assert_eq!(open_path(&long), Err(OpenError::InvalidPath));
    for error in [
        OpenError::InvalidPath,
        OpenError::NotAFile,
        OpenError::TooLarge,
    ] {
        assert!(!error.to_string().is_empty());
    }
}

// ---- launch parsing ---------------------------------------------------------

fn launch(args: &[&str]) -> Launch {
    parse_launch(args.iter().map(OsString::from))
}

#[test]
fn launch_keeps_mcp_preview_and_adds_a_file_argument() {
    assert_eq!(launch(&[]), Launch::Shell { open: None });
    assert_eq!(launch(&["--mcp-preview"]), Launch::McpPreview);
    assert_eq!(launch(&["a.svg", "--mcp-preview"]), Launch::McpPreview);
    assert_eq!(launch(&["--mcp-preview", "a.svg"]), Launch::McpPreview);
    assert_eq!(
        launch(&["a.svg"]),
        Launch::Shell {
            open: Some("a.svg".into())
        }
    );
    assert_eq!(
        launch(&["-psn_0_1", "a.svg"]),
        Launch::Shell {
            open: Some("a.svg".into())
        }
    );
    assert_eq!(
        launch(&["--", "-odd.svg"]),
        Launch::Shell {
            open: Some("-odd.svg".into())
        }
    );
    assert_eq!(launch(&["--unknown"]), Launch::Shell { open: None });
    assert!(matches!(launch(&["a.svg", "b.svg"]), Launch::Usage(_)));
}

// ---- worker supersession ----------------------------------------------------

#[test]
fn worker_delivers_only_the_newest_generation() {
    let worker = ComposeWorker::new().unwrap();
    let svg = Arc::new(fixture("compose-v1.svg"));
    let mut last = 0;
    for i in 0..30u32 {
        last = worker.request(svg.clone(), 100 + i, 80);
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    let event = loop {
        if let Some(event) = worker.poll() {
            break event;
        }
        assert!(Instant::now() < deadline, "no result");
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(event.generation, last);
    assert_eq!(event.result.unwrap().frame.width, 129);
    std::thread::sleep(Duration::from_millis(50));
    assert!(
        worker.poll().is_none(),
        "no stale or duplicate result follows"
    );
}

#[test]
fn worker_cancel_discards_pending_and_inflight_results() {
    let worker = ComposeWorker::new().unwrap();
    let svg = Arc::new(fixture("compose-v1.svg"));
    worker.request(svg, 4000, 4000);
    worker.cancel();
    std::thread::sleep(Duration::from_millis(300));
    assert!(worker.poll().is_none());
}

#[test]
fn worker_drop_joins_while_work_is_in_flight() {
    let worker = ComposeWorker::new().unwrap();
    worker.request(Arc::new(fixture("compose-v1.svg")), 4096, 4096);
    drop(worker);
}

// ---- shell view state -------------------------------------------------------

const STAGE: Rect = Rect {
    x: 60,
    y: 48,
    width: 160,
    height: 120,
};

#[test]
fn view_open_composes_reports_status_and_maps_clicks() {
    let dir = TempDir::new();
    let path = dir.file("scene.svg", &fixture("compose-v1.svg"));
    let mut view = DocumentView::new();
    let mut app = AppDocument::new(10, 10).unwrap();
    assert!(view.status().is_none());
    view.open(&mut app, &path, (STAGE.width, STAGE.height));
    assert_eq!(view.status().map(|s| s.1), Some(false));
    wait_for(&mut view, |v| v.composed().is_some());
    let (text, error) = view.status().unwrap();
    assert!(
        !error && text.starts_with("scene.svg 160x120, 7 shapes"),
        "{text}"
    );
    // Click the red rect (canvas 28,20), the empty background and outside the stage.
    assert!(view.pointer(STAGE, (STAGE.x + 28, STAGE.y + 20)));
    assert!(view.status().unwrap().0.contains("shape 0 at (14.0, 10.0)"));
    assert!(view.pointer(STAGE, (STAGE.x + 80, STAGE.y + 50)));
    assert!(
        view.status()
            .unwrap()
            .0
            .contains("no shape at (40.0, 25.0)")
    );
    assert!(!view.pointer(STAGE, (STAGE.x - 1, STAGE.y)));
    assert!(!view.pointer(STAGE, (STAGE.x + STAGE.width, STAGE.y)));
    assert_eq!(
        DocumentView::canvas_point(STAGE, (STAGE.x + 3, STAGE.y + 4)),
        Some((3.0, 4.0))
    );
}

#[test]
fn view_failures_are_typed_status_text_and_keep_the_last_document() {
    let dir = TempDir::new();
    let good = dir.file("good.svg", &fixture("compose-v1.svg"));
    let mut view = DocumentView::new();
    let mut app = AppDocument::new(10, 10).unwrap();
    view.open(&mut app, &good, (160, 120));
    wait_for(&mut view, |v| v.composed().is_some());
    let revision = app.revision();
    view.open(&mut app, &dir.0.join("missing.svg"), (160, 120));
    assert_eq!(app.revision(), revision, "failed open leaves the session");
    let (text, error) = view.status().unwrap();
    assert!(
        error && text.starts_with("Cannot open missing.svg"),
        "{text}"
    );
    assert!(view.composed().is_some(), "previous document stays");
    let mut revision = 1000;
    for (name, expect) in [
        ("unsupported-filter.svg", "unsupported in V1: filter"),
        ("unsupported-mask.svg", "unsupported in V1: mask"),
        ("unsupported-text.svg", "unsupported in V1: text"),
    ] {
        revision += 1;
        view.set_name(name);
        view.show(&fixture(name), revision, (80, 80));
        wait_for(&mut view, |v| v.status().is_some_and(|s| s.1));
        assert!(view.composed().is_none());
        let text = view.status().unwrap().0.to_owned();
        assert!(text.starts_with(name) && text.contains(expect), "{text}");
    }
    revision += 1;
    view.show(b"<svg \xff/>", revision, (80, 80));
    wait_for(&mut view, |v| {
        v.status().is_some_and(|s| s.0.contains("UTF-8"))
    });
    revision += 1;
    view.show(
        b"<svg xmlns='http://www.w3.org/2000/svg'><rect></svg>",
        revision,
        (80, 80),
    );
    wait_for(&mut view, |v| {
        v.status().is_some_and(|s| s.0.contains("malformed SVG"))
    });
}

#[test]
fn view_resize_supersedes_and_never_shows_a_stale_size() {
    let mut view = DocumentView::new();
    let mut app = AppDocument::new(10, 10).unwrap();
    view.open(
        &mut app,
        Path::new(&fixture_path("compose-v1.svg")),
        (400, 300),
    );
    let bytes = app.source_bytes().unwrap();
    for width in [301, 302, 303, 200] {
        view.show(&bytes, app.revision(), (width, 150));
    }
    wait_for(&mut view, |v| v.composed().is_some());
    let frame = &view.composed().unwrap().frame;
    assert_eq!((frame.width, frame.height), (200, 150));
    std::thread::sleep(Duration::from_millis(50));
    view.poll();
    let frame = &view.composed().unwrap().frame;
    assert_eq!((frame.width, frame.height), (200, 150));
}
