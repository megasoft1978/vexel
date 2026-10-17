//! WO-RUST-LIMITS-03: the rule-match quota admits exactly 8,388,608 matches and
//! refuses the next one with `CssError::Limit`.
use vexel_svg_css::{CssError, CssSheet, MAX_RULE_MATCHES};
use vexel_xml::{TokenKind, XmlError, tokenize_large};

#[test]
fn limit03_rule_match_quota_admits_8m_and_refuses_the_next() {
    assert_eq!(MAX_RULE_MATCHES, 8_388_608);
    let source = b"<svg><style>rect{fill:red}</style><rect/></svg>";
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    let rect = tokens
        .iter()
        .position(|token| {
            matches!(token.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && token.local_name.slice(source) == Some(b"rect")
        })
        .expect("rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    for i in 0..8_388_608usize {
        assert!(sheet.compute(rect, None).is_ok(), "match {i} refused");
    }
    assert_eq!(sheet.compute(rect, None).err(), Some(CssError::Limit));
}
