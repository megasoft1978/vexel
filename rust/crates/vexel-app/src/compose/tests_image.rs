//! WO-RUST-CORPUS-UNSUPPORTED-01: the PNG and zlib decoders behind data-URI images.
use super::inflate::inflate;
use super::png::decode;
use vexel_scene::MediaError;

fn base64(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes().filter(|c| *c != b'=') {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            _ => 63,
        };
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// 1x1 RGBA PNG used by `bench/corpus/fixtures/image-embedded-data.svg`.
const RGBA_1X1: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGNgAAIAAAUAAaX2RUAAAAAASUVORK5CYII=";
/// 1x1 gray+alpha PNG used by the external `embedded` corpus file.
const GRAY_ALPHA_1X1: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j5WQAAAAASUVORK5CYII=";

/// A dynamic-Huffman zlib stream made by zlib level 9 (text of eight words).
const DYNAMIC_RAW_LEN: usize = 227;
const DYNAMIC_RAW_ADLER: u32 = 0x0a56_5331;
const DYNAMIC_FIRST: &[u8] = b"png image in";
const DYNAMIC_ZLIB: &[u8] = &[
    0x78, 0xda, 0x65, 0x8e, 0xe1, 0x0a, 0x80, 0x20, 0x0c, 0x84, 0x5f, 0x65, 0xaf, 0x66, 0xb9, 0x64,
    0x34, 0x4d, 0x44, 0x22, 0x7a, 0xfa, 0x72, 0x5b, 0x62, 0xf5, 0x67, 0xdc, 0x7d, 0x1c, 0xb7, 0xcb,
    0x29, 0x00, 0x45, 0x17, 0x10, 0x28, 0x2d, 0xec, 0x2a, 0x42, 0x6e, 0xc4, 0xf4, 0xc9, 0x34, 0x09,
    0xf0, 0x38, 0x6f, 0xde, 0xbc, 0x1c, 0x03, 0x3b, 0x1e, 0xc8, 0x3d, 0xae, 0xae, 0xe5, 0x2b, 0xf1,
    0x63, 0x45, 0x7e, 0x98, 0x54, 0xa8, 0xb4, 0xa2, 0xe8, 0xca, 0x8a, 0xc5, 0xa6, 0x98, 0xe9, 0xdf,
    0x7f, 0x93, 0x34, 0x36, 0xe2, 0xe1, 0xd7, 0xab, 0xe3, 0xce, 0x5c, 0x0a, 0x56, 0x53, 0x31,
];

fn crc32(data: &[u8]) -> u32 {
    vexel_png::crc32(data)
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&out[4..]).to_be_bytes());
    out
}

/// zlib stream of stored blocks.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01, 1];
    out.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    out.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    out.extend_from_slice(raw);
    out.extend_from_slice(&vexel_png::adler32(raw).to_be_bytes());
    out
}

fn png_with(
    width: u32,
    height: u32,
    depth: u8,
    color: u8,
    interlace: u8,
    raw: &[u8],
    extra: &[(&[u8; 4], &[u8])],
) -> Vec<u8> {
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[depth, color, 0, 0, interlace]);
    out.extend(chunk(b"IHDR", &header));
    for (kind, data) in extra {
        out.extend(chunk(kind, data));
    }
    out.extend(chunk(b"IDAT", &zlib_stored(raw)));
    out.extend(chunk(b"IEND", &[]));
    out
}

fn png(
    width: u32,
    height: u32,
    depth: u8,
    color: u8,
    raw: &[u8],
    extra: &[(&[u8; 4], &[u8])],
) -> Vec<u8> {
    png_with(width, height, depth, color, 0, raw, extra)
}

#[test]
fn decodes_a_one_pixel_rgba_png() {
    let pixels = decode(&base64(RGBA_1X1)).unwrap();
    assert_eq!((pixels.width, pixels.height, pixels.row_stride), (1, 1, 4));
    assert_eq!(pixels.byte_count, 4);
    assert_eq!(pixels.rgba, [0, 0, 0, 0]);
}

#[test]
fn decodes_a_one_pixel_gray_alpha_png() {
    let pixels = decode(&base64(GRAY_ALPHA_1X1)).unwrap();
    assert_eq!((pixels.width, pixels.height), (1, 1));
    assert_eq!(pixels.rgba[0], pixels.rgba[1]);
    assert_eq!(pixels.rgba[1], pixels.rgba[2]);
}

