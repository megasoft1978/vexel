#![forbid(unsafe_code)]
//! Bounded RGBA tile storage. Rendering and tile caching are separate ports.

/// Largest accepted width or height, matching `RASTER_TILE_MAX_AXIS`.
pub const MAX_AXIS: u32 = 4096;

/// Maximum reserved RGBA capacity accepted when importing an owned buffer.
pub const MAX_ALLOCATED_BYTES: usize = 64 * 1024 * 1024;

/// Import failure. The supplied buffer is returned unchanged with the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileImportError {
    InvalidLayout(TileError),
    InvalidByteCount { expected: usize, actual: usize },
    CapacityLimit { maximum: usize, actual: usize },
}

/// Failure while validating or allocating tile storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileError {
    /// Either dimension is zero or exceeds [`MAX_AXIS`].
    InvalidDimensions,
    /// The RGBA byte count cannot be represented by `usize`.
    SizeOverflow,
    /// The allocation could not be reserved.
    OutOfMemory,
}

/// An owned row-major RGBA tile.
///
/// Dropping the value releases its bytes. The mutable slice permits callers to
/// edit pixels without exposing or transferring the allocation itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Tile {
    /// Import owned, tightly packed RGBA pixels without copying or allocating.
    ///
    /// The buffer's reserved capacity is retained and bounded. Every failure
    /// returns the original buffer with its bytes, length and capacity intact.
    pub fn from_owned_rgba(
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<Self, (TileImportError, Vec<u8>)> {
        if width == 0 || height == 0 || width > MAX_AXIS || height > MAX_AXIS {
            return Err((TileImportError::InvalidLayout(TileError::InvalidDimensions), rgba));
        }
        let bytes = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .and_then(|pixels| pixels.checked_mul(4));
        let Some(bytes) = bytes else {
            return Err((TileImportError::InvalidLayout(TileError::SizeOverflow), rgba));
        };
        if rgba.len() != bytes {
            return Err((TileImportError::InvalidByteCount {
                expected: bytes,
                actual: rgba.len(),
            }, rgba));
        }
        if rgba.capacity() > MAX_ALLOCATED_BYTES {
            return Err((TileImportError::CapacityLimit {
                maximum: MAX_ALLOCATED_BYTES,
                actual: rgba.capacity(),
            }, rgba));
        }
        Ok(Self { width, height, rgba })
    }

    /// Allocate a bounded deterministic opaque RGBA test tile.
    pub fn new(width: u32, height: u32) -> Result<Self, TileError> {
        if width == 0 || height == 0 || width > MAX_AXIS || height > MAX_AXIS {
            return Err(TileError::InvalidDimensions);
        }

        let bytes = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(TileError::SizeOverflow)?;

        let mut rgba = Vec::new();
        rgba.try_reserve_exact(bytes).map_err(|_| TileError::OutOfMemory)?;
        rgba.resize(bytes, 0);

        for y in 0..height {
            for x in 0..width {
                let offset = ((y as usize) * (width as usize) + (x as usize)) * 4;
                rgba[offset] = ((x * 17 + y * 3) & 0xff) as u8;
                rgba[offset + 1] = ((x * 5 + y * 29) & 0xff) as u8;
                rgba[offset + 2] = ((x * 11 + y * 7) & 0xff) as u8;
                rgba[offset + 3] = 255;
            }
        }

        Ok(Self { width, height, rgba })
    }

    /// Tile width in pixels.
    pub const fn width(&self) -> u32 { self.width }

    /// Tile height in pixels.
    pub const fn height(&self) -> u32 { self.height }

    /// Number of bytes in the tightly packed RGBA storage.
    pub const fn byte_count(&self) -> usize { self.rgba.len() }

    /// Reserved pixel-buffer capacity, including any spare bytes.
    ///
    /// This is the capacity tracked by `Vec`; allocator bookkeeping and total
    /// resident memory are measured separately. Caches charge this capacity.
    pub fn allocated_byte_count(&self) -> usize { self.rgba.capacity() }

    /// Borrow all pixels as an immutable row-major RGBA byte slice.
    pub fn rgba(&self) -> &[u8] { &self.rgba }

    /// Borrow all pixels as a mutable row-major RGBA byte slice.
    pub fn rgba_mut(&mut self) -> &mut [u8] { &mut self.rgba }
}
