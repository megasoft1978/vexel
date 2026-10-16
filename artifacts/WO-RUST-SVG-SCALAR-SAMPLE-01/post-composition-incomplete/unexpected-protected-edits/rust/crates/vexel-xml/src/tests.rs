use super::*;

#[test]
fn xml_character_ranges_and_whitespace_match_xml_10() {
    for cp in [
        0x9, 0xa, 0xd, 0x20, 0xd7ff, 0xe000, 0xfffd, 0x10000, 0x10ffff,
    ] {
        assert!(is_xml_char(cp), "U+{cp:04X}");
    }
    for cp in [0, 8, 0xb, 0x1f, 0xd800, 0xdfff, 0xfffe, 0x10000 + 0x100000] {
        assert!(!is_xml_char(cp), "U+{cp:04X}");
    }
    for byte in [b' ', b'\t', b'\r', b'\n'] {
        assert!(is_xml_whitespace(byte));
    }
    for byte in [0, b'\x0b', b'\x0c', b'\x85'] {
        assert!(!is_xml_whitespace(byte));
    }
}

#[test]
fn name_start_and_name_char_ranges_match_xml_10() {
    for cp in [
        b':' as u32,
        b'_' as u32,
        b'A' as u32,
        0xc0,
        0xd6,
        0xd8,
        0xf6,
        0xf8,
        0x2ff,
        0x370,
        0x37d,
        0x37f,
        0x1fff,
        0x200c,
        0x200d,
        0x2070,
        0x218f,
        0x2c00,
        0x2fef,
        0x3001,
        0xd7ff,
        0xf900,
        0xfdcf,
        0xfdf0,
        0xfffd,
        0x10000,
        0xeffff,
    ] {
        assert!(is_name_start(cp), "U+{cp:04X}");
        assert!(is_name_char(cp), "U+{cp:04X}");
    }
    for cp in [
        0xb7,
        b'-' as u32,
        b'.' as u32,
        b'0' as u32,
        0x300,
        0x36f,
        0x203f,
        0x2040,
    ] {
        assert!(!is_name_start(cp), "U+{cp:04X}");
        assert!(is_name_char(cp), "U+{cp:04X}");
    }
    for cp in [
        0x2f,
        0x37e,
        0x200e,
        0xd800,
        0xfdd0,
        0xf0000,
        0xf0000 + 0x10000,
    ] {
        assert!(!is_name_start(cp), "U+{cp:04X}");
        assert!(!is_name_char(cp), "U+{cp:04X}");
    }
}

#[test]
fn every_unicode_scalar_matches_the_xml_10_range_tables() {
    const CHAR_RANGES: &[(u32, u32)] = &[
        (0x9, 0xa),
        (0xd, 0xd),
        (0x20, 0xd7ff),
        (0xe000, 0xfffd),
        (0x10000, 0x10ffff),
    ];
    const NAME_START_RANGES: &[(u32, u32)] = &[
        (0x3a, 0x3a),
        (0x41, 0x5a),
        (0x5f, 0x5f),
        (0x61, 0x7a),
        (0xc0, 0xd6),
        (0xd8, 0xf6),
        (0xf8, 0x2ff),
        (0x370, 0x37d),
        (0x37f, 0x1fff),
        (0x200c, 0x200d),
        (0x2070, 0x218f),
        (0x2c00, 0x2fef),
        (0x3001, 0xd7ff),
        (0xf900, 0xfdcf),
        (0xfdf0, 0xfffd),
        (0x10000, 0xeffff),
    ];
    const NAME_EXTRA_RANGES: &[(u32, u32)] = &[
        (0x2d, 0x2e),
        (0x30, 0x39),
        (0xb7, 0xb7),
        (0x300, 0x36f),
        (0x203f, 0x2040),
    ];
    for cp in 0..=0x10ffff {
        let expected_char = CHAR_RANGES
            .iter()
            .any(|&(start, end)| (start..=end).contains(&cp));
        let expected_start = NAME_START_RANGES
            .iter()
            .any(|&(start, end)| (start..=end).contains(&cp));
        let expected_name = expected_start
            || NAME_EXTRA_RANGES
                .iter()
                .any(|&(start, end)| (start..=end).contains(&cp));
        assert_eq!(is_xml_char(cp), expected_char, "Char U+{cp:04X}");
        assert_eq!(
            is_name_start(cp),
            expected_start,
            "NameStartChar U+{cp:04X}"
        );
        assert_eq!(is_name_char(cp), expected_name, "NameChar U+{cp:04X}");
    }
}

