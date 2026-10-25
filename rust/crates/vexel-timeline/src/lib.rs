#![forbid(unsafe_code)]
//! Bounded, source preserving decoder for the native animation timeline format.
mod access;
pub mod audio;
pub mod audio_codec;
pub mod audio_envelope;
pub mod audio_mix;
pub mod blink;
pub mod clips;
mod easing;
mod edit;
mod json;
pub mod lipsync;
pub mod noise;
pub mod pathmorph;
pub mod rig;
pub mod sample;
mod schema;

#[cfg(test)]
mod audio_tests;
#[cfg(test)]
mod path_track_tests;
#[cfg(test)]
mod sample_tests;

pub use edit::{
    DEFAULT_HISTORY_BYTES, DEFAULT_HISTORY_COMMANDS, MAX_HISTORY_BYTES, MAX_HISTORY_COMMANDS,
};
pub use edit::{BulkFailure, BulkKey, BulkTrack};
pub use edit::{EditFailure, EditLimits, TimelineEdit, TimelineSourceLease};
pub use edit::{EmptyTimelineSettings, initialize_empty_timeline};
pub use json::string_copy;
pub use sample::{PathBlend, PropertySample, resolve_path, PropertyValue, sample_properties, sample_properties_keyed};
pub use schema::{
    AssetRef, CAMERA_TARGET, DecodeFailure, DetachedTimeline, Error, ItemView, LoopMode, MAX_ITEMS, MAX_ITEMS_PER_TRACK,
    MAX_SOURCE_BYTES, MAX_STRING_BYTES, MAX_TRACKS, TimelineContext, TimelineIterator,
    TimelineView, TrackView, classify, classify_cancellable, decode, decode_cancellable,
};

pub type Result<T> = std::result::Result<T, DecodeFailure>;
