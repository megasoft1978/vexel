use std::sync::Arc;

use vexel_svg_index::{
    ElementSupport, IndexSession, IndexStatus, MAX_STEP_BYTES,
};
use vexel_xml::{tokenize, XmlDiagnostic, XmlError, XmlLimits};

fn source(bytes: &[u8]) -> Arc<Vec<u8>> { Arc::new(bytes.to_vec()) }

fn finish(session: &mut IndexSession<'_>, budget: usize) -> IndexStatus {
    for _ in 0..250_000 {
        let status = session.step(budget);
        if status != IndexStatus::Ok { return status; }
        std::thread::yield_now();
    }
    panic!("index did not finish within the bounded deterministic step loop")
}

#[test]
fn incremental_progress_preview_and_full_token_spans_match_the_xml_oracle() {
    let bytes = source(b"<svg width='2' height='2'><rect width='2' height='2'/><custom/><path d='M0 0'/></svg>");
    let mut session = IndexSession::start(&bytes, 7).unwrap();
    let initial = session.progress();
    assert_eq!(initial.bytes_read, 0);
    assert_eq!(initial.generation, 7);
    assert_eq!(initial.token_count, 0);
    assert!(session.token_at(0).is_none());
    let first = session.step(1);
    assert_eq!(first, IndexStatus::Ok);
    assert!(session.progress().bytes_read <= 1);

    let mut saw_preview_before_complete = false;
    let mut previous = session.progress().bytes_read;
    let outcome = loop {
        let status = session.step(1);
        let progress = session.progress();
        assert!(progress.bytes_read >= previous);
        assert!(progress.bytes_read - previous <= 1);
        previous = progress.bytes_read;
        saw_preview_before_complete |= progress.first_preview_ready && !progress.complete;
        if status != IndexStatus::Ok { break status; }
    };
    assert_eq!(outcome, IndexStatus::Complete);
    let progress = session.progress();
    assert!(progress.complete);
    assert!(progress.first_preview_ready, "{progress:?}");
    assert!(saw_preview_before_complete);
    assert_eq!((progress.element_count, progress.attribute_count), (4, 5));

    let rect = session.element_at(1).unwrap();
    let rect_source = b"<rect width='2' height='2'/>";
    assert_eq!(rect.support, ElementSupport::Rect);
    assert_eq!(rect.depth, 1);
    assert_eq!(rect.parent, 0);
    assert!(rect.closed);
    assert_eq!(&bytes[rect.source_offset..rect.source_offset + rect.source_length], rect_source);
    assert_eq!(session.element_at(2).unwrap().support, ElementSupport::Unsupported);

    let preview = session.make_preview_source().unwrap();
    assert_eq!(preview, b"<svg width='2' height='2'><rect width='2' height='2'/></svg>");
    assert!(session.preview_generation_is_current(7));
    assert!(!session.preview_generation_is_current(8));

    let (expected, expected_result) = tokenize(bytes.as_slice(), XmlLimits::default());
    assert_eq!(expected_result.diagnostic, XmlDiagnostic::default());
    assert_eq!(progress.token_count, expected.len());
    for (index, token) in expected.iter().enumerate() {
        assert_eq!(session.token_at(index), Some(*token));
    }
}

#[test]
fn nested_element_parent_and_full_source_spans_are_retained() {
    let bytes = source(b"<svg width='2' height='2'><g><rect width='1' height='1'/></g></svg>");
    let mut session = IndexSession::start(&bytes, 1).unwrap();
    assert_eq!(finish(&mut session, MAX_STEP_BYTES), IndexStatus::Complete);
    let root = session.element_at(0).unwrap();
    let group = session.element_at(1).unwrap();
    let child = session.element_at(2).unwrap();
    assert_eq!(root.parent, usize::MAX);
    assert_eq!(root.source_offset, 0);
    assert_eq!(root.source_length, bytes.len());
    assert!(root.closed);
    assert_eq!(group.parent, 0);
    assert_eq!(&bytes[group.source_offset..group.source_offset + group.source_length], b"<g><rect width='1' height='1'/></g>");
    assert!(group.closed);
    assert_eq!(child.parent, 1);
    assert_eq!(&bytes[child.source_offset..child.source_offset + child.source_length], b"<rect width='1' height='1'/>");
}

