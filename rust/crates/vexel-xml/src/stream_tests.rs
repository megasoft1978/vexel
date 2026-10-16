use super::*;
use crate::{tokenize, tokenize_cancelable, tokenize_large_cancelable, tokenize_with_capacity};

#[test]
fn budget_one_name_scans_measure_real_source_reads_and_decoder_inputs() {
    for (scalar, width) in [("é", 2), ("中", 3), ("𐀀", 4)] {
        let source = format!("<p:{scalar} {scalar}='x'><?{scalar} x?></p:{scalar}>");
        let (expected, result) = tokenize(source.as_bytes(), XmlLimits::default());
        assert_eq!(result.diagnostic, XmlDiagnostic::default());
        let mut stream = XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
        for _ in 0..source.len() { assert_eq!(stream.step(1, false), StreamStatus::Ok); }
        assert!(stream.chars_complete);
        assert_eq!(stream.name_work.reads, 0);
        assert_eq!(stream.name_work.decoder_inputs, 0);
        let mut complete = false;
        for _ in 0..source.len() * 3 + 64 {
            let reads = stream.name_work.reads;
            let inputs = stream.name_work.decoder_inputs;
            let status = stream.step(1, false);
            let actual_reads = stream.name_work.reads - reads;
            let actual_inputs = stream.name_work.decoder_inputs - inputs;
            assert!(actual_reads <= 1, "width={width}, actual name reads={actual_reads}");
            assert!(actual_inputs <= actual_reads);
            if status == StreamStatus::Complete { complete = true; break; }
            assert_eq!(status, StreamStatus::Ok);
        }
        assert!(complete);
        // Four multibyte names, two ASCII "p:" prefixes, and four delimiter
        // probes. These are counts of actual loads/pushes, not cursor deltas.
        assert_eq!(stream.name_work.decoder_inputs, 4 * width + 4);
        assert_eq!(stream.name_work.reads, 4 * width + 8);
        assert_eq!(stream.result(), result);
        assert_eq!(stream.take().unwrap(), expected);
    }
}

#[test]
fn budget_one_name_decode_can_cancel_at_each_partial_scalar_boundary() {
    for scalar in ["é", "中", "𐀀"] {
        let source = format!("<{scalar}></{scalar}>");
        for consumed in 1..scalar.len() {
            let mut stream = XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
            for _ in 0..source.len() { assert_eq!(stream.step(1, false), StreamStatus::Ok); }
            assert!(stream.chars_complete);
            assert_eq!(stream.step(1, false), StreamStatus::Ok); // '<'
            for _ in 0..consumed {
                let reads = stream.name_work.reads;
                let inputs = stream.name_work.decoder_inputs;
                assert_eq!(stream.step(1, false), StreamStatus::Ok);
                assert_eq!(stream.name_work.reads - reads, 1);
                assert_eq!(stream.name_work.decoder_inputs - inputs, 1);
            }
            let name = stream.tag.as_ref().unwrap().name_scan.unwrap();
            assert_eq!(name.pos, 1 + consumed);
            assert_eq!(name.utf8.start, 1);
            assert!(name.utf8.incomplete());
            assert_eq!(stream.name_work.reads, consumed);
            assert_eq!(stream.name_work.decoder_inputs, consumed);
            let mut polls = 0;
            let mut cancel = || { polls += 1; true };
            assert_eq!(stream.step_with_cancel(1, &mut cancel), StreamStatus::Canceled);
            assert_eq!(polls, 1);
            assert_eq!(stream.name_work.reads, consumed);
            assert_eq!(stream.name_work.decoder_inputs, consumed);
            assert_empty_result(&mut stream, XmlDiagnostic::default());
            assert_eq!(stream.step(1, false), StreamStatus::Canceled);
            assert_eq!(stream.name_work.reads, consumed);
        }
    }
}

fn finish(stream: &mut XmlTokenStream<'_>, budget: usize) -> StreamStatus {
    for _ in 0..stream.source.len().saturating_mul(3).saturating_add(64) {
        let scanned = stream.char_cursor;
        let parsed = stream.bytes_read();
        let status = stream.step(budget, false);
        assert!(stream.char_cursor.saturating_sub(scanned)
            + stream.bytes_read().saturating_sub(parsed) <= budget.min(STEP_CAP));
        if status != StreamStatus::Ok { return status; }
    }
    panic!("stream stalled at scan={} parse={}", stream.char_cursor, stream.bytes_read());
}

