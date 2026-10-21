//! `<text>` decoding through the real Decoder with a stub layout provider (AC3-5).
mod common;
use common::Control;
use std::sync::Arc;
use vexel_base::WorkControl;
use vexel_scene::*;
use vexel_svg_decode::TraversalDecodeEvent;
use vexel_svg_engine::{SvgEngineSession, TextLayoutProvider};

const SCALE: f64 = 96.0 / 72.0;

/// One 4x6 box glyph per scalar, advance 5 points.
struct Stub {
    panic: bool,
}
impl TextLayoutProvider for Stub {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        if self.panic {
            panic!("injected provider panic");
        }
        let count = std::str::from_utf8(request.text).unwrap().chars().count();
        let mut glyphs = vec![];
        let mut commands = vec![];
        let cmd = |verb, a: f64, b: f64| TextCommand {
            verb,
            values: [a, b, 0., 0., 0., 0.],
        };
        for i in 0..count {
            let first = commands.len();
            commands.push(cmd(TextVerb::Move, 0., 0.));
            commands.push(cmd(TextVerb::Line, 4., 0.));
            commands.push(cmd(TextVerb::Line, 4., 6.));
            commands.push(cmd(TextVerb::Line, 0., 6.));
            commands.push(cmd(TextVerb::Close, 0., 0.));
            glyphs.push(TextGlyph {
                glyph_id: Some(1),
                x: 5.0 * i as f64,
                y: 0.,
                advance: 5.,
                commands: TableRange { first, count: 5 },
            });
        }
        Ok(TextLayout {
            glyphs,
            commands,
            advance_width: 5.0 * count as f64,
            ascent: 6.,
            descent: 2.,
            leading: 0.,
        })
    }
}

/// Present only so the decoder reaches the engine; the engine uses its own provider.
struct Gate;
impl MediaProvider for Gate {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        Err(MediaError::Unsupported)
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}

fn decode(svg: &str, provider: Option<Stub>) -> Result<SceneGraph> {
    let mut control = Control::default();
    let owner = Arc::new(svg.as_bytes().to_vec());
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control)?;
    let mut source = DecodeSource::new(&document, &mut control)?;
    let job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536)?)
        .unwrap_or_else(|_| panic!("job"));
    let mut gate = Gate;
    let mut session = SvgEngineSession::from_source(
        &document,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: Some(&mut gate),
        },
        &mut control,
    )?;
    if let Some(stub) = provider {
        session.set_text_provider(Box::new(stub));
    }
    loop {
        match session.step(64, &job, &mut control)? {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::Complete => break,
            TraversalDecodeEvent::NeedPattern(_) => panic!("pattern"),
        }
    }
    match session.finish(&mut control)? {
        DecodedScene::Drawable(graph) => Ok(graph),
        DecodedScene::Hidden(_) => panic!("hidden"),
    }
}

fn xs(graph: &SceneGraph, shape: usize) -> (f64, f64, f64, f64) {
    let view = graph.root();
    let edges = view.shapes[shape].edges.slice(view.edges).unwrap();
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for e in edges {
        for p in [e.a, e.b] {
            x0 = x0.min(p.x);
            x1 = x1.max(p.x);
            y0 = y0.min(p.y);
            y1 = y1.max(p.y);
        }
    }
    (x0, x1, y0, y1)
}
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}
fn svg(body: &str) -> String {
    format!("<svg xmlns='http://www.w3.org/2000/svg' width='200' height='100'>{body}</svg>")
}
fn ok(body: &str) -> SceneGraph {
    decode(&svg(body), Some(Stub { panic: false })).expect("decode")
}

#[test]
fn glyphs_are_placed_with_baseline_flip_and_unit_scale() {
    let g = ok("<text x='10' y='20' fill='#ff0000'>AB</text>");
    assert_eq!(g.root().shapes.len(), 1);
    let (x0, x1, y0, y1) = xs(&g, 0);
    assert!(near(x0, 10.0) && near(x1, 10.0 + (5.0 + 4.0) * SCALE));
    assert!(near(y0, 20.0 - 6.0 * SCALE) && near(y1, 20.0));
    let first = g.root().shapes[0].edges.first;
    assert_eq!(g.root().edge_colors[first], [255, 0, 0, 255]);
}

#[test]
fn anchor_middle_and_end_shift_by_the_advance() {
    let mid = ok("<text x='100' y='20' text-anchor='middle'>AB</text>");
    assert!(near(xs(&mid, 0).0, 100.0 - 10.0 * SCALE * 0.5));
    let end = ok("<text x='100' y='20' text-anchor='end'>AB</text>");
    assert!(near(xs(&end, 0).0, 100.0 - 10.0 * SCALE));
}

