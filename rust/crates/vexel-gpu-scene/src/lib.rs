#![forbid(unsafe_code)]
//! Safe scene encoding for the GPU display rasterizer (ADR-0035).
//!
//! This crate owns the byte layout shared between the Rust encoder and the
//! Metal Shading Language kernels ([`layout`], mirrored by
//! `shaders/layout.h`), the support classification of a scene
//! ([`Capability`]) and the runtime error and fallback policy ([`GpuError`],
//! [`policy`]). It contains no GPU calls: Metal FFI lives in
//! `vexel-platform-apple`. The CPU rasterizer in `vexel-render` stays the
//! byte-identical reference for export, `render_png`, goldens and MCP.
//!
//! The encoder input is a plain-data scene defined here rather than the
//! `vexel-render` tree, so this crate does not depend on `vexel-app` or on
//! the render crate at run time; the app lowers its `RenderNode` tree into
//! this crate's types.

pub mod bin;
mod bytes;
mod classify;
mod encode;
mod error;
pub mod input;
pub mod layout;
pub mod policy;
pub mod reference;

#[cfg(test)]
mod bench;

pub use bytes::{
    clip_bytes, edge_bytes, gradient_bytes, header_bytes, list_bytes, shape_bytes, stop_bytes,
    tile_bytes,
};
pub use classify::{
    MAX_CANVAS_AXIS, MAX_COORDINATE, MAX_EDGE_TILE_WORK, MAX_GRADIENT_STOPS, classify,
};
pub use encode::{Encoded, Encoder, encode, encode_unclipped, pack_rgba};
pub use error::{Capability, GpuError, UnsupportedReason};
pub use input::{
    GpuCap, GpuClipPath, GpuColorStop, GpuGradient, GpuGroup, GpuJoin, GpuNode, GpuPaint, GpuScene,
    GpuSegment, GpuShape, GpuSpread, GpuStroke,
};