fn assert_empty_result(stream: &mut XmlTokenStream<'_>, diagnostic: XmlDiagnostic) {
    assert!(stream.tokens().is_empty());
    assert!(stream.take().is_none());
    assert_eq!(stream.result(), XmlResult { diagnostic, ..XmlResult::default() });
}

#[test]
fn one_byte_steps_preserve_two_three_and_four_byte_scalars_and_names() {
    for (scalar, width) in [("é", 2), ("中", 3), ("𐀀", 4)] {
        assert_eq!(scalar.len(), width);
        let source = format!("<{scalar} {scalar}='{scalar}'>{scalar}</{scalar}>");
        let (expected, result) = tokenize(source.as_bytes(), XmlLimits::default());
        assert_eq!(result.diagnostic, XmlDiagnostic::default());
        assert_eq!(expected.len(), 4);
        assert_eq!(expected[0].name, XmlSpan { offset: 1, length: width });
        assert_eq!(expected[1].value, XmlSpan { offset: 4 + 2 * width, length: width });
        let mut stream = XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
        for consumed in 1..=source.len() {
            assert_eq!(stream.step(1, false), StreamStatus::Ok);
            assert_eq!(stream.char_cursor, consumed);
        }
        assert!(stream.chars_complete);
        assert_eq!(finish(&mut stream, 1), StreamStatus::Complete);
        assert_eq!(stream.result(), result);
        assert_eq!(stream.take().unwrap(), expected);

        let mut visitor = XmlTokenStream::new_visitor(source.as_bytes()).unwrap();
        let mut delivered = Vec::new();
        let mut complete = false;
        for _ in 0..source.len() * 3 + 64 {
            let status = visitor.step_with_visitor(1, false, &mut |token| { delivered.push(*token); true });
            if status == StreamStatus::Complete { complete = true; break; }
            assert_eq!(status, StreamStatus::Ok);
        }
        assert!(complete);
        assert_eq!(delivered, expected);
        assert_eq!(visitor.result(), result);
    }
    let source = "<r>é中😀</r>".as_bytes();
    let mut stream = XmlTokenStream::new(source, 16 * size_of::<XmlToken>()).unwrap();
    assert_eq!(finish(&mut stream, 1), StreamStatus::Complete);
    assert_eq!(stream.tokens()[1].source, XmlSpan { offset: 3, length: 9 });
    assert_eq!(stream.tokens()[1].value.slice(source), Some("é中😀".as_bytes()));
}

#[test]
fn cancellation_can_interrupt_each_partial_utf8_scalar_between_bytes() {
    for scalar in ["é", "中", "😀"] {
        let source = format!("<r>{scalar}</r>");
        for consumed in 1..scalar.len() {
            let mut stepped = XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
            for byte in 1..=3 + consumed {
                assert_eq!(stepped.step(1, false), StreamStatus::Ok);
                assert_eq!(stepped.char_cursor, byte);
            }
            assert!(stepped.utf8.incomplete());
            assert_eq!(stepped.step(1, true), StreamStatus::Canceled);
            assert_eq!(stepped.char_cursor, 3 + consumed);
            assert_empty_result(&mut stepped, XmlDiagnostic::default());
            let mut stream = XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
            let mut polls = 0;
            let mut cancel = || { polls += 1; polls > 1 + 3 + consumed };
            assert_eq!(stream.step_with_cancel(64, &mut cancel), StreamStatus::Canceled);
            assert_eq!(stream.char_cursor, 3 + consumed);
            assert_eq!(stream.utf8.start, 3);
            assert!(stream.utf8.incomplete());
            assert_empty_result(&mut stream, XmlDiagnostic::default());
            assert_eq!(stream.step(1, false), StreamStatus::Canceled);
        }
    }
}

