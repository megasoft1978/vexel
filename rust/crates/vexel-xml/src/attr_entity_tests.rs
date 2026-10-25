use crate::{expand_attribute_entities, tokenize, XmlError, XmlLimits};

fn expand(s: &str) -> Result<Option<String>, (XmlError, usize)> {
    expand_attribute_entities(s.as_bytes()).map(|o| o.map(|v| String::from_utf8(v).unwrap()))
}

#[test]
fn declared_entities_in_attribute_values_are_expanded_and_the_result_tokenizes() {
    let src = "<!DOCTYPE svg [<!ENTITY c \"red\"><!ENTITY ns 'http://www.w3.org/2000/svg'><!ENTITY w '&c;-&amp;'>]><svg xmlns=\"&ns;\"><rect fill=\"&c;\" stroke='a&w;b' x=\"1&amp;\"/></svg>";
    let out = expand(src).unwrap().unwrap();
    assert!(out.contains("xmlns=\"http://www.w3.org/2000/svg\""), "{out}");
    assert!(out.contains("fill=\"red\""), "{out}");
    assert!(out.contains("stroke='ared-&amp;b'"), "{out}");
    assert!(out.contains("x=\"1&amp;\""), "{out}");
    let (_, result) = tokenize(out.as_bytes(), XmlLimits::default());
    assert_eq!(result.diagnostic.category, XmlError::None);
}

#[test]
fn replacement_text_is_escaped_and_whitespace_normalized() {
    let src = "<!DOCTYPE a [<!ENTITY q 'say &quot;hi&quot; &lt;b&gt; it&apos;s\t\n'>]><a v=\"&q;\"/>";
    let out = expand(src).unwrap().unwrap();
    assert!(out.contains("v=\"say &quot;hi&quot; &lt;b> it&apos;s  \""), "{out}");
    let (_, result) = tokenize(out.as_bytes(), XmlLimits::default());
    assert_eq!(result.diagnostic.category, XmlError::None);
}

#[test]
fn nothing_to_expand_returns_none() {
    assert_eq!(expand("<svg a=\"1\"/>"), Ok(None));
    assert_eq!(expand("<!DOCTYPE a [<!ENTITY e 'x'>]><a v=\"&amp;\">&e;</a>"), Ok(None));
}

#[test]
fn undefined_recursive_and_exploding_references_have_typed_errors() {
    let undefined = "<!DOCTYPE a [<!ENTITY e 'x'>]><a v=\"&nope;\"/>";
    assert_eq!(expand(undefined).unwrap_err().0, XmlError::MalformedSyntax);
    let cycle = "<!DOCTYPE a [<!ENTITY p '&q;'><!ENTITY q '&p;'>]><a v=\"&p;\"/>";
    assert_eq!(expand(cycle).unwrap_err().0, XmlError::EntityCycle);
    let mut laughs = String::from("<!DOCTYPE l [<!ENTITY e0 'lollollol'>");
    for i in 1..=7 { laughs.push_str(&format!("<!ENTITY e{i} '{}'>", format!("&e{};", i - 1).repeat(10))); }
    laughs.push_str("]><l v=\"&e7;\"/>");
    let started = std::time::Instant::now();
    assert_eq!(expand(&laughs).unwrap_err().0, XmlError::EntityExpansionLimit);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    let deep = {
        let mut s = String::from("<!DOCTYPE a [<!ENTITY e0 'x'>");
        for i in 1..=9 { s.push_str(&format!("<!ENTITY e{i} '&e{};'>", i - 1)); }
        s.push_str("]><a v=\"&e9;\"/>");
        s
    };
    assert_eq!(expand(&deep).unwrap_err().0, XmlError::EntityDepthLimit);
}
