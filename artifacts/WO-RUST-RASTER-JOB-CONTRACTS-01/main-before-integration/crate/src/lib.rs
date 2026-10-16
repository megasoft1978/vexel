#![forbid(unsafe_code)]
//! Owned scene data. SVG decoding, scalar rendering and native providers are separate gates.
mod builder;
mod ids;
mod media;
mod memory;
mod model;
mod mutators;
mod shape_builder;
mod types;
mod validate;
pub use builder::*;
pub use ids::*;
pub use media::*;
pub use memory::*;
pub use model::*;
pub use types::*;
pub use vexel_base::{Span, Stop, WorkControl, WorkKind};
pub use vexel_geometry::{Affine, Edge, Point};
pub use vexel_svg_animation::{DocumentView as AnimationView, OwnedDocument as OwnedAnimation};
pub type SourceOwner = std::sync::Arc<Vec<u8>>;
#[cfg(test)]
mod validation_tests;

mod accounting;
mod contract_validate;
mod decode_contract;
mod pattern_ticket;
mod render_contract;
pub use accounting::*;
pub use decode_contract::*;
pub use pattern_ticket::*;
pub use render_contract::*;
mod frame;
pub use frame::*;
mod contract_storage;
mod source_context;
pub use source_context::*;

mod dash_validate;
pub use dash_validate::STROKE_GRID_JOIN_BIT;
mod lexical_document;
pub use lexical_document::*;

mod draft;
mod draft_updates;
pub use draft::DraftToken;

mod paint_publication;
mod media_source_map;
pub use paint_publication::{GradientTemplateId, PublishedStops};
