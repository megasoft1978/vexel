use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use vexel_base::BudgetControl;
use vexel_geometry::affine_identity;

const SOURCE: &[u8] = b"M0 0 C0 20 20 20 20 0 A20 20 0 0 1 40 0 M1 1 L2 2 z L3 3";
fn session(source: &[u8]) -> PathParseSession<'_> {
    PathParseSession::new(source, Span { start: 0, end: source.len() }, affine_identity(), ParseOptions::default()).unwrap()
}
type Target = fn(&PathParseSession<'_>) -> bool;
fn targets() -> Vec<(&'static str, Target)> {
    vec![
        ("validation", |s| s.phase == Phase::Validate),
        ("tolerance", |s| s.phase == Phase::Prepare),
        ("command-gap", |s| s.phase == Phase::Gap && s.reader.peek() == Some(b'C')),
        ("command", |s| s.phase == Phase::Command && s.reader.peek() == Some(b'C')),
        ("operand-separator", |s| s.phase == Phase::OperandGap && s.group.upper == b'C' && s.group.operand == 2),
        ("operand-boundary", |s| s.phase == Phase::OperandBoundary && s.group.upper == b'C' && s.group.operand == 2),
        ("operand-number", |s| s.phase == Phase::Number && s.group.upper == b'C' && s.group.operand == 2),
        ("arc-flag", |s| s.phase == Phase::Flag),
        ("metadata-reserve", |s| s.phase == Phase::RecordReserve && s.group.upper == b'C'),
        ("metadata-number", |s| s.phase == Phase::RecordNumber && s.group.upper == b'C' && s.group.metadata_index == 2),
        ("metadata-segment", |s| s.phase == Phase::RecordSegment && s.group.upper == b'C'),
        ("curve-setup", |s| s.phase == Phase::Apply && s.group.upper == b'C'),
        ("curve-node", |s| s.phase == Phase::Geometry && matches!(s.geometry, Some(GeometryCursor::Curve(_))) && s.kind() == WorkKind::Expansion),
        ("curve-append", |s| s.phase == Phase::Geometry && matches!(s.geometry, Some(GeometryCursor::Curve(_))) && s.kind() == WorkKind::Edge),
        ("arc-preparation", |s| s.phase == Phase::Apply && s.group.upper == b'A'),
        ("arc-sample", |s| s.phase == Phase::Geometry && matches!(s.geometry, Some(GeometryCursor::Arc(_))) && s.kind() == WorkKind::Expansion),
        ("arc-append", |s| s.phase == Phase::Geometry && matches!(s.geometry, Some(GeometryCursor::Arc(_))) && s.kind() == WorkKind::Edge),
        ("line-append", |s| s.phase == Phase::Geometry && matches!(s.geometry, Some(GeometryCursor::Line(..)))),
        ("complete-group", |s| s.phase == Phase::CompleteGroup && s.group.upper == b'C'),
        ("move-subpath", |s| s.phase == Phase::MoveSubpath),
        ("finish-move", |s| s.phase == Phase::FinishMove),
        ("close-subpath", |s| s.phase == Phase::CloseSubpath),
        ("finish-close", |s| s.phase == Phase::FinishClose),
        ("final-subpath", |s| s.phase == Phase::FinalSubpath),
    ]
}
fn reach<'src>(s: &mut PathParseSession<'src>, target: Target, control: &mut dyn WorkControl) {
    while !target(s) {
        assert!(matches!(s.step(1, control), ControlledStep::Pending { charged_units: 1 }), "unreached target: {:?}", s.phase);
    }
}

#[test]
fn construction_and_zero_quantum_do_not_scan_or_copy_the_selected_source() {
    let source = b"prefix X , suffix";
    let span = Span { start: 7, end: 10 };
    let transform = Affine { a: 4.0, d: 4.0, ..affine_identity() };
    let mut s = PathParseSession::new(source, span, transform, ParseOptions::default()).unwrap();
    assert_eq!(s.phase, Phase::Validate); assert_eq!(s.reader.index, 0);
    assert_eq!(s.reader.bytes.as_ptr(), source[7..10].as_ptr());
    assert_eq!(s.flatness, 0.25); // transform math is deferred too
    let before = format!("{s:?}");
    let cancel = AtomicBool::new(false); let mut control = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(s.step(0, &mut control), ControlledStep::Pending { charged_units: 0 });
    assert_eq!(format!("{s:?}"), before); assert_eq!(control.stats().total_units(), 0);
    assert_eq!(s.step(256, &mut control), ControlledStep::Failed(ControlledError::Original(failure(DiagnosticKind::Path, 9))));
    let transform = Affine { a: f64::NAN, ..affine_identity() };
    assert_eq!(PathParseSession::new(source, Span { start: 9, end: usize::MAX }, transform, ParseOptions::default()).unwrap_err(),
        ControlledError::Original(failure(DiagnosticKind::InvalidSpan, 9)));
    assert_eq!(PathParseSession::new(source, span, transform, ParseOptions::default()).unwrap_err(),
        ControlledError::Original(failure(DiagnosticKind::InvalidTransform, 7)));
}

