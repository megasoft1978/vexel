use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, ControlledStep, Span};
use vexel_geometry::affine_identity;
use vexel_svg_path::{
    parse_path, parse_path_controlled, DiagnosticKind, ParseOptions, PathParseSession,
};

#[test]
fn integer_fraction_adjacency_splits_second_decimal_across_entry_points() {
    // `1.2.3` is `1.2` then `.3`; the following `4 5` is an implicit L repeat.
    let source = b"prefix:M0 0 L1.2.3 4 5:suffix";
    let start = b"prefix:".len();
    let path = b"M0 0 L1.2.3 4 5";
    let selected_span = Span {
        start,
        end: start + path.len(),
    };
    let spaced_source = b"M0 0 L1.2 .3 4 5";
    let spaced = parse_path(
        spaced_source,
        Span {
            start: 0,
            end: spaced_source.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(spaced.numbers.len(), 6);
    assert_eq!(spaced.segments.len(), 3);

    let parsed = parse_path(
        source,
        selected_span,
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(parsed.numbers.len(), 6);
    assert_eq!(parsed.edges, spaced.edges);
    assert_eq!(parsed.segments.len(), 3);

    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        parse_path_controlled(
            source,
            selected_span,
            affine_identity(),
            ParseOptions::default(),
            &mut control
        ),
        Ok(parsed.clone())
    );

    let mut session = PathParseSession::new(
        source,
        selected_span,
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    loop {
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        match session.step(2, &mut control) {
            ControlledStep::Pending { charged_units } => assert!(charged_units <= 2),
            ControlledStep::Failed(error) => panic!("valid compact decimal failed: {error:?}"),
            ControlledStep::Complete(()) => break,
        }
    }
    assert_eq!(session.finish(), Ok(parsed));

    let compact = b"M.6.5";
    let accepted = parse_path(
        compact,
        Span {
            start: 0,
            end: compact.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(accepted.numbers.len(), 2);
    assert!((accepted.numbers[0].value - 0.6).abs() < 1.0e-12);
    assert!((accepted.numbers[1].value - 0.5).abs() < 1.0e-12);

    let compact_arc = b"prefix M-.4.6A2 3 0 011.2.4 suffix";
    let arc_span = Span {
        start: 7,
        end: compact_arc.len() - 7,
    };
    assert!(parse_path(
        compact_arc,
        arc_span,
        affine_identity(),
        ParseOptions::default()
    )
    .is_ok());
}

#[test]
fn compact_fraction_boundary_is_accepted_across_entry_points() {
    for candidate in [b"M0 0 L1.2.4Z".as_slice(), b"M-.4.6L1.2.4Z"] {
        assert!(
            parse_path(
                candidate,
                Span {
                    start: 0,
                    end: candidate.len()
                },
                affine_identity(),
                ParseOptions::default()
            )
            .is_ok(),
            "candidate {candidate:?}"
        );
    }
    let document_source = b"<svg width='8' height='8'><path d='M-.4.6L1.2.4Z'/></svg>";
    let document_span = Span { start: 35, end: 48 };
    assert_eq!(
        &document_source[document_span.start..document_span.end],
        b"M-.4.6L1.2.4Z"
    );
    parse_path(
        document_source,
        document_span,
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap_or_else(|error| panic!("XML path span failed: {error:?}"));
    let document_cancel = AtomicBool::new(false);
    let mut document_control = BudgetControl::new(&document_cancel, u64::MAX);
    assert!(parse_path_controlled(
        document_source,
        document_span,
        affine_identity(),
        ParseOptions::default(),
        &mut document_control,
    )
    .is_ok());

    let source = b"prefix:M-.4.6L1.2.4Z:suffix";
    let start = b"prefix:".len();
    let selected_span = Span {
        start,
        end: start + b"M-.4.6L1.2.4Z".len(),
    };
    let parsed = parse_path(
        source,
        selected_span,
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(parsed.numbers.len(), 4);
    assert_eq!(
        parsed
            .numbers
            .iter()
            .map(|number| number.source)
            .collect::<Vec<_>>(),
        [
            Span { start: 8, end: 11 },
            Span { start: 11, end: 13 },
            Span { start: 14, end: 17 },
            Span { start: 17, end: 19 },
        ]
    );
    assert!((parsed.numbers[2].value - 1.2).abs() < 1.0e-12);
    assert!((parsed.numbers[3].value - 0.4).abs() < 1.0e-12);

    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let controlled = parse_path_controlled(
        source,
        selected_span,
        affine_identity(),
        ParseOptions::default(),
        &mut control,
    )
    .unwrap();
    assert_eq!(controlled.numbers, parsed.numbers);

    let mut session = PathParseSession::new(
        source,
        selected_span,
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    loop {
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        match session.step(2, &mut control) {
            ControlledStep::Pending { charged_units } => assert!(charged_units <= 2),
            ControlledStep::Complete(()) => break,
            ControlledStep::Failed(error) => panic!("compact operands failed: {error:?}"),
        }
    }
    let resumed = session.finish().unwrap();
    assert_eq!(resumed.numbers, parsed.numbers);
}

#[test]
fn dot_leading_double_decimal_still_fails() {
    let source = b"M.4..6";
    let error = parse_path(
        source,
        Span {
            start: 0,
            end: source.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Path);
    assert_eq!(error.offset, 1);
}

#[test]
fn compact_arc_endpoint_allows_an_implicit_repeated_arc_group() {
    let compact = b"M0 0 A2 3 0 011.2.4 2 3 0 01 4 5";
    let compact_path = parse_path(
        compact,
        Span {
            start: 0,
            end: compact.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let explicit = b"M0 0 A2 3 0 0 1 1.2 .4 A2 3 0 0 1 4 5";
    let explicit_path = parse_path(
        explicit,
        Span {
            start: 0,
            end: explicit.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(compact_path.numbers.len(), 16);
    assert_eq!(explicit_path.numbers.len(), 16);
    assert_eq!(compact_path.edges.len(), explicit_path.edges.len());

    // `A2.3.4` is radii 2.3 and .4; the following `2 3 ...` is an implicit arc.
    let compact_radius = b"M0 0 A2.3.4 0 0 1 4 5 2 3 0 0 1 6 7";
    let radius = parse_path(
        compact_radius,
        Span {
            start: 0,
            end: compact_radius.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(radius.numbers.len(), 16);
    assert!((radius.numbers[2].value - 2.3).abs() < 1.0e-12);
    assert!((radius.numbers[3].value - 0.4).abs() < 1.0e-12);
    assert_eq!(radius.segments.len(), 3);
}
