#![allow(dead_code)]
//! Explicit test-only delegates; unexpected lower operations fail the fixture.
use std::sync::Arc;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderState};
use vexel_xml::XmlSpan;
#[derive(Default)]
pub struct Run {
    pub units: u64,
    pub stop_after: Option<u64>,
}
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, n: u64) -> std::result::Result<(), Stop> {
        let next = self.units.checked_add(n).ok_or(Stop::Overflow)?;
        if self.stop_after.is_some_and(|limit| next > limit) {
            return Err(Stop::Canceled);
        }
        self.units = next;
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.stop_after.is_some_and(|limit| self.units >= limit) {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}
pub struct RejectOperations;
impl DecodeOperations for RejectOperations {
    fn resolve_paint(
        &mut self,
        _: &mut ResolveContext<'_, '_>,
        v: XmlSpan,
        _: &[Edge],
        _: Affine,
        _: [f64; 2],
        _: &mut dyn WorkControl,
    ) -> Result<ResolvedPaint> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: v.offset,
        })
    }
    fn plan_definition<'src, 'ctx>(
        &mut self,
        _: &mut DefinitionContext<'src, 'ctx>,
        _: DefinitionKind,
        e: ElementId,
        _: ShapeId,
        _: Affine,
        _: [f64; 4],
        _: &CssPaint<'src>,
        _: &mut dyn WorkControl,
    ) -> Result<DefinitionPlan<'src>> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: e.0,
        })
    }
    fn decode_text(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        _: &mut SceneBuilder,
        _: &mut dyn WorkControl,
    ) -> Result<bool> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: r.source_map.element.start,
        })
    }
    fn decode_image(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        _: &mut SceneBuilder,
        _: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: r.source_map.element.start,
        })
    }
}
pub fn decode(input: &str) -> Result<SceneGraph> {
    decode_with(input, b"en", &mut RejectOperations)
}
pub fn decode_with(
    input: &str,
    language: &[u8],
    operations: &mut dyn DecodeOperations,
) -> Result<SceneGraph> {
    let source = Arc::new(input.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(source.clone(), 256 * 1024 * 1024, &mut control)?;
    let mut shared = DecodeSource::new(&document, &mut control)?;
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        17,
        DecodeOptions {
            limits: Limits::default(),
            system_language: language,
            media: None,
        },
        &mut control,
    )?;
    while decoder.state() == DecoderState::Running {
        let event = decoder.step(1024, operations, &mut control)?;
        if matches!(event, DecodeEvent::Progress { charged_units: 0 }) {
            panic!("empty progress")
        }
    }
    let graph = decoder.finish(&mut control)?;
    assert!(Arc::ptr_eq(graph.source_owner(), &source));
    assert_eq!(graph.source(), input.as_bytes());
    Ok(graph)
}
pub fn offset(text: &str, needle: &str) -> usize {
    text.find(needle).unwrap()
}