#[test]
fn rejects_truncated_and_corrupt_png() {
    let png = base64(RGBA_1X1);
    for cut in [0, 7, 8, 20, png.len() - 1] {
        assert_eq!(decode(&png[..cut]).unwrap_err(), MediaError::Corrupt);
    }
    let mut flipped = png.clone();
    flipped[43] ^= 1; // zlib stream byte: the Adler-32 trailer no longer matches
    assert_eq!(decode(&flipped).unwrap_err(), MediaError::Corrupt);
}

#[test]
fn gray_depths_scale_to_eight_bits_and_trns_keys_one_value() {
    // 1-bit gray, 8 pixels: 10110000
    let one = png(8, 1, 1, 0, &[0, 0b1011_0000], &[]);
    let p = decode(&one).unwrap();
    let grays: Vec<u8> = p.rgba.chunks(4).map(|c| c[0]).collect();
    assert_eq!(grays, [255, 0, 255, 255, 0, 0, 0, 0]);
    // 2-bit gray with tRNS keying value 1: 00 01 10 11
    let two = png(4, 1, 2, 0, &[0, 0b0001_1011], &[(b"tRNS", &[0, 1])]);
    let p = decode(&two).unwrap();
    let px: Vec<(u8, u8)> = p.rgba.chunks(4).map(|c| (c[0], c[3])).collect();
    assert_eq!(px, [(0, 255), (85, 0), (170, 255), (255, 255)]);
    // 4-bit gray: 0 and 15
    let four = png(2, 1, 4, 0, &[0, 0x0f], &[]);
    let p = decode(&four).unwrap();
    assert_eq!([p.rgba[0], p.rgba[4]], [0, 255]);
}

#[test]
fn palette_png_applies_plte_and_trns() {
    let plte = [255, 0, 0, 0, 255, 0, 0, 0, 255];
    let p = decode(&png(
        3,
        1,
        8,
        3,
        &[0, 0, 1, 2],
        &[(b"PLTE", &plte), (b"tRNS", &[128])],
    ))
    .unwrap();
    assert_eq!(p.rgba, [255, 0, 0, 128, 0, 255, 0, 255, 0, 0, 255, 255]);
    // A palette image without PLTE is corrupt, and an index past it is black.
    assert_eq!(
        decode(&png(1, 1, 8, 3, &[0, 0], &[])).unwrap_err(),
        MediaError::Corrupt
    );
    let p = decode(&png(1, 1, 8, 3, &[0, 5], &[(b"PLTE", &plte)])).unwrap();
    assert_eq!(p.rgba, [0, 0, 0, 255]);
}

#[test]
fn sixteen_bit_rgb_and_rgba_keep_the_high_byte() {
    let rgb = png(1, 1, 16, 2, &[0, 0x80, 0x01, 0xff, 0xff, 0x00, 0x00], &[]);
    assert_eq!(decode(&rgb).unwrap().rgba, [0x80, 0xff, 0x00, 255]);
    let rgba = png(1, 1, 16, 6, &[0, 0x10, 0, 0x20, 0, 0x30, 0, 0x40, 0], &[]);
    assert_eq!(decode(&rgba).unwrap().rgba, [0x10, 0x20, 0x30, 0x40]);
}