#[test]
fn utf8_decoder_accepts_scalars_and_preserves_position_on_bad_input() {
    let valid: &[(&[u8], u32)] = &[
        (b"A", 0x41),
        (&[0xc2, 0xa2], 0xa2),
        (&[0xe2, 0x82, 0xac], 0x20ac),
        (&[0xf0, 0x9f, 0x98, 0x80], 0x1f600),
    ];
    for &(bytes, cp) in valid {
        assert_eq!(utf8_next(bytes, 0), Some((cp, bytes.len())));
        assert_eq!(utf8_next(bytes, bytes.len()), None);
    }
    for bytes in [
        &[0x80][..],
        &[0xc0, 0xaf],
        &[0xc2],
        &[0xc2, b'A'],
        &[0xe0, 0x80, 0x80],
        &[0xed, 0xa0, 0x80],
        &[0xf0, 0x80, 0x80, 0x80],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
    ] {
        assert_eq!(utf8_next(bytes, 0), None, "{bytes:?}");
    }
}

fn collect_with_budget(
    source: &[u8],
    budget: usize,
) -> (ReaderStatus, ReaderProgress, Vec<ReaderTag>) {
    let mut reader = XmlReader::new(source).expect("within source limit");
    let mut tags = Vec::new();
    loop {
        let before = reader.progress().bytes_read;
        let status = reader.step(budget, |tag| {
            tags.push(tag);
            true
        });
        let progress = reader.progress();
        assert!(progress.bytes_read - before <= budget.min(MAX_READER_STEP_BYTES));
        if status != ReaderStatus::Ok {
            return (status, progress, tags);
        }
    }
}

#[test]
fn reader_preserves_byte_spans_and_emits_tags_across_every_chunk_size() {
    let source = b"<?xml version='1.0'?>\r\n<!--c--><?p data?><a:k x='v&amp;x' y=\"z\">x<![CDATA[z]]><b/></a:k>";
    let expected = [
        (
            TagKind::StartElement,
            b"<a:k x='v&amp;x' y=\"z\">".as_slice(),
            b"a:k".as_slice(),
            2,
        ),
        (
            TagKind::EmptyElement,
            b"<b/>".as_slice(),
            b"b".as_slice(),
            0,
        ),
        (
            TagKind::EndElement,
            b"</a:k>".as_slice(),
            b"a:k".as_slice(),
            0,
        ),
    ];
    for budget in [1, 2, 3, 7, 64, 65_537] {
        let (status, progress, tags) = collect_with_budget(source, budget);
        assert_eq!(status, ReaderStatus::Complete);
        assert!(progress.complete);
        assert_eq!(progress.bytes_read, source.len());
        assert_eq!(tags.len(), expected.len());
        for (tag, (kind, raw, name, attrs)) in tags.iter().zip(expected) {
            assert_eq!(tag.kind, kind);
            assert_eq!(tag.source.slice(source), Some(raw));
            assert_eq!(tag.name.slice(source), Some(name));
            assert_eq!(tag.attribute_count, attrs);
        }
    }
}

#[test]
fn reader_skips_doctype_without_expanding_or_interpreting_subset() {
    for source in [
        &b"<!DOCTYPE a><a/>"[..],
        &b"<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\"\n \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\"><a/>"[..],
        &b"<!DOCTYPE svg [\n<!ENTITY ns \"http://www.w3.org/2000/svg\">\n]><a/>"[..],
        &b"<!DOCTYPE svg [<!-- ]> \">\" --><!ENTITY e \"a>b]\"><?pi ]> ?>]><a/>"[..],
    ] {
        let (status, progress, tags) = collect_with_budget(source, 1);
        assert_eq!(status, ReaderStatus::Complete);
        assert_eq!(progress.diagnostic.category, XmlError::None);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].source.slice(source), Some(&b"<a/>"[..]));
    }
}

