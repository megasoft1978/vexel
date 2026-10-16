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
mod request;
pub use request::{RequestToken, DOCUMENT_TEXT_COMMAND_LIMIT};
mod ledger_buffer;
pub use ledger_buffer::*;
mod production_decode;
pub use production_decode::*;
mod pattern_publication;
pub use pattern_publication::*;

mod media_source_map;
mod paint_publication;
pub use paint_publication::{GradientTemplateId, PublishedStops};

mod raster_job;
pub use raster_job::*;
mod raster_job_views;
pub use raster_job_views::*;
mod raster_job_scratch;
pub use raster_job_scratch::*;

mod path_topology;
mod raster_composition;
pub use raster_composition::{RasterComposition, ValidatedDashView};
mod mask_post;
pub use mask_post::*;
mod decode_quantum;
pub use decode_quantum::*;

mod decode_initialization;
pub use decode_initialization::*;
