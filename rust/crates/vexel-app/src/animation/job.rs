//! Background GIF export job for the live preview: the window loop keeps
//! polling events while frames render, and closing the window cancels it.

use super::export::{ExportError, ExportRequest, ExportSummary, export_gif};
use super::native_export::{NativeExportError, NativeExportSource};
use super::native_export_message::{Tool, render_failure};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, channel};
use std::thread;
use vexel_mcp_access::{AccessError, AccessPolicy};

type Reply = SyncSender<Result<String, &'static str>>;

pub struct ExportJob {
    done: Receiver<Result<ExportResult, ExportFailure>>,
    cancel: Arc<AtomicBool>,
    reply: Option<Reply>,
}

enum ExportResult {
    Smil(ExportSummary),
    Native {
        frames: usize,
        delay_cs: u16,
        width: u32,
        height: u32,
        bytes: usize,
    },
}

enum ExportFailure {
    Smil(ExportError),
    Native(NativeExportError),
    Access(AccessError),
    Exists,
    Io,
}

impl ExportJob {
    /// Starts rendering `source` on a worker thread. The MCP `reply` is sent by
    /// [`ExportJob::poll`] when the job ends.
    pub fn spawn(
        source: Vec<u8>,
        policy: AccessPolicy,
        path: OsString,
        fps: u32,
        duration: f64,
        reply: Reply,
    ) -> Result<Self, &'static str> {
        Self::spawn_inner(source, None, policy, path, fps, duration, reply)
    }

    pub fn spawn_native(
        native: NativeExportSource,
        policy: AccessPolicy,
        path: OsString,
        reply: Reply,
    ) -> Result<Self, &'static str> {
        Self::spawn_inner(Vec::new(), Some(native), policy, path, 1, 0.01, reply)
    }

    pub fn spawn_native_packed(
        packed: Vec<u8>,
        policy: AccessPolicy,
        path: OsString,
        reply: Reply,
    ) -> Result<Self, &'static str> {
        const HEADER: &[u8] = b"VEXEL_NATIVE_GIF\0";
        let start = HEADER.len();
        if packed.len() < start + 18 || !packed.starts_with(HEADER) {
            return Err("export could not start");
        }
        let frames = u64::from_le_bytes(packed[start..start + 8].try_into().unwrap()) as usize;
        let delay_cs = u16::from_le_bytes(packed[start + 8..start + 10].try_into().unwrap());
        let width = u32::from_le_bytes(packed[start + 10..start + 14].try_into().unwrap());
        let height = u32::from_le_bytes(packed[start + 14..start + 18].try_into().unwrap());
        let bytes = packed[start + 18..].to_vec();
        Self::spawn_packed_write(bytes, frames, delay_cs, width, height, policy, path, reply)
    }

    fn spawn_packed_write(
        bytes: Vec<u8>,
        frames: usize,
        delay_cs: u16,
        width: u32,
        height: u32,
        policy: AccessPolicy,
        path: OsString,
        reply: Reply,
    ) -> Result<Self, &'static str> {
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, done) = channel();
        thread::Builder::new()
            .name("vexel-gif-export".to_owned())
            .spawn(move || {
                let result = write_native(&policy, Path::new(&path), &bytes).map(|()| {
                    ExportResult::Native {
                        frames,
                        delay_cs,
                        width,
                        height,
                        bytes: bytes.len(),
                    }
                });
                let _ = tx.send(result);
            })
            .map_err(|_| "export could not start")?;
        Ok(Self {
            done,
            cancel,
            reply: Some(reply),
        })
    }

    fn spawn_inner(
        source: Vec<u8>,
        native: Option<NativeExportSource>,
        policy: AccessPolicy,
        path: OsString,
        fps: u32,
        duration: f64,
        reply: Reply,
    ) -> Result<Self, &'static str> {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let (tx, done) = channel();
        thread::Builder::new()
            .name("vexel-gif-export".to_owned())
            .spawn(move || {
                let request = ExportRequest {
                    path: &path,
                    fps,
                    duration,
                    overwrite: false,
                };
                let result = if let Some(native) = native {
                    native
                        .export_gif()
                        .map_err(ExportFailure::Native)
                        .and_then(|gif| {
                            write_native(&policy, Path::new(&path), &gif.bytes).map(|()| {
                                ExportResult::Native {
                                    frames: gif.frames,
                                    delay_cs: gif.delay_cs,
                                    width: gif.width,
                                    height: gif.height,
                                    bytes: gif.bytes.len(),
                                }
                            })
                        })
                } else {
                    export_gif(&source, &policy, &request, &flag, &mut |_, _| {})
                        .map(ExportResult::Smil)
                        .map_err(ExportFailure::Smil)
                };
                let _ = tx.send(result);
            })
            .map_err(|_| "export could not start")?;
        Ok(Self {
            done,
            cancel,
            reply: Some(reply),
        })
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// True while the job's reply is still pending (the worker may still be writing).
    pub fn is_running(&self) -> bool {
        self.reply.is_some()
    }

    /// Returns a status line once the job has finished and its reply was sent.
    pub fn poll(&mut self) -> Option<String> {
        self.reply.as_ref()?;
        let result = match self.done.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(ExportFailure::Smil(ExportError::Cancelled)),
        };
        let (reply, status) = match result {
            Ok(ExportResult::Smil(s)) => (
                Ok(format!(
                    "{{\"ok\":true,\"frames\":{},\"delayCs\":{},\"width\":{},\"height\":{},\"bytes\":{}}}",
                    s.frames, s.delay_cs, s.width, s.height, s.bytes
                )),
                format!("Exported {} GIF frames", s.frames),
            ),
            Ok(ExportResult::Native {
                frames,
                delay_cs,
                width,
                height,
                bytes,
            }) => (
                Ok(format!(
                    "{{\"ok\":true,\"frames\":{frames},\"delayCs\":{delay_cs},\"width\":{width},\"height\":{height},\"bytes\":{bytes}}}"
                )),
                format!("Exported {frames} GIF frames"),
            ),
            Err(error) => {
                let message = failure_message(&error);
                (Err(message), message.to_owned())
            }
        };
        if let Some(sender) = self.reply.take() {
            let _ = sender.send(reply);
        }
        Some(status)
    }
}

