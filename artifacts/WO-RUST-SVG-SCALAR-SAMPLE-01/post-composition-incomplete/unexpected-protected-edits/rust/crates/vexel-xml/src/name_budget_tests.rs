use super::*;
use crate::{tokenize, tokenize_cancelable, tokenize_large_cancelable};

fn step_measured(stream: &mut XmlTokenStream<'_>) -> StreamStatus {
    let before = stream.name_work.total_reads;
    let status = stream.step(1, false);
    assert!(stream.name_work.total_reads - before <= 1);
    assert_eq!(
        stream.name_work.total_reads,
        stream.name_work.reads + stream.name_work.comparison_reads
    );
    status
}

fn validate_first(stream: &mut XmlTokenStream<'_>) {
    for _ in 0..stream.source.len() {
        assert_eq!(step_measured(stream), StreamStatus::Ok)
    }
    assert!(stream.chars_complete);
    assert_eq!(stream.name_work.total_reads, 0);
}

fn finish_measured(stream: &mut XmlTokenStream<'_>) -> StreamStatus {
    for _ in 0..stream.source.len() * 5 + 128 {
        let status = step_measured(stream);
        if status != StreamStatus::Ok {
            return status;
        }
    }
    panic!("name scanning/comparison stalled");
}

#[test]
fn all_name_reads_include_end_and_attribute_equality_with_budget_one() {
    for (scalar, width) in [("é", 2), ("中", 3), ("𐀀", 4)] {
        let source =
            format!("<p:{scalar} {scalar}a='x' {scalar}b='y'><?{scalar} data?></p:{scalar}>");
        let (expected, result) = tokenize(source.as_bytes(), XmlLimits::default());
        assert_eq!(result.diagnostic, XmlDiagnostic::default());
        let mut stream =
            XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
        validate_first(&mut stream);
        assert_eq!(finish_measured(&mut stream), StreamStatus::Complete);
        assert_eq!(stream.name_work.decoder_inputs, 5 * width + 6);
        assert_eq!(stream.name_work.reads, 5 * width + 11);
        assert_eq!(stream.name_work.comparison_reads, 4 * width + 6);
        assert_eq!(stream.name_work.total_reads, 9 * width + 17);
        assert_eq!(stream.result(), result);
        assert_eq!(stream.take().unwrap(), expected);

        let mut visitor = XmlTokenStream::new_visitor(source.as_bytes()).unwrap();
        let scratch = visitor.visitor_storage_bytes();
        let mut delivered = Vec::new();
        let mut complete = false;
        for _ in 0..source.len() * 5 + 128 {
            let before = visitor.name_work.total_reads;
            let status = visitor.step_with_visitor(1, false, &mut |token| {
                delivered.push(*token);
                true
            });
            assert!(visitor.name_work.total_reads - before <= 1);
            assert_eq!(visitor.visitor_storage_bytes(), scratch);
            if status == StreamStatus::Complete {
                complete = true;
                break;
            }
            assert_eq!(status, StreamStatus::Ok);
        }
        assert!(complete);
        assert_eq!(visitor.name_work.total_reads, 9 * width + 17);
        assert_eq!(visitor.result(), result);
        assert_eq!(delivered, expected);
    }
}