#[test]
fn invalid_utf8_and_xml_scalars_have_exact_batch_and_stream_diagnostics() {
    let cases: &[(&[u8], bool, XmlError)] = &[
        (&[0xc0, 0xaf], false, XmlError::InvalidUtf8),
        (&[0xc2, b'A'], false, XmlError::InvalidUtf8),
        (&[0xc2], true, XmlError::InvalidUtf8),
        (&[0xe0, 0x80, 0x80], false, XmlError::InvalidUtf8),
        (&[0xed, 0xa0, 0x80], false, XmlError::InvalidUtf8),
        (&[0xe2, 0x82, b'A'], false, XmlError::InvalidUtf8),
        (&[0xe2, 0x82], true, XmlError::InvalidUtf8),
        (&[0xf0, 0x80, 0x80, 0x80], false, XmlError::InvalidUtf8),
        (&[0xf4, 0x90, 0x80, 0x80], false, XmlError::InvalidUtf8),
        (&[0xf0, 0x90, 0x80, b'A'], false, XmlError::InvalidUtf8),
        (&[0xf0, 0x90, 0x80], true, XmlError::InvalidUtf8),
        (&[0x80], false, XmlError::InvalidUtf8),
        (&[0xf5, 0x80, 0x80, 0x80], false, XmlError::InvalidUtf8),
        (&[0], false, XmlError::InvalidCharacter),
        (&[0xef, 0xbf, 0xbe], false, XmlError::InvalidCharacter),
    ];
    for &(bad, truncated, category) in cases {
        let mut source = b"<r>".to_vec(); source.extend_from_slice(bad);
        if !truncated { source.extend_from_slice(b"</r>"); }
        let diagnostic = XmlDiagnostic { category, offset: 3 };
        let (tokens, result) = tokenize(&source, XmlLimits::default());
        assert!(tokens.is_empty());
        assert_eq!(result, XmlResult { diagnostic, ..XmlResult::default() });
        let mut stream = XmlTokenStream::new(&source, 16 * size_of::<XmlToken>()).unwrap();
        assert_eq!(finish(&mut stream, 1), StreamStatus::Failed);
        assert_empty_result(&mut stream, diagnostic);
        let (tokens, canceled_result, status) = tokenize_cancelable(&source, XmlLimits::default(), 16, || false);
        assert_eq!(status, StreamStatus::Failed);
        assert!(tokens.is_empty()); assert_eq!(canceled_result, result);
    }
}

#[test]
fn batch_cancelable_apis_cancel_inside_long_lexical_parsing_after_validation() {
    for (prefix, suffix) in [
        (b"<r>".as_slice(), b"</r>".as_slice()),
        (b"<r v='".as_slice(), b"'/>".as_slice()),
        (b"<r><!--".as_slice(), b"--></r>".as_slice()),
        (b"<r><![CDATA[".as_slice(), b"]]></r>".as_slice()),
        (b"<r><?p ".as_slice(), b"?></r>".as_slice()),
        (b"<!DOCTYPE r SYSTEM '".as_slice(), b"'><r/>".as_slice()),
    ] {
        let mut source = prefix.to_vec(); source.extend(std::iter::repeat_n(b'x', 256 * 1024)); source.extend_from_slice(suffix);
        let threshold = source.len() + 2048;
        let mut stream = XmlTokenStream::new(&source, 16 * size_of::<XmlToken>()).unwrap();
        let mut polls = 0;
        let mut cancel = || { polls += 1; polls == threshold };
        loop { let status = stream.step_with_cancel(STEP_CAP, &mut cancel); if status != StreamStatus::Ok { assert_eq!(status, StreamStatus::Canceled); break; } }
        assert!(stream.chars_complete);
        assert_eq!(stream.char_cursor, source.len());
        assert!(stream.bytes_read() > prefix.len());
        assert!(stream.bytes_read() < source.len());
        assert_empty_result(&mut stream, XmlDiagnostic::default());
        for large in [false, true] {
            let mut polls = 0;
            let cancel = || { polls += 1; polls == threshold };
            let (tokens, result, status) = if large { tokenize_large_cancelable(&source, 16, cancel) }
                else { tokenize_cancelable(&source, XmlLimits::default(), 16, cancel) };
            assert_eq!(status, StreamStatus::Canceled);
            assert_eq!(polls, threshold);
            assert!(tokens.is_empty()); assert_eq!(result, XmlResult::default());
        }
    }
}

