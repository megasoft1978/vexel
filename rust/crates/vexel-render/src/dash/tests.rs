//! Expander unit tests: pattern normalisation, offset, restart, seam, dots.

use super::*;

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn pattern(values: &[f64], offset: f64) -> Pattern {
    Pattern::new(values, offset)
        .expect("valid")
        .expect("dashed")
}

/// Expands one range and returns its segments and joins.
fn expand(
    pattern: &Pattern,
    cap: Linecap,
    edges: &[(Point, Point)],
    seam: bool,
) -> (Vec<Segment>, Vec<Join>) {
    let (mut segments, mut joins) = (Vec::new(), Vec::new());
    build_range(
        &mut segments,
        &mut joins,
        &mut Budget::default(),
        pattern,
        cap,
        edges,
        seam,
    )
    .expect("expand");
    (segments, joins)
}

fn line(length: f64) -> Vec<(Point, Point)> {
    vec![(p(0.0, 0.0), p(length, 0.0))]
}

fn spans(segments: &[Segment]) -> Vec<(f64, f64)> {
    segments.iter().map(|s| (s.a.x, s.b.x)).collect()
}

fn assert_spans(actual: &[Segment], expected: &[(f64, f64)]) {
    let got = spans(actual);
    assert_eq!(got.len(), expected.len(), "{got:?}");
    for (g, e) in got.iter().zip(expected) {
        assert!(
            (g.0 - e.0).abs() < 1e-9 && (g.1 - e.1).abs() < 1e-9,
            "{got:?} vs {expected:?}"
        );
    }
}

#[test]
fn odd_counts_repeat() {
    let single = pattern(&[3.0], 0.0);
    assert_eq!(single.expanded(), 2);
    assert_eq!((single.value(0), single.value(1)), (3.0, 3.0));
    let triple = pattern(&[1.0, 2.0, 3.0], 0.0);
    assert_eq!(triple.expanded(), 6);
    let values: Vec<f64> = (0..6).map(|i| triple.value(i)).collect();
    assert_eq!(values, [1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
    let (segments, _) = expand(&single, Linecap::Butt, &line(12.0), false);
    assert_spans(&segments, &[(0.0, 3.0), (6.0, 9.0)]);
    // 1 on, 2 off, 3 on, 1 off, 2 on, 3 off, 1 on.
    let (segments, _) = expand(&triple, Linecap::Butt, &line(13.0), false);
    assert_spans(
        &segments,
        &[(0.0, 1.0), (3.0, 6.0), (7.0, 9.0), (12.0, 13.0)],
    );
}

#[test]
fn offset_is_taken_modulo_the_period() {
    let cases = [
        (0.0, vec![(0.0, 3.0), (6.0, 9.0)]),
        (2.0, vec![(0.0, 1.0), (4.0, 7.0), (10.0, 12.0)]),
        (-1.0, vec![(1.0, 4.0), (7.0, 10.0)]),
        // 13 mod 6 = 1.
        (13.0, vec![(0.0, 2.0), (5.0, 8.0), (11.0, 12.0)]),
        // -13 mod 6 = -1.
        (-13.0, vec![(1.0, 4.0), (7.0, 10.0)]),
        (600.0, vec![(0.0, 3.0), (6.0, 9.0)]),
    ];
    for (offset, expected) in cases {
        let (segments, _) = expand(&pattern(&[3.0], offset), Linecap::Butt, &line(12.0), false);
        assert_spans(&segments, &expected);
    }
}

#[test]
fn zero_negative_and_nan_entries_mean_solid() {
    assert_eq!(Pattern::new(&[], 0.0), Ok(None));
    assert_eq!(Pattern::new(&[0.0], 0.0), Ok(None));
    assert_eq!(Pattern::new(&[0.0, 0.0, 0.0], 0.0), Ok(None));
    assert_eq!(Pattern::new(&[-1.0, 2.0], 0.0), Ok(None));
    assert_eq!(Pattern::new(&[2.0, -0.5], 0.0), Ok(None));
    assert_eq!(Pattern::new(&[f64::NAN], 0.0), Ok(None));
    assert_eq!(Pattern::new(&[1.0, f64::INFINITY], 0.0), Ok(None));
    assert_eq!(Pattern::new(&[1.0, 2.0], f64::NAN), Ok(None));
    assert!(matches!(Pattern::new(&[0.0, 4.0], 0.0), Ok(Some(_))));
}

#[test]
fn more_than_sixteen_values_is_a_limit_error() {
    assert!(Pattern::new(&[1.0; 16], 0.0).unwrap().is_some());
    assert_eq!(
        Pattern::new(&[1.0; 17], 0.0),
        Err(RenderError::LimitExceeded)
    );
}

#[test]
fn dashes_restart_per_subpath() {
    let dashes = pattern(&[3.0, 2.0], 0.0);
    let first = vec![(p(0.0, 0.0), p(7.0, 0.0))];
    let second = vec![(p(0.0, 10.0), p(7.0, 10.0))];
    let (mut segments, mut joins) = (Vec::new(), Vec::new());
    let mut budget = Budget::default();
    for edges in [&first, &second] {
        build_range(
            &mut segments,
            &mut joins,
            &mut budget,
            &dashes,
            Linecap::Butt,
            edges,
            false,
        )
        .unwrap();
    }
    let rows: Vec<(f64, f64, f64)> = segments.iter().map(|s| (s.a.x, s.b.x, s.a.y)).collect();
    assert_eq!(
        rows,
        [
            (0.0, 3.0, 0.0),
            (5.0, 7.0, 0.0),
            (0.0, 3.0, 10.0),
            (5.0, 7.0, 10.0)
        ]
    );
}

fn square() -> Vec<(Point, Point)> {
    let corners = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)];
    (0..4).map(|i| (corners[i], corners[(i + 1) % 4])).collect()
}

