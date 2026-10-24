//! The benchmarked operations. Each calls the code path the app uses, headless.

use crate::anim;
use crate::sha256;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Instant;
use vexel_app::animation::native_export::{NativeExportSource, NativeGif};
use vexel_app::animation::playback::ManualClock;
use vexel_app::canvas::{artboard_pixels, target_compose_size};
use vexel_app::compose::{ComposeError, Frame, compose_document};
use vexel_app::native_animation::NativeAnimationHost;
use vexel_app::preview::Control;
use vexel_app::session::AppDocument;
use vexel_v1_document::TransformSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    OpenDecode,
    Render(u32),
    ZoomRecompose,
    SelMove,
    SelRotate,
    SaveLoad,
    Tick(u32),
    Scrub,
    ExportGif,
    ExportApng,
    ExportMp4,
}

pub const ALL_OPS: [Op; 14] = [
    Op::OpenDecode,
    Op::Render(512),
    Op::Render(900),
    Op::Render(2048),
    Op::ZoomRecompose,
    Op::SelMove,
    Op::SelRotate,
    Op::SaveLoad,
    Op::Tick(900),
    Op::Tick(512),
    Op::Scrub,
    Op::ExportGif,
    Op::ExportApng,
    Op::ExportMp4,
];

impl Op {
    pub fn name(self) -> String {
        match self {
            Op::OpenDecode => "open_decode".into(),
            Op::Render(s) => format!("render_{s}"),
            Op::ZoomRecompose => "zoom_recompose".into(),
            Op::SelMove => "sel_move".into(),
            Op::SelRotate => "sel_rotate".into(),
            Op::SaveLoad => "save_load".into(),
            Op::Tick(s) => format!("tick_{s}"),
            Op::Scrub => "scrub_900".into(),
            Op::ExportGif => "export_gif".into(),
            Op::ExportApng => "export_apng".into(),
            Op::ExportMp4 => "export_mp4".into(),
        }
    }
    pub fn parse(name: &str) -> Option<Op> {
        ALL_OPS.into_iter().find(|o| o.name() == name)
    }
    /// True when the op needs the animated `.vexel` variant.
    pub fn animated(self) -> bool {
        matches!(
            self,
            Op::Tick(_) | Op::Scrub | Op::ExportGif | Op::ExportApng | Op::ExportMp4
        )
    }
    /// Encoder output that is not byte-stable across processes is not hash-gated.
    pub fn hash_stable(self) -> bool {
        self != Op::ExportMp4
    }
}

/// Deliberately not `Debug`, so any `Debug` error converts with `?`.
pub enum Failure {
    /// The case cannot run here (unsupported feature); reported, not an error.
    Skip(String),
    Fail(String),
}

impl<E: std::fmt::Debug> From<E> for Failure {
    fn from(e: E) -> Self {
        Failure::Fail(format!("{e:?}"))
    }
}

/// One trial: per-sample milliseconds and the SHA-256 of what it produced.
pub struct Trial {
    pub samples: Vec<f64>,
    pub hash: Option<String>,
}

pub struct Runner {
    op: Op,
    n: usize,
    input: PathBuf,
    tmp: PathBuf,
    doc: AppDocument,
    host: Option<NativeAnimationHost<ManualClock>>,
    gif_source: Option<NativeExportSource>,
    pub notes: Vec<String>,
}

/// Zoom steps after an initial 0.5x compose (backing scale 2): four leave the
/// cached band and recompose at about 1126, 205, 614 and 1536 px.
const ZOOMS: [f64; 9] = [0.8, 1.1, 1.4, 0.3, 0.5, 0.2, 0.3, 0.6, 1.5];
/// Ticks sampled per Animate trial (every 16th of 96).
const TICK_STEP: u32 = 16;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

pub fn frame_hash(frame: &Frame) -> String {
    let mut h = sha256::Sha256::default();
    h.update(&frame.width.to_le_bytes());
    h.update(&frame.height.to_le_bytes());
    h.update(&frame.rgba);
    h.finish_hex()
}