#[test]
fn doctype_and_rich_xml_keep_original_token_kinds_and_source_spans() {
    let doctype = source(b"<?xml version='1.0'?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"x.dtd\" [\n<!-- ]> \">\" --><!ENTITY e \"a>b]\">\n]>\n<svg><rect/></svg>");
    let mut session = IndexSession::start(&doctype, 1).unwrap();
    assert_eq!(finish(&mut session, 5), IndexStatus::Complete);
    assert_eq!(session.progress().element_count, 2);

    let rich = source(b"<?xml version='1.0' encoding=\"UTF-8\"?>\r\n<!--c--><?p data?><a:root xmlns:a='urn:x' a:k=\"v&amp;x\">x\r\ny<![CDATA[z]]><a:child/></a:root>");
    let mut session = IndexSession::start(&rich, 3).unwrap();
    assert_eq!(finish(&mut session, 3), IndexStatus::Complete);
    let (expected, result) = tokenize(rich.as_slice(), XmlLimits::default());
    assert_eq!(result.diagnostic, XmlDiagnostic::default());
    assert_eq!(session.progress().token_count, expected.len());
    for (index, token) in expected.iter().enumerate() {
        assert_eq!(session.token_at(index), Some(*token));
    }
}

#[test]
fn malformed_prefix_blocks_preview_and_malformed_tail_preserves_valid_preview() {
    let prefix = source(b"<svg width='2' height='2'>bad & raw<rect width='2' height='2'/></svg>");
    let prefix_offset = prefix.iter().position(|byte| *byte == b'&').unwrap();
    let mut session = IndexSession::start(&prefix, 2).unwrap();
    let mut preview_ever_published = false;
    let status = loop {
        let status = session.step(1);
        preview_ever_published |= session.progress().first_preview_ready;
        if status != IndexStatus::Ok { break status; }
        std::thread::yield_now();
    };
    assert_eq!(status, IndexStatus::Malformed);
    assert!(!preview_ever_published);
    assert_eq!(session.progress().diagnostic, XmlDiagnostic { category: XmlError::MalformedSyntax, offset: prefix_offset });

    let tail = source(b"<svg width='2' height='2'><rect width='1' height='1'/></broken>");
    let mut session = IndexSession::start(&tail, 3).unwrap();
    assert_eq!(finish(&mut session, 3), IndexStatus::Malformed);
    let progress = session.progress();
    assert!(!progress.complete);
    // C's harvest order permits either result depending on whether the
    // foreground discovered the child before the worker completion was polled.
    if progress.first_preview_ready {
        assert!(session.make_preview_source().is_some());
    } else {
        assert!(session.make_preview_source().is_none());
    }
    assert_eq!(progress.diagnostic, XmlDiagnostic {
        category: XmlError::MismatchedTag,
        offset: tail.windows(b"</broken>".len()).position(|part| part == b"</broken>").unwrap(),
    });
    assert_eq!(progress.diagnostic, tokenize(tail.as_slice(), XmlLimits::default()).1.diagnostic);
}