#[test]
fn per_glyph_x_list_overrides_and_dx_accumulates() {
    let g = ok("<text x='10 50' y='20'>AB</text>");
    let (x0, x1, ..) = xs(&g, 0);
    assert!(near(x0, 10.0) && near(x1, 50.0 + 4.0 * SCALE));
    // C quirk kept: dx[0] is folded into the start and added per glyph again
    // (glyph 0 loses it only when an x override resets the accumulation).
    let g = ok("<text y='20' dx='3 2'>AB</text>");
    let (x0, x1, ..) = xs(&g, 0);
    assert!(near(x0, 3.0 + 3.0));
    assert!(near(x1, 3.0 + 5.0 * SCALE + 3.0 + 2.0 + 4.0 * SCALE));
    let g = ok("<text x='10' y='20' dx='3 2'>AB</text>");
    let (x0, x1, ..) = xs(&g, 0);
    assert!(near(x0, 10.0));
    assert!(near(x1, 13.0 + 5.0 * SCALE + 2.0 + 4.0 * SCALE));
}

#[test]
fn tspan_continues_the_cursor_with_its_own_fill() {
    let g = ok("<text x='10' y='20' fill='#00ff00'>A<tspan fill='#0000ff'>B</tspan></text>");
    assert_eq!(g.root().shapes.len(), 2);
    let (x0, ..) = xs(&g, 1);
    assert!(near(x0, 10.0 + 5.0 * SCALE));
    let first = g.root().shapes[1].edges.first;
    assert_eq!(g.root().edge_colors[first], [0, 0, 255, 255]);
}

#[test]
fn source_bytes_and_element_ranges_are_untouched_with_foreign_content() {
    let input = svg("<metadata><ed:note xmlns:ed='urn:ed' ed:k='v'/></metadata>\
<text x='1' y='9' ed:flag='1' xmlns:ed='urn:ed'>AB</text>");
    let g = decode(&input, Some(Stub { panic: false })).expect("decode");
    assert_eq!(g.source(), input.as_bytes());
    let start = input.find("<text").unwrap();
    let end = input.find("</text>").unwrap() + "</text>".len();
    let span = g.root().shapes[0].source.element;
    assert_eq!((span.start, span.end), (start, end));
}

#[test]
fn without_a_provider_text_is_the_current_typed_unsupported() {
    let input = svg("<text x='1' y='9'>AB</text>");
    let error = decode(&input, None).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert!(
        error.offset == input.find("<text").unwrap()
            || error.offset == input.find("<text").unwrap() + 1
    );
}

#[test]
fn plain_stroke_publishes_stroke_state_and_no_fill() {
    let plain = ok("<text x='1' y='9' stroke='#000'>AB</text>");
    assert!(plain.root().shapes[0].filled);
    assert_eq!(plain.root().shapes[0].stroke.rgba, [0, 0, 0, 255]);
    let g = ok("<text x='10' y='20' fill='none' stroke='#fff' stroke-width='3'>AB</text>");
    assert_eq!(g.root().shapes.len(), 1);
    let shape = &g.root().shapes[0];
    assert!(!shape.filled);
    assert_eq!(shape.stroke.rgba, [255, 255, 255, 255]);
    assert!(near(shape.stroke.width, 3.0));
    assert!(shape.stroke_closed);
}

#[test]
fn stroke_dasharray_publishes_dash_values() {
    let g = ok("<text x='1' y='9' stroke='#000' stroke-dasharray='2 1'>AB</text>");
    assert!(g.root().shapes[0].stroke.dash_values.count > 0);
}

#[test]
fn fill_none_without_stroke_publishes_no_shape() {
    // Nothing visible to draw: no fill and no stroke alpha, so no shape is published.
    let g = ok("<text x='10' y='20' fill='none'>AB</text>");
    assert_eq!(g.root().shapes.len(), 0);
}

#[test]
fn stroke_under_singular_transform_fails_closed() {
    let input = svg("<text transform='scale(0)' x='1' y='9' stroke='#000'>AB</text>");
    let error = decode(&input, Some(Stub { panic: false })).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    // Offset is the element name (just past `<`), as in the stroke check.
    assert_eq!(error.offset, input.find("<text").unwrap() + 1);
}

#[test]
fn stroke_paint_server_and_text_path_fail_closed() {
    for body in [
        "<text x='1' y='9' stroke='url(#g)'>AB</text>",
        "<text x='1' y='9' fill='url(#g)'>AB</text>",
        "<text x='1' y='9'><textPath href='#p'>AB</textPath></text>",
    ] {
        let error = decode(&svg(body), Some(Stub { panic: false })).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Unsupported, "{body}");
    }
}

