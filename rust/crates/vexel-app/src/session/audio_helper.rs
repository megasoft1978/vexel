//! The optional audio generation helper process (`tools/vexel-audio`).
//!
//! Vexel spawns it on the first generation request and talks JSON lines over
//! its stdin/stdout. User text only ever travels inside the JSON on stdin;
//! the command line is fixed, nothing goes through a shell. The child is
//! killed on timeout, on a malformed or oversized reply, and when the handle
//! is dropped; it also exits by itself when its stdin closes, so a crashed or
//! killed Vexel leaves no orphan.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Environment variable naming the helper executable.
pub const HELPER_ENV: &str = "VEXEL_AUDIO_HELPER";
/// Longest accepted reply line from the helper.
pub const MAX_REPLY_BYTES: usize = 16 * 1024;
/// Longest request line sent to the helper.
pub const MAX_REQUEST_BYTES: usize = 32 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum HelperFailure {
    /// No helper executable could be found.
    NotFound,
    /// The process could not be started.
    Spawn,
    /// The helper did not answer in time and was killed.
    Timeout,
    /// The helper exited or closed its output (it will be restarted).
    Crashed,
    /// The reply was oversized, not JSON, or answered another request.
    Malformed,
}

enum Line {
    Text(Vec<u8>),
    TooLong,
    Closed,
}

pub struct Helper {
    child: Child,
    stdin: ChildStdin,
    replies: Receiver<Line>,
}

/// The helper executable: `VEXEL_AUDIO_HELPER`, else `vexel-audio` next to
/// the binary, else `tools/vexel-audio/vexel-audio` in a parent directory of
/// the binary (a repo checkout or an unpacked release).
pub fn locate() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os(HELPER_ENV) {
        let path = PathBuf::from(value);
        return path.is_file().then_some(path);
    }
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?;
    for _ in 0..6 {
        for candidate in [
            dir.join("vexel-audio"),
            dir.join("tools").join("vexel-audio").join("vexel-audio"),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        dir = dir.parent()?;
    }
    None
}

fn read_lines(source: impl Read, out: mpsc::SyncSender<Line>) {
    let mut reader = BufReader::new(source);
    loop {
        let mut buf = Vec::new();
        let read = (&mut reader)
            .take(MAX_REPLY_BYTES as u64 + 1)
            .read_until(b'\n', &mut buf);
        let line = match read {
            Ok(0) | Err(_) => Line::Closed,
            Ok(_) if buf.len() > MAX_REPLY_BYTES && buf.last() != Some(&b'\n') => Line::TooLong,
            Ok(_) => Line::Text(buf),
        };
        let stop = !matches!(line, Line::Text(_));
        if out.send(line).is_err() || stop {
            return;
        }
    }
}

impl Helper {
    pub fn spawn(path: &Path) -> Result<Self, HelperFailure> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| HelperFailure::Spawn)?;
        let (stdin, stdout) = match (child.stdin.take(), child.stdout.take()) {
            (Some(i), Some(o)) => (i, o),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(HelperFailure::Spawn);
            }
        };
        let (tx, replies) = mpsc::sync_channel(4);
        std::thread::spawn(move || read_lines(stdout, tx));
        Ok(Self {
            child,
            stdin,
            replies,
        })
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Whether the process is still running.
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Sends one request line and returns the reply line (without newline).
    /// On any failure the child is killed so the next request starts fresh.
    pub fn request(&mut self, line: &str, timeout: Duration) -> Result<Vec<u8>, HelperFailure> {
        if line.len() > MAX_REQUEST_BYTES || line.contains('\n') {
            return Err(HelperFailure::Malformed);
        }
        let sent = self
            .stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.write_all(b"\n"))
            .and_then(|()| self.stdin.flush());
        if sent.is_err() {
            self.kill();
            return Err(HelperFailure::Crashed);
        }
        let deadline = Instant::now() + timeout;
        let outcome = match self
            .replies
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(Line::Text(mut bytes)) => {
                while matches!(bytes.last(), Some(b'\n' | b'\r')) {
                    bytes.pop();
                }
                Ok(bytes)
            }
            Ok(Line::TooLong) => Err(HelperFailure::Malformed),
            Ok(Line::Closed) | Err(RecvTimeoutError::Disconnected) => Err(HelperFailure::Crashed),
            Err(RecvTimeoutError::Timeout) => Err(HelperFailure::Timeout),
        };
        if outcome.is_err() {
            self.kill();
        }
        outcome
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        self.kill();
    }
}
