#[path = "support/production.rs"]
mod production;
#[path = "support/mod.rs"]
mod support;

use production::{ProductionFixture, output};
use std::sync::Arc;
use support::Run;
use vexel_geometry::{Point, affine_identity};
use vexel_scene::*;
use vexel_svg_decode::{Decoder, GeometryBuilder};

fn custom_geometry(control: &mut Run) -> vexel_svg_decode::Geometry {
    let mut builder = GeometryBuilder::new(affine_identity(), 0).unwrap();
    builder
        .add_edge(Point { x: 0.0, y: 0.0 }, Point { x: 4.0, y: 0.0 }, control)
        .unwrap();
    builder
        .add_edge(Point { x: 4.0, y: 0.0 }, Point { x: 4.0, y: 4.0 }, control)
        .unwrap();
    builder.finish(true, true, 3)
}

fn run_custom(input: &str, style_host: &str) -> (SceneGraph, usize, ProductionFixture) {
    let source = Arc::new(input.as_bytes().to_vec());
    let mut control = Run::default();
    let doc = LexicalDocument::parse(source.clone(), 1 << 24, &mut control).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut control).unwrap();
    let host = doc
        .tokens()
        .iter()
        .position(|token| {
            token
                .local_name
                .slice(&source)
                .is_some_and(|n| n == style_host.as_bytes())
        })
        .unwrap();
    let style = shared
        .compute_style(ElementId(host), None, &mut control)
        .unwrap();
    let token = doc.tokens()[host];
    let map = SourceMap {
        element: vexel_base::Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    };
    let mut decoder = Decoder::from_source(
        &doc,
        &mut shared,
        31,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    decoder
        .enqueue_custom_geometry(
            decoder.builder().unwrap().root_id(),
            ElementId(host),
            map,
            custom_geometry(&mut control),
            style,
            [8.0, 8.0],
            &mut control,
        )
        .unwrap();
    let mut job = RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
    let mut operations = ProductionFixture::default();
    let mut patterns = 0;
    loop {
        match decoder
            .step_scoped_production(64, &mut operations, &job, &mut control)
            .unwrap_or_else(|error| {
                panic!("custom step after {patterns} resumes failed: {error:?}")
            }) {
            ScopedDecodeEvent::Progress => {}
            ScopedDecodeEvent::Pattern(request) => {
                let ticket = request.ticket();
                assert_eq!(request.child().input().static_generation(), Some(31));
                drop(request);
                let pixels = output(&mut job, 31, &mut control);
                decoder
                    .resume_pattern_production(ticket, &mut job, Ok(pixels), &mut control)
                    .unwrap();
                patterns += 1;
            }
            ScopedDecodeEvent::Complete => break,
        }
    }
    let graph = decoder.finish(&mut control).unwrap();
    assert_eq!(job.stats().output_bytes, 0);
    (graph, patterns, operations)
}

#[test]
fn custom_fill_and_stroke_patterns_publish_through_sequential_resumes() {
    for (style, expected, fill, stroke) in [
        ("fill='url(#p)'", 1, true, false),
        ("fill='none' stroke='url(#p)'", 1, false, true),
        ("fill='url(#p)' stroke='url(#p)'", 2, true, true),
    ] {
        let input = format!(
            "<svg width='8' height='8'><defs><pattern id='p'><rect width='2' height='2'/></pattern></defs><g {style}></g></svg>"
        );
        let (graph, patterns, _) = run_custom(&input, "g");
        assert_eq!(patterns, expected);
        assert_eq!(graph.root().shapes.len(), 1);
        assert_eq!(graph.root().shapes[0].fill_paint.is_some(), fill);
        assert_eq!(graph.root().shapes[0].stroke_paint.is_some(), stroke);
    }
}