#[test]
fn reader_reports_stable_malformed_categories_offsets_and_callback_failure() {
    for (source, category, offset) in [
        (&b"<"[..], XmlError::IncompleteDocument, 1),
        (&b"<!ENTITY e 'x'>"[..], XmlError::UnsupportedDtd, 0),
        (&b"<!DOCTYPE a"[..], XmlError::IncompleteDocument, 11),
        (&b"<!DOCTYPEa><a/>"[..], XmlError::MalformedSyntax, 9),
        (&b"<![CDATAx"[..], XmlError::MalformedSyntax, 0),
    ] {
        let (status, progress, _) = collect_with_budget(source, 64);
        assert_eq!(status, ReaderStatus::Malformed, "{source:?}");
        assert_eq!(progress.diagnostic.category, category, "{source:?}");
        assert_eq!(progress.diagnostic.offset, offset, "{source:?}");
    }
    let source = b"<a/>";
    let mut reader = XmlReader::new(source).unwrap();
    assert_eq!(
        reader.step(source.len(), |_| false),
        ReaderStatus::Malformed
    );
    let progress = reader.progress();
    assert_eq!(progress.diagnostic.category, XmlError::OutputExhausted);
    assert_eq!(progress.diagnostic.offset, 0);
}

#[test]
fn reader_accepts_4096_byte_names_and_rejects_4097_at_name_start_offset() {
    let mut accepted = Vec::with_capacity(MAX_READER_NAME_BYTES + 3);
    accepted.push(b'<');
    accepted.extend(std::iter::repeat_n(b'a', MAX_READER_NAME_BYTES));
    accepted.extend_from_slice(b"/>");
    let mut reader = XmlReader::new(&accepted).unwrap();
    let mut tags = Vec::new();
    assert_eq!(
        reader.step(accepted.len(), |tag| {
            tags.push(tag);
            true
        }),
        ReaderStatus::Complete
    );
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].kind, TagKind::EmptyElement);
    assert_eq!(tags[0].name.start, 1);
    assert_eq!(tags[0].name.end, 1 + MAX_READER_NAME_BYTES);
    assert_eq!(
        tags[0].name.slice(&accepted),
        Some(&accepted[1..1 + MAX_READER_NAME_BYTES])
    );

    let mut rejected = Vec::with_capacity(MAX_READER_NAME_BYTES + 4);
    rejected.push(b'<');
    rejected.extend(std::iter::repeat_n(b'a', MAX_READER_NAME_BYTES + 1));
    rejected.extend_from_slice(b"/>");
    let mut reader = XmlReader::new(&rejected).unwrap();
    let mut emitted = false;
    assert_eq!(
        reader.step(rejected.len(), |_| {
            emitted = true;
            true
        }),
        ReaderStatus::Malformed
    );
    assert!(!emitted);
    let progress = reader.progress();
    assert_eq!(progress.diagnostic.category, XmlError::InvalidName);
    assert_eq!(progress.diagnostic.offset, 1);
    assert_eq!(progress.bytes_read, 2 + MAX_READER_NAME_BYTES);
}

#[test]
fn reader_rejects_zero_budget_and_reports_completion_idempotently() {
    let source = b"<a/>";
    let mut reader = XmlReader::new(source).unwrap();
    assert_eq!(reader.step(0, |_| true), ReaderStatus::Invalid);
    assert_eq!(reader.step(64, |_| true), ReaderStatus::Complete);
    assert_eq!(reader.step(1, |_| true), ReaderStatus::Complete);
}

#[test]
fn reader_hard_caps_large_step_budgets_at_64_kib() {
    let source = vec![b'x'; MAX_READER_STEP_BYTES + 2];
    let mut reader = XmlReader::new(&source).unwrap();
    assert_eq!(reader.step(usize::MAX, |_| true), ReaderStatus::Ok);
    assert_eq!(reader.progress().bytes_read, MAX_READER_STEP_BYTES);
}

