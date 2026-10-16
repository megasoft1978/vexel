//! Window-free drawing and revision tools (ADR-0029 "draw and revise").
//!
//! Every action is a source-preserving edit of a [`vexel_v1_document::V1Document`]
//! with undo/redo; the document is the only shape store. Live canvas wiring
//! (pointer events from the native window into [`ToolController`]) is an
//! integration step that follows the compose mapping; see [`map`].

mod controller;
mod map;
mod pen;

pub use controller::{Key, Outcome, PointerInput, ToolController, ToolError};
pub use map::{CanvasMap, ScaleMap, stage_local};
pub use pen::MAX_PEN_VERTICES;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_edit;
#[cfg(test)]
mod tests_support;
