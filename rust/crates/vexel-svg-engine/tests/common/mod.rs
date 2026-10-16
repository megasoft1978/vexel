//! Shared real-Decoder harness for the V1 subset tests (drives `SvgEngineSession`).
#![allow(dead_code)]
use std::sync::Arc;
use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::*;
use vexel_svg_decode::TraversalDecodeEvent;
use vexel_svg_engine::SvgEngineSession;

/// Unlimited control that can cancel at the Nth `checkpoint` or `charge` call.
#[derive(Default)]
pub struct Control {
    pub cancel_at_call: Option<u64>,
    pub calls: u64,
}
impl Control {
    pub fn canceling_at(call: u64) -> Self {
        Self {
            cancel_at_call: Some(call),
            calls: 0,
        }
    }
    fn tick(&mut self) -> std::result::Result<(), Stop> {
        self.calls += 1;
        match self.cancel_at_call {
            Some(at) if self.calls >= at => Err(Stop::Canceled),
            _ => Ok(()),
        }
    }
}
impl WorkControl for Control {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        self.tick()
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        self.tick()
    }
}

/// Full pipeline: parse, open session, step to `Complete`, finish. Any stage error
/// is returned as the typed diagnostic; no scene escapes on error.
pub enum Outcome {
    Scene(DecodedScene),
    /// Decoder suspended on pattern work; never a successful completion.
    NeedPattern,
}

pub fn try_decode(text: &[u8], control: &mut Control) -> Result<DecodedScene> {
    match try_decode_outcome(text, control)? {
        Outcome::Scene(scene) => Ok(scene),
        Outcome::NeedPattern => panic!("pattern work reported where none was expected"),
    }
}

pub fn try_decode_outcome(text: &[u8], control: &mut Control) -> Result<Outcome> {
    let owner = Arc::new(text.to_vec());
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, control)?;
    let mut source = DecodeSource::new(&document, control)?;
    let job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536)?)
        .unwrap_or_else(|_| panic!("job setup"));
    let mut session = SvgEngineSession::from_source(
        &document,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        control,
    )?;
    loop {
        match session.step(64, &job, control)? {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::Complete => break,
            TraversalDecodeEvent::NeedPattern(_) => return Ok(Outcome::NeedPattern),
        }
    }
    session.finish(control).map(Outcome::Scene)
}

pub fn decode_ok(text: &str) -> SceneGraph {
    match try_decode(text.as_bytes(), &mut Control::default()) {
        Ok(DecodedScene::Drawable(graph)) => graph,
        Ok(DecodedScene::Hidden(_)) => panic!("expected drawable scene"),
        Err(error) => panic!("decode failed: {error:?}"),
    }
}

pub fn decode_err(text: &str) -> Diagnostic {
    match try_decode(text.as_bytes(), &mut Control::default()) {
        Err(error) => error,
        Ok(_) => panic!("decode unexpectedly succeeded"),
    }
}

pub fn element_offset(text: &str, tag: &str) -> usize {
    text.find(&format!("<{tag}"))
        .unwrap_or_else(|| panic!("no <{tag}>"))
}
