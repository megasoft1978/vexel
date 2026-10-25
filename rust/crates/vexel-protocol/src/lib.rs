#![forbid(unsafe_code)]
//! Safe, bounded MCP JSON-RPC protocol behavior for Vexel.

mod async_protocol;
mod protocol;
mod tools;
mod tools_audio;
mod tools_batch;
mod tools_native_bulk;
mod tools_native_edit;
mod tools_paint;
mod tools_face;
mod tools_rig;
mod tools_structure;

use vexel_json::{JsonBuffer, JsonDocument};

pub use async_protocol::{legacy_progress, tasks_ready, validate_async_payload, AsyncDisposition, AsyncPayloadKind, ProgressError};
pub use protocol::{handle, handle_async, request_expects_response, ProtocolSession};
pub use tools::{element_add_keys, tool_schemas, validate_tool_arguments, ToolValidationError};
use vexel_json::JsonBufferError;

/// Safe replacement for the former void-pointer synchronous callback table.
pub trait ProtocolBackend {
    fn call_tool(&mut self, request: &JsonDocument<'_>, arguments: usize, name: &str) -> Option<ToolResponse>;
    fn read_resource(&mut self, _request: &JsonDocument<'_>, _uri: &str) -> Option<Vec<u8>> { None }
}

/// Safe replacement for the former void-pointer opt-in async backend.
pub trait AsyncBackend {
    fn tasks_available(&self) -> bool;
    fn call_tool(&mut self, request: &JsonDocument<'_>, arguments: usize, name: &str, modern: bool, supports_tasks: bool) -> (AsyncDisposition, Vec<u8>);
    fn get_task(&mut self, request: &JsonDocument<'_>, task_id: &str) -> (AsyncDisposition, Vec<u8>);
    fn cancel_task(&mut self, request: &JsonDocument<'_>, task_id: &str) -> (AsyncDisposition, Vec<u8>);
    fn cancel_request(&mut self, _request: &JsonDocument<'_>, _request_id: usize) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResponse {
    pub structured_json: Vec<u8>,
    pub content_json: Vec<u8>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ToolSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub properties: &'static str,
    pub required: &'static str,
    pub fields: &'static str,
}

pub(crate) fn append_raw(buffer: &mut JsonBuffer, document: &JsonDocument<'_>, token: usize) -> Result<(), ProtocolError> {
    buffer.raw_token(document, token).map_err(ProtocolError::Buffer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError { InvalidInput, Allocation, Buffer(JsonBufferError) }

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}

pub(crate) fn append_string(buffer: &mut JsonBuffer, value: &[u8]) -> Result<(), ProtocolError> {
    buffer.quoted(value).map_err(ProtocolError::Buffer)
}
