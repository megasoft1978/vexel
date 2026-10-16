use std::sync::atomic::{AtomicBool, Ordering};
use vexel_base::{BudgetControl, ControlledError, ControlledStep, WorkControl, WorkKind};
use vexel_svg_css::{CssBuildSession, CssError, CssPaint, CssSheet, Property};
use vexel_xml::{TokenKind, XmlError, XmlToken, tokenize_large};

fn tokens(source: &[u8]) -> Vec<XmlToken> {
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    tokens
}
fn element(source: &[u8], tokens: &[XmlToken], name: &[u8]) -> usize {
    tokens
        .iter()
        .position(|t| {
            matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && t.local_name.slice(source) == Some(name)
        })
        .unwrap()
}
fn build<'a>(source: &'a [u8], tokens: &'a [XmlToken], quantum: u64) -> (CssSheet<'a>, u64) {
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut session = CssBuildSession::new(source, tokens);
    let mut pending = 0;
    loop {
        let before = control.stats();
        match session.step(quantum, &mut control) {
            ControlledStep::Pending { charged_units } => {
                assert_eq!(charged_units, quantum);
                assert_eq!(
                    control.stats().total_units() - before.total_units(),
                    charged_units
                );
                pending += 1;
            }
            ControlledStep::Complete(()) => {
                assert!(control.stats().total_units() - before.total_units() <= quantum);
                let after = control.stats();
                cancel.store(true, Ordering::Release);
                assert_eq!(
                    session.step(quantum, &mut control),
                    ControlledStep::Complete(())
                );
                assert_eq!(control.stats(), after);
                return (session.finish().unwrap(), pending);
            }
            failed => panic!("{failed:?}"),
        }
    }
}
fn paint<'a>(
    sheet: &mut CssSheet<'a>,
    element: usize,
    parent: Option<&CssPaint<'a>>,
    quantum: u64,
) -> (CssPaint<'a>, u64, vexel_base::WorkStats) {
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut session = sheet.begin_compute(element, parent);
    let mut pending = 0;
    loop {
        let before = control.stats();
        match session.step(quantum, &mut control) {
            ControlledStep::Pending { charged_units } => {
                assert_eq!(charged_units, quantum);
                assert_eq!(
                    control.stats().total_units() - before.total_units(),
                    charged_units
                );
                pending += 1;
            }
            ControlledStep::Complete(()) => {
                assert!(control.stats().total_units() - before.total_units() <= quantum);
                let after = control.stats();
                cancel.store(true, Ordering::Release);
                assert_eq!(
                    session.step(quantum, &mut control),
                    ControlledStep::Complete(())
                );
                assert_eq!(control.stats(), after);
                return (session.finish().unwrap(), pending, after);
            }
            failed => panic!("{failed:?}"),
        }
    }
}

#[test]
fn zero_quantum_early_finish_and_terminal_failure_are_real() {
    let source = b"<svg><style>rect{fill:red}</style><rect style='fill:blue'/></svg>";
    let tokens = tokens(source);
    let rect = element(source, &tokens, b"rect");
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut build = CssBuildSession::new(source, &tokens);
    assert_eq!(
        build.step(0, &mut control),
        ControlledStep::Pending { charged_units: 0 }
    );
    assert_eq!(control.stats().total_units(), 0);
    assert!(matches!(
        build.finish(),
        Err(ControlledError::Original(CssError::Limit))
    ));
    let mut build = CssBuildSession::new(source, &tokens);
    assert!(matches!(
        build.step(7, &mut control),
        ControlledStep::Pending { charged_units: 7 }
    ));
    let before = control.stats();
    cancel.store(true, Ordering::Release);
    assert_eq!(
        build.step(0, &mut control),
        ControlledStep::Failed(ControlledError::Canceled)
    );
    cancel.store(false, Ordering::Release);
    assert_eq!(
        build.step(100, &mut control),
        ControlledStep::Failed(ControlledError::Canceled)
    );
    assert_eq!(control.stats(), before);
    assert!(matches!(build.finish(), Err(ControlledError::Canceled)));

    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let mut compute = sheet.begin_compute(rect, None);
    let mut fresh = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        compute.step(0, &mut fresh),
        ControlledStep::Pending { charged_units: 0 }
    );
    assert_eq!(fresh.stats().total_units(), 0);
    assert_eq!(
        compute.finish(),
        Err(ControlledError::Original(CssError::Limit))
    );
    let mut compute = sheet.begin_compute(rect, None);
    assert!(matches!(
        compute.step(3, &mut fresh),
        ControlledStep::Pending { charged_units: 3 }
    ));
    let before = fresh.stats();
    cancel.store(true, Ordering::Release);
    assert_eq!(
        compute.step(0, &mut fresh),
        ControlledStep::Failed(ControlledError::Canceled)
    );
    cancel.store(false, Ordering::Release);
    assert_eq!(
        compute.step(100, &mut fresh),
        ControlledStep::Failed(ControlledError::Canceled)
    );
    assert_eq!(fresh.stats(), before);
    assert_eq!(compute.finish(), Err(ControlledError::Canceled));
}

