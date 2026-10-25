//! The optional audio generation helper process (`tools/vexel-audio`).
//!
//! Vexel spawns it on the first generation request and talks JSON lines over
//! its stdin/stdout. User text only ever travels inside the JSON on stdin;
//! the command line is fixed, nothing goes through a shell. The child is
//! killed on timeout, on a malformed or oversized reply, and when the handle
//! is dropped; it also exits by itself when its stdin closes, so a crashed or
//! killed Vexel leaves no orphan.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::sync::OnceLock;
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
    /// The helper file failed the ownership, mode or symlink check.
    Untrusted(&'static str),
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

fn current_uid() -> Option<u32> {
    static UID: OnceLock<Option<u32>> = OnceLock::new();
    *UID.get_or_init(|| {
        let out = Command::new("/usr/bin/id").arg("-u").output().ok()?;
        String::from_utf8(out.stdout).ok()?.trim().parse().ok()
    })
}

/// Accepts `path` only as a regular file (not a symlink) owned by the
/// current user or root and not writable by group or others.
pub fn check_trusted(path: &Path) -> Result<(), &'static str> {
    let meta = fs::symlink_metadata(path).map_err(|_| "audio helper not found")?;
    if meta.file_type().is_symlink() {
        return Err("audio helper refused: it is a symbolic link; point to the real file");
    }
    if !meta.is_file() {
        return Err("audio helper refused: it is not a regular file");
    }
    let owner = meta.uid();
    if owner != 0 && Some(owner) != current_uid() {
        return Err("audio helper refused: it is owned by another user");
    }
    if meta.mode() & 0o022 != 0 {
        return Err("audio helper refused: it is writable by group or others; run chmod go-w on it");
    }
    Ok(())
}

/// The helper executable: `VEXEL_AUDIO_HELPER`, else `vexel-audio` next to
/// the binary or in `../Resources` (app bundle), and in debug builds the
/// repository's `tools/vexel-audio`. There is no search of parent
/// directories. Every candidate must pass [`check_trusted`].
pub fn locate() -> Result<PathBuf, HelperFailure> {
    locate_with(std::env::var_os(HELPER_ENV))
}

fn locate_with(explicit: Option<std::ffi::OsString>) -> Result<PathBuf, HelperFailure> {
    let mut candidates = Vec::new();
    if let Some(value) = explicit {
        candidates.push(PathBuf::from(value));
    } else {
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            candidates.push(dir.join("vexel-audio"));
            candidates.push(dir.join("../Resources/vexel-audio"));
        }
        #[cfg(debug_assertions)]
        candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools/vexel-audio/vexel-audio"));
    }
    let mut refused = None;
    for candidate in candidates {
        if fs::symlink_metadata(&candidate).is_err() {
            continue;
        }
        match check_trusted(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(message) => refused = Some(message),
        }
    }
    Err(refused.map_or(HelperFailure::NotFound, HelperFailure::Untrusted))
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
        check_trusted(path).map_err(HelperFailure::Untrusted)?;
        let mut command = Command::new(path);
        command.env_clear();
        for (key, value) in std::env::vars_os() {
            let name = key.to_string_lossy();
            let keep = matches!(name.as_ref(), "PATH" | "HOME")
                || name.starts_with("VEXEL_")
                || name.starts_with("HF_")
                || name.starts_with("MLX");
            if keep {
                command.env(&key, &value);
            }
        }
        let mut child = command
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vexel-helper-trust-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn script(dir: &Path, mode: u32) -> PathBuf {
        let path = dir.join("vexel-audio");
        fs::write(&path, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    #[test]
    fn env_path_to_a_private_file_is_accepted() {
        let dir = scratch("ok");
        let path = script(&dir, 0o755);
        assert_eq!(locate_with(Some(path.clone().into())), Ok(path));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn group_or_world_writable_helper_is_refused() {
        let dir = scratch("writable");
        for mode in [0o775, 0o757, 0o777] {
            let path = script(&dir, mode);
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            let err = locate_with(Some(path.clone().into())).unwrap_err();
            assert!(matches!(err, HelperFailure::Untrusted(m) if m.contains("writable")), "{mode:o}: {err:?}");
            assert!(matches!(Helper::spawn(&path), Err(HelperFailure::Untrusted(_))));
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn symlinked_helper_is_refused() {
        let dir = scratch("link");
        let real = script(&dir, 0o755);
        let link = dir.join("link");
        symlink(&real, &link).unwrap();
        let err = locate_with(Some(link.clone().into())).unwrap_err();
        assert!(matches!(err, HelperFailure::Untrusted(m) if m.contains("symbolic link")), "{err:?}");
        assert!(matches!(Helper::spawn(&link), Err(HelperFailure::Untrusted(_))));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_env_path_and_directories_are_refused() {
        let dir = scratch("missing");
        assert_eq!(locate_with(Some(dir.join("nope").into())), Err(HelperFailure::NotFound));
        assert!(check_trusted(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
