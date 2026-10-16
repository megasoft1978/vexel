mod support;
use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, ControlledError, ControlledStep};
use vexel_svg_path::{parse_path, parse_path_controlled, Diagnostic, PathDocument, PathParseSession};

fn original(result: Result<PathDocument, ControlledError<Diagnostic>>) -> Result<PathDocument, Diagnostic> {
    result.map_err(|error| match error { ControlledError::Original(error) => error, other => panic!("unexpected stop: {other:?}") })
}
fn stepped(case: &support::Case, quantum: u64) -> Result<PathDocument, Diagnostic> {
    let mut session = match PathParseSession::new(&case.source, case.span, case.transform, case.options) {
        Ok(session) => session, Err(error) => return original(Err(error)),
    };
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    loop {
        let before = control.stats();
        assert_eq!(session.step(0, &mut control), ControlledStep::Pending { charged_units: 0 });
        assert_eq!(control.stats(), before);
        let result = session.step(quantum, &mut control);
        let actual = control.stats().total_units() - before.total_units();
        assert!(actual <= quantum, "{}: real work exceeded quantum", case.name);
        match result {
            ControlledStep::Pending { charged_units } => assert_eq!(charged_units, actual),
            terminal => {
                let before = control.stats();
                assert_eq!(session.step(quantum, &mut control), terminal);
                assert_eq!(session.step(0, &mut control), terminal);
                assert_eq!(control.stats(), before);
                return original(session.finish());
            }
        }
    }
}

#[test]
fn all_entry_points_match_frozen_pre_rewrite_bitwise_evidence() {
    let cancel = AtomicBool::new(false);
    let mut text = String::new();
    let cases = support::cases();
    for case in &cases {
        let expected = support::fingerprint(parse_path(&case.source, case.span, case.transform, case.options));
        text.push_str(&format!("CASE {}\n", case.name));
        text.push_str(&expected);
        for quantum in [1, 2, 3, 7, 256] {
            assert_eq!(support::fingerprint(stepped(case, quantum)), expected, "{} quantum {quantum}", case.name);
        }
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        assert_eq!(support::fingerprint(original(parse_path_controlled(&case.source, case.span, case.transform, case.options, &mut control))), expected, "{} controlled adapter", case.name);
    }
    assert_eq!(text, include_str!("../../../../artifacts/WO-RUST-SVG-PATH-CONTROL-01/baseline-golden.txt"));
}
