#![forbid(unsafe_code)]
//! Strict native Vexel project bundle v1 codec.

mod bundle;

pub use bundle::{
    ASSET_HEADER_BYTES, Asset, BundleFailure, BundleInput, BundleView, Error, HEADER_BYTES, MAGIC,
    MAX_ASSET_BYTES, MAX_ASSET_COUNT, MAX_ASSET_NAME_BYTES, MAX_ASSET_TOTAL_BYTES, MAX_BYTES,
    MAX_MANIFEST_BYTES, MAX_MEDIA_TYPE_BYTES, MAX_SCENE_BYTES, MAX_TIMELINE_BYTES, Result, VERSION,
    decode, encode, encode_into, encoded_size, has_magic,
};
