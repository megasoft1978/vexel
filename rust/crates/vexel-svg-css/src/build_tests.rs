use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use vexel_base::BudgetControl;
use vexel_xml::{XmlError, tokenize_large};

#[test]
fn cancellation_is_terminal_in_every_real_build_phase() {
    let source = b"<svg><style><![CDATA[@media screen{rect{fill:yellow}} /*c*/ rect,svg{fill:/*a*/red/*b*/!important;stroke:blue}]]></style><rect/></svg>";
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    for phase in [
        Phase::Outer,
        Phase::Inner,
        Phase::Space,
        Phase::AtRule,
        Phase::Brace,
        Phase::Close,
        Phase::Declarations,
        Phase::Comma,
        Phase::Selector,
        Phase::Reserve,
        Phase::Copy,
        Phase::Push,
    ] {
        let cancel = AtomicBool::new(false);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut session = CssBuildSession::new(source, &tokens);
        while session.phase != phase {
            assert!(
                matches!(
                    session.step(1, &mut control),
                    ControlledStep::Pending { .. }
                ),
                "unreached {phase:?}"
            );
        }
        let before = control.stats();
        let retained_rules = session.sheet.rules.len();
        cancel.store(true, Ordering::Release);
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Failed(ControlledError::Canceled)
        );
        assert_eq!(control.stats(), before);
        assert_eq!(session.sheet.rules.len(), retained_rules);
        assert_eq!(
            session.step(1, &mut control::NoStop),
            ControlledStep::Failed(ControlledError::Canceled)
        );
        assert!(matches!(session.finish(), Err(ControlledError::Canceled)));
    }
}

#[test]
fn one_unit_build_advances_real_comment_identifier_and_copy_cursors() {
    let source = b"<svg><style>/*abcdefgh*/.abcdefgh{fill:red;stroke:blue}</style><rect/></svg>";
    let (tokens, _) = tokenize_large(source);
    let mut session = CssBuildSession::new(source, &tokens);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut observed_comment = 0;
    let mut observed_copy = 0;
    loop {
        let phase = session.phase;
        let cursor = session.space.cursor;
        let copy = session.copy_index;
        let step = session.step(1, &mut control);
        if phase == Phase::Space && session.phase == Phase::Space && session.space.cursor > cursor {
            observed_comment += 1;
        }
        if phase == Phase::Copy && session.copy_index > copy {
            assert_eq!(session.copy_index, copy + 1);
            observed_copy += 1;
        }
        if step == ControlledStep::Complete(()) {
            break;
        }
        assert_eq!(step, ControlledStep::Pending { charged_units: 1 });
    }
    assert!(observed_comment >= 8);
    assert_eq!(observed_copy, 2);
    let sheet = session.finish().unwrap();
    assert_eq!(sheet.rule_count(), 1);
    assert_eq!(sheet.declaration_count, 2);
}

#[test]
fn cancel_and_limit_inside_long_comment_parser_and_declaration_copy_are_bounded() {
    let comment = "x".repeat(8000);
    let declarations = "fill:red;".repeat(600);
    let source = format!("<svg><style>/*{comment}*/rect{{{declarations}}}</style><rect/></svg>");
    let (tokens, result) = tokenize_large(source.as_bytes());
    assert_eq!(result.diagnostic.category, XmlError::None);
    for (phase, offset) in [
        (Phase::Space, 1000),
        (Phase::Declarations, 1000),
        (Phase::Copy, 100),
    ] {
        let cancel = AtomicBool::new(false);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut session = CssBuildSession::new(source.as_bytes(), &tokens);
        while session.phase != phase {
            session.step(1, &mut control);
        }
        for _ in 0..offset {
            assert_eq!(session.phase, phase);
            assert_eq!(
                session.step(1, &mut control),
                ControlledStep::Pending { charged_units: 1 }
            );
        }
        let count = control.stats().total_units();
        if phase == Phase::Space {
            assert!(session.space.cursor >= 1000);
        }
        if phase == Phase::Copy {
            assert_eq!(session.copy_index, 100);
            assert_eq!(session.copy.len(), 100);
        }
        cancel.store(true, Ordering::Release);
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Failed(ControlledError::Canceled)
        );
        assert_eq!(control.stats().total_units(), count);
        assert!(matches!(session.finish(), Err(ControlledError::Canceled)));
        cancel.store(false, Ordering::Release);
        let mut limited = BudgetControl::new(&cancel, count);
        let mut session = CssBuildSession::new(source.as_bytes(), &tokens);
        assert_eq!(
            session.step(u64::MAX, &mut limited),
            ControlledStep::Failed(ControlledError::Limit)
        );
        assert_eq!(limited.stats().total_units(), count);
        assert_eq!(session.phase, phase);
        assert!(matches!(session.finish(), Err(ControlledError::Limit)));
    }
}
