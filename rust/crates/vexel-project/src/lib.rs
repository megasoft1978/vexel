#![forbid(unsafe_code)]
//! Strict native Vexel project bundle v1 codec.

mod bundle;

pub use bundle::{
    Asset, BundleFailure, BundleInput, BundleView, Error, Result, ASSET_HEADER_BYTES,
    HEADER_BYTES, MAGIC, MAX_ASSET_BYTES, MAX_ASSET_COUNT, MAX_ASSET_NAME_BYTES,
    MAX_ASSET_TOTAL_BYTES, MAX_BYTES, MAX_MANIFEST_BYTES, MAX_MEDIA_TYPE_BYTES,
    MAX_SCENE_BYTES, MAX_TIMELINE_BYTES, VERSION, decode, encode, encode_into,
    encoded_size, has_magic,
};
