//! Serves MCP requests from the shell loop: a bounded batch per pass, never
//! while a GIF export is running. An arriving request wakes the idle loop. Also keeps the MCP link state and last outcome.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use vexel_app::animation::{driver::Driver, job::ExportJob, playback::MonotonicClock};
use vexel_app::file_ui::FileUi;
use vexel_app::model::{self, Model};
use vexel_app::native_animation::host::NativeAnimationHost;
use vexel_app::preview::{Operation, UiRequest};
use vexel_app::session::{McpGrants, McpHost, McpOutcome};
use vexel_app::tools::ToolController;
use vexel_platform_apple::PanelBackend;

use crate::reset_animation_state;

/// Requests served per pass and the time after which a pass stops taking more.
const MAX_PER_PASS: usize = 16;
const PASS_BUDGET: Duration = Duration::from_millis(8);

pub struct McpPump {
    rx: Option<Receiver<UiRequest>>,
    /// A request received while waiting for the next pass.
    woken: Option<UiRequest>,
    host: McpHost,
    export: Option<ExportJob>,
    granted: bool,
}

impl McpPump {
    pub fn new(rx: Option<Receiver<UiRequest>>, grants: McpGrants) -> Self {
        let granted = !grants.dirs().is_empty();
        Self {
            rx,
            woken: None,
            host: McpHost::with_policy(grants.policy()),
            export: None,
            granted,
        }
    }

    /// Link state when no request or export is in flight.
    pub fn idle_state(&self) -> model::McpState {
        model::McpState::for_session(self.granted, false)
    }

    /// Sleeps up to `timeout`, returning early when an MCP request arrives
    /// (it is served by the next `poll`).
    pub fn idle_wait(&mut self, timeout: Duration) {
        match (&self.rx, &self.woken) {
            (Some(rx), None) => {
                if let Ok(request) = rx.recv_timeout(timeout) {
                    self.woken = Some(request);
                }
            }
            (_, Some(_)) => {}
            (None, None) => std::thread::sleep(timeout),
        }
    }

    /// Finishes a running export and serves a bounded batch of queued
    /// requests. Returns true when the shell must redraw.
    pub fn poll<B: PanelBackend>(
        &mut self,
        files: &mut FileUi<B>,
        native: &mut NativeAnimationHost<MonotonicClock>,
        driver: &mut Driver<MonotonicClock>,
        tools: &mut ToolController,
        model: &mut Model,
    ) -> bool {
        let mut dirty = false;
        if let Some(done) = self.export.as_mut().and_then(ExportJob::poll) {
            self.export = None;
            model.mcp = self.idle_state();
            model.status = if done.starts_with("Exported") {
                "Exported GIF"
            } else {
                "GIF export failed"
            };
            dirty = true;
        }
        let started = Instant::now();
        for _ in 0..MAX_PER_PASS {
            if self.export.is_some() || (started.elapsed() >= PASS_BUDGET && dirty) {
                break;
            }
            let Some(request) = self
                .woken
                .take()
                .or_else(|| self.rx.as_ref().and_then(|rx| rx.try_recv().ok()))
            else {
                break;
            };
            let (redraw, stop) = self.serve(request, files, native, driver, tools, model);
            dirty |= redraw;
            if stop {
                break;
            }
        }
        dirty
    }

    /// Serves one request. Returns (redraw needed, stop batching).
    fn serve<B: PanelBackend>(
        &mut self,
        UiRequest { operation, reply }: UiRequest,
        files: &mut FileUi<B>,
        native: &mut NativeAnimationHost<MonotonicClock>,
        driver: &mut Driver<MonotonicClock>,
        tools: &mut ToolController,
        model: &mut Model,
    ) -> (bool, bool) {
        let opens = matches!(operation, Operation::Open { .. });
        // View-affecting operations always redraw and end the batch.
        let view = opens
            || matches!(
                operation,
                Operation::Control(_) | Operation::Native(_) | Operation::Export { .. }
            );
        let before = (
            files.document().revision(),
            files.document().state_revision(),
            files.document().is_dirty(),
            driver.is_playing(),
            model.mcp_last,
        );
        model.mcp = model::McpState::for_session(self.granted, true);
        let result = match operation {
            Operation::Control(control) if native.has_native_timeline(files.document()) => native
                .mcp_control(
                    files.document(),
                    control,
                    model.mode == model::Mode::Animate,
                )
                .map(McpOutcome::Reply),
            operation => files.handle_mcp(&mut self.host, operation, driver),
        };
        match result {
            Ok(McpOutcome::Reply(json)) => {
                model.mcp_last = Some("Done");
                if opens {
                    reset_animation_state(native, driver);
                    // Selection ids belong to the replaced document.
                    let tool = tools.tool();
                    *tools = ToolController::new();
                    tools.set_tool(tool);
                }
                let _ = reply.send(Ok(json));
            }
            Ok(McpOutcome::Export {
                source,
                path,
                fps,
                duration,
            }) => {
                let job = if source.starts_with(b"VEXEL_NATIVE_GIF\0") {
                    ExportJob::spawn_native_packed(
                        source,
                        self.host.policy().clone(),
                        path.into(),
                        reply.clone(),
                    )
                } else {
                    ExportJob::spawn(
                        source,
                        self.host.policy().clone(),
                        path.into(),
                        fps,
                        duration,
                        reply.clone(),
                    )
                };
                match job {
                    Ok(job) => self.export = Some(job),
                    Err(message) => {
                        model.mcp_last = Some(message);
                        let _ = reply.send(Err(message));
                    }
                }
            }
            Err(message) => {
                model.mcp_last = Some(message);
                let _ = reply.send(Err(message));
            }
        }
        model.mcp = model::McpState::for_session(self.granted, self.export.is_some());
        let after = (
            files.document().revision(),
            files.document().state_revision(),
            files.document().is_dirty(),
            driver.is_playing(),
            model.mcp_last,
        );
        (view || before != after, view || self.export.is_some())
    }
}
