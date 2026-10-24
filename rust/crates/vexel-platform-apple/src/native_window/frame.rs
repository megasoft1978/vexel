/// Maximum frame dimension on either axis. This matches the native window
/// sizing limit and caps a full RGBA frame at 64 MiB.
pub const MAX_FRAME_AXIS: u32 = 4096;
const BYTES_PER_PIXEL: usize = 4;
pub(super) const MAX_FRAME_BYTES: usize =
    MAX_FRAME_AXIS as usize * MAX_FRAME_AXIS as usize * BYTES_PER_PIXEL;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbaFrameError {
    ZeroDimension,
    DimensionTooLarge,
    ArithmeticOverflow,
    ByteCountMismatch,
    CapacityTooLarge,
    InvalidScale,
}

/// Owned, top-left-origin, tightly packed RGBA8 pixels.
///
/// RGB channels are straight (unassociated) sRGB values and alpha is an
/// independent linear coverage byte, matching Vexel UI pixels. Rows are
/// stored from top to bottom with no padding, so row zero is the visual top.
/// Presentation converts RGB to premultiplied sRGB bytes for CoreGraphics.
/// The backing scale follows the window boundary's supported integral scales
/// of `1`, `2`, or `3` pixels per point.
#[derive(Debug, PartialEq)]
pub struct RgbaFrame {
    width: u32,
    height: u32,
    backing_scale: f64,
    pixels: Vec<u8>,
}

