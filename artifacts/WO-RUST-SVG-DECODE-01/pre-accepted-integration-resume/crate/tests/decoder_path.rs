use std::sync::Arc;
use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderState};
use vexel_xml::XmlSpan;

struct Control {
    units: u64,
    cancel: bool,
}
impl WorkControl for Control {
    fn charge(&mut self, _: WorkKind, n: u64) -> std::result::Result<(), Stop> {
        if self.cancel {
            return Err(Stop::Canceled);
        }
        self.units = self.units.checked_add(n).ok_or(Stop::Overflow)?;
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.cancel {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}
struct NoUnexpectedOperations;
impl DecodeOperations for NoUnexpectedOperations {
    fn resolve_paint(
        &mut self,
        _: &mut ResolveContext<'_, '_>,
        value: XmlSpan,
        _: &[Edge],
        _: Affine,
        _: [f64; 2],
        _: &mut dyn WorkControl,
    ) -> Result<ResolvedPaint> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: value.offset,
        })
    }
    fn plan_definition<'src, 'ctx>(
        &mut self,
        _: &mut DefinitionContext<'src, 'ctx>,
        _: DefinitionKind,
        element: ElementId,
        _: ShapeId,
        _: Affine,
        _: [f64; 4],
        _: &vexel_svg_css::CssPaint<'src>,
        _: &mut dyn WorkControl,
    ) -> Result<DefinitionPlan<'src>> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: element.0,
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
            offset: r.element.element.0,
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
            offset: r.element.element.0,
        })
    }
}

#[test]
fn owned_transaction_steps_and_finishes_path_scene() {
    let source = Arc::new(
        b"<svg width='8' height='8'><path d='M1 1L7 1L7 7Z' fill='#336699'/></svg>".to_vec(),
    );
    let mut ctl = Control {
        units: 0,
        cancel: false,
    };
    let doc = LexicalDocument::parse(source, 1024 * 1024, &mut ctl).unwrap();
    let options = DecodeOptions {
        limits: Limits::default(),
        system_language: b"en",
        media: None,
    };
    let mut decoder = Decoder::new(&doc, 7, options, &mut ctl).unwrap();
    let mut ops = NoUnexpectedOperations;
    while decoder.state() == DecoderState::Running {
        let event = decoder.step(2, &mut ops, &mut ctl).unwrap();
        if matches!(event, DecodeEvent::Complete) {
            break;
        }
    }
    assert_eq!(decoder.state(), DecoderState::Complete);
    let graph = decoder.finish(&mut ctl).unwrap();
    let scene = graph.root();
    assert_eq!(graph.generation(), 7);
    assert_eq!(scene.shapes.len(), 1);
    assert_eq!(scene.edges.len(), 3);
    assert_eq!(scene.edge_colors[0], [51, 102, 153, 255]);
}

#[test]
fn cancellation_rolls_back_active_scene_draft() {
    let source = Arc::new(b"<svg width='8' height='8'><path d='M1 1L7 1L7 7Z'/></svg>".to_vec());
    let mut ctl = Control {
        units: 0,
        cancel: false,
    };
    let doc = LexicalDocument::parse(source, 1024 * 1024, &mut ctl).unwrap();
    let options = DecodeOptions {
        limits: Limits::default(),
        system_language: b"en",
        media: None,
    };
    let mut decoder = Decoder::new(&doc, 1, options, &mut ctl).unwrap();
    let mut ops = NoUnexpectedOperations;
    let _ = decoder.step(1, &mut ops, &mut ctl).unwrap();
    ctl.cancel = true;
    assert_eq!(
        decoder.step(1, &mut ops, &mut ctl).unwrap_err().kind,
        ErrorKind::Canceled
    );
    assert_eq!(decoder.state(), DecoderState::Failed);
    assert!(decoder.builder().unwrap().memory_stats().live_bytes > 0);
}
