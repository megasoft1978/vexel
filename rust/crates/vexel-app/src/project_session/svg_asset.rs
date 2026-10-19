//! Owned SVG drawing assets admitted to a native timeline transaction.

use crate::animation::native_drawing::{
    DrawingError, SVG_MEDIA_TYPE, SvgGeometry, validate_drawing_svg,
};
use vexel_project::{
    MAX_ASSET_BYTES, MAX_ASSET_COUNT, MAX_ASSET_NAME_BYTES, MAX_ASSET_TOTAL_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgAssetError {
    InvalidName,
    InvalidMediaType,
    DuplicateName,
    Limit,
    OutOfMemory,
    Drawing(DrawingError),
}

/// An asset owned by the pending project-payload candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedProjectAsset {
    pub name: Vec<u8>,
    pub media_type: Vec<u8>,
    pub source: Vec<u8>,
}

impl OwnedProjectAsset {
    /// Validate and copy a complete SVG document without normalizing its bytes.
    pub fn import(
        name: &[u8],
        media_type: &[u8],
        source: &[u8],
        geometry: SvgGeometry,
        existing: &[Self],
    ) -> Result<Self, SvgAssetError> {
        if name.is_empty()
            || name.len() > MAX_ASSET_NAME_BYTES
            || name == b"."
            || name == b".."
            || !name
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'-'))
        {
            return Err(SvgAssetError::InvalidName);
        }
        if media_type != SVG_MEDIA_TYPE {
            return Err(SvgAssetError::InvalidMediaType);
        }
        if existing.iter().any(|asset| asset.name == name) {
            return Err(SvgAssetError::DuplicateName);
        }
        if existing.len() >= MAX_ASSET_COUNT || source.len() > MAX_ASSET_BYTES {
            return Err(SvgAssetError::Limit);
        }
        let total = existing
            .iter()
            .try_fold(source.len(), |sum, asset| {
                sum.checked_add(asset.source.len())
            })
            .ok_or(SvgAssetError::Limit)?;
        if total > MAX_ASSET_TOTAL_BYTES {
            return Err(SvgAssetError::Limit);
        }
        validate_drawing_svg(source, geometry).map_err(SvgAssetError::Drawing)?;

        Ok(Self {
            name: copy_bytes(name)?,
            media_type: copy_bytes(media_type)?,
            source: copy_bytes(source)?,
        })
    }
}

fn copy_bytes(source: &[u8]) -> Result<Vec<u8>, SvgAssetError> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(source.len())
        .map_err(|_| SvgAssetError::OutOfMemory)?;
    owned.extend_from_slice(source);
    Ok(owned)
}