#[test]
fn closed_rect_joins_the_seam_when_on_at_both_ends() {
    let edges = square();
    // Perimeter 40 = 2 periods of 17 + 6: the last dash is on at the seam.
    let joining = pattern(&[15.0, 2.0], 0.0);
    assert!(joins_seam(&joining, &edges));
    // 10 on / 10 off ends in an off interval.
    let separate = pattern(&[10.0, 10.0], 0.0);
    assert!(!joins_seam(&separate, &edges));
    let (segments, joins) = expand(&joining, Linecap::Round, &edges, true);
    let first = segments.first().unwrap();
    let last = segments.last().unwrap();
    assert_eq!(first.start_cap, Linecap::Butt, "seam start takes a join");
    assert_eq!(first.end_cap, Linecap::Butt, "dash crosses the corner");
    assert_eq!(last.end_cap, Linecap::Butt, "seam end takes a join");
    let seam = joins.last().unwrap();
    assert_eq!((seam.vertex.x, seam.vertex.y), (0.0, 0.0));
    assert_eq!((seam.next.x, seam.next.y), (10.0, 0.0));
    let (segments, joins) = expand(&joining, Linecap::Round, &edges, false);
    assert_eq!(segments.first().unwrap().start_cap, Linecap::Round);
    assert_eq!(segments.last().unwrap().end_cap, Linecap::Round);
    assert!(
        joins
            .iter()
            .all(|j| !(j.vertex.x == 0.0 && j.vertex.y == 0.0)),
        "no seam join"
    );
}

#[test]
fn zero_length_on_interval_is_a_dot_drawn_only_with_a_cap() {
    let dots = pattern(&[0.0, 6.0], 0.0);
    let (round, _) = expand(&dots, Linecap::Round, &line(12.0), false);
    assert_spans(&round, &[(0.0, 0.0), (6.0, 6.0), (12.0, 12.0)]);
    assert!(round.iter().all(|s| s.start_cap == Linecap::Round));
    let (square, _) = expand(&dots, Linecap::Square, &line(12.0), false);
    assert_eq!(square.len(), 3);
    let (butt, joins) = expand(&dots, Linecap::Butt, &line(12.0), false);
    assert!(butt.is_empty() && joins.is_empty());
    // Offset -4 is phase 2 of the 6-unit period: the first dot is 4 away.
    let (shifted, _) = expand(
        &pattern(&[0.0, 6.0], -4.0),
        Linecap::Round,
        &line(12.0),
        false,
    );
    assert_spans(&shifted, &[(4.0, 4.0), (10.0, 10.0)]);
    // Offset 4 is phase 4: the first dot is 2 away.
    let (shifted, _) = expand(
        &pattern(&[0.0, 6.0], 4.0),
        Linecap::Round,
        &line(12.0),
        false,
    );
    assert_spans(&shifted, &[(2.0, 2.0), (8.0, 8.0)]);
}

