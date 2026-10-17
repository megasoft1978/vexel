//! Background GIF export job for the live preview: the window loop keeps
//! polling events while frames render, and closing the window cancels it.

use super::export::{ExportError, ExportRequest, ExportSummary, export_gif};
use std::ffi::OsString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, channel};
use std::thread;
use vexel_mcp_access::{AccessError, AccessPolicy};

type Reply = SyncSender<Result<String, &'static str>>;

pub struct ExportJob {
    done: Receiver<Result<ExportSummary, ExportError>>,
    cancel: Arc<AtomicBool>,
    reply: Option<Reply>,
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
                let _ = tx.send(export_gif(
                    &source,
                    &policy,
                    &request,
                    &flag,
                    &mut |_, _| {},
                ));
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
            Err(TryRecvError::Disconnected) => Err(ExportError::Cancelled),
        };
        let (reply, status) = match result {
            Ok(s) => (
                Ok(format!(
                    "{{\"ok\":true,\"frames\":{},\"delayCs\":{},\"width\":{},\"height\":{},\"bytes\":{}}}",
                    s.frames, s.delay_cs, s.width, s.height, s.bytes
                )),
                format!("Exported {} GIF frames", s.frames),
            ),
            Err(error) => {
                let message = error_message(&error);
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
