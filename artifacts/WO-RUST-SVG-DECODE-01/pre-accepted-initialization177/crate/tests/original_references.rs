mod support;
use support::*;
use vexel_scene::{ErrorKind, LimitKind};
#[test]
fn original_use_transform_order_and_host_source_maps() {
    let inputs = [
        r##"<svg width="16" height="16"><defs><rect id="r" width="1" height="1"/></defs><use href="#r" x="4" y="3" transform="scale(2)"/></svg>"##,
        r##"<svg width="16" height="16"><defs><rect id="r" width="2" height="1"/></defs><g transform="translate(8 0)"><use href="#r" x="2" y="1" transform="rotate(90)"/></g></svg>"##,
        r##"<svg width="16" height="16"><defs><symbol id="s" viewBox="0 0 2 2"><rect width="1" height="1"/></symbol></defs><use href="#s" x="2" y="1" width="4" height="4" transform="scale(2)"/></svg>"##,
    ];
    for (s, (x, y)) in inputs.into_iter().zip([(8.0, 6.0), (7.0, 2.0), (4.0, 2.0)]) {
        let g = decode(s).unwrap();
        let v = g.root();
        assert!(!v.edges.is_empty());
        assert!((v.edges[0].a.x - x).abs() < 1e-9);
        assert!((v.edges[0].a.y - y).abs() < 1e-9);
        let map = v.shapes[0].source;
        assert_eq!(
            &s[map.element.start..map.element.end],
            &s[s.find("<use").unwrap()
                ..s.find("<use").unwrap() + s[s.find("<use").unwrap()..].find("/>").unwrap() + 2]
        );
        assert!(map.transform_node.is_some());
    }
}
#[test]
fn original_local_reference_rejection_cases() {
    for s in [
        r##"<svg width="8" height="8"><use href="#missing"/></svg>"##,
        r##"<svg width="8" height="8"><use href="https://example.invalid/a.svg#x"/></svg>"##,
        r##"<svg width="8" height="8"><g id="x"><use href="#x"/></g></svg>"##,
        r##"<svg width="8" height="8"><g id="x"/><rect id="x" width="1" height="1"/></svg>"##,
        r##"<svg width="8" height="8"><use xlink:href="#x"/><rect id="x" width="1" height="1"/></svg>"##,
        r##"<svg width="8" height="8"><defs><g id="x"><use xlink:href="#y"/></g><rect id="y" width="1" height="1"/></defs><use href="#x"/></svg>"##,
        r##"<svg width="8" height="8"><defs><g id="a"><use href="#b"/></g><g id="b"><use href="#a"/></g></defs><use href="#a"/></svg>"##,
    ] {
        let e = decode(s).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Unsupported);
        assert!(e.offset < s.len());
    }
}
#[test]
fn original_reference_exact_bytes_and_depth_boundaries() {
    let fixed = "<g id=\"x\"><!----></g>".len();
    for (size, good) in [(64 * 1024, true), (64 * 1024 + 1, false)] {
        let s=format!("<svg width=\"1\" height=\"1\"><defs><g id=\"x\"><!--{}--></g></defs><use href=\"#x\"/></svg>","a".repeat(size-fixed));
        let result = decode(&s);
        if good {
            assert!(result.unwrap().root().shapes.is_empty())
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                ErrorKind::Limit(LimitKind::References)
            )
        }
    }
    for count in [16, 17] {
        let mut s = String::from("<svg width=\"8\" height=\"8\"><defs>");
        for i in 0..count {
            s.push_str(&format!("<g id=\"d{i}\">"));
            if i + 1 < count {
                s.push_str(&format!("<use href=\"#d{}\"/>", i + 1))
            } else {
                s.push_str("<rect width=\"1\" height=\"1\"/>")
            }
            s.push_str("</g>");
        }
        s.push_str("</defs><use href=\"#d0\"/></svg>");
        let result = decode(&s);
        if count == 16 {
            assert_eq!(result.unwrap().root().shapes.len(), 1)
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                ErrorKind::Limit(LimitKind::References)
            )
        }
    }
}
#[test]
fn original_mapped_error_offset_and_definition_only() {
    let s = r##"<svg width="8" height="8"><defs><g id="bad"><foreign/></g></defs><use href="#bad"/></svg>"##;
    let e = decode(s).unwrap_err();
    assert_eq!(
        (e.kind, e.offset),
        (ErrorKind::Unsupported, offset(s, "foreign"))
    );
    let s = r##"<svg width="8" height="8"><defs><g id="x"><rect width="2" height="2"/></g><use href="#x"/></defs><rect width="1" height="1"/></svg>"##;
    assert_eq!(decode(s).unwrap().root().shapes.len(), 1);
}
#[test]
fn original_w5_switch_language_data() {
    let source = r##"<svg width="12" height="6"><a href="https://invalid.example/"><switch><rect systemLanguage="it" width="12" height="6" fill="red"/><rect systemLanguage="en" width="12" height="6" fill="blue"/><rect width="12" height="6" fill="green"/></switch></a></svg>"##;
    for (language, expected) in [
        (b"en".as_slice(), [0, 0, 255, 255]),
        (b"it", [255, 0, 0, 255]),
        (b"de", [0, 128, 0, 255]),
    ] {
        let g = decode_with(source, language, &mut RejectOperations).unwrap();
        assert_eq!(g.root().viewport.width, 12);
        assert_eq!(g.root().viewport.height, 6);
        assert_eq!(g.root().shapes.len(), 1);
        assert_eq!(g.root().edge_colors[0], expected);
    }
}
#[test]
fn original_use_instance_boundary_and_aggregate_byte_limit() {
    let prefix = "<svg width=\"1\" height=\"1\"><defs><g id=\"g\"/></defs>";
    let shape = "<use href=\"#g\"/>";
    for (n, good) in [(65536, true), (65537, false)] {
        let s = format!("{prefix}{}</svg>", shape.repeat(n));
        let result = decode(&s);
        if good {
            assert!(result.unwrap().root().shapes.is_empty())
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                ErrorKind::Limit(LimitKind::References)
            )
        }
    }
    let s=format!("<svg width=\"8\" height=\"8\"><defs><g id=\"x\"><!--{}--><rect width=\"1\" height=\"1\"/></g></defs>{}</svg>","b".repeat(48*1024),"<use href=\"#x\"/>".repeat(22));
    assert_eq!(
        decode(&s).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::References)
    );
}
