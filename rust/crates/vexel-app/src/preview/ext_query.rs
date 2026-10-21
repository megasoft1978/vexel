//! Lane-owned: element_get/element_query/tree_get routed onto the live document (coordinator stub).

use super::PreviewArgumentError;
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonDocument;

/// One validated request of this tool family.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {}

/// `None` when `tool` is not in this family.
pub(super) fn parse(
    _doc: &JsonDocument<'_>,
    _args: usize,
    _tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    None
}

pub(crate) fn apply(_app: &mut AppDocument, op: Op) -> Result<McpOutcome, &'static str> {
    match op {}
}
