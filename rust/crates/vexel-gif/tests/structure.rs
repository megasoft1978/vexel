//! AC2: GIF89a structure.
mod support;
use support::*;
use vexel_gif::{Frame, delay_cs_for_fps, encode};

fn frame(rgba: &[u8], w: u32, h: u32) -> Frame<'_> {
    Frame {
        width: w,
        height: h,
        rgba,
        delay_cs: None,
    }
}

#[test]
fn header_screen_loop_gce_and_trailer() {
    let a = n_colors(5, 3, 4);
    let b = n_colors(5, 3, 2);
    let c = n_colors(5, 3, 3);
    let frames = [frame(&a, 5, 3), frame(&b, 5, 3), frame(&c, 5, 3)];
    let bytes = encode(&frames, 10).unwrap();
    assert_eq!(&bytes[..6], b"GIF89a");
    assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 5);
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 3);
    assert_eq!(*bytes.last().unwrap(), 0x3B);
    let needle = [&[0x21u8, 0xFF, 0x0B][..], b"NETSCAPE2.0", &[3, 1, 0, 0, 0]].concat();
    assert_eq!(
        bytes
            .windows(needle.len())
            .filter(|w| *w == needle.as_slice())
            .count(),
        1
    );
    let d = decode(&bytes).unwrap();
    assert_eq!((d.width, d.height), (5, 3));
    assert_eq!(d.loop_count, Some(0), "infinite loop");
    assert_eq!(d.frames.len(), 3);
    for f in &d.frames {
        assert_eq!(
            f.gce_count, 1,
            "exactly one graphic control block per frame"
        );
        assert_eq!(f.delay_cs, 10);
    }
}

#[test]
fn delay_rounds_to_nearest_centisecond_with_minimum_one() {
    let table = [
        (1, 100),
        (2, 50),
        (3, 33),
        (7, 14),
        (10, 10),
        (24, 4),
        (30, 3),
        (59, 2),
        (60, 2),
    ];
    for (fps, cs) in table {
        assert_eq!(delay_cs_for_fps(fps).unwrap(), cs, "fps {fps}");
    }
    let px = n_colors(2, 2, 2);
    for (fps, cs) in table {
        let bytes = encode(&[frame(&px, 2, 2), frame(&px, 2, 2)], fps).unwrap();
        let d = decode(&bytes).unwrap();
        assert!(d.frames.iter().all(|f| f.delay_cs == cs), "fps {fps}");
    }
}

#[test]
fn per_frame_delays_override_fps() {
    let px = n_colors(2, 2, 2);
    let frames = [
        Frame {
            delay_cs: Some(7),
            ..frame(&px, 2, 2)
        },
        frame(&px, 2, 2),
        Frame {
            delay_cs: Some(65535),
            ..frame(&px, 2, 2)
        },
    ];
    let d = decode(&encode(&frames, 30).unwrap()).unwrap();
    let delays: Vec<u16> = d.frames.iter().map(|f| f.delay_cs).collect();
    assert_eq!(delays, [7, 3, 65535]);
}

#[test]
fn transparent_pixels_use_gce_transparency_and_restore_disposal() {
    let mut px = n_colors(4, 1, 2);
    px[7] = 0; // pixel 1 alpha -> transparent
    let d = decode(&encode(&[frame(&px, 4, 1)], 10).unwrap()).unwrap();
    let f = &d.frames[0];
    assert!(f.transparent.is_some());
    assert_eq!(f.disposal, 2);
}
