use crate::{decode_attribute_value_with, tokenize, tokenize_cancelable, EntityTable, StreamStatus, TokenKind, XmlDiagnostic, XmlError, XmlLimits, XmlSpan,
    MAX_ENTITIES, MAX_ENTITY_EXPANDED_BYTES, MAX_ENTITY_VALUE_BYTES};
use std::time::{Duration, Instant};

fn diagnostic(source: &[u8]) -> XmlDiagnostic {
    let (tokens, result) = tokenize(source, XmlLimits::default());
    let (_, streamed, status) = tokenize_cancelable(source, XmlLimits::default(), 1 << 16, || false);
    assert_eq!(streamed.diagnostic, result.diagnostic, "stream and one-shot agree: {}", String::from_utf8_lossy(&source[..source.len().min(80)]));
    assert_eq!(status == StreamStatus::Complete, result.diagnostic.category == XmlError::None);
    if result.diagnostic.category != XmlError::None { assert!(tokens.is_empty()); }
    result.diagnostic
}
fn at(source: &[u8], needle: &[u8]) -> usize { source.windows(needle.len()).rposition(|w| w == needle).unwrap() }
fn bytes(s: &str) -> Vec<u8> { s.as_bytes().to_vec() }

fn decode(source: &[u8], start: &[u8], len: usize) -> Result<String, usize> {
    let table = EntityTable::from_document(source).expect("table");
    let raw = XmlSpan { offset: at(source, start), length: len };
    let mut out = vec![0u8; 4096];
    let n = decode_attribute_value_with(source, raw, &table, &mut out)?;
    Ok(String::from_utf8(out[..n].to_vec()).unwrap())
}

#[test]
fn simple_entity_in_attribute_value_and_text_is_accepted_and_expands() {
    let attr = b"<!DOCTYPE svg [<!ENTITY ns \"http://www.w3.org/2000/svg\">]><svg xmlns=\"&ns;\" a=\"1&amp;&ns;2\"/>";
    assert_eq!(diagnostic(attr), XmlDiagnostic::default());
    assert_eq!(decode(attr, b"&ns;\" a", 4).unwrap(), "http://www.w3.org/2000/svg");
    assert_eq!(decode(attr, b"1&amp;&ns;2", 11).unwrap(), "1&http://www.w3.org/2000/svg2");
    let text = b"<!DOCTYPE svg [<!ENTITY t 'he said &#34;hi&#35;&#x41;'>]><svg><text>a &t; b</text></svg>";
    assert_eq!(diagnostic(text), XmlDiagnostic::default());
    let (tokens, _) = tokenize(text, XmlLimits::default());
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Text && t.source.slice(text) == Some(&b"a &t; b"[..])));
}

#[test]
fn nested_entities_expand_up_to_depth_eight() {
    let src = b"<!DOCTYPE a [<!ENTITY x 'X'><!ENTITY y '[&x;&x;]'><!ENTITY z '(&y;)'>]><a v='&z;'/>";
    assert_eq!(diagnostic(src), XmlDiagnostic::default());
    assert_eq!(decode(src, b"&z;", 3).unwrap(), "([XX])");
    let mut chain = String::from("<!DOCTYPE a [<!ENTITY e0 'x'>");
    for i in 1..8 { chain.push_str(&format!("<!ENTITY e{i} '&e{};'>", i - 1)); }
    chain.push_str("]><a v='&e7;'/>");
    assert_eq!(diagnostic(chain.as_bytes()), XmlDiagnostic::default());
    assert_eq!(decode(chain.as_bytes(), b"&e7;", 4).unwrap(), "x");
}

#[test]
fn cycles_are_rejected_at_the_reference_with_a_typed_error() {
    let cases: [(&[u8], &[u8]); 3] = [
        (b"<!DOCTYPE a [<!ENTITY e '&e;'>]><a>&e;</a>", b"&e;</a>"),
        (b"<!DOCTYPE a [<!ENTITY e '&e;'>]><a x='&e;'/>", b"&e;'/>"),
        (b"<!DOCTYPE a [<!ENTITY p '&q;'><!ENTITY q '&p;'>]><a>&p;</a>", b"&p;</a>"),
    ];
    for (src, needle) in cases {
        assert_eq!(diagnostic(src), XmlDiagnostic { category: XmlError::EntityCycle, offset: at(src, needle) });
    }
}

#[test]
fn billion_laughs_is_rejected_quickly_by_depth_or_expansion_limit() {
    let started = Instant::now();
    let mut deep = String::from("<!DOCTYPE l [<!ENTITY lol0 'lol'>");
    for i in 1..=9 { deep.push_str(&format!("<!ENTITY lol{i} '&lol{p};&lol{p};&lol{p};&lol{p};&lol{p};&lol{p};&lol{p};&lol{p};&lol{p};&lol{p};'>", p = i - 1)); }
    let wide = format!("{deep}]><l>&lol9;</l>");
    let src = bytes(&wide);
    assert_eq!(diagnostic(&src), XmlDiagnostic { category: XmlError::EntityDepthLimit, offset: at(&src, b"&lol9;") });
    let mut shallow = String::from("<!DOCTYPE l [<!ENTITY lol0 'lol'>");
    for i in 1..=7 { shallow.push_str(&format!("<!ENTITY lol{i} '{}'>", format!("&lol{};", i - 1).repeat(10))); }
    shallow.push_str(&format!("]><l>&lol7;</l>"));
    let src = bytes(&shallow);
    assert_eq!(diagnostic(&src).category, XmlError::EntityExpansionLimit);
    // Empty replacement text must not hide a combinatorial visit count.
    let mut empty = String::from("<!DOCTYPE l [<!ENTITY e0 ''>");
    for i in 1..=7 { empty.push_str(&format!("<!ENTITY e{i} '{}'>", format!("&e{};", i - 1).repeat(50))); }
    empty.push_str("]><l>&e7;</l>");
    assert_eq!(diagnostic(empty.as_bytes()).category, XmlError::EntityExpansionLimit);
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
}