#[test]
fn tokenizer_emits_byte_exact_tokens_and_preserves_qname_and_raw_values() {
    let source = b"<?xml version='1.0' encoding=\"UTF-8\"?>\r\n<!--c--><?p data?><a:root xmlns:a='urn:x' a:k=\"v&amp;x\">x\r\ny<![CDATA[z]]><a:child/></a:root>";
    let (tokens, result) = tokenize(source, XmlLimits::default());
    assert_eq!(result.diagnostic.category, XmlError::None);
    assert_eq!((result.element_count, result.attribute_count), (2, 2));
    let token = |kind| tokens.iter().find(|t| t.kind == kind).unwrap();
    assert_eq!(
        token(TokenKind::Declaration).source.slice(source),
        Some(&source[..38])
    );
    assert_eq!(
        token(TokenKind::Comment).value.slice(source),
        Some(&b"c"[..])
    );
    assert_eq!(
        token(TokenKind::ProcessingInstruction).value.slice(source),
        Some(&b"data"[..])
    );
    let root = tokens
        .iter()
        .find(|t| t.kind == TokenKind::StartElement)
        .unwrap();
    assert_eq!(
        root.source.slice(source),
        Some(&b"<a:root xmlns:a='urn:x' a:k=\"v&amp;x\">"[..])
    );
    assert_eq!(root.prefix.slice(source), Some(&b"a"[..]));
    assert_eq!(root.local_name.slice(source), Some(&b"root"[..]));
    let attr = tokens
        .iter()
        .find(|t| t.kind == TokenKind::Attribute && t.name.slice(source) == Some(&b"a:k"[..]))
        .unwrap();
    assert_eq!(attr.value.slice(source), Some(&b"v&amp;x"[..]));
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::Text && t.value.slice(source) == Some(&b"x\r\ny"[..]))
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::Cdata && t.value.slice(source) == Some(&b"z"[..]))
    );
}

#[test]
fn processing_instruction_target_keeps_legacy_colon_and_digit_name_behavior() {
    for source in [&b"<?a:b:c data?><a/>"[..], &b"<?a:1 data?><a/>"[..]] {
        let (tokens, result) = tokenize(source, XmlLimits::default());
        assert_eq!(result.diagnostic.category, XmlError::None);
        assert_eq!(tokens[0].kind, TokenKind::ProcessingInstruction);
        assert_eq!(
            tokens[0].name.slice(source).unwrap(),
            &source[2..source.iter().position(|b| *b == b' ').unwrap()]
        );
    }
}

#[test]
fn tokenizer_preserves_doctype_lexically_without_entity_expansion() {
    for source in [
        &b"<!DOCTYPE a><a/>"[..],
        &b"<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\"\n \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\"><a/>"[..],
        &b"<!DOCTYPE svg [\n<!ENTITY ns \"http://www.w3.org/2000/svg\">\n]><a/>"[..],
        &b"<!DOCTYPE svg [<!-- ]> \">\" --><!ENTITY e \"a>b]\"><?pi ]> ?>]><a/>"[..],
    ] {
        let (tokens, result) = tokenize(source, XmlLimits::default());
        assert_eq!(result.diagnostic.category, XmlError::None, "{source:?}");
        let doctype = tokens.iter().find(|t| t.kind == TokenKind::Doctype).unwrap();
        assert_eq!(doctype.source.slice(source).unwrap().last(), Some(&b'>'));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::EmptyElement));
    }
    for source in [
        &b"<!DOCTYPE a [<!ENTITY e \"x\">]><a>&e;</a>"[..],
        &b"<!DOCTYPE a><a/><!DOCTYPE b>"[..],
        &b"<a/><!DOCTYPE a>"[..],
        &b"<!DOCTYPEa><a/>"[..],
        &b"<!DOCTYPE a"[..],
        &b"<!ENTITY e 'x'><a/>"[..],
    ] {
        assert_ne!(
            tokenize(source, XmlLimits::default()).1.diagnostic.category,
            XmlError::None,
            "{source:?}"
        );
    }
}