#[test]
fn long_identifiers_classes_comments_and_inline_values_retain_actual_byte_progress() {
    let name = "r".repeat(3000);
    let id = "i".repeat(12000);
    let class = "c".repeat(17000);
    let comment = "z".repeat(19000);
    let value = "v".repeat(23000);
    let source = format!(
        "<svg><style><![CDATA[/*{comment}*/svg g {name}#{id}.{class}{{stroke:/*{comment}*/blue!important/*{comment}*/}}]]></style><g><{name} id='{id}' class='other {class}' style='font-family:{value}/*{comment}*/;fill:inherit'/></g></svg>"
    );
    let tokens = tokens(source.as_bytes());
    let rect = element(source.as_bytes(), &tokens, name.as_bytes());
    let mut legacy = CssSheet::create(source.as_bytes(), &tokens).unwrap();
    let expected = legacy.compute(rect, None).unwrap();
    for quantum in [1, 7, 257] {
        let (mut sheet, pending_build) = build(source.as_bytes(), &tokens, quantum);
        assert_eq!(sheet.css_bytes(), legacy.css_bytes());
        assert_eq!(sheet.rule_count(), legacy.rule_count());
        assert!(pending_build * quantum > 100_000);
        let (found, pending_compute, stats) = paint(&mut sheet, rect, None, quantum);
        assert_eq!(found, expected);
        assert_eq!(found.value(Property::Stroke).unwrap().bytes, b"blue");
        assert_eq!(
            found.value(Property::FontFamily).unwrap().bytes,
            value.as_bytes()
        );
        assert!(pending_compute * quantum > 100_000);
        assert!(stats.units(WorkKind::SourceRead) > 100_000);
        assert_eq!(found.value(Property::Fill), None);
    }
}

#[test]
fn many_rules_ancestors_attributes_and_cdata_runs_make_measurable_progress() {
    let rules = "absent rect{fill:red}".repeat(40);
    let ancestors = "<g>".repeat(200);
    let closes = "</g>".repeat(200);
    let attributes = (0..600).map(|i| format!(" a{i}='x'")).collect::<String>();
    let source = format!(
        "<svg><style><![CDATA[{rules}]]><![CDATA[svg g rect.k{{fill:blue}}]]></style>{ancestors}<rect{attributes} class='k'/>{closes}</svg>"
    );
    let tokens = tokens(source.as_bytes());
    let rect = element(source.as_bytes(), &tokens, b"rect");
    let (mut sheet, pending_build) = build(source.as_bytes(), &tokens, 1);
    assert_eq!(sheet.rule_count(), 41);
    assert!(pending_build > 1000);
    let (found, pending_compute, stats) = paint(&mut sheet, rect, None, 1);
    assert_eq!(found.value(Property::Fill).unwrap().bytes, b"blue");
    assert!(pending_compute > 32_000);
    assert!(stats.units(WorkKind::TokenVisit) > 32_000);
}

#[test]
fn temporary_parent_and_sheet_borrows_do_not_shorten_source_lifetime() {
    let source = b"<svg><style>g{fill:red;opacity:.4;clip-rule:evenodd}rect{fill:inherit;opacity:unset}</style><g><rect/></g></svg>";
    let tokens = tokens(source);
    let group = element(source, &tokens, b"g");
    let rect = element(source, &tokens, b"rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let mut compute = {
        let parent = sheet.compute(group, None).unwrap();
        sheet.begin_compute(rect, Some(&parent))
    };
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    while matches!(
        compute.step(1, &mut control),
        ControlledStep::Pending { .. }
    ) {}
    let paint = compute.finish().unwrap();
    drop(sheet);
    assert_eq!(paint.value(Property::Fill).unwrap().bytes, b"red");
    assert_eq!(paint.value(Property::ClipRule).unwrap().bytes, b"evenodd");
    assert_eq!(paint.value(Property::Opacity), None);
}

#[test]
fn true_overflow_and_invalid_source_errors_are_typed_and_sticky() {
    let source = b"<svg/>";
    let tokens = tokens(source);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    control.charge(WorkKind::SourceRead, u64::MAX).unwrap();
    let mut build = CssBuildSession::new(source, &tokens);
    assert_eq!(
        build.step(1, &mut control),
        ControlledStep::Failed(ControlledError::Overflow)
    );
    assert_eq!(
        build.step(1, &mut control),
        ControlledStep::Failed(ControlledError::Overflow)
    );
    assert_eq!(control.stats().total_units(), u64::MAX);
    assert!(matches!(build.finish(), Err(ControlledError::Overflow)));
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(
        sheet.compute_controlled(usize::MAX, None, &mut control),
        Err(ControlledError::Original(CssError::Limit))
    );
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    control.charge(WorkKind::SourceRead, u64::MAX).unwrap();
    assert_eq!(
        sheet.compute_controlled(0, None, &mut control),
        Err(ControlledError::Overflow)
    );
}

