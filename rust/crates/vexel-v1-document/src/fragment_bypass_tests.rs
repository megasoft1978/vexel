use crate::{V1Document, V1DocumentError};

#[test]
fn encoding_and_namespace_bypasses_are_refused() {
    let mut doc = V1Document::new(100, 100).unwrap();
    let before = doc.source().unwrap();
    for xml in [
        r#"<use id="u" href="&#x6A;avascript:alert(1)"/>"#,
        r#"<use id="u" href="&#106;avascript:alert(1)"/>"#,
        r#"<rect id="r" fill="url(&#35;a)" stroke="u&#114;l(http://x)"/>"#,
        r#"<rect id="r" fill="u&#114;l(http://x)"/>"#,
        r#"<rect id="r" fill="\75rl(http://x)"/>"#,
        r#"<rect id="r" fill="URL (http://x)"/>"#,
        r#"<rect id="r" fill="u/**/rl(http://x)"/>"#,
        r#"<use id="u" xmlns:x="http://www.w3.org/1999/xlink" x:href="http://x/y"/>"#,
        r#"<use id="u" xlink:HREF="http://x/y"/>"#,
        r#"<use id="u" HREF="http://x/y"/>"#,
        r#"<g id="g" xmlns:x="http://evil/"/>"#,
        r#"<g id="g" xmlns="http://evil/"/>"#,
        r#"<g id="g" foo:bar="1"/>"#,
        r#"<rect id="r" style="@import 'http://x';fill:red"/>"#,
        r#"<rect id="r" style="fill:&#x75;rl(http://x)"/>"#,
        r#"<rect id="r" style="behavior:expression(alert(1))"/>"#,
        r#"<rect id="r" style="background:java&#x73;cript:x"/>"#,
        r#"<rect id="r" fill="&bogus;"/>"#,
        r#"<g id="g" xlink:onclick="x()"/>"#,
    ] {
        assert!(doc.add_fragment("root", xml.as_bytes()).is_err(), "{xml}");
        assert_eq!(doc.source().unwrap(), before, "{xml}");
    }
    assert_eq!(
        doc.add_fragment("root", br#"<image id="i" href="&#104;ttp://x/a.png"/>"#),
        Err(V1DocumentError::ExternalReference)
    );
}

#[test]
fn valid_references_still_pass() {
    let mut doc = V1Document::new(100, 100).unwrap();
    doc.add_fragment(
        "root",
        br##"<g id="ok" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xml:space="preserve"><defs><linearGradient id="g"><stop offset="0" stop-color="#fff"/></linearGradient><filter id="f"/><clipPath id="c"/><mask id="m"/><marker id="mk"/></defs><rect id="r1" fill="url(#g)" filter="url(#f)" clip-path="url(#c)" mask="url(#m)" marker-end="url(&quot;#mk&quot;)"/><rect id="r2" fill="url( '#g' )" style="fill:url(#g)"/><use id="u1" href="#r1"/><use id="u2" xlink:href="#r1"/><image id="im" href="data:image/png;base64,AAAA"/><image id="im2" xlink:href="data:image/png;base64,AAAA"/></g>"##,
    )
    .unwrap();
}
