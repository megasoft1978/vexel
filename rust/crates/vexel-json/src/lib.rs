#![forbid(unsafe_code)]
//! Bounded JSON tokenizer, decoded accessors, raw spans, and serializer buffer.
mod buffer;
mod document;

pub use buffer::{JsonBuffer, JsonBufferError};
pub use document::{
    JsonDocument, JsonError, JsonToken, JsonType, MAX_BYTES, MAX_DEPTH, MAX_TOKENS,
};