#[test]
fn step_and_byte_budgets_are_enforced() {
    let mut budget = Budget {
        steps: MAX_STROKE_STEPS,
        bytes: 0,
    };
    assert_eq!(budget.step(1), Err(RenderError::LimitExceeded));
    let mut budget = Budget {
        steps: 0,
        bytes: MAX_STROKE_BYTES - SEGMENT_BYTES,
    };
    assert_eq!(budget.primitives(1, 0), Ok(()));
    assert_eq!(budget.primitives(1, 0), Err(RenderError::LimitExceeded));
    // A tiny period over a long line exceeds the step budget instead of hanging.
    let tiny = pattern(&[0.001, 0.001], 0.0);
    let (mut segments, mut joins) = (Vec::new(), Vec::new());
    let result = build_range(
        &mut segments,
        &mut joins,
        &mut Budget::default(),
        &tiny,
        Linecap::Butt,
        &line(100_000.0),
        false,
    );
    assert_eq!(result, Err(RenderError::LimitExceeded));
}

// WO-RUST-LIMITS-03 boundary pins, written before the constants were raised.
// The literals are the frozen targets; the constants must equal them.
#[test]
fn limit03_step_budget_admits_6m_and_refuses_the_next() {
    assert_eq!(MAX_STROKE_STEPS, 6_000_000);
    let mut budget = Budget { steps: 0, bytes: 0 };
    assert_eq!(budget.step(6_000_000), Ok(()));
    assert_eq!(budget.steps, 6_000_000);
    assert_eq!(budget.step(1), Err(RenderError::LimitExceeded));
}

#[test]
fn limit03_byte_budget_admits_192_mib_and_refuses_the_next() {
    assert_eq!(MAX_STROKE_BYTES, 192 * 1024 * 1024);
    // 4,194,304 joins of JOIN_BYTES (48) are exactly 192 MiB.
    let mut budget = Budget { steps: 0, bytes: 0 };
    assert_eq!(budget.primitives(0, 4_194_304), Ok(()));
    assert_eq!(budget.bytes, 192 * 1024 * 1024);
    assert_eq!(budget.primitives(0, 1), Err(RenderError::LimitExceeded));
}

#[test]
fn limit03_refused_primitive_allocates_nothing() {
    // The byte charge precedes the reservation: a refused push leaves capacity 0.
    let seg = Segment {
        a: p(0.0, 0.0),
        b: p(1.0, 0.0),
        start_cap: Linecap::Butt,
        end_cap: Linecap::Butt,
    };
    let join = Join {
        previous: p(0.0, 0.0),
        vertex: p(1.0, 0.0),
        next: p(1.0, 1.0),
    };
    let mut segments = Vec::new();
    let mut joins = Vec::new();
    let mut budget = Budget {
        steps: 0,
        bytes: MAX_STROKE_BYTES,
    };
    assert_eq!(
        push_segment(&mut segments, &mut budget, seg),
        Err(RenderError::LimitExceeded)
    );
    assert_eq!(
        push_join(&mut joins, &mut budget, join),
        Err(RenderError::LimitExceeded)
    );
    assert_eq!(segments.capacity(), 0);
    assert_eq!(joins.capacity(), 0);
}

#[test]
fn limit03_one_edge_range_costs_five_steps() {
    // Pattern [64, 64] over one 128-unit edge: pattern (2) + edge (1) + two dashes (2).
    let pat = pattern(&[64.0, 64.0], 0.0);
    let mut budget = Budget { steps: 0, bytes: 0 };
    let (mut segments, mut joins) = (Vec::new(), Vec::new());
    let edges = [(p(0.0, 0.0), p(128.0, 0.0))];
    build_range(
        &mut segments,
        &mut joins,
        &mut budget,
        &pat,
        Linecap::Butt,
        &edges,
        false,
    )
    .unwrap();
    assert_eq!(budget.steps, 5);
}

#[test]
fn limit03_record_sizes_match_the_budget_constants() {
    assert_eq!(std::mem::size_of::<Segment>(), SEGMENT_BYTES);
    assert_eq!(std::mem::size_of::<Join>(), JOIN_BYTES);
}
