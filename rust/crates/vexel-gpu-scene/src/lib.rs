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

mod error;
pub mod layout;
pub mod policy;

pub use error::{Capability, GpuError, UnsupportedReason};
