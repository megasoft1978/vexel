#![forbid(unsafe_code)]
//! Source-preserving linear and radial SVG paint resolution and sampling.
//!
//! The crate returns owned paint plans. Source spans and CSS values never
//! escape their source owner, while stop data is returned by value for the
//! decoder's draft transaction to append atomically.
mod paint;
pub use paint::*;
mod cache;
mod control;
mod gradient_data;
mod resolver;
pub use resolver::{PaintReference, classify_fill, resolve_fill};
