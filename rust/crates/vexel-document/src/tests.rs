use crate::parse::{parse_document_with_storage, LargeDocumentError, OwnedParseError};
use crate::types::DocumentIdAllocator;
use crate::{parse_document, DocumentError, DocumentSourceSpan};
use std::mem::size_of;
use std::sync::{Arc, Weak};
use vexel_xml::{XmlError, XmlToken};

const SOURCE: &[u8] = b"<?xml version='1.0'?>\r\n<svg xmlns='urn:svg' xmlns:i='urn:inkscape'><g i:label='keep'><rect id='one'/><i:unknown><path d='M0 0'/></i:unknown></g><circle/></svg>\n";

fn name_is(model: &crate::DocumentModel<'_>, handle: crate::DocumentHandle, expected: &[u8]) -> bool {
    model.tag_name(handle) == Some(expected)
}

#[test]
fn original_fixture_preserves_source_spans_attributes_and_tree_queries() {
    let original = SOURCE.to_vec();
    let first = parse_document(SOURCE).expect("valid source");
    let second = parse_document(SOURCE).expect("same source can open twice");
    assert_eq!(first.node_count(), 6);
    assert_eq!(first.live_node_count(), 6);
    assert_ne!(first.document_id(), second.document_id());
    assert_eq!(first.source(), SOURCE);

    let root = first.root().expect("root");
    assert!(name_is(&first, root, b"svg"));
    let group = first.first_child(root).expect("group");
    assert_eq!(first.first_child(root), Some(group));
    assert!(name_is(&first, group, b"g"));
    assert_eq!(first.parent(group), Some(root));
    assert_eq!(first.depth(root), Some(0));
    assert_eq!(first.depth(group), Some(1));
    assert_eq!(first.node_at(0), Some(root));
    assert_eq!(first.node_at(first.node_count()), None);

    let rectangle = first.first_child(group).expect("rect");
    assert!(name_is(&first, rectangle, b"rect"));
    let unknown = first.next_sibling(rectangle).expect("unknown editor node");
    assert!(name_is(&first, unknown, b"i:unknown"));
    let path = first.first_child(unknown).expect("path");
    assert!(name_is(&first, path, b"path"));
    assert_eq!(first.depth(path), Some(3));
    let circle = first.next_sibling(group).expect("circle");
    assert!(name_is(&first, circle, b"circle"));
    assert_eq!(first.next_sibling(circle), None);
    assert_eq!(first.next_sibling(root), None);

    assert_eq!(first.attribute_count(group), 1);
    assert_eq!(first.attribute_count(root), 2);
    assert_eq!(first.attribute_at(group, 0), Some((&b"i:label"[..], &b"keep"[..])));
    assert_eq!(first.attribute_at(group, 1), None);
    assert_eq!(first.attribute_at(root, 0), Some((&b"xmlns"[..], &b"urn:svg"[..])));
    let group_span = first.source_span(group).expect("group span");
    let expected_group = b"<g i:label='keep'><rect id='one'/><i:unknown><path d='M0 0'/></i:unknown></g>";
    assert_eq!(&SOURCE[group_span.offset..group_span.offset + group_span.length], expected_group);
    let circle_span = first.source_span(circle).expect("empty element span");
    assert_eq!(&SOURCE[circle_span.offset..circle_span.offset + circle_span.length], b"<circle/>");
    assert_eq!(SOURCE, original.as_slice());
}

#[test]
fn foreign_wrong_generation_and_deleted_handles_are_rejected() {
    let mut first = parse_document(SOURCE).unwrap();
    let second = parse_document(SOURCE).unwrap();
    let root = first.root().unwrap();
    let group = first.first_child(root).unwrap();
    let rectangle = first.first_child(group).unwrap();
    let unknown = first.next_sibling(rectangle).unwrap();
    let path = first.first_child(unknown).unwrap();
    assert_eq!(second.tag_name(root), None);
    assert_eq!(first.tag_name(crate::DocumentHandle { slot: u32::MAX, ..root }), None);
    assert_eq!(first.tag_name(crate::DocumentHandle { generation: 0, ..root }), None);
    assert_eq!(first.tag_name(crate::DocumentHandle { generation: root.generation + 1, ..root }), None);
    assert!(first.remove_subtree(group));
    assert_eq!(first.live_node_count(), 2);
    for stale in [group, rectangle, unknown, path] {
        assert_eq!(first.tag_name(stale), None);
        assert_eq!(first.source_span(stale), None);
        assert_eq!(first.parent(stale), None);
    }
    assert_eq!(first.first_child(root).map(|handle| first.tag_name(handle)), Some(Some(&b"circle"[..])));
    assert!(!first.remove_subtree(group));
    assert!(first.remove_subtree(root));
    assert_eq!(first.live_node_count(), 0);
    assert_eq!(first.root(), None);
    assert_eq!(first.node_at(0), None);
    assert_eq!(first.source(), SOURCE);
}

