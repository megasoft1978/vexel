#![forbid(unsafe_code)]
//! Bounded, source preserving decoder for the native animation timeline format.
mod access;
mod easing;
mod edit;
mod json;
pub mod sample;
mod schema;

#[cfg(test)]
mod sample_tests;

pub use edit::{
    DEFAULT_HISTORY_BYTES, DEFAULT_HISTORY_COMMANDS, MAX_HISTORY_BYTES, MAX_HISTORY_COMMANDS,
};
pub use edit::{EditFailure, EditLimits, TimelineEdit, TimelineSourceLease};
pub use edit::{EmptyTimelineSettings, initialize_empty_timeline};
pub use json::string_copy;
pub use sample::{PropertySample, PropertyValue, sample_properties};
pub use schema::{
    AssetRef, DecodeFailure, Error, ItemView, LoopMode, MAX_ITEMS, MAX_ITEMS_PER_TRACK,
    MAX_SOURCE_BYTES, MAX_STRING_BYTES, MAX_TRACKS, TimelineContext, TimelineIterator,
    TimelineView, TrackView, classify, classify_cancellable, decode, decode_cancellable,
};

pub type Result<T> = std::result::Result<T, DecodeFailure>;
