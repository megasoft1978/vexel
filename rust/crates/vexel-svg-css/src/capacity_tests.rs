use super::*;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use vexel_base::BudgetControl;
use vexel_xml::{XmlError, tokenize_large};

#[test]
fn allocation_getter_counts_actual_rule_and_every_declaration_capacity() {
    let css = (0..51)
        .map(|i| {
            format!(
                "rect,.c{i}{{{}}}",
                "fill:red;stroke:blue;".repeat(i % 7 + 1)
            )
        })
        .collect::<String>();
    let source = format!("<svg><style>{css}</style><rect style='fill:green'/></svg>");
    let (tokens, result) = tokenize_large(source.as_bytes());
    assert_eq!(result.diagnostic.category, XmlError::None);
    let mut sheet = CssSheet::create(source.as_bytes(), &tokens).unwrap();
    assert_eq!(sheet.rule_count(), 102);
    assert!(sheet.rules.capacity() > sheet.rules.len());
    let expected = sheet.rules.capacity() * size_of::<Rule>()
        + sheet
            .rules
            .iter()
            .map(|r| r.declarations.capacity() * size_of::<Declaration>())
            .sum::<usize>();
    assert_eq!(sheet.allocated_bytes(), expected);
    assert!(
        sheet
            .rules
            .iter()
            .all(|r| r.declarations.capacity() >= r.declarations.len())
    );
    assert!(expected > sheet.rules.len() * size_of::<Rule>());
    let rect = tokens
        .iter()
        .position(|t| t.local_name.slice(source.as_bytes()) == Some(b"rect") && is_element(t))
        .unwrap();
    sheet.compute(rect, None).unwrap();
    assert_eq!(sheet.allocated_bytes(), expected);
    let empty_source = b"<svg/>";
    let (empty_tokens, _) = tokenize_large(empty_source);
    assert_eq!(
        CssSheet::create(empty_source, &empty_tokens)
            .unwrap()
            .allocated_bytes(),
        0
    );
}

#[test]
fn real_fallible_reserve_capacity_failure_preserves_oom_mapping() {
    let mut buffer: Vec<Declaration> = Vec::new();
    let error = reserve(&mut buffer, usize::MAX, true).unwrap_err();
    assert_eq!(error, CssError::OutOfMemory);
    assert_eq!(control::original(error), ControlledError::OutOfMemory);
    assert_eq!(
        control::legacy_error(control::original(error)),
        CssError::OutOfMemory
    );
    assert_eq!(buffer.capacity(), 0);
    let mut buffer: Vec<Rule> = Vec::new();
    assert_eq!(
        reserve(&mut buffer, usize::MAX, false),
        Err(CssError::OutOfMemory)
    );
    assert_eq!(buffer.capacity(), 0);
}

#[test]
fn dropping_stopped_owned_buffers_leaves_source_and_future_sessions_usable() {
    let source =
        b"<svg><style>rect,svg{fill:red;stroke:blue}</style><rect style='fill:green'/></svg>";
    let (tokens, _) = tokenize_large(source);
    let rect = tokens
        .iter()
        .position(|t| t.local_name.slice(source) == Some(b"rect") && is_element(t))
        .unwrap();
    let cancel = AtomicBool::new(false);
    for _ in 0..30 {
        let mut session = CssBuildSession::new(source, &tokens);
        let mut control = BudgetControl::new(&cancel, 120);
        assert_eq!(
            session.step(u64::MAX, &mut control),
            ControlledStep::Failed(ControlledError::Limit)
        );
        drop(session);
        let mut sheet = CssSheet::create(source, &tokens).unwrap();
        let before = sheet.allocated_bytes();
        let mut session = sheet.begin_compute(rect, None);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        assert!(matches!(
            session.step(140, &mut control),
            ControlledStep::Pending { .. }
        ));
        cancel.store(true, Ordering::Release);
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Failed(ControlledError::Canceled)
        );
        drop(session);
        cancel.store(false, Ordering::Release);
        assert_eq!(sheet.allocated_bytes(), before);
        assert_eq!(
            sheet
                .compute(rect, None)
                .unwrap()
                .value(Property::Fill)
                .unwrap()
                .bytes,
            b"green"
        );
    }
    assert_eq!(
        source,
        b"<svg><style>rect,svg{fill:red;stroke:blue}</style><rect style='fill:green'/></svg>"
    );
}