#[test]
fn cancellation_and_limits_are_terminal_at_every_real_parser_phase() {
    for (name, target) in targets() {
        for cancel_mode in [true, false] {
            let cancel = AtomicBool::new(false);
            let mut driving = BudgetControl::new(&cancel, u64::MAX);
            let mut s = session(SOURCE); reach(&mut s, target, &mut driving);
            let before_stats = driving.stats();
            let before_doc = s.document.clone(); let before_index = s.reader.index;
            let expected = if cancel_mode { ControlledError::Canceled } else { ControlledError::Limit };
            if cancel_mode {
                cancel.store(true, Ordering::Release);
                assert_eq!(s.step(256, &mut driving), ControlledStep::Failed(expected), "{name}");
            } else {
                let mut zero_budget = BudgetControl::new(&cancel, 0);
                assert_eq!(s.step(256, &mut zero_budget), ControlledStep::Failed(expected), "{name}");
                assert_eq!(zero_budget.stats().total_units(), 0);
            }
            assert_eq!(driving.stats(), before_stats, "completed charges rolled back at {name}");
            assert_eq!(s.document, before_doc, "partial owned state changed at {name}");
            assert_eq!(s.reader.index, before_index);
            cancel.store(false, Ordering::Release);
            let mut fresh = BudgetControl::new(&cancel, u64::MAX);
            assert_eq!(s.step(1, &mut fresh), ControlledStep::Failed(expected), "{name}");
            assert_eq!(s.step(0, &mut fresh), ControlledStep::Failed(expected), "{name}");
            assert_eq!(fresh.stats().total_units(), 0);
            assert_eq!(s.finish(), Err(expected), "partial document escaped at {name}");
        }
    }
}

#[test]
fn zero_quantum_preserves_all_numeric_geometry_and_metadata_state() {
    for (name, target) in targets() {
        let cancel = AtomicBool::new(false); let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut s = session(SOURCE); reach(&mut s, target, &mut control);
        let state = format!("{s:?}"); let stats = control.stats();
        assert_eq!(s.step(0, &mut control), ControlledStep::Pending { charged_units: 0 });
        assert_eq!(format!("{s:?}"), state, "{name}"); assert_eq!(control.stats(), stats, "{name}");
    }
}

#[test]
fn cubic_and_arc_steps_pause_before_append_and_keep_reader_position() {
    let cancel = AtomicBool::new(false); let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut s = session(b"M0 0 C0 20 20 20 20 0");
    reach(&mut s, |s| s.phase == Phase::Geometry, &mut control);
    let index = s.reader.index; assert_eq!(s.document.edges.len(), 0);
    assert_eq!(s.kind(), WorkKind::Expansion);
    assert_eq!(s.step(1, &mut control), ControlledStep::Pending { charged_units: 1 });
    assert_eq!(s.document.edges.len(), 0); assert_eq!(s.reader.index, index);
    reach(&mut s, |s| s.phase == Phase::Geometry && s.kind() == WorkKind::Edge, &mut control);
    assert_eq!(s.step(1, &mut control), ControlledStep::Pending { charged_units: 1 });
    assert_eq!(s.document.edges.len(), 1); assert_eq!(s.reader.index, index);
    assert_eq!(s.document.segments[1].edge_count, 0); // metadata waits for traversal
    assert_eq!(s.finish(), Err(ControlledError::Original(failure(DiagnosticKind::Path, index))));

    let mut s = session(b"M-900000 0 A900000 900000 0 1 1 900000 0");
    reach(&mut s, |s| s.phase == Phase::Apply && s.group.upper == b'A', &mut control);
    let index = s.reader.index;
    assert_eq!(s.step(1, &mut control), ControlledStep::Pending { charged_units: 1 });
    assert_eq!(s.phase, Phase::Geometry); assert_eq!(s.kind(), WorkKind::Expansion);
    assert_eq!(s.document.edges.len(), 0);
    assert_eq!(s.step(1, &mut control), ControlledStep::Pending { charged_units: 1 });
    assert_eq!(s.kind(), WorkKind::Edge); assert_eq!(s.document.edges.len(), 0);
    assert_eq!(s.step(1, &mut control), ControlledStep::Pending { charged_units: 1 });
    assert_eq!(s.kind(), WorkKind::Expansion); assert_eq!(s.document.edges.len(), 1);
    assert_eq!(s.reader.index, index);
}

#[test]
fn command_counter_overflow_is_checked_and_typed() {
    let cancel = AtomicBool::new(false); let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut s = session(b"M0 0"); reach(&mut s, |s| s.phase == Phase::Command, &mut control);
    s.command_count = usize::MAX;
    assert_eq!(s.step(1, &mut control), ControlledStep::Failed(ControlledError::Overflow));
    assert_eq!(s.finish(), Err(ControlledError::Overflow));
}

