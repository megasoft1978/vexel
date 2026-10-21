//! `batch`: applies parsed operations in order inside one journal group, so
//! the whole batch is one undo step and, when atomic, all or nothing.

use super::AppDocument;
use super::McpHost;
use super::mcp::McpOutcome;
use crate::animation::driver::Driver;
use crate::animation::playback::LoopClock;
use crate::preview::batch::Batch;

fn escape(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for c in message.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn entry(tool: &str, result: &Result<String, &str>) -> String {
    match result {
        Ok(json) => format!("{{\"tool\":\"{tool}\",\"ok\":true,\"result\":{json}}}"),
        Err(message) => format!(
            "{{\"tool\":\"{tool}\",\"ok\":false,\"error\":\"{}\"}}",
            escape(message)
        ),
    }
}

impl McpHost {
    pub(super) fn batch<C: LoopClock>(
        &mut self,
        app: &mut AppDocument,
        batch: Batch,
        driver: &mut Driver<C>,
    ) -> Result<McpOutcome, &'static str> {
        let group = app
            .begin_group()
            .map_err(|_| "project history unavailable")?;
        let mut results: Vec<String> = Vec::with_capacity(batch.ops.len());
        let mut failures = 0usize;
        for (index, op) in batch.ops.into_iter().enumerate() {
            let outcome = match self.handle_with(app, op.operation, driver) {
                Ok(McpOutcome::Reply(json)) => Ok(json),
                Ok(McpOutcome::Export { .. }) => Err("tool is not allowed in a batch"),
                Err(message) => Err(message),
            };
            if let (Err(message), true) = (&outcome, batch.atomic) {
                app.rollback_group(group)
                    .map_err(|_| "batch rollback failed: project history unavailable")?;
                return Ok(McpOutcome::Reply(format!(
                    "{{\"ok\":false,\"failedIndex\":{index},\"error\":\"{}\",\"results\":[{}]}}",
                    escape(message),
                    results.join(",")
                )));
            }
            failures += usize::from(outcome.is_err());
            results.push(entry(&op.tool, &outcome));
        }
        app.end_group(group)
            .map_err(|_| "project history unavailable")?;
        Ok(McpOutcome::Reply(format!(
            "{{\"ok\":{},\"failedCount\":{failures},\"results\":[{}]}}",
            failures == 0,
            results.join(",")
        )))
    }
}
