//! Test-only `AVAssetReader` decode used to verify `VideoWriter` output.
//! Built for this crate's tests and for other crates' tests via `test-support`.

use std::path::Path;

use objc2_av_foundation::{AVAssetReader, AVAssetReaderTrackOutput, AVMediaTypeVideo, AVURLAsset};
use objc2_core_foundation::CGSize;
use objc2_core_video::{
    CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferGetHeight,
    CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags,
    CVPixelBufferUnlockBaseAddress, kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_32BGRA,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

pub struct DecodedFrame {
    pub pts_seconds: f64,
    pub width: usize,
    pub height: usize,
    /// RGBA, tightly packed.
    pub rgba: Vec<u8>,
}

pub struct Decoded {
    pub duration_seconds: f64,
    pub natural_size: (f64, f64),
    pub frames: Vec<DecodedFrame>,
}

#[allow(deprecated)] // sync track load is fine for a test-only reader
pub fn decode(path: &Path) -> Result<Decoded, String> {
    // SAFETY: all objects are retained locals used on this thread; pixel
    // buffers are locked while read.
    unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str().ok_or("path")?));
        let asset = AVURLAsset::URLAssetWithURL_options(&url, None);
        let duration = asset.duration().seconds();
        let media = AVMediaTypeVideo.ok_or("media type")?;
        let tracks = asset.tracksWithMediaType(media);
        let track = tracks.firstObject().ok_or("no video track")?;
        let CGSize { width, height } = track.naturalSize();
        let reader =
            AVAssetReader::assetReaderWithAsset_error(&asset).map_err(|e| e.to_string())?;
        let key = &*(kCVPixelBufferPixelFormatTypeKey as *const _ as *const NSString);
        let n = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
        let settings: objc2::rc::Retained<NSDictionary<NSString, objc2::runtime::AnyObject>> =
            NSDictionary::from_slices(&[key], &[&*n]);
        let output = AVAssetReaderTrackOutput::assetReaderTrackOutputWithTrack_outputSettings(
            &track,
            Some(&settings),
        );
        reader.addOutput(&output);
        if !reader.startReading() {
            return Err(format!("startReading: {:?}", reader.error()));
        }
        let mut frames = Vec::new();
        while let Some(sample) = output.copyNextSampleBuffer() {
            let pts = sample.presentation_time_stamp().seconds();
            let Some(buf) = sample.image_buffer() else {
                continue;
            };
            let flags = CVPixelBufferLockFlags::ReadOnly;
            CVPixelBufferLockBaseAddress(&buf, flags);
            let (w, h) = (CVPixelBufferGetWidth(&buf), CVPixelBufferGetHeight(&buf));
            let stride = CVPixelBufferGetBytesPerRow(&buf);
            let base = CVPixelBufferGetBaseAddress(&buf).cast::<u8>();
            let src = std::slice::from_raw_parts(base, stride * h);
            let mut rgba = Vec::with_capacity(w * h * 4);
            for y in 0..h {
                for px in src[y * stride..y * stride + w * 4].chunks_exact(4) {
                    rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                }
            }
            CVPixelBufferUnlockBaseAddress(&buf, flags);
            frames.push(DecodedFrame {
                pts_seconds: pts,
                width: w,
                height: h,
                rgba,
            });
        }
        Ok(Decoded {
            duration_seconds: duration,
            natural_size: (width, height),
            frames,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VideoCodec, VideoSpec, VideoWriter};

    const N: usize = 24;
    const SIZE: usize = 64;
    const TS: i32 = 600;
    const STEP: i64 = 25; // 24 fps at timescale 600

    fn gradient(i: usize) -> Vec<u8> {
        let mut px = Vec::with_capacity(SIZE * SIZE * 4);
        for y in 0..SIZE {
            for x in 0..SIZE {
                px.extend_from_slice(&[
                    (x * 255 / (SIZE - 1)) as u8,
                    (y * 255 / (SIZE - 1)) as u8,
                    (i * 255 / (N - 1)) as u8,
                    255,
                ]);
            }
        }
        px
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vexel-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn round_trip(codec: VideoCodec, name: &str) {
        let path = scratch(name);
        let mut w = VideoWriter::create(VideoSpec {
            path: path.clone(),
            width: SIZE as u32,
            height: SIZE as u32,
            codec,
            timescale: TS,
            background: [255, 255, 255],
            alpha: false,
            bit_rate: Some(20_000_000),
        })
        .expect("create");
        for i in 0..N {
            w.append(&gradient(i), i as i64 * STEP).expect("append");
        }
        w.finish_at(N as i64 * STEP).expect("finish");

        let d = decode(&path).expect("decode");
        assert_eq!(d.frames.len(), N, "frame count");
        assert!(
            (d.duration_seconds - 1.0).abs() < 0.01,
            "duration {}",
            d.duration_seconds
        );
        assert_eq!(d.natural_size, (SIZE as f64, SIZE as f64));
        for (i, f) in d.frames.iter().enumerate() {
            let want_t = i as f64 * STEP as f64 / f64::from(TS);
            assert!(
                (f.pts_seconds - want_t).abs() < 1e-6,
                "pts {i}: {}",
                f.pts_seconds
            );
            assert_eq!((f.width, f.height), (SIZE, SIZE));
            let want = gradient(i);
            for (j, (a, b)) in f.rgba.chunks_exact(4).zip(want.chunks_exact(4)).enumerate() {
                for c in 0..3 {
                    let diff = (i32::from(a[c]) - i32::from(b[c])).abs();
                    assert!(diff <= 6, "frame {i} px {j} ch {c}: {} vs {}", a[c], b[c]);
                }
            }
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn h264_mp4_round_trips_gradient() {
        round_trip(VideoCodec::H264, "h264.mp4");
    }

    #[test]
    fn hevc_mp4_round_trips_gradient() {
        round_trip(VideoCodec::Hevc, "hevc.mp4");
    }

    #[test]
    fn transparent_pixels_composite_onto_background() {
        let path = scratch("bg.mp4");
        let mut w = VideoWriter::create(VideoSpec {
            path: path.clone(),
            width: 16,
            height: 16,
            codec: VideoCodec::H264,
            timescale: TS,
            background: [200, 40, 40],
            alpha: false,
            bit_rate: Some(2_000_000),
        })
        .unwrap();
        for i in 0..4 {
            w.append(&vec![0u8; 16 * 16 * 4], i * STEP).unwrap();
        }
        w.finish().unwrap();
        let d = decode(&path).unwrap();
        let px = &d.frames[0].rgba[..3];
        for (a, b) in px.iter().zip([200u8, 40, 40]) {
            assert!((i32::from(*a) - i32::from(b)).abs() <= 6, "{px:?}");
        }
        let _ = std::fs::remove_file(path);
    }

    fn alpha_spec(name: &str, codec: VideoCodec) -> VideoSpec {
        VideoSpec {
            path: scratch(name),
            width: 32,
            height: 32,
            codec,
            timescale: TS,
            background: [255, 255, 255],
            alpha: true,
            bit_rate: Some(4_000_000),
        }
    }

    #[test]
    fn alpha_requires_hevc_in_mov() {
        for (name, codec) in [("a.mov", VideoCodec::H264), ("a.mp4", VideoCodec::Hevc)] {
            let e = VideoWriter::create(alpha_spec(name, codec))
                .err()
                .expect("rejected");
            assert!(e.to_string().contains("alpha needs HEVC"), "{e}");
        }
    }

    #[test]
    fn hevc_alpha_mov_keeps_transparency() {
        let mut w = match VideoWriter::create(alpha_spec("alpha.mov", VideoCodec::Hevc)) {
            Ok(w) => w,
            Err(e) => panic!("HEVC-with-alpha encoder unavailable: {e}"),
        };
        let path = w.path().to_owned();
        // Left half opaque red, right half 50% blue, bottom rows fully clear.
        let mut frame = Vec::new();
        for y in 0..32 {
            for x in 0..32 {
                frame.extend_from_slice(match (y >= 24, x < 16) {
                    (true, _) => &[0, 0, 0, 0],
                    (false, true) => &[255, 0, 0, 255],
                    (false, false) => &[0, 0, 255, 128],
                });
            }
        }
        for i in 0..4 {
            w.append(&frame, i * STEP).expect("append");
        }
        w.finish().expect("finish");
        let d = decode(&path).expect("decode");
        assert_eq!(d.frames.len(), 4);
        let rgba = &d.frames[0].rgba;
        let alpha = |x: usize, y: usize| rgba[(y * 32 + x) * 4 + 3];
        assert!(alpha(4, 4) > 200, "opaque side {}", alpha(4, 4));
        assert!(alpha(28, 4) < 255, "half-transparent side {}", alpha(28, 4));
        assert!(alpha(16, 30) < 40, "clear rows {}", alpha(16, 30));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_bad_requests() {
        let spec = |w, h| VideoSpec {
            path: scratch("bad.mp4"),
            width: w,
            height: h,
            codec: VideoCodec::H264,
            timescale: TS,
            background: [0; 3],
            alpha: false,
            bit_rate: None,
        };
        assert!(VideoWriter::create(spec(63, 64)).is_err());
        assert!(VideoWriter::create(spec(8192, 64)).is_err());
    }
}