#[test]
fn document_total_expansion_is_capped_across_references() {
    let value = "v".repeat(60 * 1024);
    let refs = "&big;".repeat(MAX_ENTITY_EXPANDED_BYTES / value.len() + 1);
    let src = bytes(&format!("<!DOCTYPE a [<!ENTITY big '{value}'>]><a>{refs}</a>"));
    let d = diagnostic(&src);
    assert_eq!(d.category, XmlError::EntityExpansionLimit);
    assert_eq!(src[d.offset..].starts_with(b"&big;"), true);
}

#[test]
fn external_parameter_and_over_limit_declarations_are_rejected() {
    let sys = b"<!DOCTYPE a [<!ENTITY e SYSTEM \"file:///etc/passwd\">]><a>&e;</a>";
    assert_eq!(diagnostic(sys), XmlDiagnostic { category: XmlError::ExternalEntity, offset: at(sys, b"SYSTEM") });
    let public = b"<!DOCTYPE a [<!ENTITY e PUBLIC \"-//X//\" \"http://x/y\">]><a/>";
    assert_eq!(diagnostic(public), XmlDiagnostic { category: XmlError::ExternalEntity, offset: at(public, b"PUBLIC") });
    let param = b"<!DOCTYPE a [<!ENTITY % p \"x\"><!ENTITY e \"%p;\">]><a/>";
    assert_eq!(diagnostic(param), XmlDiagnostic { category: XmlError::UnsupportedDtd, offset: at(param, b"% p") });
    let pref = b"<!DOCTYPE a [<!ENTITY e \"%p;\">]><a/>";
    assert_eq!(diagnostic(pref), XmlDiagnostic { category: XmlError::UnsupportedDtd, offset: at(pref, b"%p;") });
    let markup = b"<!DOCTYPE a [<!ENTITY e \"<b/>\">]><a>&e;</a>";
    assert_eq!(diagnostic(markup).category, XmlError::UnsupportedDtd);
    let big = bytes(&format!("<!DOCTYPE a [<!ENTITY e '{}'>]><a/>", "x".repeat(MAX_ENTITY_VALUE_BYTES + 1)));
    assert_eq!(diagnostic(&big), XmlDiagnostic { category: XmlError::EntityValueLimit, offset: at(&big, b"'x") + 1 });
    let ok = bytes(&format!("<!DOCTYPE a [<!ENTITY e '{}'>]><a>&e;</a>", "x".repeat(MAX_ENTITY_VALUE_BYTES)));
    assert_eq!(diagnostic(&ok), XmlDiagnostic::default());
    let many = |n: usize| { let mut s = String::from("<!DOCTYPE a ["); for i in 0..n { s.push_str(&format!("<!ENTITY e{i} 'v'>")); } s.push_str("]><a/>"); bytes(&s) };
    assert_eq!(diagnostic(&many(MAX_ENTITIES)), XmlDiagnostic::default());
    let over = many(MAX_ENTITIES + 1);
    assert_eq!(diagnostic(&over), XmlDiagnostic { category: XmlError::EntityCountLimit, offset: at(&over, b"<!ENTITY e64") });
}

#[test]
fn undeclared_and_predefined_references_keep_their_behavior() {
    for src in [&b"<!DOCTYPE a [<!ENTITY e 'x'>]><a>&e</a>"[..], b"<!DOCTYPE a [<!ENTITY e 'x'>]><a>&nope;</a>", b"<a>&nope;</a>", b"<!DOCTYPE a [<!ENTITY e 'x'>]><a>&e</a>"] {
        assert_eq!(diagnostic(src).category, XmlError::MalformedSyntax);
    }
    let src = b"<a v='&amp;&lt;&gt;&quot;&apos;&#65;'>&amp;</a>";
    assert_eq!(diagnostic(src), XmlDiagnostic::default());
    assert_eq!(decode(src, b"&amp;&lt;", 30).unwrap(), "&<>\"'A");
    assert_eq!(decode(b"<a v='&zz;'/>", b"&zz;", 4), Err(0));
}

#[test]
fn decode_fails_closed_on_unvalidated_tables() {
    let src = b"<!DOCTYPE a [<!ENTITY e '&e;'>]><a v='&e;'/>";
    assert!(decode(src, b"&e;'/>", 3).is_err());
    let mut out = [0u8; 2];
    let src = b"<!DOCTYPE a [<!ENTITY e 'long'>]><a v='&e;'/>";
    let table = EntityTable::from_document(src).unwrap();
    assert!(decode_attribute_value_with(src, XmlSpan { offset: at(src, b"&e;'/>"), length: 3 }, &table, &mut out).is_err());
}

#[test]
fn doctype_with_external_id_and_no_entities_is_unchanged() {
    let src = b"<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.0//EN\" \"http://www.w3.org/x.dtd\" [\n<!ENTITY ns \"u\">\n]>\n<svg xmlns=\"&ns;\"/>";
    assert_eq!(diagnostic(src), XmlDiagnostic::default());
    assert!(EntityTable::from_document(b"<svg/>").unwrap().is_empty());
}
