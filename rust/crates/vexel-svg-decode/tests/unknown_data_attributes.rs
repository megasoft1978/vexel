mod support;
use support::*;
use vexel_scene::IgnoredKind;

fn paths(count: usize, attribute: &str) -> String {
    let mut out = String::from(r##"<svg width="8" height="8">"##);
    for _ in 0..count {
        out.push_str(&format!(r##"<path d="M0 0 L1 1" {attribute}/>"##));
    }
    out.push_str("</svg>");
    out
}

#[test]
fn many_data_region_attributes_decode_without_a_count_cliff() {
    for count in [200, 2000] {
        let source = paths(count, r#"data-region="a""#);
        let decoded =
            decode(&source).unwrap_or_else(|error| panic!("count {count}: {:?}", error.kind));
        assert_eq!(
            decoded.root().ignored(IgnoredKind::UnknownAttribute).count,
            count
        );
    }
}

#[test]
fn class_only_control_decodes_with_no_ignored_attributes() {
    for count in [200, 2000] {
        let source = paths(count, r#"class="a""#);
        let decoded =
            decode(&source).unwrap_or_else(|error| panic!("count {count}: {:?}", error.kind));
        assert_eq!(
            decoded.root().ignored(IgnoredKind::UnknownAttribute).count,
            0
        );
    }
}