impl RgbaFrame {
    pub fn new(
        width: u32,
        height: u32,
        backing_scale: f64,
        pixels: Vec<u8>,
    ) -> Result<Self, RgbaFrameError> {
        if width == 0 || height == 0 {
            return Err(RgbaFrameError::ZeroDimension);
        }
        if width > MAX_FRAME_AXIS || height > MAX_FRAME_AXIS {
            return Err(RgbaFrameError::DimensionTooLarge);
        }
        if !backing_scale.is_finite()
            || !(1.0..=3.0).contains(&backing_scale)
            || backing_scale.fract() != 0.0
        {
            return Err(RgbaFrameError::InvalidScale);
        }
        let expected = checked_byte_len(width as usize, height as usize)?;
        if pixels.len() != expected {
            return Err(RgbaFrameError::ByteCountMismatch);
        }
        if pixels.capacity() > MAX_FRAME_BYTES {
            return Err(RgbaFrameError::CapacityTooLarge);
        }
        Ok(Self {
            width,
            height,
            backing_scale,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn backing_scale(&self) -> f64 {
        self.backing_scale
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    /// Converts straight-alpha sRGB input to the premultiplied RGBA encoding
    /// declared to CoreGraphics. Integer rounding is deterministic.
    pub(crate) fn into_premultiplied_pixels(self) -> Vec<u8> {
        let mut pixels = self.pixels;
        premultiply_in_place(&mut pixels);
        pixels
    }
}

/// `(c * a + 127) / 255` without a division, exact for every `c`, `a`.
#[inline(always)]
fn mul_div255(c: u32, a: u32) -> u8 {
    let t = c * a + 127;
    ((t + (t >> 8) + 1) >> 8) as u8
}

/// Straight to premultiplied RGBA8. Opaque pixels (the common UI case) are
/// untouched and fully transparent ones are zeroed, both exactly as the
/// rounded formula would produce.
pub(crate) fn premultiply_in_place(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(BYTES_PER_PIXEL) {
        match pixel[3] {
            255 => {}
            0 => pixel[..3].fill(0),
            alpha => {
                let alpha = u32::from(alpha);
                for channel in &mut pixel[..3] {
                    *channel = mul_div255(u32::from(*channel), alpha);
                }
            }
        }
    }
}

fn checked_byte_len(width: usize, height: usize) -> Result<usize, RgbaFrameError> {
    let bytes = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(BYTES_PER_PIXEL))
        .ok_or(RgbaFrameError::ArithmeticOverflow)?;
    if bytes > MAX_FRAME_BYTES {
        return Err(RgbaFrameError::DimensionTooLarge);
    }
    Ok(bytes)
}

/// Returns the candidate only after the caller's single publish operation
/// succeeds. The caller's current presenter remains untouched on failure.
pub(super) fn publish_candidate<T, E>(
    candidate: T,
    publish: impl FnOnce(&T) -> Result<(), E>,
) -> Result<T, E> {
    publish(&candidate)?;
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_publish_keeps_prior_presenter_state() {
        let mut current = Some("prior presenter");
        let result = publish_candidate("candidate", |_| Err::<(), _>("injected publish failure"));
        if let Ok(candidate) = result {
            current = Some(candidate);
        }
        assert_eq!(current, Some("prior presenter"));
    }

    #[test]
    fn rejects_zero_and_oversized_dimensions() {
        assert_eq!(
            RgbaFrame::new(0, 1, 1.0, vec![]).unwrap_err(),
            RgbaFrameError::ZeroDimension
        );
        assert_eq!(
            RgbaFrame::new(1, 0, 1.0, vec![]).unwrap_err(),
            RgbaFrameError::ZeroDimension
        );
        assert_eq!(
            RgbaFrame::new(MAX_FRAME_AXIS + 1, 1, 1.0, vec![]).unwrap_err(),
            RgbaFrameError::DimensionTooLarge
        );
    }

    #[test]
    fn checks_byte_arithmetic_and_exact_pixel_length() {
        assert_eq!(
            checked_byte_len(usize::MAX, 2),
            Err(RgbaFrameError::ArithmeticOverflow)
        );
        assert_eq!(
            RgbaFrame::new(2, 2, 1.0, vec![0; 15]).unwrap_err(),
            RgbaFrameError::ByteCountMismatch
        );
        assert_eq!(
            RgbaFrame::new(2, 2, 1.0, vec![0; 17]).unwrap_err(),
            RgbaFrameError::ByteCountMismatch
        );
    }

    #[test]
    fn rejects_nonfinite_nonpositive_and_unbounded_scale() {
        for scale in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.0,
            -1.0,
            0.5,
            2.5,
            3.01,
        ] {
            assert_eq!(
                RgbaFrame::new(1, 1, scale, vec![0; 4]).unwrap_err(),
                RgbaFrameError::InvalidScale
            );
        }
    }

    #[test]
    fn accepts_axis_boundary_and_minimum_scale() {
        let frame =
            RgbaFrame::new(MAX_FRAME_AXIS, 1, 1.0, vec![0; MAX_FRAME_AXIS as usize * 4]).unwrap();
        assert_eq!(frame.width(), MAX_FRAME_AXIS);
        assert_eq!(frame.backing_scale(), 1.0);
        let tall =
            RgbaFrame::new(1, MAX_FRAME_AXIS, 3.0, vec![0; MAX_FRAME_AXIS as usize * 4]).unwrap();
        assert_eq!(tall.height(), MAX_FRAME_AXIS);
    }

    #[test]
    fn premultiplies_semitransparent_straight_rgba_deterministically() {
        let frame = RgbaFrame::new(1, 1, 1.0, vec![200, 100, 50, 128]).unwrap();
        assert_eq!(frame.pixels(), &[200, 100, 50, 128]);
        assert_eq!(frame.into_premultiplied_pixels(), [100, 50, 25, 128]);
    }

    #[test]
    fn fast_premultiply_matches_rounded_division_for_every_input() {
        for c in 0..=255u32 {
            for a in 0..=255u32 {
                assert_eq!(u32::from(mul_div255(c, a)), (c * a + 127) / 255, "{c} {a}");
            }
        }
    }

    #[test]
    fn rejects_vec_capacity_over_frame_bound() {
        let mut pixels = Vec::with_capacity(MAX_FRAME_BYTES + 1);
        pixels.extend_from_slice(&[0; 4]);
        assert_eq!(
            RgbaFrame::new(1, 1, 1.0, pixels).unwrap_err(),
            RgbaFrameError::CapacityTooLarge
        );
    }
}