#[test]
fn terminal_failure_and_cancellation_remove_previously_exposed_tokens() {
    for (source, capacity, diagnostic) in [
        (b"<r><a/></wrong>".as_slice(), 16, XmlDiagnostic { category: XmlError::MismatchedTag, offset: 7 }),
        (b"<r><a/></r>".as_slice(), 1, XmlDiagnostic { category: XmlError::OutputExhausted, offset: 3 }),
    ] {
        let mut stream = XmlTokenStream::new(source, capacity * size_of::<XmlToken>()).unwrap();
        let mut saw_tokens = false;
        loop {
            let status = stream.step(1, false);
            if status != StreamStatus::Ok { assert_eq!(status, StreamStatus::Failed); break; }
            saw_tokens |= !stream.tokens().is_empty();
        }
        assert!(saw_tokens);
        assert_empty_result(&mut stream, diagnostic);
        assert_eq!(stream.step(1, false), StreamStatus::Failed);
        assert_empty_result(&mut stream, diagnostic);
    }
    let source = b"<r><a/></r>";
    let mut stream = XmlTokenStream::new(source, 16 * size_of::<XmlToken>()).unwrap();
    while stream.tokens().is_empty() { assert_eq!(stream.step(1, false), StreamStatus::Ok); }
    assert_eq!(stream.step(1, true), StreamStatus::Canceled);
    assert_empty_result(&mut stream, XmlDiagnostic::default());
    assert_eq!(stream.step(1, false), StreamStatus::Canceled);
    let mut visitor = XmlTokenStream::new_visitor(source).unwrap();
    let mut delivered = 0;
    loop {
        let status = visitor.step_with_visitor(1, false, &mut |_| { delivered += 1; delivered != 2 });
        if status != StreamStatus::Ok { assert_eq!(status, StreamStatus::Canceled); break; }
    }
    assert_eq!(delivered, 2);
    assert!(visitor.tag_scratch.is_empty());
    assert_empty_result(&mut visitor, XmlDiagnostic::default());
}

#[test]
fn byte_storage_budget_bounds_actual_token_capacity_at_every_boundary() {
    let token_bytes=size_of::<XmlToken>();
    let source=b"<r><a/></r>";
    for budget in [0,token_bytes-1] {
        assert!(matches!(XmlTokenStream::new(source,budget),Err(StreamStatus::Failed)));
    }
    for (budget,capacity,expected_status,diagnostic) in [
        (token_bytes,1,StreamStatus::Failed,Some(XmlDiagnostic{category:XmlError::OutputExhausted,offset:3})),
        (2*token_bytes+token_bytes-1,2,StreamStatus::Failed,Some(XmlDiagnostic{category:XmlError::OutputExhausted,offset:7})),
        (3*token_bytes,3,StreamStatus::Complete,None),
        (5*token_bytes,5,StreamStatus::Complete,None),
    ] {
        let mut stream=XmlTokenStream::new(source,budget).unwrap();
        let status=loop {
            let status=stream.step(usize::MAX,false);
            assert!(stream.token_storage_bytes()<=budget);
            assert_eq!(stream.token_storage_bytes()%token_bytes,0);
            assert!(stream.open_name_storage_bytes()<=MAX_DEPTH*size_of::<XmlSpan>());
            if status!=StreamStatus::Ok {break status}
        };
        assert_eq!(status,expected_status,"budget={budget}");
        assert_eq!(stream.result().diagnostic,diagnostic.unwrap_or_default(),"budget={budget}");
        if expected_status==StreamStatus::Complete {
            assert_eq!(stream.tokens().len(),3);
            let taken=stream.take().unwrap();
            assert_eq!(taken.len(),3);
            assert_eq!(taken.capacity(),3);
            assert!(taken.capacity()<=capacity);
            assert_eq!(stream.token_storage_bytes(),0);
            assert!(stream.take().is_none());
        } else {
            assert!(stream.tokens().is_empty());
            assert!(stream.take().is_none());
        }
    }
}