#[test]
fn comments_doctypes_unicode_and_unknown_raw_metadata_remain_in_source() {
    let source = b"<?xml version='1.0'?><!-- note --><!DOCTYPE svg [<!ENTITY local 'raw'>]><svg x:unknown='v&amp;' title='\xc3\xa9'><x:thing/></svg>";
    let model = parse_document(source).expect("supported document constructs");
    assert_eq!(model.node_count(), 2);
    let root = model.root().unwrap();
    assert_eq!(model.attribute_at(root, 0), Some((&b"x:unknown"[..], &b"v&amp;"[..])));
    assert_eq!(model.attribute_at(root, 1), Some((&b"title"[..], &b"\xc3\xa9"[..])));
    assert_eq!(model.tag_name(model.first_child(root).unwrap()), Some(&b"x:thing"[..]));
    let span = model.source_span(root).unwrap();
    assert_eq!(&source[span.offset..span.offset + span.length], b"<svg x:unknown='v&amp;' title='\xc3\xa9'><x:thing/></svg>");
    assert_eq!(model.source(), source);
}

#[test]
fn parse_errors_keep_xml_categories_and_offsets() {
    let mismatch = parse_document(b"<svg><g></svg>").unwrap_err();
    assert_eq!(mismatch.error, DocumentError::Parse);
    assert_eq!(mismatch.diagnostic.category, XmlError::MismatchedTag);
    assert_eq!(mismatch.diagnostic.offset, 8);

    let invalid_utf8 = parse_document(b"<svg>\xff</svg>").unwrap_err();
    assert_eq!(invalid_utf8.error, DocumentError::Parse);
    assert_eq!(invalid_utf8.diagnostic.category, XmlError::InvalidUtf8);
    assert_eq!(invalid_utf8.diagnostic.offset, 5);

    let invalid_character = parse_document(b"<svg>\0</svg>").unwrap_err();
    assert_eq!(invalid_character.error, DocumentError::Parse);
    assert_eq!(invalid_character.diagnostic.category, XmlError::InvalidCharacter);
    assert_eq!(invalid_character.diagnostic.offset, 5);

    let duplicate = parse_document(b"<svg a='1' a='2'/>").unwrap_err();
    assert_eq!(duplicate.error, DocumentError::Parse);
    assert_eq!(duplicate.diagnostic.category, XmlError::DuplicateAttribute);
    assert_eq!(duplicate.diagnostic.offset, 11);
}

#[test]
fn input_depth_and_actual_token_storage_limits_map_without_success_hooks() {
    let mut oversized = Vec::new();
    oversized.try_reserve_exact(vexel_xml::MAX_INPUT_BYTES + 1).unwrap();
    oversized.resize(vexel_xml::MAX_INPUT_BYTES + 1, b' ');
    let too_large = parse_document(&oversized).unwrap_err();
    assert_eq!(too_large.error, DocumentError::Limit);
    assert_eq!(too_large.diagnostic.category, XmlError::InputLimit);
    assert_eq!(too_large.diagnostic.offset, 0);

    let mut deep = Vec::new();
    for _ in 0..=256 { deep.extend_from_slice(b"<x>"); }
    for _ in 0..=256 { deep.extend_from_slice(b"</x>"); }
    let depth_error = parse_document(&deep).unwrap_err();
    assert_eq!(depth_error.error, DocumentError::Limit);
    assert_eq!(depth_error.diagnostic.category, XmlError::DepthLimit);
    assert_eq!(depth_error.diagnostic.offset, 256 * 3);

    let at_capacity = parse_document_with_storage(b"<svg/>", size_of::<XmlToken>()).unwrap();
    assert_eq!(at_capacity.node_count(), 1);
    let partial_byte_capacity = parse_document_with_storage(b"<svg/>", size_of::<XmlToken>() * 2 - 1).unwrap();
    assert_eq!(partial_byte_capacity.node_count(), 1);
    let exhausted = parse_document_with_storage(b"<svg><a/><b/></svg>", size_of::<XmlToken>() * 3).unwrap_err();
    assert_eq!(exhausted.error, DocumentError::ResourceExhausted);
    assert_eq!(exhausted.diagnostic.category, XmlError::OutputExhausted);
    assert_eq!(exhausted.diagnostic.offset, 13);
    let below_one_token = parse_document_with_storage(b"<svg/>", size_of::<XmlToken>() - 1).unwrap_err();
    assert_eq!(below_one_token.error, DocumentError::ResourceExhausted);
}