#[test]
fn build_and_compute_limit_keep_all_accepted_work_without_partial_results() {
    let source = b"<svg><style>rect{fill:red;stroke:blue}</style><rect style='fill:green'/></svg>";
    let tokens = tokens(source);
    let rect = element(source, &tokens, b"rect");
    let cancel = AtomicBool::new(false);
    let mut full = BudgetControl::new(&cancel, u64::MAX);
    let mut sheet = CssSheet::create_controlled(source, &tokens, &mut full).unwrap();
    let build_units = full.stats().total_units();
    let mut full = BudgetControl::new(&cancel, u64::MAX);
    let expected = sheet.compute_controlled(rect, None, &mut full).unwrap();
    let compute_units = full.stats().total_units();
    for limit in 0..build_units {
        let mut control = BudgetControl::new(&cancel, limit);
        let mut build = CssBuildSession::new(source, &tokens);
        assert_eq!(
            build.step(u64::MAX, &mut control),
            ControlledStep::Failed(ControlledError::Limit)
        );
        assert_eq!(control.stats().total_units(), limit);
        assert_eq!(
            build.step(u64::MAX, &mut control),
            ControlledStep::Failed(ControlledError::Limit)
        );
        assert!(matches!(build.finish(), Err(ControlledError::Limit)));
    }
    for limit in 0..compute_units {
        let mut sheet = CssSheet::create(source, &tokens).unwrap();
        let mut control = BudgetControl::new(&cancel, limit);
        let mut compute = sheet.begin_compute(rect, None);
        assert_eq!(
            compute.step(u64::MAX, &mut control),
            ControlledStep::Failed(ControlledError::Limit)
        );
        assert_eq!(control.stats().total_units(), limit);
        assert_eq!(
            compute.step(u64::MAX, &mut control),
            ControlledStep::Failed(ControlledError::Limit)
        );
        assert_eq!(compute.finish(), Err(ControlledError::Limit));
        assert_eq!(sheet.compute(rect, None).unwrap(), expected);
    }
}

#[test]
fn original_unsupported_and_original_limit_remain_first_sticky_failure() {
    let source = b"<svg><style>stop{unknown:red}</style><stop/></svg>";
    let tokens = tokens(source);
    let stop = element(source, &tokens, b"stop");
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let mut session = sheet.begin_compute(stop, None);
    let failure = ControlledStep::Failed(ControlledError::Original(CssError::Unsupported));
    assert_eq!(session.step(u64::MAX, &mut control), failure);
    let before = control.stats();
    cancel.store(true, Ordering::Release);
    assert_eq!(session.step(0, &mut control), failure);
    assert_eq!(session.step(u64::MAX, &mut control), failure);
    assert_eq!(control.stats(), before);
    assert_eq!(
        session.finish(),
        Err(ControlledError::Original(CssError::Unsupported))
    );

    let css = " ".repeat(vexel_svg_css::MAX_BYTES + 1);
    let source = format!("<svg><style>{css}</style></svg>");
    let parsed = self::tokens(source.as_bytes());
    cancel.store(false, Ordering::Release);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut session = CssBuildSession::new(source.as_bytes(), &parsed);
    let failure = ControlledStep::Failed(ControlledError::Original(CssError::Limit));
    assert_eq!(session.step(u64::MAX, &mut control), failure);
    let before = control.stats();
    cancel.store(true, Ordering::Release);
    assert_eq!(session.step(u64::MAX, &mut control), failure);
    assert_eq!(control.stats(), before);
    assert!(matches!(
        session.finish(),
        Err(ControlledError::Original(CssError::Limit))
    ));
}

#[test]
fn accepted_per_kind_caps_stop_the_actual_parser_and_cascade_work() {
    use vexel_base::WorkLimits;
    let source = b"<svg><style>rect{fill:red}</style><rect style='stroke:blue'/></svg>";
    let tokens = tokens(source);
    let rect = element(source, &tokens, b"rect");
    let cancel = AtomicBool::new(false);
    for kind in [
        WorkKind::TokenVisit,
        WorkKind::SourceRead,
        WorkKind::Expansion,
    ] {
        let mut control = BudgetControl::with_limits(
            &cancel,
            u64::MAX,
            WorkLimits::unlimited().with_limit(kind, 0),
        );
        let result = CssSheet::create_controlled(source, &tokens, &mut control);
        assert!(matches!(result, Err(ControlledError::Limit)));
        assert_eq!(control.stats().units(kind), 0);
    }
    for kind in [
        WorkKind::TokenVisit,
        WorkKind::SourceRead,
        WorkKind::PaintVisit,
    ] {
        let mut sheet = CssSheet::create(source, &tokens).unwrap();
        let mut control = BudgetControl::with_limits(
            &cancel,
            u64::MAX,
            WorkLimits::unlimited().with_limit(kind, 0),
        );
        assert_eq!(
            sheet.compute_controlled(rect, None, &mut control),
            Err(ControlledError::Limit)
        );
        assert_eq!(control.stats().units(kind), 0);
    }
}