#[test]
fn tokenizer_reports_frozen_error_categories_and_offsets() {
    for (source, category, offset) in [
        (&b"<a x='1' x=\"2\"/>"[..], XmlError::DuplicateAttribute, 9),
        (&b"<a x='1'y='2'/>"[..], XmlError::MalformedSyntax, 8),
        (&b"<a:1/>"[..], XmlError::InvalidName, 3),
        (&b"<a:b:c/>"[..], XmlError::InvalidName, 2),
        (&b"<a><b></a>"[..], XmlError::MismatchedTag, 6),
        (&b"<a/><b/>"[..], XmlError::MultipleRoots, 4),
        (&b"<a>"[..], XmlError::IncompleteDocument, 3),
        (&b"<a><!-- -- --></a>"[..], XmlError::MalformedSyntax, 8),
        (
            &b"<?xml version='1.0' encoding='UTF-16'?><a/>"[..],
            XmlError::UnsupportedEncoding,
            30,
        ),
        (&b"<a>\x01</a>"[..], XmlError::InvalidCharacter, 3),
        (&b"<a>&#0;</a>"[..], XmlError::InvalidCharacter, 3),
    ] {
        let (_, result) = tokenize(source, XmlLimits::default());
        assert_eq!(
            result.diagnostic,
            XmlDiagnostic { category, offset },
            "{source:?}"
        );
        assert_eq!(
            result.token_count, 0,
            "failed documents expose no partial tokens"
        );
    }
    let (_, result) = tokenize(b"<1a/>", XmlLimits::default());
    assert_eq!(result.diagnostic.category, XmlError::InvalidName);
}

#[test]
fn tokenizer_limits_capacity_and_attribute_decoder_are_bounded() {
    let mut limits = XmlLimits::default();
    limits.nesting_depth = 1;
    assert_eq!(
        tokenize(b"<a><b/></a>", limits).1.diagnostic.category,
        XmlError::DepthLimit
    );
    let mut limits = XmlLimits::default();
    limits.nodes = 1;
    assert_eq!(
        tokenize(b"<a>x</a>", limits).1.diagnostic.category,
        XmlError::NodeLimit
    );
    let mut limits = XmlLimits::default();
    limits.attributes_per_element = 1;
    assert_eq!(
        tokenize(b"<a x='1' y='2'/>", limits).1.diagnostic.category,
        XmlError::AttributeLimit
    );
    let mut limits = XmlLimits::default();
    limits.total_attributes = 1;
    let total = tokenize(b"<a x='1'><b y='2'/></a>", limits).1;
    assert_eq!(
        total.diagnostic,
        XmlDiagnostic {
            category: XmlError::AttributeLimit,
            offset: 12
        }
    );
    let mut limits = XmlLimits::default();
    limits.text_run_bytes = 2;
    assert_eq!(
        tokenize(b"<a>abc</a>", limits).1.diagnostic,
        XmlDiagnostic {
            category: XmlError::TextLimit,
            offset: 3
        }
    );
    let mut limits = XmlLimits::default();
    limits.text_run_bytes = 2;
    assert_eq!(
        tokenize(b"<a><![CDATA[abc]]></a>", limits).1.diagnostic,
        XmlDiagnostic {
            category: XmlError::TextLimit,
            offset: 12
        }
    );
    let (_, result) = tokenize_with_capacity(b"<a/>", XmlLimits::default(), 0);
    assert_eq!(result.diagnostic.category, XmlError::OutputExhausted);
    let raw = b"a&amp;&#65;&#x1f600;z";
    let mut out = [0; 32];
    let length = decode_attribute_value(
        raw,
        XmlSpan {
            offset: 0,
            length: raw.len(),
        },
        &mut out,
    )
    .unwrap();
    assert_eq!(&out[..length], "a&A😀z".as_bytes());
    for bad in [
        &b"&unknown;"[..],
        &b"&#;"[..],
        &b"&#x;"[..],
        &b"&#1114112;"[..],
        &b"&amp"[..],
    ] {
        assert_eq!(
            decode_attribute_value(
                bad,
                XmlSpan {
                    offset: 0,
                    length: bad.len()
                },
                &mut out
            ),
            Err(0)
        );
    }
}