#[test]
fn production_token_storage_cap_matches_original_boundary_case() {
    let token_capacity = (64 * 1024 * 1024) / size_of::<XmlToken>();
    assert!(token_capacity < crate::model::MAX_NODES);
    let child_bytes = token_capacity.checked_mul(4).unwrap();
    let total_bytes = 5usize.checked_add(child_bytes).unwrap().checked_add(6).unwrap();
    let mut source = Vec::new();
    source.try_reserve_exact(total_bytes).unwrap();
    source.extend_from_slice(b"<svg>");
    for _ in 0..token_capacity { source.extend_from_slice(b"<a/>"); }
    source.extend_from_slice(b"</svg>");
    let exhausted = parse_document(&source).unwrap_err();
    assert_eq!(exhausted.error, DocumentError::ResourceExhausted);
    assert_eq!(exhausted.diagnostic.category, XmlError::OutputExhausted);
    let expected_offset = 5 + (token_capacity - 1) * 4;
    assert_eq!(exhausted.diagnostic.offset, expected_offset);
}

#[test]
fn id_exhaustion_seam_never_wraps_or_allocates_reserved_values() {
    let mut normal = DocumentIdAllocator::starting_at(41);
    assert_eq!(normal.allocate(), Some(41));
    assert_eq!(normal.allocate(), Some(42));
    let mut zero = DocumentIdAllocator::starting_at(0);
    assert_eq!(zero.allocate(), None);
    assert_eq!(zero.allocate(), None);
    let mut max = DocumentIdAllocator::starting_at(u64::MAX);
    assert_eq!(max.allocate(), None);
    assert_eq!(max.allocate(), None);
    let mut before_max = DocumentIdAllocator::starting_at(u64::MAX - 1);
    assert_eq!(before_max.allocate(), Some(u64::MAX - 1));
    assert_eq!(before_max.allocate(), None);
}

#[test]
fn source_span_value_has_checked_plain_byte_coordinates() {
    let span = DocumentSourceSpan { offset: 3, length: 4 };
    assert_eq!(span.offset, 3);
    assert_eq!(span.length, 4);
}

#[test]
fn stable_error_names_match_the_legacy_model_contract() {
    assert_eq!(DocumentError::InvalidArgument.as_str(), "invalid_argument");
    assert_eq!(DocumentError::Parse.as_str(), "parse");
    assert_eq!(DocumentError::Limit.as_str(), "limit");
    assert_eq!(DocumentError::ResourceExhausted.as_str(), "resource_exhausted");
    assert_eq!(DocumentError::OutOfMemory.as_str(), "out_of_memory");
    assert_eq!(DocumentError::IdExhausted.as_str(), "id_exhausted");
}

#[test]
fn owned_model_shares_source_and_preserves_full_query_delete_and_handle_behavior() {
    let source = Arc::new(SOURCE.to_vec());
    let weak: Weak<Vec<u8>> = Arc::downgrade(&source);
    let original_ptr = source.as_ptr();
    let mut model = crate::parse_owned_document(&source).expect("owned parse");
    assert_eq!(Arc::strong_count(&source), 2);
    assert_eq!(model.source().as_ptr(), original_ptr);
    assert!(Arc::ptr_eq(model.source_owner(), &source));
    assert_eq!(weak.strong_count(), 2);
    let root = model.root().unwrap();
    let group = model.first_child(root).unwrap();
    let rect = model.first_child(group).unwrap();
    let unknown = model.next_sibling(rect).unwrap();
    let path = model.first_child(unknown).unwrap();
    let circle = model.next_sibling(group).unwrap();
    let other = crate::parse_owned_document(&source).unwrap();
    assert_ne!(model.document_id(), other.document_id());
    assert_eq!(other.tag_name(root), None);
    drop(other);
    assert_eq!(model.parent(group), Some(root));
    assert_eq!(model.parent(path), Some(unknown));
    assert_eq!(model.next_sibling(group), Some(circle));
    assert_eq!(model.next_sibling(circle), None);
    assert_eq!(model.node_at(path.slot as usize), Some(path));
    assert_eq!(model.node_at(model.node_count()), None);
    assert_eq!(model.tag_name(group), Some(&b"g"[..]));
    assert_eq!(model.attribute_at(group, 0), Some((&b"i:label"[..], &b"keep"[..])));
    assert_eq!(model.attribute_count(root), 2);
    assert_eq!(model.depth(rect), Some(2));
    let rectangle = model.source_span(rect).unwrap();
    assert_eq!(&model.source()[rectangle.offset..rectangle.offset + rectangle.length], b"<rect id='one'/>");
    assert_eq!(model.tag_name(crate::DocumentHandle { generation: 0, ..root }), None);
    assert_eq!(model.tag_name(crate::DocumentHandle { slot: u32::MAX, ..root }), None);
    assert!(model.remove_subtree(group));
    assert_eq!(model.live_node_count(), 2);
    assert_eq!(model.tag_name(rect), None);
    assert_eq!(model.first_child(root), Some(circle));
    assert!(!model.remove_subtree(group));
    assert!(model.remove_subtree(root));
    assert_eq!(model.root(), None);
    assert_eq!(model.live_node_count(), 0);
    assert_eq!(model.source().as_ptr(), original_ptr);

    drop(source);
    assert!(weak.upgrade().is_some());
    let shared = Arc::new(model);
    let reader = Arc::clone(&shared);
    let joined = std::thread::spawn(move || {
        assert_eq!(reader.live_node_count(), 0);
        assert_eq!(&reader.source()[..4], b"<?xm");
        reader.source().as_ptr() as usize
    }).join().expect("Send + Sync owned model");
    assert_eq!(joined, original_ptr as usize);
    drop(shared);
    assert!(weak.upgrade().is_none());
}

