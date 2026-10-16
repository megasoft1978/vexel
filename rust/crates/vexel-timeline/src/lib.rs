#![forbid(unsafe_code)]
//! Bounded, source preserving decoder for the native animation timeline format.
mod access;
mod json;
mod schema;
mod edit;

pub use json::string_copy;
pub use schema::{decode, decode_cancellable, classify, classify_cancellable, DecodeFailure, Error, LoopMode,
    TimelineContext, TimelineView, TrackView, ItemView, TimelineIterator, AssetRef,
    MAX_ITEMS, MAX_ITEMS_PER_TRACK, MAX_STRING_BYTES, MAX_TRACKS, MAX_SOURCE_BYTES};
pub use edit::{EditFailure, EditLimits, TimelineEdit, TimelineSourceLease};
pub use edit::{DEFAULT_HISTORY_BYTES, DEFAULT_HISTORY_COMMANDS, MAX_HISTORY_BYTES, MAX_HISTORY_COMMANDS};

pub type Result<T> = std::result::Result<T, DecodeFailure>;
