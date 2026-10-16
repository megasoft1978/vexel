#![forbid(unsafe_code)]
//! Vexel application library: the window-free document session, MCP preview
//! routing, tools, animation playback and composition used by the `vexel`
//! binary and by integration tests.

pub mod animation;
pub mod chrome;
pub mod compose;
pub mod file_flow;
pub mod file_ui;
mod inspector;
pub mod model;
pub mod preview;
pub mod preview_app;
pub mod project_session;
pub mod render;
pub mod session;
pub mod tools;