fn compose_bytes(bytes: &[u8], w: u32, h: u32) -> Result<Frame, Failure> {
    match compose_document(bytes, w, h, &|| false) {
        Ok(composed) => Ok(composed.frame),
        Err(ComposeError::LimitRender) => Err(Failure::Skip(format!(
            "app render limit reached at {w}x{h} (LimitRender)"
        ))),
        Err(e) => Err(Failure::Fail(format!("{e:?}"))),
    }
}

fn compose(doc: &AppDocument, size: u32) -> Result<Frame, Failure> {
    compose_bytes(&doc.source_bytes()?, size, size)
}

fn open(path: &Path) -> Result<AppDocument, Failure> {
    let mut doc = AppDocument::new(512, 512)?;
    doc.open_path(path)?;
    Ok(doc)
}

impl Runner {
    pub fn new(op: Op, profile: &str, n: usize, input: &Path, tmp: &Path) -> Result<Self, Failure> {
        let doc = open(input)?;
        let mut notes = Vec::new();
        // Probe: a scene the decoder rejects or defers is not comparable.
        match compose(&doc, 64) {
            Ok(probe) if !probe.disclosures.is_empty() => {
                let list = format!("{:?}", probe.disclosures);
                if profile == "blur" {
                    return Err(Failure::Skip(format!("decoder defers features: {list}")));
                }
                notes.push(format!("disclosures: {list}"));
            }
            Ok(_) => {}
            Err(Failure::Fail(why)) if profile == "blur" => {
                return Err(Failure::Skip(format!(
                    "decoder rejects feGaussianBlur filters: {why}"
                )));
            }
            Err(other) => return Err(other),
        }
        let mut host = None;
        let mut gif_source = None;
        if op.animated() {
            if matches!(op, Op::Tick(_) | Op::Scrub) {
                let mut h = NativeAnimationHost::new(ManualClock::default());
                h.snapshot(&doc, None)
                    .map_err(|e| Failure::Fail(format!("native snapshot: {e:?}")))?;
                host = Some(h);
            }
            if op == Op::ExportGif {
                gif_source = Some(
                    NativeExportSource::from_document(&doc)
                        .ok_or_else(|| Failure::Fail("scene has no native tracks".into()))?,
                );
            }
        }
        Ok(Self {
            op,
            n,
            input: input.to_path_buf(),
            tmp: tmp.to_path_buf(),
            doc,
            host,
            gif_source,
            notes,
        })
    }

    pub fn trial(&mut self) -> Result<Trial, Failure> {
        match self.op {
            Op::OpenDecode => self.open_decode(),
            Op::Render(size) => {
                let t = Instant::now();
                let frame = compose(&self.doc, size)?;
                Ok(Trial {
                    samples: vec![ms(t)],
                    hash: Some(frame_hash(&frame)),
                })
            }
            Op::ZoomRecompose => self.zoom(),
            Op::SelMove => self.select_edit(TransformSpec {
                translate: (24.0, -17.0),
                ..TransformSpec::default()
            }),
            Op::SelRotate => self.select_edit(TransformSpec {
                rotate_deg: 37.0,
                pivot: (256.0, 256.0),
                ..TransformSpec::default()
            }),
            Op::SaveLoad => self.save_load(),
            Op::Tick(size) => self.ticks(size, (0..6).map(|k| k * TICK_STEP).collect()),
            Op::Scrub => {
                let mut rng = crate::scene::Lcg::new(0x5C0B);
                let seeks = (0..6).map(|_| rng.below(anim::TICKS)).collect();
                self.ticks(900, seeks)
            }
            Op::ExportGif => self.export_gif(),
            Op::ExportApng | Op::ExportMp4 => self.export_file(),
        }
    }

    fn open_decode(&mut self) -> Result<Trial, Failure> {
        let mut doc = AppDocument::new(512, 512)?;
        let t = Instant::now();
        doc.open_path(&self.input)?;
        let sample = ms(t);
        Ok(Trial {
            samples: vec![sample],
            hash: Some(sha256::hex(&doc.source_bytes()?)),
        })
    }