#[test]
fn owned_rejection_keeps_caller_arc_and_large_entrypoints_keep_legacy_cap() {
    let source = Arc::new(vec![b'<', b's', b'v', b'g', b'/', b'>']);
    let before = Arc::strong_count(&source);
    let error = crate::parse_owned_document_large_cancellable(&source, || true).unwrap_err();
    assert_eq!(error, OwnedParseError::Canceled { offset: 0 });
    assert_eq!(Arc::strong_count(&source), before);

    let mut large = Vec::new();
    large.try_reserve_exact(vexel_xml::MAX_INPUT_BYTES + 1).unwrap();
    large.resize(vexel_xml::MAX_INPUT_BYTES + 1, b' ');
    let error = crate::parse_document(&large).unwrap_err();
    assert_eq!(error.error, DocumentError::Limit);
    assert_eq!(error.diagnostic.category, XmlError::InputLimit);
    assert_eq!(error.diagnostic.offset, 0);
    let owned = Arc::new(large);
    let before = Arc::strong_count(&owned);
    assert_eq!(crate::parse_owned_document(&owned).unwrap_err(), OwnedParseError::Parse(error));
    assert_eq!(Arc::strong_count(&owned), before);
}

#[test]
fn large_cancellation_is_reported_during_validation_and_table_build() {
    let validation_source = b"<svg><g/></svg>";
    let mut polls = 0usize;
    let canceled = crate::parse_document_large_cancellable(validation_source, || {
        polls += 1;
        polls == 4
    }).unwrap_err();
    assert_eq!(canceled, LargeDocumentError::Canceled { offset: 0 });

    let source = b"<svg><a/><b/><c/></svg>";
    let mut completed_polls = 0usize;
    let parsed = crate::parse_document_large_cancellable(source, || {
        completed_polls += 1;
        false
    }).unwrap();
    let table_polls = parsed.node_count() + 1; // one poll for each element/end token
    assert!(completed_polls > table_polls);
    let cancel_at = completed_polls - table_polls + 2;
    let mut polls = 0usize;
    let canceled = crate::parse_document_large_cancellable(source, || {
        polls += 1;
        polls == cancel_at
    }).unwrap_err();
    assert!(matches!(canceled, LargeDocumentError::Canceled { offset } if offset > 0));
}

#[test]
fn large_sources_over_legacy_cap_and_one_hundred_megabytes_parse_without_copying() {
    let mut over_legacy = Vec::new();
    let payload = vexel_xml::MAX_INPUT_BYTES + 1;
    let total = b"<svg a='".len() + payload + b"'/>".len();
    over_legacy.try_reserve_exact(total).unwrap();
    over_legacy.extend_from_slice(b"<svg a='");
    over_legacy.resize(total - 3, b'x');
    over_legacy.extend_from_slice(b"'/>");
    assert_eq!(over_legacy.len(), total);
    let borrowed = crate::parse_document_large(&over_legacy).expect("large borrowed parse");
    assert_eq!(borrowed.node_count(), 1);
    assert_eq!(borrowed.attribute_at(borrowed.root().unwrap(), 0).unwrap().1.len(), payload);

    let payload = 100 * 1024 * 1024;
    let total = b"<svg a='".len() + payload + b"'/>".len();
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(total).unwrap();
    bytes.extend_from_slice(b"<svg a='");
    bytes.resize(total - 3, b'z');
    bytes.extend_from_slice(b"'/>");
    assert_eq!(bytes.len(), total);
    let source = Arc::new(bytes);
    let ptr = source.as_ptr();
    let model = crate::parse_owned_document_large(&source).expect("100 MiB owned source");
    assert_eq!(model.source().len(), total);
    assert_eq!(model.source().as_ptr(), ptr);
    assert_eq!(model.attribute_at(model.root().unwrap(), 0).unwrap().1.len(), payload);
    assert!(Arc::ptr_eq(model.source_owner(), &source));
}
