use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, ControlledError, ControlledStep};
use vexel_svg_css::{CssBuildSession, CssError, CssSheet, MAX_BYTES, Property};
use vexel_xml::{TokenKind, XmlError, tokenize_large};

#[test]
fn inline_byte_cap_is_exact_and_does_not_limit_containing_svg() {
    for extra in [0, 1] {
        let value = "v".repeat(MAX_BYTES - 5 + extra);
        let source = format!("<svg><rect style='fill:{value}'/></svg>");
        let (tokens, result) = tokenize_large(source.as_bytes());
        assert_eq!(result.diagnostic.category, XmlError::None);
        let rect = tokens
            .iter()
            .position(|t| t.kind == TokenKind::EmptyElement)
            .unwrap();
        let mut sheet = CssSheet::create(source.as_bytes(), &tokens).unwrap();
        let cancel = AtomicBool::new(false);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut session = sheet.begin_compute(rect, None);
        while matches!(
            session.step(7, &mut control),
            ControlledStep::Pending { .. }
        ) {}
        let found = session.finish();
        if extra == 0 {
            let found = found.unwrap().value(Property::Fill).unwrap();
            assert_eq!(found.bytes.len(), MAX_BYTES - 5);
            assert_eq!(found.span.slice(source.as_bytes()), Some(found.bytes));
            assert_eq!(found.bytes, value.as_bytes());
            assert!(control.stats().total_units() > MAX_BYTES as u64);
        } else {
            assert_eq!(found, Err(ControlledError::Original(CssError::Limit)));
        }
        assert_eq!(sheet.css_bytes(), 0);
        assert_eq!(sheet.allocated_bytes(), 0);
    }
}

#[test]
fn selector_class_and_attribute_class_caps_keep_original_grammar() {
    let classes = (0..16).map(|i| format!(".c{i}")).collect::<String>();
    let values = (0..16)
        .map(|i| format!("c{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!(
        "<svg><style>svg g g rect{classes}{{fill:blue}}</style><g><g><rect class='{values}'/></g></g></svg>"
    );
    let (tokens, result) = tokenize_large(source.as_bytes());
    assert_eq!(result.diagnostic.category, XmlError::None);
    let rect = tokens
        .iter()
        .position(|t| t.kind == TokenKind::EmptyElement)
        .unwrap();
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut session = CssBuildSession::new(source.as_bytes(), &tokens);
    while matches!(
        session.step(1, &mut control),
        ControlledStep::Pending { .. }
    ) {}
    let mut sheet = session.finish().unwrap();
    assert_eq!(sheet.rule_count(), 1);
    assert_eq!(
        sheet
            .compute_controlled(rect, None, &mut control)
            .unwrap()
            .value(Property::Fill)
            .unwrap()
            .bytes,
        b"blue"
    );
    for count in [255, 256] {
        let preceding = (0..count).map(|i| format!("other{i} ")).collect::<String>();
        let source = format!(
            "<svg><style>.wanted{{fill:red}}</style><rect class='{preceding}wanted'/></svg>"
        );
        let (tokens, result) = tokenize_large(source.as_bytes());
        assert_eq!(result.diagnostic.category, XmlError::None);
        let rect = tokens
            .iter()
            .position(|t| t.kind == TokenKind::EmptyElement)
            .unwrap();
        let mut sheet =
            CssSheet::create_controlled(source.as_bytes(), &tokens, &mut control).unwrap();
        let mut session = sheet.begin_compute(rect, None);
        while matches!(
            session.step(1, &mut control),
            ControlledStep::Pending { .. }
        ) {}
        let paint = session.finish().unwrap();
        assert_eq!(
            paint.value(Property::Fill).map(|v| v.bytes),
            if count == 255 {
                Some(&b"red"[..])
            } else {
                None
            }
        );
    }
}

#[test]
fn unknown_declarations_fail_only_matched_unprefixed_stop_stylesheet_rules() {
    let source = b"<svg xmlns:x='urn:x'><style>stop{unknown:red}.unmatched{unknown:blue}</style><rect style='unknown:red;fill:green'/><x:stop/><stop/></svg>";
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut sheet = CssSheet::create_controlled(source, &tokens, &mut control).unwrap();
    for (i, token) in tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| t.kind == TokenKind::EmptyElement)
    {
        let mut session = sheet.begin_compute(i, None);
        while matches!(
            session.step(1, &mut control),
            ControlledStep::Pending { .. }
        ) {}
        let paint = session.finish();
        if token.local_name.slice(source) == Some(b"stop") && token.prefix.length == 0 {
            assert_eq!(paint, Err(ControlledError::Original(CssError::Unsupported)));
        } else if token.local_name.slice(source) == Some(b"rect") {
            assert_eq!(
                paint.unwrap().value(Property::Fill).unwrap().bytes,
                b"green"
            );
        } else {
            assert_eq!(paint.unwrap().value(Property::Fill), None);
        }
    }
}
