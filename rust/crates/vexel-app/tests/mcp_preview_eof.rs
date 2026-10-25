//! `vexel --mcp-preview` must exit 0 shortly after stdin closes.
//! Ignored by default: launching the binary opens a native macOS window, which
//! the headless test runs must not do. Run on a desktop session with
//! `cargo test -p vexel-app --test mcp_preview_eof -- --ignored`.
//! The in-process coverage is `mcp_pump::tests` (EOF ends `idle_wait`).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
#[ignore = "opens a native window; run manually on a desktop session"]
fn mcp_preview_exits_after_stdin_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vexel"))
        .arg("--mcp-preview")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    drop(child.stdin.take());
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            panic!("vexel --mcp-preview still running 5 s after stdin EOF");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "exit status {status}");
}
