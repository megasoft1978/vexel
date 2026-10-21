mod common;

use vexel_png::{ApngFrame, MAX_FRAMES, PngError, crc32, encode_apng};

const F0: [u8; 16] = [
    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 128,
];
const F1: [u8; 16] = [
    0, 0, 0, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120,
];

fn two_frames() -> Vec<u8> {
    let frames = [
        ApngFrame {
            rgba: &F0,
            delay_num: 1,
            delay_den: 10,
        },
        ApngFrame {
            rgba: &F1,
            delay_num: 2,
            delay_den: 25,
        },
    ];
    encode_apng(&frames, 2, 2, 3).unwrap()
}

/// (kind, body) per chunk, with CRC and length checked.
fn chunks(png: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut pos = 8;
    let mut out = Vec::new();
    while pos < png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let body = &png[pos + 8..pos + 8 + len];
        let crc = u32::from_be_bytes(png[pos + 8 + len..pos + 12 + len].try_into().unwrap());
        assert_eq!(crc, crc32(&png[pos + 4..pos + 8 + len]));
        out.push((png[pos + 4..pos + 8].to_vec(), body.to_vec()));
        pos += 12 + len;
    }
    out
}

fn be(b: &[u8]) -> u32 {
    u32::from_be_bytes(b[..4].try_into().unwrap())
}

#[test]
fn golden_two_frames() {
    let hex: String = two_frames().iter().map(|b| format!("{b:02x}")).collect();
    // Generated once from this encoder (2x2, 2 frames, 3 plays); any change
    // to the byte stream must be deliberate.
    assert_eq!(hex, GOLDEN);
}

#[test]
fn deterministic() {
    assert_eq!(two_frames(), two_frames());
}

#[test]
fn chunk_order_and_sequences() {
    let cs = chunks(&two_frames());
    let kinds: Vec<&[u8]> = cs.iter().map(|c| c.0.as_slice()).collect();
    assert_eq!(
        kinds,
        [
            &b"IHDR"[..],
            b"acTL",
            b"fcTL",
            b"IDAT",
            b"fcTL",
            b"fdAT",
            b"IEND"
        ]
    );
    assert_eq!(be(&cs[1].1), 2, "num_frames");
    assert_eq!(be(&cs[1].1[4..]), 3, "num_plays");
    let mut expected = 0;
    for (kind, body) in &cs {
        if kind == b"fcTL" {
            assert_eq!(body.len(), 26);
            assert_eq!(be(body), expected);
            assert_eq!((be(&body[4..]), be(&body[8..])), (2, 2));
            assert_eq!((be(&body[12..]), be(&body[16..])), (0, 0));
            assert_eq!(&body[24..], &[0, 0], "dispose NONE, blend SOURCE");
            expected += 1;
        } else if kind == b"fdAT" {
            assert_eq!(be(body), expected);
            expected += 1;
        }
    }
    assert_eq!(expected, 3);
    assert_eq!(&cs[2].1[20..24], &[0, 1, 0, 10]);
    assert_eq!(&cs[4].1[20..24], &[0, 2, 0, 25]);
}

#[test]
fn frames_decode_back() {
    let cs = chunks(&two_frames());
    for (data, want) in [(&cs[3].1[..], &F0), (&cs[5].1[4..], &F1)] {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        for (kind, body) in [
            (&b"IHDR"[..], &cs[0].1[..]),
            (b"IDAT", data),
            (b"IEND", &[]),
        ] {
            png.extend_from_slice(&(body.len() as u32).to_be_bytes());
            let start = png.len();
            png.extend_from_slice(kind);
            png.extend_from_slice(body);
            let crc = crc32(&png[start..]);
            png.extend_from_slice(&crc.to_be_bytes());
        }
        let d = common::decode(&png);
        assert_eq!((d.width, d.height), (2, 2));
        assert_eq!(&d.rgba[..], &want[..]);
    }
}

#[test]
fn errors() {
    let f = |rgba| ApngFrame {
        rgba,
        delay_num: 1,
        delay_den: 10,
    };
    assert_eq!(encode_apng(&[], 2, 2, 0), Err(PngError::ZeroFrames));
    assert_eq!(
        encode_apng(&[f(&F0[..15])], 2, 2, 0),
        Err(PngError::InvalidLength)
    );
    assert_eq!(
        encode_apng(&[f(&F0), f(&F1[..8])], 2, 2, 0),
        Err(PngError::InvalidLength)
    );
    assert_eq!(
        encode_apng(&[f(&F0)], 0, 2, 0),
        Err(PngError::ZeroDimension)
    );
    assert_eq!(
        encode_apng(&[f(&F0)], 20000, 1, 0),
        Err(PngError::DimensionTooLarge)
    );
    let many = vec![f(&F0); MAX_FRAMES + 1];
    assert_eq!(encode_apng(&many, 2, 2, 0), Err(PngError::TooManyFrames));
    assert!(encode_apng(&many[..MAX_FRAMES], 2, 2, 0).is_ok());
}

const GOLDEN: &str = "89504e470d0a1a0a0000000d494844520000000200000002080600000072b60d24000000086163544c00000002000000036a84c2ca0000001a6663544c00000000000000020000000200000000000000000001000a0000e854dc000000001249444154780163f8cf00442008a441a0010049490978f75eeb710000001a6663544c00000001000000020000000200000000000000000002001900002ae7512d0000001b666441540000000278016300012e11390d06231bb780a894bc0a000eae030de6d15db20000000049454e44ae426082";
