//! Native raster-to-vector tracing (ADR-0036 B5): k-means colour
//! quantisation, contour following, Bezier fitting, region segmentation and
//! inpainting. Everything here is safe, allocation-bounded and deterministic
//! (fixed seed, fixed iteration order, no trigonometry), so the same input
//! always yields the same bytes.

mod contour;
mod fit;
mod layers;
mod quant;

#[cfg(test)]
mod tests;

pub use layers::{Layer, TraceOptions, trace, trace_image};
pub use quant::{NONE, Quantized, quantize};

/// Most pixels an input image may have (checked before any allocation).
pub const MAX_PIXELS: usize = 16 * 1024 * 1024;
/// Most palette colours.
pub const MAX_COLOURS: usize = 64;
/// Most traced contours across all layers.
pub const MAX_CONTOURS: usize = 60_000;
/// Most path-data bytes across all layers.
pub const MAX_OUTPUT_BYTES: usize = 3 * 1024 * 1024;

/// Why a trace, segmentation or inpaint request was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceError {
    /// Zero-sized, mismatched buffer, or more than [`MAX_PIXELS`] pixels.
    BadImage,
    /// `k` outside `1..=MAX_COLOURS`.
    BadColours,
    /// More than [`MAX_CONTOURS`] contours; raise `minArea` or lower `k`.
    TooManyContours,
    /// More than [`MAX_OUTPUT_BYTES`] of path data; raise `tolerance`.
    OutputTooLarge,
    /// A seed, polygon or mask argument is invalid.
    BadRegion,
}

impl TraceError {
    pub fn message(self) -> &'static str {
        match self {
            Self::BadImage => "image is empty, inconsistent or larger than 16 megapixels",
            Self::BadColours => "k must be 1 to 64",
            Self::TooManyContours => "trace produced too many contours; raise minArea or lower k",
            Self::OutputTooLarge => "trace output too large; raise tolerance or minArea",
            Self::BadRegion => "seeds, polygons or mask do not fit the image",
        }
    }
}

/// Borrowed straight-alpha RGBA8 image.
#[derive(Clone, Copy, Debug)]
pub struct Rgba<'a> {
    pub width: usize,
    pub height: usize,
    pub data: &'a [u8],
}

impl<'a> Rgba<'a> {
    /// Checks dimensions and buffer length before anything is allocated.
    pub fn new(width: usize, height: usize, data: &'a [u8]) -> Result<Self, TraceError> {
        let pixels = width.checked_mul(height).ok_or(TraceError::BadImage)?;
        if pixels == 0 || pixels > MAX_PIXELS || data.len() != pixels * 4 {
            return Err(TraceError::BadImage);
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }
}
