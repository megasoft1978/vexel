//! Serves MCP requests from the shell loop: one request per pass, never while
//! a GIF export is running. Also keeps the MCP link state and last outcome.

use std::sync::mpsc::Receiver;

use vexel_app::animation::{driver::Driver, job::ExportJob, playback::MonotonicClock};
use vexel_app::file_ui::FileUi;
use vexel_app::model::{self, Model};
use vexel_app::native_animation::host::NativeAnimationHost;
use vexel_app::preview::{Operation, UiRequest};
use vexel_app::session::{McpGrants, McpHost, McpOutcome};
use vexel_app::tools::ToolController;
use vexel_platform_apple::PanelBackend;

use crate::reset_animation_state;

pub struct McpPump {
    rx: Option<Receiver<UiRequest>>,
    host: McpHost,
    export: Option<ExportJob>,
    granted: bool,
}

impl McpPump {
    pub fn new(rx: Option<Receiver<UiRequest>>, grants: McpGrants) -> Self {
        let granted = !grants.dirs().is_empty();
        Self {
            rx,
            host: McpHost::with_policy(grants.policy()),
            export: None,
            granted,
        }
    }

    /// Link state when no request or export is in flight.
    pub fn idle_state(&self) -> model::McpState {
        model::McpState::for_session(self.granted, false)
    }

    /// Finishes a running export and serves at most one queued request.
    /// Returns true when the shell must redraw.
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
        if self.export.is_some() {
            return dirty;
        }
        let Some(UiRequest { operation, reply }) =
            self.rx.as_ref().and_then(|rx| rx.try_recv().ok())
        else {
            return dirty;
        };
        let opens = matches!(operation, Operation::Open { .. });
        model.mcp = model::McpState::for_session(self.granted, true);
        match files.handle_mcp(&mut self.host, operation, driver) {
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
                match ExportJob::spawn(
                    source,
                    self.host.policy().clone(),
                    path.into(),
                    fps,
                    duration,
                    reply.clone(),
                ) {
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
        true
    }
}
