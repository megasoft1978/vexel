use super::*;
use vexel_scene::{ErrorKind, FilterInput, FilterKind};
use vexel_xml::{XmlLimits, tokenize};
fn parsed(xml: &[u8], value: &[u8]) -> (Vec<XmlToken>, XmlSpan, usize) {
    let (tokens, result) = tokenize(xml, XmlLimits::default());
    assert_eq!(result.diagnostic.category, vexel_xml::XmlError::None);
    let offset = xml.windows(value.len()).position(|w| w == value).unwrap();
    let span = XmlSpan {
        offset,
        length: value.len(),
    };
    let element = tokens
        .iter()
        .position(|t| {
            t.kind == TokenKind::StartElement && span_bytes(xml, t.local_name) == Some(b"filter")
        })
        .unwrap();
    (tokens, span, element)
}
#[test]
fn filter_supported_primitives_and_named_results() {
    let xml=b"<svg style='fill:url(#f)'><filter id='f'><feFlood flood-color='#ff0000' result='a'/><feBlend in='a' in2='SourceAlpha' mode='multiply'/></filter></svg>";
    let (tokens, value, element) = parsed(xml, b"url(#f)");
    let p = parse_filter(xml, &tokens, value, element).unwrap();
    assert_eq!(p.primitives.len(), 2);
    assert_eq!(p.primitives[0].kind, FilterKind::Flood);
    assert_eq!(p.primitives[0].rgba, [255, 0, 0, 255]);
    assert_eq!(p.primitives[1].input, FilterInput::Result(0));
    assert_eq!(p.primitives[1].input2, FilterInput::Alpha);
    assert_eq!(p.primitives[1].mode, 1);
}
#[test]
fn filter_merge_and_matrix_defaults() {
    let xml=b"<svg style='fill:url(#f)'><filter id='f'><feMerge><feMergeNode/><feMergeNode in='SourceAlpha'/></feMerge><feColorMatrix/></filter></svg>";
    let (tokens, value, element) = parsed(xml, b"url(#f)");
    let p = parse_filter(xml, &tokens, value, element).unwrap();
    assert_eq!(p.primitives[0].merge_count, 2);
    assert_eq!(
        p.primitives[1].values,
        [
            1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0.
        ]
    );
}
#[test]
fn unsupported_filter_keeps_specific_error() {
    let xml = b"<svg style='fill:url(#f)'><filter id='f'><feTurbulence/></filter></svg>";
    let (tokens, value, element) = parsed(xml, b"url(#f)");
    assert_eq!(
        parse_filter(xml, &tokens, value, element).unwrap_err().kind,
        ErrorKind::UnsupportedFeTurbulence
    );
}
#[test]
fn filter_rejects_unknown_result_and_invalid_region() {
    let xml=b"<svg style='fill:url(#f)'><filter id='f' width='0'><feOffset in='missing'/></filter></svg>";
    let (tokens, value, element) = parsed(xml, b"url(#f)");
    assert_eq!(
        parse_filter(xml, &tokens, value, element).unwrap_err().kind,
        ErrorKind::Number
    );
    let xml = b"<svg style='fill:url(#f)'><filter id='f'><feOffset in='missing'/></filter></svg>";
    let (tokens, value, element) = parsed(xml, b"url(#f)");
    assert_eq!(
        parse_filter(xml, &tokens, value, element).unwrap_err().kind,
        ErrorKind::Unsupported
    );
}
#[test]
fn local_url_bounds_and_whitespace() {
    let source = b"  url(  #x  ) ";
    let s = XmlSpan {
        offset: 0,
        length: source.len(),
    };
    let id = parse_local_url(source, s).unwrap();
    assert_eq!(span_bytes(source, id), Some(&b"x"[..]));
    assert_eq!(
        parse_local_url(
            b"url(https://example.test)",
            XmlSpan {
                offset: 0,
                length: 25
            }
        )
        .unwrap_err()
        .kind,
        ErrorKind::Unsupported
    );
}
#[test]
fn filter_color_uses_full_css_named_and_function_grammar() {
    assert_eq!(
        parse_color(b"Rebeccapurple", vexel_base::Span { start: 0, end: 13 }),
        Some([102, 51, 153, 255])
    );
    assert_eq!(
        parse_color(b"hsl(120 100% 50%)", vexel_base::Span { start: 0, end: 17 }),
        Some([0, 255, 0, 255])
    );
}
#[test]
fn filter_number_lists_accept_signed_adjacency_and_reject_extra_values() {
    let xml=b"<svg style='filter:url(#f)'><filter id='f'><feGaussianBlur stdDeviation='1-2'/></filter></svg>";
    let (tokens, value, element) = parsed(xml, b"url(#f)");
    let p = parse_filter(xml, &tokens, value, element).unwrap_err();
    assert_eq!(p.kind, ErrorKind::Limit(vexel_scene::LimitKind::Filters));
    assert_eq!(
        super::common::number_list(
            b"1-2",
            XmlSpan {
                offset: 0,
                length: 3
            },
            2
        )
        .unwrap(),
        vec![1., -2.]
    );
}

#[test]
fn marker_attribute_parser_keeps_units_orient_and_viewbox_mapping() {
    let xml=b"<svg><marker markerWidth='4' markerHeight='3' refX='2' refY='1' markerUnits='userSpaceOnUse' orient='auto-start-reverse' viewBox='0 0 2 1'/></svg>";
    let (tokens, result) = tokenize(xml, XmlLimits::default());
    assert_eq!(result.diagnostic.category, vexel_xml::XmlError::None);
    let element = tokens
        .iter()
        .position(|t| {
            t.kind == TokenKind::EmptyElement && span_bytes(xml, t.local_name) == Some(b"marker")
        })
        .unwrap();
    let p = parse_marker_attributes(xml, &tokens, element).unwrap();
    assert!(!p.stroke_width_units);
    assert_eq!(p.orient, MarkerOrient::AutoStartReverse);
    assert_eq!(p.child_viewport, [2., 1.]);
    assert_eq!([p.ref_x, p.ref_y], [4., 2.5]);
}

#[test]
fn controlled_element_scan_stops_at_its_work_budget() {
    let xml = b"<svg><g><path/></g></svg>";
    let (tokens, result) = tokenize(xml, XmlLimits::default());
    assert_eq!(result.diagnostic.category, vexel_xml::XmlError::None);
    let element = tokens
        .iter()
        .position(|t| {
            t.kind == TokenKind::StartElement && span_bytes(xml, t.local_name) == Some(b"g")
        })
        .unwrap();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let mut control = vexel_base::BudgetControl::new(&cancel, 0);
    let error = matching_end_controlled(&tokens, element, &mut control).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(vexel_scene::LimitKind::Work));
}
