mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
use vexel_svg_decode::*;
use vexel_xml::XmlSpan;
#[test]
fn numeric_variable_loops_preserve_cancellation() {
    let text = "1.0000000000000000000000000000000000000000000000000";
    let span = XmlSpan {
        offset: 0,
        length: text.len(),
    };
    for which in 0..3 {
        let mut w = Run {
            units: 0,
            stop_after: Some(8),
        };
        let result = match which {
            0 => read_length_controlled(text.as_bytes(), span, 10.0, &mut w).map(|_| ()),
            1 => parse_dasharray(text.as_bytes(), span, 10.0, &mut w).map(|_| ()),
            _ => parse_opacity_value_controlled(text.as_bytes(), span, &mut w).map(|_| ()),
        };
        assert_eq!(result.unwrap_err().kind, ErrorKind::Canceled);
    }
}
#[test]
fn actual_style_context_current_color_alpha_and_dash_limits() {
    let graph=decode("<svg width=\"20\" height=\"20\" color=\"red\"><g fill=\"currentColor\"><rect width=\"2\" height=\"2\" fill-opacity=\"50%\" stroke=\"currentColor\" stroke-opacity=\"25%\" stroke-width=\"2\" stroke-dasharray=\"1 2 3\" stroke-linejoin=\"arcs\"/></g></svg>").unwrap();
    let root = graph.root();
    assert_eq!(root.edge_colors[0], [255, 0, 0, 128]);
    assert_eq!(root.shapes[0].stroke.rgba, [255, 0, 0, 64]);
    assert_eq!(root.shapes[0].stroke.join, Linejoin::Round);
    assert_eq!(root.shapes[0].stroke.dash_values.count, 3);
    assert_eq!(root.dash_values, &[1.0, 2.0, 3.0]);
    assert_eq!(decode("<svg width=\"8\" height=\"8\"><rect width=\"2\" height=\"2\" stroke-dasharray=\"1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17\"/></svg>").unwrap_err().kind,ErrorKind::Number);
    assert_eq!(decode("<svg width=\"8\" height=\"8\" color=\"currentColor\"><rect width=\"2\" height=\"2\" fill=\"currentColor\"/></svg>").unwrap_err().kind,ErrorKind::Unsupported);
}
#[test]
fn actual_step_cancel_rolls_back_and_forbids_finish() {
    let text="<svg width=\"8\" height=\"8\"><rect width=\"2\" height=\"2\"/><rect width=\"3\" height=\"3\"/></svg>";
    let mut w = Run::default();
    let doc = LexicalDocument::parse(
        Arc::new(text.as_bytes().to_vec()),
        256 * 1024 * 1024,
        &mut w,
    )
    .unwrap();
    let mut source = DecodeSource::new(&doc, &mut w).unwrap();
    let mut d = Decoder::from_source(
        &doc,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut w,
    )
    .unwrap();
    w.stop_after = Some(w.units + 20);
    assert_eq!(
        d.step(512, &mut RejectOperations, &mut w).unwrap_err().kind,
        ErrorKind::Canceled
    );
    assert_eq!(d.state(), DecoderState::Failed);
    assert!(d.finish(&mut Run::default()).is_err());
}
#[test]
fn actual_non_scaling_stroke_uniformity_and_paint_order_offsets() {
    let g=decode("<svg width=\"16\" height=\"16\"><path d=\"M1 1 L3 3\" transform=\"scale(2)\" stroke=\"red\" stroke-width=\"4\" vector-effect=\"non-scaling-stroke\" paint-order=\"stroke fill markers\"/></svg>").unwrap();
    assert_eq!(g.root().shapes[0].stroke.width, 2.);
    assert_eq!(g.root().shapes[0].stroke.paint_order, 1);
    let s="<svg width=\"16\" height=\"16\"><path d=\"M1 1 L3 3\" transform=\"scale(2 1)\" stroke=\"red\" vector-effect=\"non-scaling-stroke\"/></svg>";
    let e = decode(s).unwrap_err();
    assert_eq!(
        (e.kind, e.offset),
        (
            ErrorKind::Unsupported,
            s.find("non-scaling-stroke").unwrap()
        )
    );
    let s="<svg width=\"8\" height=\"8\"><rect width=\"1\" height=\"1\" paint-order=\"fill stroke fill\"/></svg>";
    let e = decode(s).unwrap_err();
    assert_eq!(
        (e.kind, e.offset),
        (ErrorKind::Unsupported, s.rfind("fill").unwrap())
    );
}

#[test]
fn original_empty_group_clip_does_not_resolve_a_missing_definition() {
    for group in [
        "<g clip-path=\"url(#missing)\"/>",
        "<g clip-path=\"url(#missing)\"></g>",
    ] {
        let graph = decode(&format!("<svg width=\"8\" height=\"8\">{group}</svg>")).unwrap();
        assert!(graph.root().shapes.is_empty());
    }
}