#[test]
fn custom_clip_is_finalized_and_missing_clip_keeps_source_offset() {
    let good = "<svg width='8' height='8'><defs><clipPath id='c'><rect width='2' height='2'/></clipPath></defs><g clip-path='url(#c)'></g></svg>";
    let (graph, patterns, _) = run_custom(good, "g");
    assert_eq!(patterns, 0);
    assert_eq!(graph.root().shapes.len(), 1);
    assert!(graph.root().shapes[0].clip.is_some());

    let bad = "<svg width='8' height='8'><g clip-path='url(#missing)'></g></svg>";
    let source = Arc::new(bad.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(source.clone(), 1 << 24, &mut control).unwrap();
    let mut shared = DecodeSource::new(&document, &mut control).unwrap();
    let host = document
        .tokens()
        .iter()
        .position(|t| t.local_name.slice(&source) == Some(b"g" as &[u8]))
        .unwrap();
    let style = shared
        .compute_style(ElementId(host), None, &mut control)
        .unwrap();
    let token = document.tokens()[host];
    let map = SourceMap {
        element: vexel_base::Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    };
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        31,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    decoder
        .enqueue_custom_geometry(
            decoder.builder().unwrap().root_id(),
            ElementId(host),
            map,
            custom_geometry(&mut control),
            style,
            [8.0, 8.0],
            &mut control,
        )
        .unwrap();
    let job = RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
    let mut ops = ProductionFixture::default();
    let err = loop {
        match decoder.step_scoped_production(64, &mut ops, &job, &mut control) {
            Ok(ScopedDecodeEvent::Progress) => {}
            Ok(ScopedDecodeEvent::Pattern(_)) => panic!("unexpected pattern"),
            Ok(ScopedDecodeEvent::Complete) => panic!("missing clip was accepted"),
            Err(error) => break error,
        }
    };
    assert_eq!(err.offset, bad.find("url(#missing)").unwrap());
}

#[test]
fn custom_hidden_geometry_publishes_no_shape() {
    let input = "<svg width='8' height='8'><g display='none'></g></svg>";
    let (graph, patterns, _) = run_custom(input, "g");
    assert_eq!(patterns, 0);
    assert!(graph.root().shapes.is_empty());
}

#[test]
fn custom_solid_geometry_uses_guarded_request_publication() {
    let input = "<svg width='8' height='8'><g fill='red'></g></svg>";
    let (graph, patterns, _) = run_custom(input, "g");
    assert_eq!(patterns, 0);
    assert_eq!(graph.root().shapes.len(), 1);
}

#[test]
fn nested_custom_pattern_child_resumes_before_its_parent() {
    let input = "<svg width='8' height='8'><defs><pattern id='outer'><rect width='2' height='2' fill='url(#inner)'/></pattern><pattern id='inner'><rect width='1' height='1' fill='red'/></pattern></defs><g fill='url(#outer)'></g></svg>";
    let (graph, patterns, _) = run_custom(input, "g");
    assert_eq!(patterns, 2);
    assert_eq!(graph.root().shapes.len(), 1);
    assert!(graph.root().shapes[0].fill_paint.is_some());
}

#[test]
fn custom_singular_transform_reports_the_original_paint_offset() {
    let input = "<svg width='8' height='8'><defs><pattern id='p'><rect width='1' height='1'/></pattern></defs><g fill='url(#p)'></g></svg>";
    let source = Arc::new(input.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(source.clone(), 1 << 24, &mut control).unwrap();
    let mut shared = DecodeSource::new(&document, &mut control).unwrap();
    let host = document
        .tokens()
        .iter()
        .position(|t| t.local_name.slice(&source) == Some(b"g" as &[u8]))
        .unwrap();
    let style = shared
        .compute_style(ElementId(host), None, &mut control)
        .unwrap();
    let token = document.tokens()[host];
    let span = style.value(vexel_svg_css::Property::Fill).unwrap().span;
    let map = SourceMap {
        element: vexel_base::Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: vexel_geometry::Affine {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        },
        transform_node: None,
    };
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        31,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    decoder
        .enqueue_custom_geometry(
            decoder.builder().unwrap().root_id(),
            ElementId(host),
            map,
            custom_geometry(&mut control),
            style,
            [8.0, 8.0],
            &mut control,
        )
        .unwrap();
    let job = RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
    let mut ops = ProductionFixture::default();
    let error = loop {
        match decoder.step_scoped_production(64, &mut ops, &job, &mut control) {
            Ok(ScopedDecodeEvent::Progress) => {}
            Ok(ScopedDecodeEvent::Pattern(_)) => {
                panic!("singular paint must fail before suspension")
            }
            Ok(ScopedDecodeEvent::Complete) => panic!("singular transform was accepted"),
            Err(error) => break error,
        }
    };
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(error.offset, span.offset);
}

#[test]
fn custom_resume_rejects_wrong_dimensions_and_generation_and_reclaims_output() {
    for wrong_generation in [false, true] {
        let input = "<svg width='8' height='8'><defs><pattern id='p'><rect width='1' height='1'/></pattern></defs><g fill='url(#p)'></g></svg>";
        let source = Arc::new(input.as_bytes().to_vec());
        let mut control = Run::default();
        let document = LexicalDocument::parse(source.clone(), 1 << 24, &mut control).unwrap();
        let mut shared = DecodeSource::new(&document, &mut control).unwrap();
        let host = document
            .tokens()
            .iter()
            .position(|t| t.local_name.slice(&source) == Some(b"g" as &[u8]))
            .unwrap();
        let style = shared
            .compute_style(ElementId(host), None, &mut control)
            .unwrap();
        let token = document.tokens()[host];
        let map = SourceMap {
            element: vexel_base::Span {
                start: token.source.offset,
                end: token.source.offset + token.source.length,
            },
            transform: affine_identity(),
            transform_node: None,
        };
        let mut decoder = Decoder::from_source(
            &document,
            &mut shared,
            31,
            DecodeOptions {
                limits: Limits::default(),
                system_language: b"en",
                media: None,
            },
            &mut control,
        )
        .unwrap();
        decoder
            .enqueue_custom_geometry(
                decoder.builder().unwrap().root_id(),
                ElementId(host),
                map,
                custom_geometry(&mut control),
                style,
                [8.0, 8.0],
                &mut control,
            )
            .unwrap();
        let mut job = RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
        let mut ops = ProductionFixture::default();
        let ticket = loop {
            match decoder
                .step_scoped_production(64, &mut ops, &job, &mut control)
                .unwrap()
            {
                ScopedDecodeEvent::Progress => {}
                ScopedDecodeEvent::Pattern(request) => break request.ticket(),
                ScopedDecodeEvent::Complete => panic!("custom pattern did not suspend"),
            }
        };
        let mut rendered = output(
            &mut job,
            if wrong_generation { 32 } else { 31 },
            &mut control,
        );
        if wrong_generation {
            rendered.generation = 32;
        } else {
            rendered.width = 2;
        }
        assert!(
            decoder
                .resume_pattern_production(ticket, &mut job, Ok(rendered), &mut control)
                .is_err()
        );
        assert_eq!(job.stats().output_bytes, 0);
        assert!(
            decoder
                .builder()
                .unwrap()
                .traversal_scene(decoder.builder().unwrap().root_id())
                .unwrap()
                .shapes
                .is_empty()
        );
    }
}

#[test]
fn custom_resume_cancellation_rolls_back_and_reclaims_output() {
    let input = "<svg width='8' height='8'><defs><pattern id='p'><rect width='1' height='1'/></pattern></defs><g fill='url(#p)'></g></svg>";
    let source = Arc::new(input.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(source.clone(), 1 << 24, &mut control).unwrap();
    let mut shared = DecodeSource::new(&document, &mut control).unwrap();
    let host = document
        .tokens()
        .iter()
        .position(|t| t.local_name.slice(&source) == Some(b"g" as &[u8]))
        .unwrap();
    let style = shared
        .compute_style(ElementId(host), None, &mut control)
        .unwrap();
    let token = document.tokens()[host];
    let map = SourceMap {
        element: vexel_base::Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    };
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        31,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    decoder
        .enqueue_custom_geometry(
            decoder.builder().unwrap().root_id(),
            ElementId(host),
            map,
            custom_geometry(&mut control),
            style,
            [8.0, 8.0],
            &mut control,
        )
        .unwrap();
    let mut job = RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
    let mut ops = ProductionFixture::default();
    let ticket = loop {
        match decoder
            .step_scoped_production(64, &mut ops, &job, &mut control)
            .unwrap()
        {
            ScopedDecodeEvent::Progress => {}
            ScopedDecodeEvent::Pattern(request) => break request.ticket(),
            ScopedDecodeEvent::Complete => panic!("custom pattern did not suspend"),
        }
    };
    let rendered = output(&mut job, 31, &mut control);
    control.stop_after = Some(control.units);
    assert!(
        decoder
            .resume_pattern_production(ticket, &mut job, Ok(rendered), &mut control)
            .is_err()
    );
    assert_eq!(job.stats().output_bytes, 0);
    assert!(
        decoder
            .builder()
            .unwrap()
            .traversal_scene(decoder.builder().unwrap().root_id())
            .unwrap()
            .shapes
            .is_empty()
    );
}

#[test]
fn custom_wrong_job_and_reused_ticket_fail_without_retaining_output() {
    let input = "<svg width='8' height='8'><defs><pattern id='p'><rect width='1' height='1'/></pattern></defs><g fill='url(#p)'></g></svg>";
    let source = Arc::new(input.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(source.clone(), 1 << 24, &mut control).unwrap();
    let mut shared = DecodeSource::new(&document, &mut control).unwrap();
    let host = document
        .tokens()
        .iter()
        .position(|t| t.local_name.slice(&source) == Some(b"g" as &[u8]))
        .unwrap();
    let style = shared
        .compute_style(ElementId(host), None, &mut control)
        .unwrap();
    let token = document.tokens()[host];
    let map = SourceMap {
        element: vexel_base::Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    };
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        31,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    decoder
        .enqueue_custom_geometry(
            decoder.builder().unwrap().root_id(),
            ElementId(host),
            map,
            custom_geometry(&mut control),
            style,
            [8.0, 8.0],
            &mut control,
        )
        .unwrap();
    let mut job = RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
    let mut wrong_job =
        RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
    let mut ops = ProductionFixture::default();
    let ticket = loop {
        match decoder
            .step_scoped_production(64, &mut ops, &job, &mut control)
            .unwrap()
        {
            ScopedDecodeEvent::Progress => {}
            ScopedDecodeEvent::Pattern(request) => break request.ticket(),
            ScopedDecodeEvent::Complete => panic!("custom pattern did not suspend"),
        }
    };
    let rendered = output(&mut wrong_job, 31, &mut control);
    let foreign_output = rendered.pixels;
    assert!(
        decoder
            .resume_pattern_production(ticket, &mut job, Ok(rendered), &mut control)
            .is_err()
    );
    assert_eq!(job.stats().output_bytes, 0);
    assert_eq!(wrong_job.stats().output_bytes, 4);
    wrong_job.release(foreign_output).unwrap();
    assert_eq!(wrong_job.stats().output_bytes, 0);

    let reused = output(&mut job, 31, &mut control);
    assert!(
        decoder
            .resume_pattern_production(ticket, &mut job, Ok(reused), &mut control)
            .is_err()
    );
    assert_eq!(job.stats().output_bytes, 0);
    assert!(
        decoder
            .builder()
            .unwrap()
            .traversal_scene(decoder.builder().unwrap().root_id())
            .unwrap()
            .shapes
            .is_empty()
    );
}

#[test]
fn custom_late_pattern_failure_rolls_back_custom_suffix() {
    let input = "<svg width='8' height='8'><defs><pattern id='p'><rect width='1' height='1'/></pattern></defs><g fill='url(#p)'></g></svg>";
    let source = Arc::new(input.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(source.clone(), 1 << 24, &mut control).unwrap();
    let mut shared = DecodeSource::new(&document, &mut control).unwrap();
    let host = document
        .tokens()
        .iter()
        .position(|t| t.local_name.slice(&source) == Some(b"g" as &[u8]))
        .unwrap();
    let style = shared
        .compute_style(ElementId(host), None, &mut control)
        .unwrap();
    let token = document.tokens()[host];
    let map = SourceMap {
        element: vexel_base::Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    };
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        31,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    decoder
        .enqueue_custom_geometry(
            decoder.builder().unwrap().root_id(),
            ElementId(host),
            map,
            custom_geometry(&mut control),
            style,
            [8.0, 8.0],
            &mut control,
        )
        .unwrap();
    let mut job = RasterJob::new(ChargeLedger::new(&source, 1 << 24, 65536).unwrap()).unwrap();
    let mut ops = ProductionFixture::default();
    let ticket = loop {
        match decoder
            .step_scoped_production(64, &mut ops, &job, &mut control)
            .unwrap()
        {
            ScopedDecodeEvent::Progress => {}
            ScopedDecodeEvent::Pattern(request) => break request.ticket(),
            ScopedDecodeEvent::Complete => panic!("custom pattern did not suspend"),
        }
    };
    let error = Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: input.find("<pattern").unwrap(),
    };
    assert!(
        decoder
            .resume_pattern_production(ticket, &mut job, Err(error), &mut control)
            .is_err()
    );
    assert!(
        decoder
            .builder()
            .unwrap()
            .traversal_scene(decoder.builder().unwrap().root_id())
            .unwrap()
            .shapes
            .is_empty()
    );
}