#[test]
fn exact_diagnostics_unsupported_gate_generation_invalidation_and_cancellation() {
    for source_bytes in [
        &b"<svg><rect/></bad>"[..],
        &b"<svg><rect width=1/></svg>"[..],
        &b"<svg><!-- -- --></svg>"[..],
        &b"<!ENTITY e \"x\"><svg/>"[..],
        &b"<svg><rect x='1' x='2'/></svg>"[..],
        &b"<svg><rect"[..],
    ] {
        let bytes = source(source_bytes);
        let mut session = IndexSession::start(&bytes, 1).unwrap();
        assert_eq!(finish(&mut session, 3), IndexStatus::Malformed);
        assert_eq!(session.progress().diagnostic, tokenize(source_bytes, XmlLimits::default()).1.diagnostic);
    }

    let unsupported = source(b"<svg width='2' height='2'><rect width='1' height='1' transform='translate(1)'/><rect width='1' height='1'/></svg>");
    let mut session = IndexSession::start(&unsupported, 4).unwrap();
    assert_eq!(finish(&mut session, MAX_STEP_BYTES), IndexStatus::Complete);
    assert_eq!(session.element_at(1).unwrap().support, ElementSupport::Unsupported);
    assert_eq!(session.element_at(2).unwrap().support, ElementSupport::Rect);
    assert_eq!(session.make_preview_source().unwrap(), b"<svg width='2' height='2'><rect width='1' height='1'/></svg>");
    assert_eq!(session.advance_generation(3), IndexStatus::Invalid);
    assert_eq!(session.advance_generation(5), IndexStatus::Ok);
    assert!(!session.progress().first_preview_ready);
    assert!(!session.preview_generation_is_current(4));
    assert!(!session.preview_generation_is_current(5));

    let path = source(b"<svg xmlns='http://www.w3.org/2000/svg' width='2' height='2'><path fill='red' d='M0 0'/></svg>");
    let mut session = IndexSession::start(&path, 2).unwrap();
    assert_eq!(finish(&mut session, MAX_STEP_BYTES), IndexStatus::Complete);
    assert_eq!(session.element_at(1).unwrap().support, ElementSupport::Path);
    assert_eq!(session.make_preview_source().unwrap(), path.as_slice());

    let non_svg = source(b"<g><rect width='2' height='2'/></g>");
    let mut session = IndexSession::start(&non_svg, 4).unwrap();
    assert_eq!(finish(&mut session, MAX_STEP_BYTES), IndexStatus::Complete);
    assert!(!session.progress().first_preview_ready);
    assert!(session.make_preview_source().is_none());

    for target_percent in [10, 50, 90] {
        let mut data = vec![b' '; 1024 * 1024];
        let prefix = b"<svg><rect width='1' height='1'/>";
        let suffix = b"</svg>";
        data[..prefix.len()].copy_from_slice(prefix);
        let tail_start = data.len() - suffix.len();
        data[tail_start..].copy_from_slice(suffix);
        let bytes = Arc::new(data);
        let mut canceled = IndexSession::start(&bytes, 6).unwrap();
        let mut progress = canceled.progress();
        while progress.bytes_read.saturating_mul(100) / progress.source_bytes < target_percent {
            assert_eq!(canceled.step(MAX_STEP_BYTES), IndexStatus::Ok);
            progress = canceled.progress();
        }
        assert!(!progress.complete);
        canceled.cancel();
        assert_eq!(canceled.step(64), IndexStatus::Canceled);
        assert!(canceled.progress().canceled);
        drop(canceled); // joins the validator before the borrowed source can be released
    }
}

#[test]
fn large_start_tag_is_indexed_without_becoming_preview_eligible() {
    let mut bytes = Vec::new();
    let prefix = b"<svg width='";
    let suffix = b"' height='1'><rect width='1' height='1'/></svg>";
    let target = 128 * 1024;
    bytes.extend_from_slice(prefix);
    bytes.resize(target - suffix.len(), b'1');
    bytes.extend_from_slice(suffix);
    let bytes = Arc::new(bytes);
    let mut session = IndexSession::start(&bytes, 9).unwrap();
    assert_eq!(finish(&mut session, MAX_STEP_BYTES), IndexStatus::Complete);
    assert!(!session.progress().first_preview_ready);
    assert!(session.element_at(1).is_some());
}
