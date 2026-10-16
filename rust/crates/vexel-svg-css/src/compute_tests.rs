use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use vexel_base::BudgetControl;
use vexel_xml::{XmlError, tokenize_large};

#[test]
fn cancellation_in_matching_inline_offer_and_winner_never_publishes_partial_paint() {
    let source = b"<svg><style>svg g rect.k{fill:red;stroke:blue}</style><g><rect class='k' fill='yellow' style='fill:green;stroke:inherit'/></g></svg>";
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    let rect = tokens
        .iter()
        .position(|t| crate::is_element(t) && t.local_name.slice(source) == Some(b"rect"))
        .unwrap();
    for phase in [
        Phase::Initialize,
        Phase::Reset,
        Phase::Rule,
        Phase::Match,
        Phase::Offer,
        Phase::Presentation,
        Phase::InlineAttribute,
        Phase::InlineParse,
        Phase::InlineOffer,
        Phase::Winner,
    ] {
        let mut sheet = CssSheet::create(source, &tokens).unwrap();
        let cancel = AtomicBool::new(false);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut session = sheet.begin_compute(rect, None);
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
        let match_tests = session.sheet.match_tests;
        let selector_visits = session.sheet.selector_visits;
        cancel.store(true, Ordering::Release);
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Failed(ControlledError::Canceled)
        );
        assert_eq!(control.stats(), before);
        assert_eq!(session.sheet.match_tests, match_tests);
        assert_eq!(session.sheet.selector_visits, selector_visits);
        assert_eq!(
            session.step(1, &mut control::NoStop),
            ControlledStep::Failed(ControlledError::Canceled)
        );
        assert_eq!(session.finish(), Err(ControlledError::Canceled));
        assert_eq!(sheet.match_tests, match_tests);
        assert_eq!(sheet.selector_visits, selector_visits);
        assert!(!sheet.hit_limit);
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
}

#[test]
fn cumulative_rule_and_selector_quotas_and_failure_priorities_are_unchanged() {
    let source = b"<svg><style>absent rect{fill:red}rect{stroke:blue}</style><g><rect/></g></svg>";
    let (tokens, _) = tokenize_large(source);
    let rect = tokens
        .iter()
        .position(|t| crate::is_element(t) && t.local_name.slice(source) == Some(b"rect"))
        .unwrap();
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    sheet.match_tests = MAX_RULE_MATCHES - 1;
    assert_eq!(sheet.compute(rect, None), Err(CssError::Limit));
    assert_eq!(sheet.match_tests, MAX_RULE_MATCHES);
    assert!(sheet.selector_visits > 0);
    assert!(sheet.hit_limit);
    let old = sheet.selector_visits;
    assert_eq!(sheet.compute(rect, None), Err(CssError::Limit));
    assert_eq!(sheet.selector_visits, old);
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    sheet.selector_visits = MAX_RULE_MATCHES - 1;
    assert_eq!(sheet.compute(rect, None), Err(CssError::Limit));
    assert_eq!(sheet.selector_visits, MAX_RULE_MATCHES);
    assert_eq!(sheet.match_tests, 1);
    assert!(sheet.hit_limit);

    // Source validation precedes rules, and does not set the stylesheet's hit_limit flag.
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    sheet.match_tests = MAX_RULE_MATCHES;
    assert_eq!(sheet.compute(usize::MAX, None), Err(CssError::Limit));
    assert!(!sheet.hit_limit);
    assert_eq!(sheet.match_tests, MAX_RULE_MATCHES);
    let other_source = source.to_vec();
    let mut other_sheet = CssSheet::create(&other_source, &tokens).unwrap();
    let parent = other_sheet.compute(rect, None).unwrap();
    assert_eq!(sheet.compute(rect, Some(&parent)), Err(CssError::Limit));
    assert!(!sheet.hit_limit);
    assert_eq!(sheet.selector_visits, 0);
}

#[test]
fn stopped_rule_work_remains_charged_and_is_not_rolled_back_on_retry() {
    let source = b"<svg><style>rect{fill:red}</style><rect/></svg>";
    let (tokens, _) = tokenize_large(source);
    let rect = tokens
        .iter()
        .position(|t| crate::is_element(t) && t.local_name.slice(source) == Some(b"rect"))
        .unwrap();
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut session = sheet.begin_compute(rect, None);
    while session.phase != Phase::Match {
        session.step(1, &mut control);
    }
    assert_eq!(session.sheet.match_tests, 1);
    let before = control.stats();
    cancel.store(true, Ordering::Release);
    assert_eq!(
        session.step(1, &mut control),
        ControlledStep::Failed(ControlledError::Canceled)
    );
    assert_eq!(session.finish(), Err(ControlledError::Canceled));
    assert_eq!(control.stats(), before);
    assert_eq!(sheet.match_tests, 1);
    sheet.compute(rect, None).unwrap();
    assert_eq!(sheet.match_tests, 2);
}

#[test]
fn cancellation_and_limits_interrupt_actual_matching_inline_and_winner_work() {
    let class = "c".repeat(8000);
    let value = "v".repeat(8000);
    let declarations = "fill:red;".repeat(600);
    let inline = "stroke:blue;".repeat(600);
    let source = format!(
        "<svg><style>rect.{class}{{{declarations}}}</style><rect class='{class}' style='{inline}font-family:{value}'/></svg>"
    );
    let (tokens, result) = tokenize_large(source.as_bytes());
    assert_eq!(result.diagnostic.category, XmlError::None);
    let rect = tokens
        .iter()
        .position(|t| {
            crate::is_element(t) && t.local_name.slice(source.as_bytes()) == Some(b"rect")
        })
        .unwrap();
    for (phase, offset) in [
        (Phase::Match, 1000),
        (Phase::Offer, 100),
        (Phase::InlineParse, 1000),
        (Phase::InlineOffer, 100),
        (Phase::Winner, 10),
    ] {
        let mut sheet = CssSheet::create(source.as_bytes(), &tokens).unwrap();
        let cancel = AtomicBool::new(false);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut session = sheet.begin_compute(rect, None);
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
        if matches!(phase, Phase::Offer | Phase::InlineOffer) {
            assert_eq!(session.declaration, 100);
        }
        if phase == Phase::Winner {
            assert_eq!(session.property, 10);
        }
        assert_eq!(session.sheet.match_tests, 1);
        cancel.store(true, Ordering::Release);
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Failed(ControlledError::Canceled)
        );
        assert_eq!(control.stats().total_units(), count);
        assert_eq!(session.finish(), Err(ControlledError::Canceled));
        assert_eq!(sheet.match_tests, 1);
        cancel.store(false, Ordering::Release);
        let mut limited = BudgetControl::new(&cancel, count);
        let mut sheet = CssSheet::create(source.as_bytes(), &tokens).unwrap();
        let mut session = sheet.begin_compute(rect, None);
        assert_eq!(
            session.step(u64::MAX, &mut limited),
            ControlledStep::Failed(ControlledError::Limit)
        );
        assert_eq!(limited.stats().total_units(), count);
        assert_eq!(session.phase, phase);
        assert_eq!(session.finish(), Err(ControlledError::Limit));
        assert_eq!(sheet.match_tests, 1);
        assert!(!sheet.hit_limit);
    }
}
