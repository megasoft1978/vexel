//! Schema of the preview-only `element_reparent` tool.
use super::ToolSchema;

pub(crate) const STRUCTURE_TOOLS: &[ToolSchema] = &[ToolSchema {
    name: "element_reparent",
    description: "Move an element into another group (or back to the root) in one undoable edit. parentId is \"root\" or a group id that is neither the element nor inside it. index is the zero-based position among the destination's other children; omitted or past the end appends. The element keeps its id; transforms of the old and new ancestors are not compensated. Example: {\"elementId\":\"vexel-object-3\",\"parentId\":\"vexel-object-9\",\"index\":0}.",
    properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"parentId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"index\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":100000}}",
    required: "documentId,elementId,parentId",
    fields: "documentId:string;elementId:string;parentId:string;index:integer",
}];