#[test]
fn cached_pi_and_entity_classes_preserve_exact_batch_results_and_diagnostics() {
    let sources: &[&[u8]] = &[
        b"<?xml version='1.0'?><r/>",
        b"<?XML x?><r/>",
        b"<?XmL x?><r/>",
        b"<?xmlx x?><r/>",
        b"<?:xml x?><r/>",
        b"<?xml:foo x?><r/>",
        "<?é?><r/>".as_bytes(),
        "<?中 x?><r/>".as_bytes(),
        "<?𐀀 x?><r/>".as_bytes(),
        b"<r>&lt;&gt;&amp;&apos;&quot;</r>",
        b"<r>&AMP;</r>",
        "<é></ê>".as_bytes(),
        "<中></串>".as_bytes(),
        "<𐀀></𐀁>".as_bytes(),
        b"<Aa></aa>",
        "<é></é>".as_bytes(),
        "<r é='1' é='2'/>".as_bytes(),
    ];
    for &source in sources {
        let (expected, result) = tokenize(source, XmlLimits::default());
        let expected_status = if result.diagnostic.category == XmlError::None {
            StreamStatus::Complete
        } else {
            StreamStatus::Failed
        };
        let mut stream = XmlTokenStream::new(source, 16 * size_of::<XmlToken>()).unwrap();
        assert_eq!(finish_measured(&mut stream), expected_status);
        assert_eq!(stream.result(), result, "source={source:?}");
        assert_eq!(stream.tokens(), expected);
        if source.starts_with(b"<?") {
            assert_eq!(stream.name_work.comparison_reads, 0)
        }
        let (actual, actual_result, status) =
            tokenize_cancelable(source, XmlLimits::default(), 16, || false);
        assert_eq!(status, expected_status);
        assert_eq!(actual_result, result);
        assert_eq!(actual, expected);
        let (actual, actual_result, status) = tokenize_large_cancelable(source, 16, || false);
        assert_eq!(status, expected_status);
        assert_eq!(actual_result, result);
        assert_eq!(actual, expected);
    }
    let mut mismatch =
        XmlTokenStream::new("<é></ê>".as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
    assert_eq!(finish_measured(&mut mismatch), StreamStatus::Failed);
    assert_eq!(
        mismatch.result().diagnostic,
        XmlDiagnostic {
            category: XmlError::MismatchedTag,
            offset: 4
        }
    );
    assert_eq!(mismatch.name_work.comparison_reads, 4);
}

#[test]
fn pi_decode_and_two_operand_end_matching_cancel_without_extra_name_reads() {
    for scalar in ["é", "中", "𐀀"] {
        let source = format!("<?{scalar} data?><r/>");
        for cut in 1..scalar.len() {
            let mut stream =
                XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
            validate_first(&mut stream);
            for _ in 0..source.len() * 2 + 64 {
                assert_eq!(step_measured(&mut stream), StreamStatus::Ok);
                if let Pending::Pi {
                    name_scan: Some(name),
                    ..
                } = stream.pending
                {
                    if name.pos - name.start == cut {
                        assert!(name.utf8.incomplete());
                        break;
                    }
                }
            }
            assert_eq!(stream.name_work.total_reads, cut);
            assert_eq!(
                stream.step_with_cancel(1, &mut || true),
                StreamStatus::Canceled
            );
            assert_eq!(stream.name_work.total_reads, cut);
            assert_eq!(stream.result(), XmlResult::default());
            assert!(stream.tokens().is_empty());
        }
        let source = format!("<{scalar}></{scalar}>");
        for cut in 1..2 * scalar.len() {
            let mut stream =
                XmlTokenStream::new(source.as_bytes(), 16 * size_of::<XmlToken>()).unwrap();
            validate_first(&mut stream);
            let mut matching = false;
            for _ in 0..source.len() * 2 + 64 {
                assert_eq!(step_measured(&mut stream), StreamStatus::Ok);
                if stream
                    .tag
                    .as_ref()
                    .is_some_and(|t| t.phase == TagPhase::EndMatch)
                {
                    matching = true;
                    break;
                }
            }
            assert!(matching);
            assert_eq!(stream.name_work.comparison_reads, 0);
            let frontier = stream.bytes_read();
            for consumed in 1..=cut {
                assert_eq!(step_measured(&mut stream), StreamStatus::Ok);
                assert_eq!(stream.name_work.comparison_reads, consumed);
                assert_eq!(stream.bytes_read(), frontier);
            }
            let comparison = stream.tag.as_ref().unwrap().comparison.unwrap();
            assert_eq!(comparison.index, cut / 2);
            assert_eq!(comparison.left_byte.is_some(), cut % 2 == 1);
            let before = stream.name_work.total_reads;
            assert_eq!(
                stream.step_with_cancel(1, &mut || true),
                StreamStatus::Canceled
            );
            assert_eq!(stream.name_work.total_reads, before);
            assert_eq!(stream.name_work.comparison_reads, cut);
            assert_eq!(stream.result(), XmlResult::default());
            assert!(stream.tokens().is_empty());
            assert!(stream.take().is_none());
        }
    }
}

#[test]
fn maximum_byte_length_name_matching_uses_fixed_state_and_budgeted_reads() {
    let name = "é".repeat(MAX_TOKEN_NAME_BYTES / 2);
    let source = format!("<{name}></{name}>");
    let (expected, result) = tokenize(source.as_bytes(), XmlLimits::default());
    assert_eq!(result.diagnostic, XmlDiagnostic::default());
    let mut stream = XmlTokenStream::new_visitor(source.as_bytes()).unwrap();
    let scratch = stream.visitor_storage_bytes();
    let mut delivered = Vec::new();
    let mut complete = false;
    for _ in 0..source.len() * 5 + 128 {
        let before = stream.name_work.total_reads;
        let status = stream.step_with_visitor(1, false, &mut |token| {
            delivered.push(*token);
            true
        });
        assert!(stream.name_work.total_reads - before <= 1);
        assert_eq!(stream.visitor_storage_bytes(), scratch);
        if status == StreamStatus::Complete {
            complete = true;
            break;
        }
        assert_eq!(status, StreamStatus::Ok);
    }
    assert!(complete);
    assert_eq!(stream.name_work.decoder_inputs, 2 * MAX_TOKEN_NAME_BYTES);
    assert_eq!(stream.name_work.comparison_reads, 2 * MAX_TOKEN_NAME_BYTES);
    assert_eq!(stream.name_work.total_reads, 4 * MAX_TOKEN_NAME_BYTES + 2);
    assert_eq!(stream.result(), result);
    assert_eq!(delivered, expected);
}