#[test]
fn token_stream_is_resumable_cancellable_and_storage_bounded() {
    let source = b"<a><b/></a>";
    let mut stream = XmlTokenStream::new(source, 32 * core::mem::size_of::<XmlToken>()).unwrap();
    loop {
        let before = stream.bytes_read();
        match stream.step(2, false) {
            StreamStatus::Ok => {}
            StreamStatus::Complete => break,
            status => panic!("unexpected stream status: {status:?}"),
        }
        assert!(stream.bytes_read().saturating_sub(before) <= 2);
    }
    assert_eq!(stream.result().element_count, 2);
    assert_eq!(stream.take().unwrap().len(), 3);
    let mut canceled = XmlTokenStream::new(source, 64 * core::mem::size_of::<XmlToken>()).unwrap();
    assert_eq!(canceled.step(1, true), StreamStatus::Canceled);
    let mut too_small = XmlTokenStream::new(source, core::mem::size_of::<XmlToken>()).unwrap();
    assert_eq!(too_small.step(usize::MAX, false), StreamStatus::Failed);
    assert_eq!(
        too_small.result().diagnostic.category,
        XmlError::OutputExhausted
    );

    let mut seen = Vec::new();
    let mut visitor_stream = XmlTokenStream::new_visitor(source).unwrap();
    loop {
        let status = visitor_stream.step_with_visitor(3, false, &mut |token| {
            seen.push(token.kind);
            true
        });
        assert!(
            visitor_stream.visitor_storage_bytes()
                <= (MAX_ATTRIBUTES_PER_ELEMENT + 1) * core::mem::size_of::<XmlToken>()
        );
        if status == StreamStatus::Complete {
            break;
        }
        assert_eq!(status, StreamStatus::Ok);
    }
    assert_eq!(
        seen,
        [
            TokenKind::StartElement,
            TokenKind::EmptyElement,
            TokenKind::EndElement
        ]
    );
    let entity_source = b"<r>&#123456;&amp;</r>";
    let mut entity_stream = XmlTokenStream::new_visitor(entity_source).unwrap();
    let mut entity_tokens = Vec::new();
    let mut entity_steps = 0;
    loop {
        let before = entity_stream.bytes_read();
        let status = entity_stream.step_with_visitor(1, false, &mut |token| {
            entity_tokens.push(token.kind);
            true
        });
        assert!(entity_stream.bytes_read().saturating_sub(before) <= 1);
        entity_steps += 1;
        assert!(
            entity_steps <= entity_source.len() * 4,
            "steps={entity_steps}, bytes={}, status={status:?}",
            entity_stream.bytes_read()
        );
        if status == StreamStatus::Complete {
            break;
        }
        assert_eq!(
            status,
            StreamStatus::Ok,
            "diagnostic={:?}, bytes={}",
            entity_stream.result().diagnostic,
            entity_stream.bytes_read()
        );
    }
    assert_eq!(
        entity_tokens,
        [
            TokenKind::StartElement,
            TokenKind::Text,
            TokenKind::EndElement
        ]
    );
    let marker_source = b"<r><!--c--><?p x?><![CDATA[y]]></r>";
    let mut marker_stream = XmlTokenStream::new_visitor(marker_source).unwrap();
    let mut marker_tokens = Vec::new();
    loop {
        let before = marker_stream.bytes_read();
        let status = marker_stream.step_with_visitor(1, false, &mut |token| {
            marker_tokens.push(token.kind);
            true
        });
        assert!(marker_stream.bytes_read().saturating_sub(before) <= 1);
        if status == StreamStatus::Complete {
            break;
        }
        assert_eq!(
            status,
            StreamStatus::Ok,
            "diagnostic={:?}",
            marker_stream.result().diagnostic
        );
    }
    assert_eq!(
        marker_tokens,
        [
            TokenKind::StartElement,
            TokenKind::Comment,
            TokenKind::ProcessingInstruction,
            TokenKind::Cdata,
            TokenKind::EndElement
        ]
    );
    let mut stopped = XmlTokenStream::new_visitor(source).unwrap();
    loop {
        let status = stopped.step_with_visitor(64, false, &mut |_| false);
        if status == StreamStatus::Canceled {
            break;
        }
        assert_eq!(status, StreamStatus::Ok);
    }
    assert_eq!(stopped.step(64, false), StreamStatus::Canceled);

    let mut many = b"<r>".to_vec();
    for _ in 0..2_000 {
        many.extend_from_slice(b"<a/>");
    }
    many.extend_from_slice(b"</r>");
    let mut visitor = XmlTokenStream::new_visitor(&many).unwrap();
    let mut delivered = 0usize;
    loop {
        let status = visitor.step_with_visitor(4096, false, &mut |_| {
            delivered += 1;
            true
        });
        assert!(
            visitor.visitor_storage_bytes()
                <= (MAX_ATTRIBUTES_PER_ELEMENT + 1) * core::mem::size_of::<XmlToken>()
        );
        if status == StreamStatus::Complete {
            break;
        }
        assert_eq!(status, StreamStatus::Ok);
    }
    assert_eq!(delivered, 2_002);

    let mut large = b"<r>".to_vec();
    large.extend(std::iter::repeat_n(b'x', 256 * 1024));
    large.extend_from_slice(b"</r>");
    let mut parsing = XmlTokenStream::new_visitor(&large).unwrap();
    let mut checks = 0usize;
    let mut keep = |_: &XmlToken| true;
    let mut cancel = || {
        checks += 1;
        checks > large.len() + 128
    };
    let mut status = StreamStatus::Ok;
    while status == StreamStatus::Ok {
        status = parsing.step_with_visitor_and_cancel(64 * 1024, &mut keep, &mut cancel)
    }
    assert_eq!(status, StreamStatus::Canceled);
    assert!(parsing.bytes_read() > 3);
    assert!(parsing.bytes_read() < large.len());
}

