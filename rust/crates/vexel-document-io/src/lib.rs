#![forbid(unsafe_code)]
//! Byte-preserving source document I/O.

mod source_save;

pub use source_save::{
    source_save_error_name, source_save_noop, SourceSaveError, SourceSaveHooks,
    SourceSaveResult,
};
