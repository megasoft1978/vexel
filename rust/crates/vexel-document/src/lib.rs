#![forbid(unsafe_code)]
//! Borrowing, source-preserving SVG document model.

mod model;
mod parse;
mod types;

pub use model::{DocumentModel, OwnedDocumentModel};
pub use parse::{parse_document, parse_document_large, parse_document_large_cancellable, parse_owned_document,
    parse_owned_document_large, parse_owned_document_large_cancellable, DocumentError, DocumentParseError,
    LargeDocumentError, OwnedParseError};
pub use types::{DocumentHandle, DocumentSourceSpan};

#[cfg(test)]
mod tests;