#[test]
fn tokenizer_covers_output_callbacks_input_limits_and_error_names() {
    assert_eq!(XmlError::UnsupportedDtd.as_str(), "unsupported_dtd");
    let mut limits = XmlLimits::default();
    limits.input_bytes = 2;
    let (_, limited) = tokenize(b"<a/>", limits);
    assert_eq!(
        limited.diagnostic,
        XmlDiagnostic {
            category: XmlError::InputLimit,
            offset: 2
        }
    );
    let (_, _, status) = tokenize_cancelable(b"<a/>", XmlLimits::default(), 8, || true);
    assert_eq!(status, StreamStatus::Canceled);
    let (_, _, status) = tokenize_large_cancelable(b"<a/>", 8, || true);
    assert_eq!(status, StreamStatus::Canceled);
    let large = vec![b'x'; 128 * 1024];
    let mut cancellation_checks = 0;
    let (_, _, status) = tokenize_cancelable(&large, XmlLimits::default(), 8, || {
        cancellation_checks += 1;
        cancellation_checks == 3
    });
    assert_eq!(status, StreamStatus::Canceled);
    assert_eq!(cancellation_checks, 3);
    let mut visited = 0;
    assert_eq!(
        visit_tokens(b"<a/>", XmlLimits::default(), &mut |_| {
            visited += 1;
            false
        }),
        StreamStatus::Canceled
    );
    assert_eq!(visited, 1);

    let raw = b"&#x1f600;";
    let mut tiny = [0; 3];
    assert_eq!(
        decode_attribute_value(
            raw,
            XmlSpan {
                offset: 0,
                length: raw.len()
            },
            &mut tiny
        ),
        Err(0)
    );
    let malformed = [b'<', b'a', b'>', 0xc0, 0xaf, b'<', b'/', b'a', b'>'];
    let (_, invalid) = tokenize(&malformed, XmlLimits::default());
    assert_eq!(
        invalid.diagnostic,
        XmlDiagnostic {
            category: XmlError::InvalidUtf8,
            offset: 3
        }
    );
}