#[test]
fn bounded_token_storage_survives_cancel_and_visitor_stacks_report_capacity() {
    let token_bytes=size_of::<XmlToken>();
    let mut canceled=XmlTokenStream::new(b"<r><a/></r>",5*token_bytes).unwrap();
    while canceled.tokens().is_empty() { assert_eq!(canceled.step(1,false),StreamStatus::Ok); }
    let retained=canceled.token_storage_bytes();
    assert!(retained<=5*token_bytes);
    assert_eq!(canceled.step(1,true),StreamStatus::Canceled);
    assert!(canceled.tokens().is_empty());
    assert_eq!(canceled.token_storage_bytes(),retained);
    assert_eq!(canceled.result(),XmlResult::default());

    let mut visitor=XmlTokenStream::new_visitor(b"<r a='v'><a/></r>").unwrap();
    let scratch=visitor.visitor_storage_bytes();
    let names=visitor.open_name_storage_bytes();
    assert_eq!(scratch,(MAX_ATTRIBUTES_PER_ELEMENT+1)*token_bytes);
    assert_eq!(names,MAX_DEPTH*size_of::<XmlSpan>());
    loop {
        let status=visitor.step_with_visitor(8,false,&mut |_|true);
        assert_eq!(visitor.visitor_storage_bytes(),scratch);
        assert_eq!(visitor.open_name_storage_bytes(),names);
        if status==StreamStatus::Complete {break}
        assert_eq!(status,StreamStatus::Ok);
    }
}

#[test]
fn cancelable_batch_and_stream_match_batch_tokens_errors_limits_and_capacity() {
    let cases: &[&[u8]] = &[
        b"<?xml version='1.0' encoding='UTF-8'?>\n<!DOCTYPE r [<!ENTITY q 'v'>]><r p:a='&amp;'><!--x--><?p x?><![CDATA[y]]><a/>z</r>",
        "<é 中='😀'>𐀀</é>".as_bytes(), b"<r/>", b"<a:b:c/>", b"<:a/>",
        b"<r>&;</r>", b"<r>&amp;</r>", b"<r/>&amp;", b"<r/>&#32;",
        b"<r><!--x--y--></r>", b"<r><a/></wrong>", b"<r a='1' a='2'/>", b"<r>",
    ];
    for &source in cases {
        for capacity in [0, 1, 64] {
            let (expected, result) = tokenize_with_capacity(source, XmlLimits::default(), capacity);
            let (tokens, actual, status) = tokenize_cancelable(source, XmlLimits::default(), capacity, || false);
            assert_eq!(actual, result, "source={source:?} capacity={capacity}");
            assert_eq!(tokens, expected);
            assert_eq!(status, if result.diagnostic.category == XmlError::None { StreamStatus::Complete } else { StreamStatus::Failed });
            let (large_tokens, large_result, large_status) = tokenize_large_cancelable(source, capacity, || false);
            assert_eq!(large_result, result);
            assert_eq!(large_tokens, expected);
            assert_eq!(large_status, status);
            let mut stream = XmlTokenStream::with_limits(source, XmlLimits::default(), capacity).unwrap();
            assert_eq!(finish(&mut stream, 1), status);
            assert_eq!(stream.result(), result);
            assert_eq!(stream.tokens(), expected);
        }
    }
    for limits in [
        XmlLimits { input_bytes: 2, ..XmlLimits::default() },
        XmlLimits { nesting_depth: 0, ..XmlLimits::default() },
        XmlLimits { nodes: 1, ..XmlLimits::default() },
        XmlLimits { attributes_per_element: 0, ..XmlLimits::default() },
        XmlLimits { total_attributes: 0, ..XmlLimits::default() },
        XmlLimits { text_run_bytes: 1, ..XmlLimits::default() },
    ] {
        let source = b"<r a='v'>xx<a/></r>";
        let (tokens, result) = tokenize_with_capacity(source, limits, 64);
        let (actual_tokens, actual, _) = tokenize_cancelable(source, limits, 64, || false);
        assert_eq!(actual, result); assert_eq!(actual_tokens, tokens);
    }
}