    /// Zoom path of the canvas: recompose only when the wanted resolution
    /// leaves the cached band (`canvas::target_compose_size`).
    fn zoom(&mut self) -> Result<Trial, Failure> {
        let (w, h) = self.doc.size();
        let natural = (f64::from(w), f64::from(h));
        let bytes = self.doc.source_bytes()?;
        let mut current = Some(artboard_pixels(natural, 0.5, 2));
        let (mut samples, mut log) = (Vec::new(), sha256::Sha256::default());
        for zoom in ZOOMS {
            let target = target_compose_size(natural, zoom, 2, current);
            log.update(format!("z{zoom}:{target:?};").as_bytes());
            if Some(target) == current {
                continue;
            }
            let t = Instant::now();
            let frame = compose_bytes(&bytes, target.0, target.1)?;
            samples.push(ms(t));
            log.update(frame_hash(&frame).as_bytes());
            current = Some(target);
        }
        Ok(Trial {
            samples,
            hash: Some(log.finish_hex()),
        })
    }

    /// Selection move/rotate: edit, re-materialise the source, recompose at
    /// display resolution. The edit is undone afterwards (untimed).
    fn select_edit(&mut self, spec: TransformSpec) -> Result<Trial, Failure> {
        let id = format!("s{}", self.n / 2);
        let t = Instant::now();
        self.doc.edit(|d| d.set_transform(&id, &spec))?;
        let frame = compose(&self.doc, 900)?;
        let sample = ms(t);
        self.doc.undo()?;
        Ok(Trial {
            samples: vec![sample],
            hash: Some(frame_hash(&frame)),
        })
    }

    fn save_load(&mut self) -> Result<Trial, Failure> {
        let path = self.tmp.join("saveload.vexel");
        let t = Instant::now();
        self.doc.save_as(path.as_os_str().as_bytes())?;
        let reopened = open(&path)?;
        let sample = ms(t);
        Ok(Trial {
            samples: vec![sample],
            hash: Some(sha256::hex(&reopened.source_bytes()?)),
        })
    }

    /// Native Animate: seek then sample one frame at `size`; one sample per tick.
    fn ticks(&mut self, size: u32, seeks: Vec<u32>) -> Result<Trial, Failure> {
        let host = self.host.as_mut().ok_or("no animation host")?;
        let (mut samples, mut log) = (Vec::new(), sha256::Sha256::default());
        for tick in seeks {
            let seconds = f64::from(tick) / f64::from(anim::TPS);
            host.mcp_control(&self.doc, Control::Seek(seconds), true)
                .map_err(|e| Failure::Fail(format!("seek: {e}")))?;
            let t = Instant::now();
            let frame = host.tick(&self.doc, (size, size));
            samples.push(ms(t));
            let frame = frame.ok_or_else(|| Failure::Fail(format!("no frame at tick {tick}")))?;
            log.update(frame_hash(&frame).as_bytes());
        }
        Ok(Trial {
            samples,
            hash: Some(log.finish_hex()),
        })
    }

    fn export_gif(&mut self) -> Result<Trial, Failure> {
        let source = self.gif_source.as_ref().ok_or("no export source")?;
        let t = Instant::now();
        let NativeGif { bytes, frames, .. } = source.export_gif()?;
        let sample = ms(t);
        if frames != anim::TICKS as usize {
            return Err(Failure::Fail(format!("expected 96 frames, got {frames}")));
        }
        Ok(Trial {
            samples: vec![sample],
            hash: Some(sha256::hex(&bytes)),
        })
    }

    fn export_file(&mut self) -> Result<Trial, Failure> {
        let mp4 = self.op == Op::ExportMp4;
        let path = self.tmp.join(if mp4 { "out.mp4" } else { "out.apng" });
        let _ = std::fs::remove_file(&path);
        let t = Instant::now();
        let status = if mp4 {
            vexel_app::video_export::write_video(&self.doc, path.clone())
        } else {
            vexel_app::frames_export::write_apng(&mut self.doc, path.clone())
        };
        let sample = ms(t);
        status.map_err(|e| Failure::Fail(format!("export: {e}")))?;
        let bytes = std::fs::read(&path)?;
        let _ = std::fs::remove_file(&path);
        Ok(Trial {
            samples: vec![sample],
            hash: self.op.hash_stable().then(|| sha256::hex(&bytes)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_names_round_trip() {
        for op in ALL_OPS {
            assert_eq!(Op::parse(&op.name()), Some(op));
        }
        assert_eq!(Op::parse("nope"), None);
    }
}
