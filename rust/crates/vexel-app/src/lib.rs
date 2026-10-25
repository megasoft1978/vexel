#![forbid(unsafe_code)]
//! Vexel application library: the window-free document session, MCP preview
//! routing, tools, animation playback and composition used by the `vexel`
//! binary and by integration tests.

pub mod animation;
pub mod canvas;
pub mod chrome;
pub mod compose;
pub mod file_flow;
pub mod file_ui;
pub mod frames_export;
pub mod history;
mod inspector;
pub mod layout;
pub mod model;
pub mod motion;
pub mod native_animation;
pub mod native_timeline;
pub mod overlay;
pub mod preview;
pub mod preview_app;
pub mod profile;
pub mod project_session;
pub mod recovery_job;
pub mod recovery_policy;
pub mod render;
pub mod session;
pub mod timeline;
pub mod toolbar_actions;
pub mod tools;
pub mod trace;
pub(crate) mod ui_kit;
pub mod video_export;