#[test]
fn all_five_row_filters_are_reversed() {
    // 3 pixels x 2 rows of RGB; pixel values chosen so every predictor matters.
    let rows: [[u8; 9]; 5] = [
        [10, 20, 30, 40, 50, 60, 70, 80, 90],
        [12, 22, 33, 44, 55, 66, 77, 88, 99],
        [200, 100, 50, 25, 12, 6, 3, 1, 0],
        [9, 8, 7, 6, 5, 4, 3, 2, 1],
        [250, 240, 230, 220, 210, 200, 190, 180, 170],
    ];
    let paeth = |a: i32, b: i32, c: i32| {
        let p = a + b - c;
        let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
        if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        }
    };
    let mut raw = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let filter = y as u8; // 0 none, 1 sub, 2 up, 3 average, 4 paeth
        raw.push(filter);
        for i in 0..9 {
            let left = if i >= 3 { i32::from(row[i - 3]) } else { 0 };
            let above = if y > 0 { i32::from(rows[y - 1][i]) } else { 0 };
            let corner = if y > 0 && i >= 3 {
                i32::from(rows[y - 1][i - 3])
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => above,
                3 => (left + above) / 2,
                _ => paeth(left, above, corner),
            };
            raw.push((i32::from(row[i]) - predictor) as u8);
        }
    }
    let p = decode(&png(3, 5, 8, 2, &raw, &[])).unwrap();
    for (y, row) in rows.iter().enumerate() {
        for x in 0..3 {
            let at = (y * 3 + x) * 4;
            assert_eq!(
                &p.rgba[at..at + 3],
                &row[x * 3..x * 3 + 3],
                "row {y} pixel {x}"
            );
            assert_eq!(p.rgba[at + 3], 255);
        }
    }
    // An unknown filter type is corrupt.
    let mut bad = raw.clone();
    bad[0] = 9;
    assert_eq!(
        decode(&png(3, 5, 8, 2, &bad, &[])).unwrap_err(),
        MediaError::Corrupt
    );
}

#[test]
fn unsupported_and_oversized_payloads_are_typed() {
    let raw = [0u8, 1, 2, 3, 4];
    assert_eq!(
        decode(&png_with(1, 1, 8, 6, 1, &raw, &[])).unwrap_err(),
        MediaError::Unsupported
    );
    assert_eq!(
        decode(&png(1, 1, 8, 6, &raw, &[(b"acTL", &[0; 8])])).unwrap_err(),
        MediaError::Animated
    );
    assert_eq!(
        decode(&png(1, 1, 8, 6, &raw, &[(b"CgBI", &[0; 4])])).unwrap_err(),
        MediaError::Unsupported
    );
    assert_eq!(
        decode(&png(1, 1, 7, 6, &raw, &[])).unwrap_err(),
        MediaError::Corrupt
    );
    assert_eq!(
        decode(&png(0, 1, 8, 6, &raw, &[])).unwrap_err(),
        MediaError::Corrupt
    );
    // Dimensions far beyond the decoder bound never allocate.
    assert_eq!(
        decode(&png(60000, 60000, 8, 6, &raw, &[])).unwrap_err(),
        MediaError::Limit
    );
    // The stream must hold exactly the expected number of bytes.
    assert_eq!(
        decode(&png(1, 1, 8, 6, &[0, 1, 2, 3, 4, 5], &[])).unwrap_err(),
        MediaError::Corrupt
    );
}

#[test]
fn inflate_decodes_dynamic_huffman_blocks() {
    let out = inflate(DYNAMIC_ZLIB, DYNAMIC_RAW_LEN).unwrap();
    assert_eq!(out.len(), DYNAMIC_RAW_LEN);
    assert_eq!(&out[..DYNAMIC_FIRST.len()], DYNAMIC_FIRST);
    assert_eq!(vexel_png::adler32(&out), DYNAMIC_RAW_ADLER);
    // Asking for a different size, or flipping a data bit, is corrupt.
    assert_eq!(
        inflate(DYNAMIC_ZLIB, DYNAMIC_RAW_LEN - 1).unwrap_err(),
        MediaError::Corrupt
    );
    let mut flipped = DYNAMIC_ZLIB.to_vec();
    flipped[40] ^= 0x10;
    assert_eq!(
        inflate(&flipped, DYNAMIC_RAW_LEN).unwrap_err(),
        MediaError::Corrupt
    );
}

#[test]
fn inflate_decodes_stored_blocks_and_rejects_bad_headers() {
    let raw: Vec<u8> = (0..=255).collect();
    assert_eq!(inflate(&zlib_stored(&raw), 256).unwrap(), raw);
    let mut bad = zlib_stored(&raw);
    bad[1] ^= 1; // header check bits
    assert_eq!(inflate(&bad, 256).unwrap_err(), MediaError::Corrupt);
    let mut bad = zlib_stored(&raw);
    bad[4] ^= 0xff; // stored length no longer matches its complement
    assert_eq!(inflate(&bad, 256).unwrap_err(), MediaError::Corrupt);
    assert_eq!(inflate(&[0x78], 1).unwrap_err(), MediaError::Corrupt);
}
