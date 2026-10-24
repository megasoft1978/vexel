#![forbid(unsafe_code)]
//! Source-preserving linear and radial SVG paint resolution and sampling.
//!
//! The crate returns owned paint plans. Source spans and CSS values never
//! escape their source owner, while stop data is returned by value for the
//! decoder's draft transaction to append atomically.
mod paint;
#[cfg(test)]
mod paint_reference;
pub use paint::*;
mod cache;
mod control;
mod gradient_data;
mod handoff;
mod resolver;
pub use handoff::{CachedPaint, CachedResolveContext, OwnedPaint};
pub use resolver::{
    PaintReference, classify_fill, resolve_cached_context, resolve_cached_fill, resolve_fill,
};
