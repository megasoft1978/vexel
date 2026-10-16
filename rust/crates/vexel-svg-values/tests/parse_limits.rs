use vexel_base::Span;
use vexel_svg_values::{
    MAX_SOURCE_BYTES, NumberError, ValueError, ValueLimits, parse_length, parse_length_with_limits,
    parse_number, parse_number_list, parse_number_list_with_limits, parse_number_with_limits,
};

fn find(source: &[u8], needle: &[u8]) -> Span {
    let start = source
        .windows(needle.len())
        .position(|part| part == needle)
        .unwrap();
    Span {
        start,
        end: start + needle.len(),
    }
}

fn limits(source_bytes: usize) -> ValueLimits {
    ValueLimits {
        source_bytes,
        attributes: 4,
    }
}

#[test]
fn larger_policy_parses_small_spans_inside_oversized_original_sources() {
    let number = b"12.5";
    let list = b"1 2 3";
    let length = b"3px";
    let mut source = vec![b' '; MAX_SOURCE_BYTES + 1];
    let number_at = source.len();
    source.extend_from_slice(number);
    source.push(b' ');
    let list_at = source.len();
    source.extend_from_slice(list);
    source.push(b' ');
    let length_at = source.len();
    source.extend_from_slice(length);

    let number_span = Span {
        start: number_at,
        end: number_at + number.len(),
    };
    let list_span = Span {
        start: list_at,
        end: list_at + list.len(),
    };
    let length_span = Span {
        start: length_at,
        end: length_at + length.len(),
    };
    let policy = limits(source.len());

    assert_eq!(
        parse_number(&source, number_span),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_number_list(&source, list_span, 3),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_length(&source, length_span, 100.0),
        Err(ValueError::SourceLimit)
    );

    assert_eq!(
        parse_number_with_limits(&source, number_span, policy),
        parse_number(number, find(number, number))
    );
    assert_eq!(
        parse_number_list_with_limits(&source, list_span, 3, policy),
        parse_number_list(list, find(list, list), 3)
    );
    assert_eq!(
        parse_length_with_limits(&source, length_span, 100.0, policy),
        parse_length(length, find(length, length), 100.0)
    );
}

#[test]
fn exact_default_source_boundary_and_default_policy_match_legacy_apis() {
    let number = b"12.5";
    let list = b"1 2 3";
    let length = b"3px";
    let payload_len = number.len() + 1 + list.len() + 1 + length.len();
    let mut source = vec![b' '; MAX_SOURCE_BYTES - payload_len];
    let number_at = source.len();
    source.extend_from_slice(number);
    source.push(b' ');
    let list_at = source.len();
    source.extend_from_slice(list);
    source.push(b' ');
    let length_at = source.len();
    source.extend_from_slice(length);
    assert_eq!(source.len(), MAX_SOURCE_BYTES);

    let number_span = Span {
        start: number_at,
        end: number_at + number.len(),
    };
    let list_span = Span {
        start: list_at,
        end: list_at + list.len(),
    };
    let length_span = Span {
        start: length_at,
        end: length_at + length.len(),
    };
    let default = ValueLimits::default();
    assert_eq!(
        parse_number_with_limits(&source, number_span, default),
        parse_number(&source, number_span)
    );
    assert_eq!(
        parse_number_list_with_limits(&source, list_span, 3, default),
        parse_number_list(&source, list_span, 3)
    );
    assert_eq!(
        parse_length_with_limits(&source, length_span, 100.0, default),
        parse_length(&source, length_span, 100.0)
    );
}

#[test]
fn smaller_limits_precede_span_count_and_value_errors_but_preserve_them_under_cap() {
    let source = b"x1,,3rem";
    let small = limits(source.len() - 1);
    let invalid_span = Span {
        start: source.len(),
        end: source.len() + 1,
    };

    assert_eq!(
        parse_number_with_limits(source, invalid_span, small),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_number_list_with_limits(source, invalid_span, 0, small),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_number_list_with_limits(source, Span { start: 1, end: 4 }, 2, small),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_number_with_limits(source, Span { start: 0, end: 1 }, small),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_length_with_limits(source, invalid_span, 10.0, small),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_number_list_with_limits(source, Span { start: 0, end: 1 }, 0, small),
        Err(ValueError::SourceLimit)
    );
    assert_eq!(
        parse_length_with_limits(
            source,
            Span {
                start: 4,
                end: source.len()
            },
            10.0,
            small
        ),
        Err(ValueError::SourceLimit)
    );

    let allowed = limits(source.len());
    assert_eq!(
        parse_number_with_limits(source, invalid_span, allowed),
        Err(ValueError::InvalidSpan)
    );
    assert_eq!(
        parse_number_list_with_limits(source, invalid_span, 0, allowed),
        Err(ValueError::InvalidSpan)
    );
    assert_eq!(
        parse_length_with_limits(source, invalid_span, 10.0, allowed),
        Err(ValueError::InvalidSpan)
    );
    assert_eq!(
        parse_number_with_limits(source, Span { start: 0, end: 1 }, allowed),
        Err(ValueError::Number(NumberError { offset: 0 }))
    );
    let embedded = b"prefix x";
    assert_eq!(
        parse_number_with_limits(embedded, Span { start: 7, end: 8 }, limits(embedded.len())),
        Err(ValueError::Number(NumberError { offset: 7 }))
    );
    assert_eq!(
        parse_number_list_with_limits(source, Span { start: 1, end: 4 }, 0, allowed),
        Err(ValueError::Number(NumberError { offset: 1 }))
    );
    assert_eq!(
        parse_number_list_with_limits(source, Span { start: 1, end: 4 }, 2, allowed),
        Err(ValueError::Number(NumberError { offset: 3 }))
    );
    assert_eq!(
        parse_length_with_limits(source, Span { start: 4, end: 8 }, 10.0, allowed),
        Err(ValueError::UnsupportedUnit { offset: 5 })
    );
}
