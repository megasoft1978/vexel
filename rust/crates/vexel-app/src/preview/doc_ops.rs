//! Routing of document-tool families (structure, paint/transform/text,
//! query) onto the live `AppDocument`. Each family lives in its own module.

use super::{Operation, PreviewArgumentError, ext_attr, ext_paint, ext_query, ext_structure};
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonDocument;

#[derive(Clone, Debug, PartialEq)]
pub enum DocOp {
    Structure(ext_structure::Op),
    Paint(ext_paint::Op),
    Attr(ext_attr::Op),
    Query(ext_query::Op),
}

/// `None` when no family handles `tool`.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Operation, PreviewArgumentError>> {
    if let Some(parsed) = ext_structure::parse(doc, args, tool) {
        return Some(parsed.map(|op| Operation::Doc(DocOp::Structure(op))));
    }
    if let Some(parsed) = ext_paint::parse(doc, args, tool) {
        return Some(parsed.map(|op| Operation::Doc(DocOp::Paint(op))));
    }
    if let Some(parsed) = ext_attr::parse(doc, args, tool) {
        return Some(parsed.map(|op| Operation::Doc(DocOp::Attr(op))));
    }
    ext_query::parse(doc, args, tool)
        .map(|parsed| parsed.map(|op| Operation::Doc(DocOp::Query(op))))
}

pub(crate) fn apply(app: &mut AppDocument, op: DocOp) -> Result<McpOutcome, &'static str> {
    match op {
        DocOp::Structure(op) => ext_structure::apply(app, op),
        DocOp::Paint(op) => ext_paint::apply(app, op),
        DocOp::Attr(op) => ext_attr::apply(app, op),
        DocOp::Query(op) => ext_query::apply(app, op),
    }
}
