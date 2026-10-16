//! Independent parity against the exact current-source baseline saved before editing.
#[allow(dead_code)]
#[path = "../../../fixtures/legacy/css-baseline/lib.rs"]
mod original;

use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, ControlledError, ControlledStep};
use vexel_svg_css::{CssBuildSession, CssError, CssSheet, Property};
use vexel_xml::{TokenKind, XmlError, XmlToken, tokenize_large};

fn error(error: ControlledError<CssError>) -> String {
    match error {
        ControlledError::Original(e) => format!("{e:?}"),
        e => format!("{e:?}"),
    }
}
fn compare(source: &[u8], tokens: &[XmlToken], quantum: u64) {
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut session = CssBuildSession::new(source, tokens);
    while matches!(
        session.step(quantum, &mut control),
        ControlledStep::Pending { .. }
    ) {}
    let current = session.finish();
    let baseline = original::CssSheet::create(source, tokens);
    let (mut current, mut baseline) = match (current, baseline) {
        (Ok(current), Ok(baseline)) => (current, baseline),
        (Err(current), Err(baseline)) => {
            assert_eq!(error(current), format!("{baseline:?}"));
            return;
        }
        _ => panic!(
            "build result differed on {}",
            String::from_utf8_lossy(source)
        ),
    };
    assert_eq!(
        current.rule_count(),
        baseline.rule_count(),
        "{}",
        String::from_utf8_lossy(source)
    );
    assert_eq!(current.css_bytes(), baseline.css_bytes());
    let mut current_parent = None;
    let mut baseline_parent = None;
    for (i, token) in tokens.iter().enumerate() {
        if !matches!(
            token.kind,
            TokenKind::StartElement | TokenKind::EmptyElement
        ) {
            continue;
        }
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let mut session = current.begin_compute(i, current_parent.as_ref());
        while matches!(
            session.step(quantum, &mut control),
            ControlledStep::Pending { .. }
        ) {}
        let found = session.finish();
        let expected = baseline.compute(i, baseline_parent.as_ref());
        match (found, expected) {
            (Ok(found), Ok(expected)) => {
                for (p, old_p) in Property::ALL.into_iter().zip(original::Property::ALL) {
                    assert_eq!(
                        found.value(p).map(|v| (v.span, v.bytes)),
                        expected.value(old_p).map(|v| (v.span, v.bytes)),
                        "{p:?}, element {i}, input {}",
                        String::from_utf8_lossy(source)
                    );
                }
                current_parent = Some(found);
                baseline_parent = Some(expected);
            }
            (Err(found), Err(expected)) => assert_eq!(error(found), format!("{expected:?}")),
            _ => panic!(
                "compute result differed on element {i}, input {}",
                String::from_utf8_lossy(source)
            ),
        }
    }
    let legacy = CssSheet::create(source, tokens).unwrap();
    assert_eq!(legacy.rule_count(), baseline.rule_count());
}

#[test]
fn grammar_cascade_and_malformed_inputs_match_independent_baseline() {
    let selectors = [
        "rect",
        "svg g rect",
        ".k",
        "rect.k.other",
        "#x.k",
        "missing rect",
        "g rect",
        "svg g g rect",
        "g>rect",
        "rect:hover",
        "*",
        "x:rect",
        "rect,,#x",
        "rect.k.k",
        "rect#x#x",
        "réct",
        "rect .k",
        "rect[title=\",\"]",
        "svg g svg g rect",
    ];
    let declarations = [
        "fill:red;stroke:blue",
        "FiLl:/*leading*/ red /*trailing*/",
        "fill:'a;b';font-family:'!important'",
        "fill:blue/*trailing*/ ! important",
        "fill:initial;stroke:unset",
        "clip-rule:evenodd;opacity:.4;display:none",
        "unknown-property:red",
        "fill:blue!important;fill:green",
        "fill:red;fill:blue!important",
        "fill:red !important /*trail*/",
        "fill:blue/*c*/!important",
        "font-family:\"a\\\";b\";fill:red",
        "fill:func(a;b);stroke:var(--x)",
        "fill:;stroke:blue",
        "bad;:blue;fill:red",
        "fill:/**/;stroke:blue",
        "fill:red/*x*//*y*/;stroke:blue",
        "fill:'unterminated!important",
        "fill:red/*unterminated",
        "fill:{red};stroke:blue",
        "fill:initial!important;opacity:inherit",
    ];
    for (i, selector) in selectors.iter().enumerate() {
        for (j, declaration) in declarations.iter().enumerate() {
            let source = format!(
                "<svg><style><![CDATA[@import url('x;y');@media (x){{ignored{{fill:red}}}}{selector}{{{declaration}}}stop{{{declaration}}} broken]]></style><g fill='white' opacity='.7'><rect id='x' class='k other' stroke='cyan' style='font-family:serif;stroke:inherit;opacity:unset'/><stop/></g></svg>"
            );
            let (tokens, result) = tokenize_large(source.as_bytes());
            assert_eq!(result.diagnostic.category, XmlError::None);
            compare(source.as_bytes(), &tokens, [1, 2, 7, 256, 257][(i + j) % 5]);
        }
    }
}

#[test]
fn deterministic_byte_grammar_cases_match_independent_baseline() {
    let alphabet = b" abcdefillstrokred:;{}/*()!important#.,@\\\"'\t\n";
    let mut seed = 0x81c5_u64;
    for case in 0..600 {
        let mut css = String::new();
        for _ in 0..(case % 350 + 1) {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            css.push(alphabet[(seed >> 32) as usize % alphabet.len()] as char);
        }
        let source = format!(
            "<svg><style><![CDATA[rect{{fill:red}}{css}]]></style><g fill='white'><rect id='x' class='k other'/><stop/></g></svg>"
        );
        let (tokens, result) = tokenize_large(source.as_bytes());
        assert_eq!(result.diagnostic.category, XmlError::None);
        compare(source.as_bytes(), &tokens, case % 13 + 1);
    }
}

#[test]
fn malformed_token_spans_and_split_style_runs_preserve_original_error_priority() {
    let source = b"<svg><style><![CDATA[rect{fill:]]><![CDATA[red}]]><![CDATA[rect{stroke:blue}]]></style><rect fill='red' style='stroke:green'/></svg>";
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    compare(source, &tokens, 1);
    for i in 0..tokens.len() {
        for span in [
            vexel_xml::XmlSpan {
                offset: usize::MAX,
                length: 1,
            },
            vexel_xml::XmlSpan {
                offset: 0,
                length: usize::MAX,
            },
            vexel_xml::XmlSpan {
                offset: source.len(),
                length: 1,
            },
        ] {
            let mut modified = tokens.clone();
            modified[i].value = span;
            compare(source, &modified, 1);
            let mut modified = tokens.clone();
            modified[i].local_name = span;
            compare(source, &modified, 7);
        }
    }
}