/// Fixed messages safe to return to an MCP client (no paths, no OS text).
pub fn error_message(error: &ExportError) -> &'static str {
    match error {
        ExportError::InvalidFps | ExportError::InvalidDuration => "export arguments are invalid",
        ExportError::NotAnimated => "export needs an animation",
        ExportError::DurationExceedsLoop => "export duration exceeds the animation loop",
        ExportError::Access(AccessError::NotGranted) => "export path is not in a granted directory",
        ExportError::Access(_) => "export path is invalid",
        ExportError::Exists => "export file already exists",
        ExportError::Frame(_) => "export could not render the animation",
        ExportError::Encode(_) => "export exceeds frame or size limits",
        ExportError::Cancelled => "export cancelled",
        ExportError::Io(_) => "export could not write the file",
    }
}

fn failure_message(error: &ExportFailure) -> &'static str {
    match error {
        ExportFailure::Smil(error) => error_message(error),
        ExportFailure::Native(NativeExportError::Timeline) => "export needs an animation",
        ExportFailure::Native(
            error @ (NativeExportError::Viewport(_)
            | NativeExportError::Frame(_)
            | NativeExportError::TimeOutOfRange { .. }),
        ) => render_failure(Tool::ExportGif, error),
        ExportFailure::Native(NativeExportError::Encode(_)) => {
            "export exceeds frame or size limits"
        }
        ExportFailure::Access(AccessError::NotGranted) => {
            "export path is not in a granted directory"
        }
        ExportFailure::Access(_) => "export path is invalid",
        ExportFailure::Exists => "export file already exists",
        ExportFailure::Io => "export could not write the file",
    }
}

fn write_native(policy: &AccessPolicy, path: &Path, bytes: &[u8]) -> Result<(), ExportFailure> {
    let target = policy
        .resolve_save_path(path.as_os_str())
        .map_err(ExportFailure::Access)?;
    if target.exists() {
        return Err(ExportFailure::Exists);
    }
    let name = target.file_name().ok_or(ExportFailure::Io)?;
    let temp = target.with_file_name(format!(
        ".{}.vexel-export-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| ExportFailure::Io)?;
        file.write_all(bytes).map_err(|_| ExportFailure::Io)?;
        file.sync_all().map_err(|_| ExportFailure::Io)?;
        drop(file);
        fs::hard_link(&temp, &target).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                ExportFailure::Exists
            } else {
                ExportFailure::Io
            }
        })
    })();
    if temp.exists() {
        let _ = fs::remove_file(temp);
    }
    result
}