#[test]
fn provider_panic_is_a_typed_error_not_an_unwind() {
    let error = decode(
        &svg("<text x='1' y='9'>AB</text>"),
        Some(Stub { panic: true }),
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn word_spacing_matches_the_c_byte_cursor_quirk() {
    // C's glyph scan skips the whole ASCII tail after glyph 0, so word spacing
    // never moves glyphs of ASCII text (golden: word-spacing fixture), while the
    // cursor still advances by spaces * spacing.
    let plain = ok("<text x='10' y='20'>A B</text>");
    let spaced = ok("<text x='10' y='20' word-spacing='9'>A B</text>");
    assert!(near(xs(&plain, 0).1, xs(&spaced, 0).1));
    let next = ok("<text x='10' y='20' word-spacing='9'>A B<tspan>C</tspan></text>");
    assert!(near(xs(&next, 1).0, 10.0 + 15.0 * SCALE + 9.0));
}

fn first_color(g: &SceneGraph, shape: usize) -> [u8; 4] {
    g.root().edge_colors[g.root().shapes[shape].edges.first]
}

#[test]
fn tspan_absolute_x_y_start_a_new_chunk() {
    let g = ok("<text x='10' y='20'>A<tspan x='50' y='60'>B</tspan><tspan>C</tspan></text>");
    assert_eq!(g.root().shapes.len(), 3);
    let (x0, _, _, y1) = xs(&g, 1);
    assert!(near(x0, 50.0) && near(y1, 60.0));
    // The next run continues from the end of the tspan.
    assert!(near(xs(&g, 2).0, 50.0 + 5.0 * SCALE));
    assert!(near(xs(&g, 2).3, 60.0));
    assert!(near(xs(&g, 0).0, 10.0));
}

#[test]
fn tspan_dx_dy_follow_the_legacy_c_folding_from_the_cursor() {
    // Same C quirk as text (dx[0]/dy[0] fold into the start and again per
    // glyph); the render-text parity goldens encode it.
    let g = ok("<text x='10' y='20'>A<tspan dx='7' dy='3'>B</tspan></text>");
    let (x0, _, _, y1) = xs(&g, 1);
    assert!(near(x0, 10.0 + 5.0 * SCALE + 14.0));
    assert!(near(y1, 26.0));
    // With an absolute x the glyph-0 dx is reset, as for text.
    let g = ok("<text x='10' y='20'>A<tspan x='40' dx='2'>B</tspan></text>");
    assert!(near(xs(&g, 1).0, 40.0));
}

#[test]
fn tspan_inherits_and_overrides_fill() {
    let g = ok(
        "<text x='10' y='20' fill='#ff0000'>A<tspan>B</tspan><tspan fill='#00ff00'>C</tspan></text>",
    );
    assert_eq!(first_color(&g, 1), [255, 0, 0, 255]);
    assert_eq!(first_color(&g, 2), [0, 255, 0, 255]);
}

#[test]
fn nested_tspans_inherit_from_their_parent_and_continue_the_cursor() {
    let g = ok(
        "<text x='10' y='20' fill='#ff0000'>A<tspan fill='#00ff00'>B<tspan>C</tspan>\
<tspan fill='#0000ff'>D</tspan></tspan></text>",
    );
    assert_eq!(g.root().shapes.len(), 4);
    let colors: Vec<_> = (0..4).map(|i| first_color(&g, i)).collect();
    assert_eq!(colors[1], [0, 255, 0, 255]);
    assert_eq!(colors[2], [0, 255, 0, 255]);
    assert_eq!(colors[3], [0, 0, 255, 255]);
    for i in 0..4 {
        assert!(near(xs(&g, i).0, 10.0 + 5.0 * SCALE * i as f64));
    }
}

#[test]
fn tspan_anchor_middle_centres_each_positioned_chunk() {
    let g = ok(
        "<text x='100' y='20' text-anchor='middle'><tspan x='100' y='20'>AB</tspan>\
<tspan x='50' y='40'>ABCD</tspan></text>",
    );
    assert!(near(xs(&g, 0).0, 100.0 - 10.0 * SCALE * 0.5));
    assert!(near(xs(&g, 1).0, 50.0 - 20.0 * SCALE * 0.5));
    assert!(near(xs(&g, 1).3, 40.0));
}

#[test]
fn unsupported_tspan_attributes_fail_at_the_attribute() {
    for attribute in [
        "rotate='10'",
        "textLength='30'",
        "lengthAdjust='spacing'",
        "baseline-shift='super'",
        "style='baseline-shift:sub'",
        "x='1 2'",
        "dy='1,2'",
    ] {
        let input = svg(&format!(
            "<text x='1' y='9'>A<tspan {attribute}>B</tspan></text>"
        ));
        let error = decode(&input, Some(Stub { panic: false })).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Unsupported, "{attribute}");
        let name = attribute.split(['=', ':']).next().unwrap();
        let at = input.find(&format!("<tspan {attribute}")).unwrap() + "<tspan ".len();
        let value = at + attribute.find('\'').unwrap() + 1;
        assert!(
            error.offset == at || error.offset == value,
            "{attribute} {name}"
        );
    }
    // A no-op baseline-shift declaration is accepted.
    let ok_input =
        svg("<text x='1' y='9'>A<tspan style='baseline-shift:baseline'>B</tspan></text>");
    assert!(decode(&ok_input, Some(Stub { panic: false })).is_ok());
}
