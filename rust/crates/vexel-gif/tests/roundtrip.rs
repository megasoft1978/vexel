//! AC3: LZW round trip through the test-only decoder.
mod support;
use support::*;
use vexel_gif::{Frame, MAX_QUANT_ERROR, encode, palette_is_exact};

fn one(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let f = [Frame {
        width: w,
        height: h,
        rgba,
        delay_cs: None,
    }];
    encode(&f, 10).unwrap()
}

fn assert_exact(rgba: &[u8], w: u32, h: u32) {
    let d = decode(&one(rgba, w, h)).unwrap();
    assert_eq!(d.rgba(0), rgba);
}

#[test]
fn one_by_one_and_single_colour() {
    assert_exact(&[12, 34, 56, 255], 1, 1);
    assert_exact(&[0, 0, 0, 0], 1, 1);
    let flat: Vec<u8> = [200, 100, 50, 255].repeat(64 * 64);
    assert_exact(&flat, 64, 64);
    let tall: Vec<u8> = [9, 9, 9, 255].repeat(1 * 4096);
    assert_exact(&tall, 1, 4096);
}

#[test]
fn palette_boundaries_255_256_are_exact_and_257_is_quantized() {
    for colors in [1u32, 2, 3, 4, 5, 16, 17, 254, 255, 256] {
        let px = n_colors(40, 40, colors);
        let f = [Frame {
            width: 40,
            height: 40,
            rgba: &px,
            delay_cs: None,
        }];
        assert!(palette_is_exact(&f), "{colors} colours");
        assert_exact(&px, 40, 40);
    }
    let px = n_colors(40, 40, 257);
    let f = [Frame {
        width: 40,
        height: 40,
        rgba: &px,
        delay_cs: None,
    }];
    assert!(!palette_is_exact(&f), "257 colours must quantize");
    assert_within_tolerance(&px, 40, 40);
}

#[test]
fn transparent_slot_counts_against_capacity() {
    // 255 opaque colours + the transparent slot = 256 slots: exact.
    let mut px = n_colors(40, 40, 255);
    px[1599 * 4..1599 * 4 + 4].copy_from_slice(&[0, 0, 0, 0]);
    let f = [Frame {
        width: 40,
        height: 40,
        rgba: &px,
        delay_cs: None,
    }];
    assert!(palette_is_exact(&f));
    assert_exact(&px, 40, 40);
    // 256 opaque colours + transparent = 257 slots: quantized, transparency kept.
    let mut px = n_colors(40, 40, 256);
    px[1599 * 4..1599 * 4 + 4].copy_from_slice(&[0, 0, 0, 0]);
    let f = [Frame {
        width: 40,
        height: 40,
        rgba: &px,
        delay_cs: None,
    }];
    assert!(!palette_is_exact(&f));
    let d = decode(&one(&px, 40, 40)).unwrap();
    assert_eq!(&d.rgba(0)[1599 * 4..1599 * 4 + 4], &[0, 0, 0, 0]);
    assert!(d.rgba(0).chunks_exact(4).filter(|p| p[3] == 0).count() == 1);
}

#[test]
fn many_colour_frames_stay_within_stated_error() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (w, h) = (97u32, 61u32);
    let mut px = Vec::new();
    for _ in 0..w * h {
        let r = rng.next();
        px.extend_from_slice(&[r as u8, (r >> 8) as u8, (r >> 16) as u8, 255]);
    }
    assert_within_tolerance(&px, w, h);
    // Full channel sweeps hit every value of every channel.
    let mut ramp = Vec::new();
    for v in 0..=255u32 {
        ramp.extend_from_slice(&[v as u8, (255 - v) as u8, (v * 3) as u8, 255]);
    }
    assert_within_tolerance(&ramp, 256, 1);
}

fn assert_within_tolerance(px: &[u8], w: u32, h: u32) {
    let d = decode(&one(px, w, h)).unwrap();
    let out = d.rgba(0);
    assert_eq!(out.len(), px.len());
    let mut worst = 0u8;
    for (a, b) in px.chunks_exact(4).zip(out.chunks_exact(4)) {
        assert_eq!(b[3], 255);
        for c in 0..3 {
            worst = worst.max(a[c].abs_diff(b[c]));
        }
    }
    assert!(
        worst <= MAX_QUANT_ERROR,
        "max per-channel error {worst} > {MAX_QUANT_ERROR}"
    );
}

#[test]
fn error_bound_is_tight_for_every_channel_value() {
    // Each of the 256 values of each channel, alone, stays within the constant.
    let mut px = Vec::new();
    for v in 0..=255u8 {
        px.extend_from_slice(&[v, 0, 0, 255]);
        px.extend_from_slice(&[0, v, 0, 255]);
        px.extend_from_slice(&[0, 0, v, 255]);
    }
    px.extend_from_slice(&[1, 2, 3, 255]); // keep >256 colours: add more distinct
    for v in 0..=255u8 {
        px.extend_from_slice(&[v, v, v, 255]);
    }
    let w = (px.len() / 4) as u32;
    let f = [Frame {
        width: w,
        height: 1,
        rgba: &px,
        delay_cs: None,
    }];
    assert!(!palette_is_exact(&f));
    let d = decode(&encode(&f, 10).unwrap()).unwrap();
    let mut worst = 0;
    for (a, b) in px.chunks_exact(4).zip(d.rgba(0).chunks_exact(4)) {
        for c in 0..3 {
            worst = worst.max(a[c].abs_diff(b[c]));
        }
    }
    assert!(
        worst <= MAX_QUANT_ERROR && worst >= MAX_QUANT_ERROR - 2,
        "worst {worst}"
    );
}

#[test]
fn multi_frame_round_trip_shares_one_palette() {
    let a = n_colors(30, 20, 100);
    let b: Vec<u8> = a.iter().rev().copied().collect::<Vec<_>>();
    // `rev` swaps channels; rebuild as a valid RGBA frame from the same colour set.
    let b: Vec<u8> = b
        .chunks_exact(4)
        .flat_map(|p| [p[3], p[2], p[1], p[0]])
        .collect();
    let frames = [
        Frame {
            width: 30,
            height: 20,
            rgba: &a,
            delay_cs: None,
        },
        Frame {
            width: 30,
            height: 20,
            rgba: &b,
            delay_cs: None,
        },
    ];
    let d = decode(&encode(&frames, 12).unwrap()).unwrap();
    assert_eq!(d.rgba(0), a);
    assert_eq!(d.rgba(1), b);
}

#[test]
fn long_streams_cross_lzw_table_resets() {
    // Pseudo-random 4-colour data exercises code-width growth and clear codes.
    let mut rng = Rng(7);
    let (w, h) = (300u32, 300u32);
    let mut px = Vec::new();
    for _ in 0..w * h {
        let c = rng.next() % 5;
        px.extend_from_slice(&[(c * 50) as u8, (c * 7) as u8, (255 - c * 9) as u8, 255]);
    }
    assert_exact(&px, w, h);
    // 256-colour noise: maximal code diversity, several resets.
    let mut px = Vec::new();
    for _ in 0..w * h {
        let c = rng.next() % 256;
        px.extend_from_slice(&[c as u8, (c * 3) as u8, (c ^ 0x55) as u8, 255]);
    }
    assert_exact(&px, w, h);
}
