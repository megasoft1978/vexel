//! AC4: typed limits before allocation, determinism and a no-panic size sweep.
mod support;
use support::*;
use vexel_gif::{Frame, GifError, MAX_FRAMES, MAX_TOTAL_PIXELS, delay_cs_for_fps, encode};

fn f(rgba: &[u8], w: u32, h: u32) -> Frame<'_> {
    Frame {
        width: w,
        height: h,
        rgba,
        delay_cs: None,
    }
}

#[test]
fn empty_input_is_typed() {
    assert_eq!(encode(&[], 10), Err(GifError::Empty));
}

#[test]
fn frame_count_limit_is_120() {
    let px = [1u8, 2, 3, 255];
    let ok: Vec<Frame<'_>> = (0..MAX_FRAMES).map(|_| f(&px, 1, 1)).collect();
    assert!(encode(&ok, 10).is_ok());
    let over: Vec<Frame<'_>> = (0..MAX_FRAMES + 1).map(|_| f(&px, 1, 1)).collect();
    assert_eq!(encode(&over, 10), Err(GifError::TooManyFrames));
}

#[test]
fn fps_limit_is_60() {
    let px = [1u8, 2, 3, 255];
    assert!(encode(&[f(&px, 1, 1)], 60).is_ok());
    assert_eq!(encode(&[f(&px, 1, 1)], 61), Err(GifError::InvalidFps));
    assert_eq!(encode(&[f(&px, 1, 1)], 0), Err(GifError::InvalidFps));
    assert_eq!(delay_cs_for_fps(61), Err(GifError::InvalidFps));
}

#[test]
fn dimension_limit_is_4096_checked_before_data_is_read() {
    // The data slice is empty: a dimension error must win without touching pixels.
    assert_eq!(
        encode(&[f(&[], 4097, 1)], 10),
        Err(GifError::InvalidDimensions)
    );
    assert_eq!(
        encode(&[f(&[], 1, 4097)], 10),
        Err(GifError::InvalidDimensions)
    );
    assert_eq!(
        encode(&[f(&[], 0, 1)], 10),
        Err(GifError::InvalidDimensions)
    );
    assert_eq!(
        encode(&[f(&[], 1, 0)], 10),
        Err(GifError::InvalidDimensions)
    );
    assert_eq!(
        encode(&[f(&[], 4096, 4096)], 10),
        Err(GifError::InvalidPixelData)
    );
}

#[test]
fn aggregate_pixel_limit_is_32m_checked_before_data_is_read() {
    // Eight 4096x1000 frames are 32,768,000 pixels: rejected with no pixel data.
    let frames: Vec<Frame<'_>> = (0..8).map(|_| f(&[], 4096, 1000)).collect();
    assert!(8 * 4096 * 1000 > MAX_TOTAL_PIXELS);
    assert_eq!(encode(&frames, 10), Err(GifError::TooManyPixels));
}

#[test]
fn mismatched_sizes_and_data_are_typed() {
    let a = [0u8; 16];
    let b = [0u8; 4];
    assert_eq!(
        encode(&[f(&a, 2, 2), f(&b, 1, 1)], 10),
        Err(GifError::FrameSizeMismatch)
    );
    assert_eq!(encode(&[f(&a, 3, 2)], 10), Err(GifError::InvalidPixelData));
    assert_eq!(
        encode(&[f(&a[..15], 2, 2)], 10),
        Err(GifError::InvalidPixelData)
    );
}

#[test]
fn encoding_is_deterministic() {
    let mut rng = Rng(42);
    let mut px = Vec::new();
    for _ in 0..70 * 50 {
        let r = rng.next();
        px.extend_from_slice(&[
            r as u8,
            (r >> 8) as u8,
            (r >> 16) as u8,
            if r & 0x1000 != 0 { 255 } else { 0 },
        ]);
    }
    let frames = [f(&px, 70, 50), f(&px, 70, 50)];
    assert_eq!(encode(&frames, 15).unwrap(), encode(&frames, 15).unwrap());
}

#[test]
fn size_sweep_never_panics_and_round_trips() {
    let mut rng = Rng(0xDEAD_BEEF);
    for w in 0..=34u32 {
        for h in 0..=34u32 {
            let len = (w * h * 4) as usize;
            let mut px = Vec::with_capacity(len);
            let palette = 1 + (rng.next() % 300);
            for _ in 0..w * h {
                let c = rng.next() % palette;
                let a = if rng.next() % 7 == 0 { 0 } else { 255 };
                px.extend_from_slice(&[c as u8, (c >> 8) as u8, (c * 5) as u8, a]);
            }
            let r = encode(&[f(&px, w, h)], 1 + (w + h) % 60);
            if w == 0 || h == 0 {
                assert_eq!(r, Err(GifError::InvalidDimensions), "{w}x{h}");
                continue;
            }
            let d = decode(&r.unwrap()).unwrap_or_else(|e| panic!("{w}x{h}: {e}"));
            assert_eq!((d.width as u32, d.height as u32), (w, h));
            assert_eq!(d.frames[0].indices.len(), (w * h) as usize);
        }
    }
    // Large-ish and degenerate shapes.
    for (w, h) in [(1u32, 4096u32), (4096, 1), (255, 255), (256, 257), (513, 3)] {
        let px = n_colors(w, h, 200);
        let d = decode(&encode(&[f(&px, w, h)], 24).unwrap()).unwrap();
        assert_eq!(d.rgba(0), px, "{w}x{h}");
    }
}
