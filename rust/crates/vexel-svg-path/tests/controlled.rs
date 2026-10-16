use std::sync::atomic::{AtomicBool, Ordering};
use vexel_base::{
    BudgetControl, ControlledError, ControlledStep, Span, Stop, WorkControl, WorkKind, WorkLimits,
};
use vexel_geometry::{Affine, Edge, Point, affine_identity};
use vexel_svg_path::{
    Diagnostic, DiagnosticKind, MAX_COORDINATE, ParseOptions, PathDocument, PathParseSession,
    flatten_cubic, flatten_cubic_controlled, parse_path, parse_path_controlled,
};

fn span(source: &[u8]) -> Span {
    Span {
        start: 0,
        end: source.len(),
    }
}
fn drive(source: &[u8], quantum: u64) -> (PathDocument, u64, u64, u64, usize) {
    let mut session = PathParseSession::new(
        source,
        span(source),
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut pending = 0;
    loop {
        let before = control.stats().total_units();
        let result = session.step(quantum, &mut control);
        let actual = control.stats().total_units() - before;
        assert!(actual <= quantum);
        match result {
            ControlledStep::Pending { charged_units } => {
                assert_eq!(charged_units, actual);
                assert!(actual > 0);
                pending += 1;
            }
            ControlledStep::Complete(()) => break,
            ControlledStep::Failed(error) => panic!("{error:?}"),
        }
    }
    (
        session.finish().unwrap(),
        control.stats().units(WorkKind::SourceRead),
        control.stats().units(WorkKind::Expansion),
        control.stats().units(WorkKind::Edge),
        pending,
    )
}

#[test]
fn long_significand_fraction_exponent_whitespace_and_comma_neighbors_really_yield() {
    let mut literal = vec![b'0'; 5000];
    literal.extend_from_slice(b"1.23456789");
    literal.extend(vec![b'0'; 5000]);
    literal.extend_from_slice(b"e-");
    literal.extend(vec![b'0'; 5000]);
    literal.push(b'0');
    let mut source = b"M".to_vec();
    source.extend_from_slice(&literal);
    source.extend(vec![b' '; 10000]);
    source.extend_from_slice(b"0 L1");
    source.extend(vec![b' '; 5000]);
    source.push(b',');
    source.extend(vec![b' '; 5000]);
    source.push(b'2');
    let (path, reads, _, edges, pending) = drive(&source, 7);
    let expected = vexel_svg_values::parse_number(&literal, span(&literal)).unwrap();
    assert_eq!(path.numbers[0].value.to_bits(), expected.to_bits());
    assert_eq!(
        path.numbers[0].source,
        Span {
            start: 1,
            end: literal.len() + 1
        }
    );
    assert_eq!(path.edges.len(), 2);
    assert_eq!(edges, 2);
    assert!(pending > source.len() / 7);
    assert!(reads >= source.len() as u64 * 2);
    assert!(
        reads < source.len() as u64 * 4 + 100,
        "unexpected whole-input rescans: {reads}"
    );
    println!(
        "METRIC long-number-whitespace: bytes={} SourceRead={reads} Edge={edges} Pending={pending} quantum=7",
        source.len()
    );
}

#[test]
fn long_valid_comma_stream_has_linear_visits_and_complete_metadata() {
    let mut source = b"M0,0".to_vec();
    for _ in 0..2000 {
        source.extend_from_slice(b" l1 , \t 0");
    }
    let (path, reads, _, edges, pending) = drive(&source, 3);
    assert_eq!(path.segments.len(), 2001);
    assert_eq!(path.numbers.len(), 4002);
    assert_eq!(path.edges.len(), 2001);
    assert_eq!(edges, 2001);
    assert!(pending > source.len() / 3);
    assert!(reads < source.len() as u64 * 5 + 100);
    assert_eq!(path.edges[1999].b, Point { x: 2000.0, y: 0.0 });
    println!(
        "METRIC comma-stream: bytes={} SourceRead={reads} Edge={edges} Pending={pending} quantum=3",
        source.len()
    );
}

#[test]
fn highly_subdivided_cubic_and_arc_pause_at_real_nodes_samples_and_edges() {
    for source in [
        b"M0 0 C0 900000 900000 900000 900000 0".as_slice(),
        b"M-900000 0 A900000 900000 0 1 1 900000 0",
    ] {
        let expected = parse_path(
            source,
            span(source),
            affine_identity(),
            ParseOptions::default(),
        )
        .unwrap();
        let (path, reads, expansion, edges, pending) = drive(source, 1);
        assert_eq!(path, expected);
        let segment_edges = path.segments[1].edge_count as u64;
        assert!(segment_edges > 1000);
        assert_eq!(edges, segment_edges + 1);
        assert_eq!(
            expansion,
            if source.contains(&b'C') {
                2 * segment_edges - 1
            } else {
                segment_edges
            }
        );
        assert!(pending as u64 >= edges + expansion);
        assert!(reads < source.len() as u64 * 4 + 100);
        println!(
            "METRIC {}: bytes={} SourceRead={reads} Expansion={expansion} Edge={edges} Pending={pending} quantum=1 segment_edges={segment_edges}",
            if source.contains(&b'C') {
                "cubic"
            } else {
                "arc"
            },
            source.len()
        );
    }
}

#[test]
fn sessions_borrow_only_the_selected_source_and_outlive_temporary_controls() {
    let mut source = vec![b' '; 16 * 1024 * 1024 + 32];
    let start = 16 * 1024 * 1024 + 8;
    source[start..start + 8].copy_from_slice(b"M1 2L3 4");
    let mut session = PathParseSession::new(
        &source,
        Span {
            start,
            end: start + 8,
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let mut total = 0;
    loop {
        // The policy borrow ends here on every step; it is not the source borrow.
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let result = session.step(1, &mut control);
        total += control.stats().total_units();
        if result == ControlledStep::Complete(()) {
            break;
        }
        assert_eq!(result, ControlledStep::Pending { charged_units: 1 });
    }
    let path = session.finish().unwrap();
    assert_eq!(path.edges.len(), 2);
    assert_eq!(path.numbers[0].source.start, start + 1);
    assert!(total < 100);
    let mut abandoned = PathParseSession::new(
        &source,
        Span {
            start,
            end: start + 8,
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    assert!(matches!(
        abandoned.step(12, &mut control),
        ControlledStep::Pending { .. }
    ));
    drop(abandoned);
    source[start] = b'm'; // borrow released after dropping active owned state
    assert_eq!(source[start], b'm');
}

#[test]
fn zero_quantum_early_finish_and_terminal_results_preserve_first_failure() {
    let source = b"M0 0L1 1";
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut session = PathParseSession::new(
        source,
        span(source),
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    for _ in 0..20 {
        assert_eq!(
            session.step(0, &mut control),
            ControlledStep::Pending { charged_units: 0 }
        );
    }
    assert_eq!(control.stats().total_units(), 0);
    assert_eq!(
        session.finish(),
        Err(ControlledError::Original(Diagnostic {
            kind: DiagnosticKind::Path,
            offset: 0
        }))
    );

    let mut session = PathParseSession::new(
        source,
        span(source),
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    cancel.store(true, Ordering::Release);
    assert_eq!(
        session.step(0, &mut control),
        ControlledStep::Failed(ControlledError::Canceled)
    );
    cancel.store(false, Ordering::Release);
    let mut fresh = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        session.step(100, &mut fresh),
        ControlledStep::Failed(ControlledError::Canceled)
    );
    assert_eq!(fresh.stats().total_units(), 0);
    assert_eq!(session.finish(), Err(ControlledError::Canceled));

    let bad = b"X ,";
    let mut session =
        PathParseSession::new(bad, span(bad), affine_identity(), ParseOptions::default()).unwrap();
    let expected = ControlledError::Original(Diagnostic {
        kind: DiagnosticKind::Path,
        offset: 2,
    });
    assert_eq!(
        session.step(256, &mut fresh),
        ControlledStep::Failed(expected)
    );
    let before = fresh.stats();
    cancel.store(true, Ordering::Release);
    assert_eq!(
        session.step(1, &mut fresh),
        ControlledStep::Failed(expected)
    );
    assert_eq!(fresh.stats(), before);
    assert_eq!(session.finish(), Err(expected));
}

#[test]
fn accepted_base_counter_overflow_is_typed_sticky_and_never_rolled_back() {
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    control.charge(WorkKind::SourceRead, u64::MAX).unwrap();
    let source = b"M0 0";
    let mut session = PathParseSession::new(
        source,
        span(source),
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let before = control.stats();
    assert_eq!(
        session.step(1, &mut control),
        ControlledStep::Failed(ControlledError::Overflow)
    );
    assert_eq!(control.stats(), before);
    let mut fresh = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        session.step(0, &mut fresh),
        ControlledStep::Failed(ControlledError::Overflow)
    );
    assert_eq!(fresh.stats().total_units(), 0);
    assert_eq!(session.finish(), Err(ControlledError::Overflow));
}

#[test]
fn controlled_cubic_keeps_original_wrapper_transforms_degeneracy_and_partial_appends() {
    let points = [
        Point { x: 1.0, y: 2.0 },
        Point { x: 2.0, y: 9.0 },
        Point { x: 8.0, y: -1.0 },
        Point { x: 10.0, y: 3.0 },
    ];
    let cancel = AtomicBool::new(false);
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
        let path = parse_path(source, span(source), transform, ParseOptions::default()).unwrap();
        let segment = path.segments[1];
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut out = Vec::new();
        flatten_cubic_controlled(
            points,
            transform,
            ParseOptions::default(),
            71,
            &mut out,
            &mut control,
        )
        .unwrap();
        assert_eq!(
            &out,
            &path.edges[segment.first_edge..segment.first_edge + segment.edge_count]
        );
        assert_eq!(
            control.stats().units(WorkKind::Expansion),
            2 * out.len() as u64 - 1
        );
        assert_eq!(control.stats().units(WorkKind::Edge), out.len() as u64);
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
        let mut out = Vec::new();
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        flatten_cubic_controlled(
            points,
            affine_identity(),
            ParseOptions::default(),
            0,
            &mut out,
            &mut control,
        )
        .unwrap();
        assert_eq!(
            out,
            vec![Edge {
                a: points[0],
                b: points[3]
            }]
        );
    }
    let mut full = Vec::new();
    flatten_cubic(
        points,
        affine_identity(),
        ParseOptions::default(),
        79,
        &mut full,
    )
    .unwrap();
    let sentinel = Edge {
        a: Point { x: -1.0, y: 0.0 },
        b: Point { x: 0.0, y: 0.0 },
    };
    let mut out = vec![sentinel];
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        flatten_cubic_controlled(
            points,
            affine_identity(),
            ParseOptions {
                max_edges: 2,
                ..ParseOptions::default()
            },
            79,
            &mut out,
            &mut control
        ),
        Err(ControlledError::Original(Diagnostic {
            kind: DiagnosticKind::Limit,
            offset: 79
        }))
    );
    assert_eq!(out, vec![sentinel, full[0]]);
    let mut out = Vec::new();
    let mut limited = BudgetControl::with_limits(
        &cancel,
        u64::MAX,
        WorkLimits::unlimited().with_limit(WorkKind::Edge, 2),
    );
    assert_eq!(
        flatten_cubic_controlled(
            points,
            affine_identity(),
            ParseOptions::default(),
            79,
            &mut out,
            &mut limited
        ),
        Err(ControlledError::Limit)
    );
    assert_eq!(out, full[..2]);
    assert_eq!(limited.stats().units(WorkKind::Edge), 2);
}

#[test]
fn cubic_validation_precedes_mutation_and_keeps_original_diagnostics() {
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let sentinel = Edge {
        a: Point { x: 2.0, y: 3.0 },
        b: Point { x: 4.0, y: 5.0 },
    };
    let mut out = vec![sentinel];
    let valid = [Point { x: 0.0, y: 0.0 }; 4];
    let transform = Affine {
        a: f64::NAN,
        ..affine_identity()
    };
    assert_eq!(
        flatten_cubic_controlled(
            valid,
            transform,
            ParseOptions::default(),
            41,
            &mut out,
            &mut control
        ),
        Err(ControlledError::Original(Diagnostic {
            kind: DiagnosticKind::InvalidTransform,
            offset: 41
        }))
    );
    for x in [f64::INFINITY, MAX_COORDINATE.next_up()] {
        let mut points = valid;
        points[1].x = x;
        assert_eq!(
            flatten_cubic_controlled(
                points,
                affine_identity(),
                ParseOptions::default(),
                53,
                &mut out,
                &mut control
            ),
            Err(ControlledError::Original(Diagnostic {
                kind: DiagnosticKind::Number,
                offset: 53
            }))
        );
    }
    assert_eq!(out, vec![sentinel]);
    assert_eq!(control.stats().total_units(), 0);
    assert_eq!(
        flatten_cubic_controlled(
            valid,
            affine_identity(),
            ParseOptions {
                max_edges: 0,
                ..ParseOptions::default()
            },
            79,
            &mut out,
            &mut control
        ),
        Err(ControlledError::Original(Diagnostic {
            kind: DiagnosticKind::Limit,
            offset: 79
        }))
    );
    assert_eq!(out, vec![sentinel]);
}

#[test]
fn original_limits_fail_without_returning_partial_documents() {
    let source = b"M0 0L1 1";
    let cancel = AtomicBool::new(false);
    for (options, offset) in [
        (
            ParseOptions {
                max_commands: 1,
                ..ParseOptions::default()
            },
            5,
        ),
        (
            ParseOptions {
                max_edges: 1,
                ..ParseOptions::default()
            },
            5,
        ),
        (
            ParseOptions {
                max_numbers: 1,
                ..ParseOptions::default()
            },
            0,
        ),
    ] {
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        assert_eq!(
            parse_path_controlled(
                source,
                span(source),
                affine_identity(),
                options,
                &mut control
            ),
            Err(ControlledError::Original(Diagnostic {
                kind: DiagnosticKind::Limit,
                offset
            }))
        );
        assert!(control.stats().total_units() > 0);
    }
}

#[test]
fn all_uppercase_commands_and_relative_repetitions_retain_authored_metadata() {
    for (source, commands) in [
        (
            b"M0 0 L1 2 H3 V4 C4 5 5 6 6 7 S8 9 10 11 Q12 13 14 15 T18 19 A2 3 30 0 1 20 21 Z"
                .as_slice(),
            b"MLHVCSQTAZ".as_slice(),
        ),
        (
            b"m1 1 2 2 -1-1 h2 3 v4 -1 c1 0 2 1 3 2 s1 2 3 4 q1 2 3 4 t2 3 a4 5 30 011 2 z",
            b"mllhhvvcsqtaz".as_slice(),
        ),
    ] {
        let expected = parse_path(
            source,
            span(source),
            affine_identity(),
            ParseOptions::default(),
        )
        .unwrap();
        let (document, ..) = drive(source, 1);
        assert_eq!(document, expected);
        assert_eq!(
            document
                .segments
                .iter()
                .map(|segment| segment.command)
                .collect::<Vec<_>>(),
            commands
        );
        assert!(document.subpaths[0].closed);
        for number in &document.numbers {
            assert!(number.source.start < number.source.end);
        }
    }
}

#[test]
fn comma_precedence_and_separator_dependent_failure_offsets_remain_exact() {
    let cancel = AtomicBool::new(false);
    let mut source = b"M".to_vec();
    source.extend(vec![b'9'; 1000]);
    source.extend_from_slice(b" 0,");
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        parse_path_controlled(
            &source,
            span(&source),
            affine_identity(),
            ParseOptions::default(),
            &mut control
        ),
        Err(ControlledError::Original(Diagnostic {
            kind: DiagnosticKind::Path,
            offset: source.len() - 1
        }))
    );
    // A trailing bad comma takes priority even over early significand overflow.
    source.pop();
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        parse_path_controlled(
            &source,
            span(&source),
            affine_identity(),
            ParseOptions::default(),
            &mut control
        ),
        Err(ControlledError::Original(Diagnostic {
            kind: DiagnosticKind::Path,
            offset: 1
        }))
    );
    for (source, offset) in [(b"M0 0 L1".as_slice(), 7), (b"M0 0 L1 ".as_slice(), 6)] {
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        assert_eq!(
            parse_path_controlled(
                source,
                span(source),
                affine_identity(),
                ParseOptions::default(),
                &mut control
            ),
            Err(ControlledError::Original(Diagnostic {
                kind: DiagnosticKind::Path,
                offset
            }))
        );
    }
}

#[test]
fn completion_is_sticky_and_dropping_partial_geometry_releases_the_source() {
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let source = b"M0 0L1 1";
    let mut session = PathParseSession::new(
        source,
        span(source),
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(
        session.step(256, &mut control),
        ControlledStep::Complete(())
    );
    let before = control.stats();
    cancel.store(true, Ordering::Release);
    assert_eq!(session.step(0, &mut control), ControlledStep::Complete(()));
    assert_eq!(
        session.step(256, &mut control),
        ControlledStep::Complete(())
    );
    assert_eq!(control.stats(), before);
    assert_eq!(session.finish().unwrap().edges.len(), 2);

    cancel.store(false, Ordering::Release);
    let mut source = b"M0 0 C0 900000 900000 900000 900000 0".to_vec();
    let mut session = PathParseSession::new(
        &source,
        span(&source),
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    while control.stats().units(WorkKind::Edge) == 0 {
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Pending { charged_units: 1 }
        );
    }
    drop(session);
    assert!(control.stats().units(WorkKind::Expansion) > 1);
    source[0] = b'm';
    assert_eq!(source[0], b'm');
}

#[test]
fn controlled_cubic_cancellation_keeps_completed_appends_and_charges() {
    struct CancelAfterEdge<'a> {
        flag: &'a AtomicBool,
        budget: BudgetControl<'a>,
    }
    impl WorkControl for CancelAfterEdge<'_> {
        fn checkpoint(&mut self) -> Result<(), Stop> {
            self.budget.checkpoint()
        }
        fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop> {
            self.budget.charge(kind, units)?;
            if kind == WorkKind::Edge {
                self.flag.store(true, Ordering::Release);
            }
            Ok(())
        }
    }
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
        31,
        &mut expected,
    )
    .unwrap();
    let flag = AtomicBool::new(false);
    let mut control = CancelAfterEdge {
        flag: &flag,
        budget: BudgetControl::new(&flag, u64::MAX),
    };
    let mut out = Vec::new();
    assert_eq!(
        flatten_cubic_controlled(
            points,
            affine_identity(),
            ParseOptions::default(),
            31,
            &mut out,
            &mut control
        ),
        Err(ControlledError::Canceled)
    );
    assert_eq!(out, expected[..1]);
    assert_eq!(control.budget.stats().units(WorkKind::Edge), 1);
    assert!(control.budget.stats().units(WorkKind::Expansion) > 1);
}
