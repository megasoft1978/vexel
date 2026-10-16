use vexel_base::Span;
use vexel_svg_values::{
    Attribute, AttributeList, MAX_SOURCE_BYTES, ValueLimits, parse_root_viewport,
};

#[test]
fn large_original_source_policy_flows_through_root_viewbox_and_aspect() {
    let payload = b"svg width 47.2 height 24 viewBox 10 20 12 6 preserveAspectRatio xMidYMid meet";
    let prefix = MAX_SOURCE_BYTES + 9;
    let mut source = vec![b' '; prefix];
    source.extend_from_slice(payload);
    let span_of = |needle: &[u8]| {
        let start = source
            .windows(needle.len())
            .position(|window| window == needle)
            .unwrap();
        Span {
            start,
            end: start + needle.len(),
        }
    };
    let element = span_of(b"svg");
    let attrs = [
        Attribute {
            name: span_of(b"width"),
            value: span_of(b"47.2"),
            prefix: None,
        },
        Attribute {
            name: span_of(b"height"),
            value: span_of(b"24"),
            prefix: None,
        },
        Attribute {
            name: span_of(b"viewBox"),
            value: span_of(b"10 20 12 6"),
            prefix: None,
        },
        Attribute {
            name: span_of(b"preserveAspectRatio"),
            value: span_of(b"xMidYMid meet"),
            prefix: None,
        },
    ];
    let limits = ValueLimits {
        source_bytes: source.len(),
        attributes: 4,
    };
    let list = AttributeList::new_with_limits(&source, element, &attrs, limits).unwrap();
    assert_eq!(list.limits(), limits);
    let root = parse_root_viewport(list).unwrap();
    assert_eq!((root.width, root.height), (48, 24));
    assert_eq!(
        (root.view_x, root.view_y, root.view_width, root.view_height),
        (10.0, 20.0, 12.0, 6.0)
    );
    assert!(root.preserve_aspect.set);
    assert_eq!(
        AttributeList::new_with_limits(
            &source,
            element,
            &attrs,
            ValueLimits {
                source_bytes: MAX_SOURCE_BYTES,
                attributes: 4
            }
        )
        .unwrap_err(),
        vexel_svg_values::AdapterError::SourceLimit
    );
}