#[test]
fn real_reservations_check_counts_bytes_and_frozen_capacity() {
    let mut out = vec![7u64];
    assert_eq!(reserve(&mut out, usize::MAX, usize::MAX, 19), Err(ControlledError::Overflow));
    assert_eq!(out, vec![7]);
    let mut out = Vec::<u64>::new();
    assert_eq!(reserve(&mut out, usize::MAX / 2, usize::MAX, 19), Err(ControlledError::Overflow));
    assert_eq!(out.capacity(), 0);
    for item in 0..3 { reserve(&mut out, 1, 3, 19).unwrap(); out.push(item); assert!(out.capacity() <= 3); }
    assert_eq!(out.capacity(), 3);
    assert_eq!(reserve(&mut out, 1, 3, 19), Err(ControlledError::Original(failure(DiagnosticKind::Limit, 19))));
    assert_eq!(out, vec![0, 1, 2]);
}

#[test]
fn original_depth_sixteen_and_transformed_tolerance_are_retained() {
    let cancel = AtomicBool::new(false); let mut control = BudgetControl::new(&cancel, u64::MAX);
    let points = [ZERO, Point { x: 0.0, y: 100.0 }, Point { x: 100.0, y: 100.0 }, Point { x: 100.0, y: 0.0 }];
    let transform = Affine { a: 1.0e12, d: 1.0e12, ..affine_identity() };
    let mut edges = Vec::new();
    assert_eq!(crate::flatten_cubic_controlled(points, transform, ParseOptions::default(), 73, &mut edges, &mut control),
        Err(ControlledError::Original(failure(DiagnosticKind::Limit, 73))));
    assert_eq!(control.stats().units(WorkKind::Expansion), 17);
    assert_eq!(control.stats().units(WorkKind::Edge), 0); assert!(edges.is_empty());
}

#[test]
fn all_frozen_option_caps_are_clamped_without_new_source_limits() {
    let s = PathParseSession::new(b"", Span { start: 0, end: 0 }, affine_identity(),
        ParseOptions { max_commands: usize::MAX, max_edges: usize::MAX, max_numbers: usize::MAX }).unwrap();
    assert_eq!(s.max_commands, MAX_PATH_COMMANDS); assert_eq!(s.max_edges, MAX_PATH_EDGES); assert_eq!(s.max_numbers, MAX_PATH_NUMBERS);
}

#[test]
fn comma_neighbors_and_each_numeric_subcursor_resume_and_stop_independently() {
    let source = b"M \t0001.2500e-0001 , \t 0";
    for (validation, stage) in [(true, "left"), (true, "right"), (true, "check"),
        (false, "sign"), (false, "integer"), (false, "fraction"), (false, "exponent"),
        (false, "exponent-sign"), (false, "exponent-probe"), (false, "exponent-digits"), (false, "finish")] {
        for canceled in [false, true] {
            let cancel = AtomicBool::new(false); let mut driving = BudgetControl::new(&cancel, u64::MAX);
            let mut s = session(source);
            while !(if validation { s.phase == Phase::Validate && s.comma.stage() == stage }
                else { s.phase == Phase::Number && s.number.stage() == stage }) {
                assert_eq!(s.step(1, &mut driving), ControlledStep::Pending { charged_units: 1 });
            }
            let snapshot = format!("{s:?}"); let before = driving.stats();
            assert_eq!(s.step(0, &mut driving), ControlledStep::Pending { charged_units: 0 });
            assert_eq!(format!("{s:?}"), snapshot); assert_eq!(driving.stats(), before);
            let expected = if canceled { ControlledError::Canceled } else { ControlledError::Limit };
            if canceled {
                cancel.store(true, Ordering::Release);
                assert_eq!(s.step(1, &mut driving), ControlledStep::Failed(expected));
            } else {
                let mut limited = BudgetControl::new(&cancel, 0);
                assert_eq!(s.step(1, &mut limited), ControlledStep::Failed(expected));
            }
            assert_eq!(driving.stats(), before);
            assert_eq!(s.finish(), Err(expected), "{stage}");
        }
        // A separate uninterrupted run really advances this particular cursor
        // and still produces the original numeric value and source spans.
        let mut s = session(source);
        while !(if validation { s.phase == Phase::Validate && s.comma.stage() == stage }
            else { s.phase == Phase::Number && s.number.stage() == stage }) {
            assert_eq!(s.step(1, &mut NoStop), ControlledStep::Pending { charged_units: 1 });
        }
        assert_eq!(s.step(1, &mut NoStop), ControlledStep::Pending { charged_units: 1 });
        while matches!(s.step(1, &mut NoStop), ControlledStep::Pending { .. }) {}
        let document = s.finish().unwrap();
        assert_eq!(document.numbers[0].value.to_bits(), (1.25_f64 * 10.0_f64.powi(-1)).to_bits());
        assert_eq!(document.numbers[0].source, Span { start: 3, end: 18 });
    }
}
