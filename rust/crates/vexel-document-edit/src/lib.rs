#![forbid(unsafe_code)]
//! Byte-preserving, source-owning SVG edit sessions.
//!
//! The editor stores immutable source fragments in an Arc-backed piece table.
//! A model is built once for each committed source state and is invalidated by
//! every successful edit, undo, or redo.

mod alloc;
mod cache;
mod group;
mod history;
mod insert;
mod ops;
mod piece_table;
mod query;
mod snapshot;
mod storage;
mod types;
mod view;

pub use alloc::allocation_gate;
pub use history::{HistoryEntry, HistoryState};
pub use insert::graphics_tag;
pub use insert::{TemplateAttribute, TemplateKind, XML_FRAGMENT_LIMIT};
pub use ops::{encode_attribute, encode_text, name_valid};
pub use piece_table::Editor;
pub use query::{Attribute, SourceSpan, decode_bytes};
pub use snapshot::EditSnapshot;
pub use types::*;
pub use vexel_document::DocumentHandle;
pub use view::{
    DocumentViewRow, document_view_attribute_at, document_view_attribute_count,
    document_view_first_child, document_view_next_sibling, document_view_node_count,
    document_view_range, document_view_root, document_view_tag,
};

/// Owned Rust spelling of the C document_edit API.
pub type DocumentEdit = Editor;
pub type DocumentEditSnapshot = EditSnapshot;

#[cfg(test)]
mod internal_tests;
