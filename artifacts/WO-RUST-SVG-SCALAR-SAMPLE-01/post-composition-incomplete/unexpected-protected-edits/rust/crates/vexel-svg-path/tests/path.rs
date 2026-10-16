use std::sync::atomic::AtomicBool;
use vexel_base::Span;
use vexel_base::{BudgetControl, ControlledError};
use vexel_geometry::{Affine, Edge, Point, affine_identity};
use vexel_svg_path::{
    Diagnostic, DiagnosticKind, MAX_COORDINATE, MAX_PATH_COMMANDS, MAX_PATH_EDGES,
    MAX_PATH_NUMBERS, ParseOptions, append_edge, flatten_cubic, parse_path, transformed_tolerance,
};

fn parse(data: &[u8]) -> vexel_svg_path::PathDocument {
    parse_path(
        data,
        Span {
            start: 0,
            end: data.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-10, "{actual} != {expected}");
}

#[test]
fn supports_every_command_relative_forms_and_authored_command_provenance() {
    let path =
        parse(b"M1 2 L3 4 h2 v-1 C6 5 7 6 8 7 s2 2 3 3 Q12 12 14 10 t2 0 A2 3 0 0 1 19 12 z");
    assert_eq!(
        path.segments
            .iter()
            .map(|segment| segment.command)
            .collect::<Vec<_>>(),
        b"MLhvCsQtAz"
    );
    assert_eq!(path.subpaths.len(), 1);
    assert!(path.subpaths[0].closed);
    assert_eq!(path.subpaths[0].first_edge, 0);
    assert!(path.subpaths[0].edge_count > 8);
    for segment in &path.segments {
        assert!(segment.source.start < segment.source.end);
        assert_eq!(
            segment.number_count,
            if segment.command.to_ascii_uppercase() == b'Z' {
                0
            } else {
                path.numbers[segment.first_number..segment.first_number + segment.number_count]
                    .len()
            }
        );
    }
    assert_eq!(path.numbers.len(), 29);
}

#[test]
fn implicit_repeats_move_to_lines_and_subpath_closure_match_reference() {
    let path = parse(b"M1 1 4 1 4 4 M8 8 l2 0 z");
    assert_eq!(
        path.segments
            .iter()
            .map(|segment| segment.command)
            .collect::<Vec<_>>(),
        b"MLLMlz"
    );
    assert_eq!(path.segments[0].command_offset, Some(0));
    assert_eq!(path.segments[1].command_offset, None);
    assert_eq!(path.subpaths.len(), 2);
    assert!(!path.subpaths[0].closed);
    assert!(path.subpaths[1].closed);
    assert_eq!(path.subpaths[0].edge_count, 3);
    assert_eq!(path.subpaths[1].edge_count, 2);
    assert_eq!(path.edges[2].a, Point { x: 4.0, y: 4.0 });
    assert_eq!(path.edges[2].b, Point { x: 1.0, y: 1.0 });
}

#[test]
fn smooth_reflection_and_reset_rules_have_exact_control_geometry() {
    let cubic = parse(b"M0 0 C1 0 2 0 3 0 S5 1 6 0 L7 0 S8 1 9 0");
    let first_cubic = cubic
        .edges
        .iter()
        .position(|edge| edge.a == Point { x: 3.0, y: 0.0 })
        .unwrap();
    assert_eq!(cubic.edges[first_cubic].b, Point { x: 4.5, y: 0.375 });
    let reset_start = cubic
        .edges
        .iter()
        .position(|edge| edge.a == Point { x: 7.0, y: 0.0 })
        .unwrap();
    assert_eq!(cubic.edges[reset_start].b, Point { x: 7.625, y: 0.375 });

    let quad = parse(b"M0 0 Q2 2 4 0 T8 0 L9 0 T11 0");
    let reflected_start = quad
        .edges
        .iter()
        .position(|edge| edge.a == Point { x: 4.0, y: 0.0 })
        .unwrap();
    assert!(quad.edges[reflected_start].b.y < 0.0);
    let reflected_segment = &quad.segments[2];
    let reflected_end = reflected_segment.first_edge + reflected_segment.edge_count - 1;
    assert_eq!(quad.edges[reflected_end].b, Point { x: 8.0, y: 0.0 });
    let reset_start = quad
        .edges
        .iter()
        .position(|edge| edge.a == Point { x: 9.0, y: 0.0 })
        .unwrap();
    assert_eq!(quad.edges[reset_start].a, Point { x: 9.0, y: 0.0 });
}

#[test]
fn compact_numbers_and_arc_flags_keep_exact_source_ranges() {
    let source = b"prefix M-.4.6A2 3 0 011.2.4 suffix";
    let start = 7;
    let end = source.len() - 7;
    let path = parse_path(
        source,
        Span { start, end },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(path.numbers.len(), 9);
    let literals: [&[u8]; 9] = [b"-.4", b".6", b"2", b"3", b"0", b"0", b"1", b"1.2", b".4"];
    let mut search = start + 1;
    for (number, literal) in path.numbers.iter().zip(literals) {
        let relative = source[search..end]
            .windows(literal.len())
            .position(|window| window == literal)
            .unwrap();
        let expected_offset = search + relative;
        assert_eq!(number.source.start, expected_offset);
        assert_eq!(&source[number.source.start..number.source.end], literal);
        search = number.source.end;
    }
    close(path.numbers[0].value, -0.4);
    close(path.numbers[1].value, 0.6);
    assert_eq!(
        path.segments[0].source,
        Span {
            start,
            end: start + 6
        }
    );
    assert_eq!(path.segments[1].command_offset, Some(start + 6));
}

#[test]
fn arcs_cover_degenerate_same_end_corrected_and_flag_validation_cases() {
    let zero_radius = parse(b"M0 0 A0 3 0 0 1 4 5");
    assert_eq!(zero_radius.edges.len(), 2);
    assert_eq!(zero_radius.edges[0].b, Point { x: 4.0, y: 5.0 });
    let same_point = parse(b"M2 2 A3 4 45 1 1 2 2");
    assert_eq!(same_point.edges.len(), 1);
    let corrected = parse(b"M0 0 A1 1 0 0 1 10 0");
    assert!(corrected.edges.len() >= 2);
    assert_eq!(
        corrected.edges[corrected.edges.len() - 2].b,
        Point { x: 10.0, y: 0.0 }
    );
    assert_eq!(corrected.edges.last().unwrap().b, Point { x: 0.0, y: 0.0 });
    for data in [
        b"M0 0 A1 1 0 2 0 4 4".as_slice(),
        b"M0 0 A-1 1 0 0 0 4 4",
        b"M0 0 A1 1 0 0.0 0 4 4",
    ] {
        let error = parse_path(
            data,
            Span {
                start: 0,
                end: data.len(),
            },
            affine_identity(),
            ParseOptions::default(),
        )
        .unwrap_err();
        assert_eq!(error.kind, DiagnosticKind::Path);
        assert_eq!(error.offset, 6);
    }
}

#[test]
fn transform_changes_edge_coordinates_and_flatness() {
    let data = b"M0 0 C0 10 10 10 10 0";
    let identity = parse(data);
    let scale = Affine {
        a: 4.0,
        d: 4.0,
        ..affine_identity()
    };
    let scaled = parse_path(
        data,
        Span {
            start: 0,
            end: data.len(),
        },
        scale,
        ParseOptions::default(),
    )
    .unwrap();
    assert!(scaled.edges.len() > identity.edges.len());
    assert_eq!(scaled.edges.first().unwrap().a, Point { x: 0.0, y: 0.0 });
    assert_eq!(
        scaled.edges[scaled.edges.len() - 2].b,
        Point { x: 40.0, y: 0.0 }
    );
    let translated = Affine {
        e: 12.0,
        f: -5.0,
        ..affine_identity()
    };
    let line = parse_path(
        b"M1 2L3 4",
        Span { start: 0, end: 8 },
        translated,
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(line.edges[0].a, Point { x: 13.0, y: -3.0 });
    assert_eq!(line.edges[0].b, Point { x: 15.0, y: -1.0 });
}

#[test]
fn malformed_grammar_and_numeric_failures_report_absolute_original_offsets() {
    let source = b"long source: M0 0 S1 2 3";
    let error = parse_path(
        source,
        Span {
            start: 13,
            end: source.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Path);
    assert_eq!(error.offset, 24);
    let mut malformed = vec![b' '; 101];
    malformed.extend_from_slice(b"M.4..6");
    let error = parse_path(
        &malformed,
        Span {
            start: 101,
            end: 107,
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Path);
    assert_eq!(error.offset, 102);
    let overflow = b"M999999 0 l2 0";
    let error = parse_path(
        overflow,
        Span {
            start: 0,
            end: overflow.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Number);
    assert_eq!(error.offset, 11);
    let bad_transform = Affine {
        a: f64::NAN,
        ..affine_identity()
    };
    let mut prefixed = vec![b' '; 20];
    prefixed.extend_from_slice(b"M0 0");
    let error = parse_path(
        &prefixed,
        Span { start: 20, end: 24 },
        bad_transform,
        ParseOptions::default(),
    )
    .unwrap_err();
    assert_eq!(
        error,
        vexel_svg_path::Diagnostic {
            kind: DiagnosticKind::InvalidTransform,
            offset: 20
        }
    );
}

#[test]
fn path_span_validation_and_hard_limits_are_explicit() {
    assert_eq!(
        parse_path(
            b"abc",
            Span { start: 2, end: 4 },
            affine_identity(),
            ParseOptions::default()
        )
        .unwrap_err()
        .kind,
        DiagnosticKind::InvalidSpan
    );
    let options = ParseOptions {
        max_commands: 1,
        ..ParseOptions::default()
    };
    let error = parse_path(
        b"M0 0L1 1",
        Span { start: 0, end: 8 },
        affine_identity(),
        options,
    )
    .unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Limit);
    assert_eq!(error.offset, 5);
    let options = ParseOptions {
        max_edges: 1,
        ..ParseOptions::default()
    };
    let error = parse_path(
        b"M0 0L1 1",
        Span { start: 0, end: 8 },
        affine_identity(),
        options,
    )
    .unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Limit);
    assert_eq!(error.offset, 5);
    let options = ParseOptions {
        max_numbers: 1,
        ..ParseOptions::default()
    };
    let error = parse_path(
        b"M0 0",
        Span { start: 0, end: 4 },
        affine_identity(),
        options,
    )
    .unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Limit);
    assert_eq!(error.offset, 0);
    for (data, offset, kind) in [
        (b"L0 0".as_slice(), 0, DiagnosticKind::Path),
        (b"M0 0X".as_slice(), 4, DiagnosticKind::Unsupported),
    ] {
        let error = parse_path(
            data,
            Span {
                start: 0,
                end: data.len(),
            },
            affine_identity(),
            ParseOptions::default(),
        )
        .unwrap_err();
        assert_eq!(error.offset, offset);
        assert_eq!(error.kind, kind);
    }
    assert_eq!(ParseOptions::default().max_commands, MAX_PATH_COMMANDS);
    assert_eq!(ParseOptions::default().max_edges, MAX_PATH_EDGES);
    assert_eq!(ParseOptions::default().max_numbers, MAX_PATH_NUMBERS);
}

#[test]
fn full_container_can_exceed_values_crate_source_cap() {
    let mut source = vec![b' '; 16 * 1024 * 1024 + 32];
    let start = 16 * 1024 * 1024 + 8;
    source[start..start + 8].copy_from_slice(b"M1 2L3 4");
    let path = parse_path(
        &source,
        Span {
            start,
            end: start + 8,
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(path.edges.len(), 2);
    assert_eq!(path.numbers[0].source.start, start + 1);
}

#[test]
fn cubic_wrapper_matches_parser_for_transforms_and_degenerate_curves() {
    let points = [
        Point { x: 1.0, y: 2.0 },
        Point { x: 2.0, y: 9.0 },
        Point { x: 8.0, y: -1.0 },
        Point { x: 10.0, y: 3.0 },
    ];
    for transform in [
        affine_identity(),
        Affine {
            a: 3.0,
            d: 3.0,
            ..affine_identity()
        },
        Affine {
            a: 1.0,
            b: 0.5,
            c: -0.75,
            d: 1.0,
            ..affine_identity()
        },
    ] {
        let source = b"M1 2 C2 9 8 -1 10 3";
        let document = parse_path(
            source,
            Span {
                start: 0,
                end: source.len(),
            },
            transform,
            ParseOptions::default(),
        )
        .unwrap();
        let segment = &document.segments[1];
        let expected = &document.edges[segment.first_edge..segment.first_edge + segment.edge_count];
        let mut actual = Vec::new();
        flatten_cubic(points, transform, ParseOptions::default(), 71, &mut actual).unwrap();
        assert_eq!(&actual, expected);
    }
    for points in [
        [Point { x: 0.0, y: 0.0 }; 4],
        [
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 3.0, y: 0.0 },
        ],
    ] {
        let mut actual = Vec::new();
        flatten_cubic(
            points,
            affine_identity(),
            ParseOptions::default(),
            0,
            &mut actual,
        )
        .unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(
            actual[0],
            Edge {
                a: points[0],
                b: points[3]
            }
        );
    }
    assert_eq!(transformed_tolerance(affine_identity()), 0.25);
    assert!(
        transformed_tolerance(Affine {
            a: 4.0,
            d: 4.0,
            ..affine_identity()
        }) < 0.25
    );
}

#[test]
fn edge_wrapper_transforms_appends_and_observes_prefilled_cap() {
    let mut edges = vec![Edge {
        a: Point { x: -1.0, y: -1.0 },
        b: Point { x: 0.0, y: 0.0 },
    }];
    let transform = Affine {
        a: 2.0,
        d: 3.0,
        e: 4.0,
        f: -2.0,
        ..affine_identity()
    };
    append_edge(
        Point { x: 1.0, y: 2.0 },
        Point { x: 3.0, y: 4.0 },
        transform,
        2,
        19,
        &mut edges,
    )
    .unwrap();
    assert_eq!(edges[0].a, Point { x: -1.0, y: -1.0 });
    assert_eq!(
        edges[1],
        Edge {
            a: Point { x: 6.0, y: 4.0 },
            b: Point { x: 10.0, y: 10.0 }
        }
    );
    assert_eq!(
        append_edge(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            transform,
            2,
            23,
            &mut edges
        ),
        Err(Diagnostic {
            kind: DiagnosticKind::Limit,
            offset: 23
        })
    );
    assert_eq!(
        append_edge(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            transform,
            MAX_PATH_EDGES + 1,
            0,
            &mut edges
        ),
        Ok(())
    );
}

#[test]
fn wrappers_validate_before_mutation_and_preserve_failure_offsets() {
    let original = Edge {
        a: Point { x: 2.0, y: 3.0 },
        b: Point { x: 4.0, y: 5.0 },
    };
    let mut edges = vec![original];
    let invalid_transform = Affine {
        a: f64::NAN,
        ..affine_identity()
    };
    assert_eq!(
        append_edge(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            invalid_transform,
            5,
            41,
            &mut edges
        ),
        Err(Diagnostic {
            kind: DiagnosticKind::InvalidTransform,
            offset: 41
        })
    );
    let invalid_points = [
        Point { x: 0.0, y: 0.0 },
        Point {
            x: f64::INFINITY,
            y: 0.0,
        },
        Point { x: 1.0, y: 1.0 },
        Point { x: 2.0, y: 0.0 },
    ];
    assert_eq!(
        flatten_cubic(
            invalid_points,
            affine_identity(),
            ParseOptions::default(),
            53,
            &mut edges
        ),
        Err(Diagnostic {
            kind: DiagnosticKind::Number,
            offset: 53
        })
    );
    let out_of_bounds = Point {
        x: MAX_COORDINATE.next_up(),
        y: 0.0,
    };
    assert_eq!(
        append_edge(
            out_of_bounds,
            Point { x: 0.0, y: 0.0 },
            affine_identity(),
            5,
            61,
            &mut edges
        ),
        Err(Diagnostic {
            kind: DiagnosticKind::Number,
            offset: 61
        })
    );
    assert_eq!(edges, vec![original]);
    let points = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 0.0, y: 100.0 },
        Point { x: 100.0, y: 100.0 },
        Point { x: 100.0, y: 0.0 },
    ];
    assert_eq!(
        flatten_cubic(
            points,
            affine_identity(),
            ParseOptions {
                max_edges: 0,
                ..ParseOptions::default()
            },
            79,
            &mut edges
        ),
        Err(Diagnostic {
            kind: DiagnosticKind::Limit,
            offset: 79
        })
    );
    assert_eq!(edges, vec![original]);
}

#[test]
fn controlled_cubic_matches_legacy_geometry_and_obeys_work_limit() {
    let points = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 0.0, y: 100.0 },
        Point { x: 100.0, y: 100.0 },
        Point { x: 100.0, y: 0.0 },
    ];
    let mut expected = Vec::new();
    flatten_cubic(
        points,
        affine_identity(),
        ParseOptions::default(),
        17,
        &mut expected,
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut actual = Vec::new();
    vexel_svg_path::flatten_cubic_controlled(
        points,
        affine_identity(),
        ParseOptions::default(),
        17,
        &mut actual,
        &mut control,
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert!(control.stats().total_units() > 1);

    let mut limited = BudgetControl::new(&cancel, 0);
    let mut untouched = vec![Edge {
        a: Point { x: -1.0, y: 0.0 },
        b: Point { x: 0.0, y: 0.0 },
    }];
    let original = untouched.clone();
    assert_eq!(
        vexel_svg_path::flatten_cubic_controlled(
            points,
            affine_identity(),
            ParseOptions::default(),
            29,
            &mut untouched,
            &mut limited
        ),
        Err(ControlledError::Limit),
    );
    assert_eq!(untouched, original);
}
