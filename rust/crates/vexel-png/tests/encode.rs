mod common;

use vexel_png::{Compression, MAX_DIMENSION, PngError, adler32, crc32, encode_rgba8};

const BOTH: [Compression; 2] = [Compression::Stored, Compression::Fixed];

fn gradient(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            v.extend_from_slice(&[
                (x * 255 / w.max(1)) as u8,
                (y * 255 / h.max(1)) as u8,
                ((x + y) & 0xff) as u8,
                255,
            ]);
        }
    }
    v
}

fn random(len: usize) -> Vec<u8> {
    let mut state = 0x1234_5678_9ABC_DEF0u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

#[test]
fn checksum_known_vectors() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    assert_eq!(crc32(b""), 0);
    assert_eq!(adler32(b""), 1);
}

#[test]
fn one_pixel_stored_is_byte_exact() {
    let expected: [u8; 73] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR length, type
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, //
        0x1F, 0x15, 0xC4, 0x89, // IHDR CRC
        0x00, 0x00, 0x00, 0x10, 0x49, 0x44, 0x41, 0x54, // IDAT length, type
        0x78, 0x01, 0x01, 0x05, 0x00, 0xFA, 0xFF, // zlib header, stored block
        0x00, 0x0A, 0x14, 0x1E, 0x28, // filter 0 + pixel
        0x00, 0xCD, 0x00, 0x65, // Adler-32
        0x7B, 0x27, 0x54, 0xBA, // IDAT CRC
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, // IEND
    ];
    let png = encode_rgba8(1, 1, &[10, 20, 30, 40], Compression::Stored).unwrap();
    assert_eq!(png, expected);
}

#[test]
fn round_trips_exactly() {
    let flat: Vec<u8> = [200u8, 100, 50, 255].repeat(64 * 64);
    let cases: Vec<(u32, u32, Vec<u8>)> = vec![
        (1, 1, vec![1, 2, 3, 4]),
        (3, 5, gradient(3, 5)),
        (64, 64, gradient(64, 64)),
        (257, 3, gradient(257, 3)),
        (300, 300, gradient(300, 300)),
        (64, 64, flat),
        (128, 128, random(128 * 128 * 4)),
        (300, 300, random(300 * 300 * 4)),
    ];
    for compression in BOTH {
        for (w, h, rgba) in &cases {
            let png = encode_rgba8(*w, *h, rgba, compression).unwrap();
            let decoded = common::decode(&png);
            assert_eq!((decoded.width, decoded.height), (*w, *h));
            assert_eq!(&decoded.rgba, rgba, "{compression:?} {w}x{h}");
        }
    }
}

#[test]
fn stored_crosses_the_block_boundary() {
    // 300 * 301 = 90_300 bytes > 65_535: two stored blocks.
    let png = encode_rgba8(300, 300, &gradient(300, 300), Compression::Stored).unwrap();
    let d = common::decode(&png);
    assert_eq!(d.rgba.len(), 300 * 300 * 4);
}

#[test]
fn fixed_shrinks_flat_and_never_exceeds_stored_on_random() {
    let flat = [30u8, 60, 90, 255].repeat(256 * 256);
    let stored = encode_rgba8(256, 256, &flat, Compression::Stored).unwrap();
    let fixed = encode_rgba8(256, 256, &flat, Compression::Fixed).unwrap();
    println!(
        "flat 256x256: stored={} fixed={}",
        stored.len(),
        fixed.len()
    );
    assert!(
        fixed.len() < stored.len(),
        "{} vs {}",
        fixed.len(),
        stored.len()
    );
    assert_eq!(common::decode(&fixed).rgba, flat);

    let noise = random(128 * 128 * 4);
    let stored = encode_rgba8(128, 128, &noise, Compression::Stored).unwrap();
    let fixed = encode_rgba8(128, 128, &noise, Compression::Fixed).unwrap();
    assert!(fixed.len() as f64 <= stored.len() as f64 * 1.1);
    println!(
        "noise 128x128: stored={} fixed={}",
        stored.len(),
        fixed.len()
    );
}

#[test]
fn fixed_uses_far_and_long_matches() {
    // A 3000-byte random block repeated: matches at distance 3000 with the
    // maximum length of 258, plus a gradient with short-distance matches.
    let block = random(3000);
    let mut rgba = Vec::new();
    while rgba.len() < 128 * 128 * 4 {
        rgba.extend_from_slice(&block);
    }
    rgba.truncate(128 * 128 * 4);
    let stored = encode_rgba8(128, 128, &rgba, Compression::Stored).unwrap();
    let fixed = encode_rgba8(128, 128, &rgba, Compression::Fixed).unwrap();
    println!(
        "128x128 repeated block: stored={} fixed={}",
        stored.len(),
        fixed.len()
    );
    assert!(fixed.len() < stored.len());
    assert_eq!(common::decode(&fixed).rgba, rgba);
}

#[test]
fn large_flat_image_splits_idat() {
    let rgba = random(512 * 512 * 4);
    let png = encode_rgba8(512, 512, &rgba, Compression::Stored).unwrap();
    let d = common::decode(&png);
    assert!(d.idat_chunks >= 2);
    assert_eq!(d.rgba, rgba);
}

#[test]
fn typed_errors_without_panics() {
    let px = [0u8; 4];
    for c in BOTH {
        assert_eq!(encode_rgba8(0, 1, &px, c), Err(PngError::ZeroDimension));
        assert_eq!(encode_rgba8(1, 0, &px, c), Err(PngError::ZeroDimension));
        assert_eq!(
            encode_rgba8(MAX_DIMENSION + 1, 1, &[], c),
            Err(PngError::DimensionTooLarge)
        );
        assert_eq!(
            encode_rgba8(1, 16385, &[], c),
            Err(PngError::DimensionTooLarge)
        );
        assert_eq!(
            encode_rgba8(1, 1, &px[..3], c),
            Err(PngError::InvalidLength)
        );
        assert_eq!(encode_rgba8(2, 2, &px, c), Err(PngError::InvalidLength));
        // 16384 * 16384 * 4 = 1 GiB: over the byte bound before any length
        // comparison or allocation.
        assert_eq!(
            encode_rgba8(16384, 16384, &[], c),
            Err(PngError::TooManyBytes)
        );
        assert_eq!(
            encode_rgba8(u32::MAX, u32::MAX, &[], c),
            Err(PngError::DimensionTooLarge)
        );
    }
}
