//! Closure tests for named C error priorities, caps, root/namespace rules,
//! exact sibling gaps, retained IDs and safe output ownership adaptations.
mod common;
use common::*;
use std::sync::Arc;
use vexel_document_edit::*;
#[test]
fn nested_duplicate_ids_and_references_are_byte_identical() {
    let source = b"<svg><g id='outer'><path id='inner' d='M0 0'/><use href='#inner'/></g></svg>";
    let mut edit = create(source);
    let node = id(&edit, b"outer");
    let duplicate = edit.duplicate_element(node).unwrap();
    assert_eq!(edit.tag_name(duplicate), Some(&b"g"[..]));
    assert_eq!(bytes(&edit),b"<svg><g id='outer'><path id='inner' d='M0 0'/><use href='#inner'/></g><g id='outer'><path id='inner' d='M0 0'/><use href='#inner'/></g></svg>");
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
    let root = edit.root().unwrap();
    assert_eq!(
        edit.duplicate_element(root),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(edit.wrap_in_group(root), Err(EditError::InvalidOperation));
}
#[test]
fn fragment_graphic_root_policy_prefixes_collisions_and_entities() {
    for tag in [
        b"a".as_slice(),
        b"circle",
        b"ellipse",
        b"image",
        b"line",
        b"path",
        b"polygon",
        b"polyline",
        b"rect",
        b"text",
        b"use",
        b"g",
        b"x:g",
        b"x:path",
    ] {
        assert!(graphics_tag(tag));
    }
    assert!(!graphics_tag(b"defs"));
    assert!(!graphics_tag(b"x:metadata"));
    let mut edit=create(b"<svg xmlns:x='urn:x'><rect id='vexel-object-&#49;'/><g id='layer'/><metadata id='face-&#x1f600;'/></svg>");
    assert!(edit.find_id("face-😀".as_bytes()).is_some());
    let layer = id(&edit, b"layer");
    let h = edit
        .insert_xml_fragment(layer, b"<x:g><x:path id='nested'/></x:g>")
        .unwrap();
    assert_eq!(id(&edit, b"vexel-object-2"), h);
    assert!(contains(
        &bytes(&edit),
        b"<x:g id=\"vexel-object-2\"><x:path id='nested'/></x:g>"
    ));
    let layer = id(&edit, b"layer");
    assert_eq!(
        edit.insert_xml_fragment(layer, b"<x:rect id='face-&#x1f600;'/>"),
        Err(EditError::DuplicateId)
    );
    assert!(edit.tag_name(layer).is_some());
    let layer = id(&edit, b"layer");
    edit.insert_xml_fragment(layer, b"<defs><path id='nested'/></defs>")
        .unwrap();
    assert!(contains(&bytes(&edit), b"<defs><path id='nested'/></defs>"));
    let mut output = [0; 8];
    assert_eq!(decode_bytes(b"&#X1F600;", &mut output), Ok(4));
    assert_eq!(&output[..4], "😀".as_bytes());
}
#[test]
fn auto_id_4096_attempts_reject_atomically() {
    let mut source = b"<svg><g id='layer'/>".to_vec();
    for i in 1..=4096 {
        source.extend_from_slice(format!("<rect id='vexel-object-{i}'/>").as_bytes());
    }
    source.extend_from_slice(b"</svg>");
    let mut edit = create(&source);
    let layer = id(&edit, b"layer");
    assert_eq!(
        edit.insert_xml_fragment(layer, b"<path/>"),
        Err(EditError::Limit)
    );
    assert_eq!(bytes(&edit), source);
    assert_eq!(edit.history_count(), 0);
    assert!(!edit.is_dirty());
    assert!(edit.tag_name(layer).is_some());
}
#[test]
fn source_whitespace_and_reverse_sibling_order_match_c() {
    let mut edit = create(b"<svg><rect id='a'\t x = '1'  /></svg>");
    let a = id(&edit, b"a");
    edit.add_attribute(a, b"class", b"c").unwrap();
    assert_eq!(
        bytes(&edit),
        b"<svg><rect id='a'\t x = '1' class=\"c\"  /></svg>"
    );
    let a = id(&edit, b"a");
    edit.remove_attribute(a, b"x").unwrap();
    assert_eq!(bytes(&edit), b"<svg><rect id='a'\t  class=\"c\"  /></svg>");
    let source = b"<svg><a id='a'/><!--gap--><b id='b'/><c/></svg>";
    let mut edit = create(source);
    let (a, b) = (id(&edit, b"a"), id(&edit, b"b"));
    edit.reorder_sibling(b, a, true).unwrap();
    assert_eq!(
        bytes(&edit),
        b"<svg><b id='b'/><a id='a'/><!--gap--><c/></svg>"
    );
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
    let (a, b) = (id(&edit, b"a"), id(&edit, b"b"));
    edit.reorder_sibling(a, b, true).unwrap();
    assert_eq!(edit.history_cursor(), 0);
    assert!(edit.tag_name(a).is_some());
}
#[test]
fn group_ungroup_sibling_sorting_gap_bytes_and_rejection_rules() {
    let source=b"<svg xmlns:i='urn:i'><g id='p'> \n<rect id='a'/><!--gap--><circle id='b'/>\t<ellipse id='c'/></g><g id='q'><path id='d'/></g></svg>";
    let mut edit = create(source);
    let (a, b, c, d) = (
        id(&edit, b"a"),
        id(&edit, b"b"),
        id(&edit, b"c"),
        id(&edit, b"d"),
    );
    assert_eq!(
        edit.group_siblings(&[a, c]),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(
        edit.group_siblings(&[a, d]),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(
        edit.group_siblings(&[a, a]),
        Err(EditError::InvalidOperation)
    );
    let group = edit.group_siblings(&[b, a]).unwrap();
    assert!(contains(
        &bytes(&edit),
        b"<g><rect id='a'/><!--gap--><circle id='b'/></g>"
    ));
    edit.ungroup_siblings(&[group]).unwrap();
    assert_eq!(bytes(&edit), source);
    let root = edit.root().unwrap();
    assert_eq!(edit.group_siblings(&[root]), Err(EditError::StaleHandle));
    assert_eq!(edit.group_siblings(&[]), Err(EditError::InvalidArgument));
    assert_eq!(edit.group_siblings(&vec![root; 129]), Err(EditError::Limit));
    assert_eq!(
        edit.ungroup_siblings(&vec![root; 129]),
        Err(EditError::Limit)
    );
    let mut edit =
        create(b"<svg><g id='a'><rect/></g><!--gap--><g id='b'/><g id='c'><path/></g></svg>");
    let (a, b, c) = (id(&edit, b"a"), id(&edit, b"b"), id(&edit, b"c"));
    assert_eq!(
        edit.ungroup_siblings(&[a, c]),
        Err(EditError::InvalidOperation)
    );
    edit.ungroup_siblings(&[c, b, a]).unwrap();
    assert_eq!(bytes(&edit), b"<svg><rect/><!--gap--><path/></svg>");
    edit.undo().unwrap();
    assert_eq!(
        bytes(&edit),
        b"<svg><g id='a'><rect/></g><!--gap--><g id='b'/><g id='c'><path/></g></svg>"
    );
}
#[test]
fn ungroup_root_and_namespace_names_follow_original_rules() {
    let mut edit = create(b"<g><svg/></g>");
    let root = edit.root().unwrap();
    edit.ungroup(root).unwrap();
    assert_eq!(bytes(&edit), b"<svg/>");
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), b"<g><svg/></g>");
    let mut edit = create(b"<g><rect/><circle/></g>");
    let root = edit.root().unwrap();
    assert_eq!(edit.ungroup(root), Err(EditError::MalformedXml));
    assert_eq!(bytes(&edit), b"<g><rect/><circle/></g>");
    assert!(edit.tag_name(root).is_some());
    let mut edit = create(b"<svg xmlns:i='urn:i'><i:g id='g'><path/></i:g></svg>");
    let h = id(&edit, b"g");
    assert_eq!(edit.ungroup(h), Err(EditError::InvalidOperation));
    assert_eq!(
        edit.ungroup_siblings(&[h]),
        Err(EditError::InvalidOperation)
    );
}
#[test]
fn xml_region_raw_source_boundaries_names_and_error_priority() {
    let source = b"<svg><g id='g'><rect id='r' x='1'/></g><path/></svg>";
    let mut edit = create(source);
    let group = id(&edit, b"g");
    let rect = id(&edit, b"r");
    let region = edit.source_span(rect).unwrap();
    assert_eq!(
        edit.replace_xml_region(
            group,
            SourceSpan {
                offset: 0,
                length: 1
            },
            b""
        ),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(
        edit.replace_xml_region(group, region, &vec![b'x'; MAX_FRAGMENT_BYTES + 1]),
        Err(EditError::Limit)
    );
    assert_eq!(
        edit.replace_source_span(
            SourceSpan {
                offset: usize::MAX,
                length: 1
            },
            b""
        ),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(
        edit.replace_source_span(
            SourceSpan {
                offset: 0,
                length: 0
            },
            &vec![b'x'; MAX_FRAGMENT_BYTES + 1]
        ),
        Err(EditError::Limit)
    );
    assert_eq!(
        edit.set_attribute(rect, b"bad name", b"x"),
        Err(EditError::InvalidName)
    );
    assert_eq!(
        edit.add_attribute(rect, b"x", &[1]),
        Err(EditError::DuplicateAttribute)
    );
    let old = rect;
    edit.set_attribute(rect, b"x", b"2").unwrap();
    assert_eq!(
        edit.set_attribute(old, b"bad name", b"x"),
        Err(EditError::InvalidName)
    );
    assert_eq!(
        edit.set_attribute(old, b"x", b"1"),
        Err(EditError::StaleHandle)
    );
    assert!(name_valid("π:属性".as_bytes()));
    assert!(!name_valid(b""));
    assert!(!name_valid(&vec![b'a'; 4097]));
    assert_eq!(
        encode_attribute(b"'\"<&", b'\''),
        Ok(b"&apos;\"&lt;&amp;".to_vec())
    );
    assert_eq!(
        encode_attribute(b"x", b'?'),
        Err(EditError::InvalidArgument)
    );
}
#[test]
fn output_capacities_never_write_partial_and_report_original_lengths() {
    let edit = create(b"<svg><text id='t'>A&amp;B<![CDATA[<c>]]></text></svg>");
    let h = id(&edit, b"t");
    let mut output = [0xa5; 2];
    let mut length = 99;
    assert!(!edit.materialize_into(&mut output, &mut length));
    assert_eq!(length, edit.byte_length());
    assert_eq!(output, [0xa5; 2]);
    let snapshot = edit.snapshot().unwrap();
    assert!(!snapshot.materialize_into(&mut output, &mut length));
    assert_eq!(length, 0);
    assert_eq!(output, [0xa5; 2]);
    assert!(!edit.text_content_into(h, &mut output, &mut length));
    assert_eq!(length, 6);
    assert_eq!(output, [0xa5; 2]);
    assert!(!edit.element_path_into(h, &mut output, &mut length));
    assert_eq!(length, b"/svg[1]/text[1]".len());
    assert_eq!(output, [0xa5; 2]);
    let edit = create(b"<svg x='A&amp;B'/>");
    let root = edit.root().unwrap();
    let a = edit.attribute_at(root, 0).unwrap();
    assert!(!edit.decode_span_into(a.value, &mut output, &mut length));
    assert_eq!(length, 3);
    assert_eq!(output, [0xa5; 2]);
    assert_eq!(
        decode_bytes(b"&unknown;", &mut output),
        Err(EditError::MalformedXml)
    );
    assert_eq!(output, [0xa5; 2]);
}
#[test]
fn source_history_depth_and_actual_64mib_token_caps() {
    let source = Arc::new(vec![b'x'; DEFAULT_SOURCE_BYTES + 1]);
    let r = Editor::create(
        &source,
        EditLimits {
            history_commands: usize::MAX,
            history_bytes: usize::MAX,
        },
    );
    assert_eq!(r.error, EditError::Limit);
    assert_eq!(
        r.diagnostic,
        vexel_xml::XmlDiagnostic {
            category: vexel_xml::XmlError::InputLimit,
            offset: 0
        }
    );
    for limits in [
        EditLimits {
            history_commands: MAX_HISTORY_COMMANDS + 1,
            history_bytes: 1,
        },
        EditLimits {
            history_commands: 1,
            history_bytes: MAX_HISTORY_BYTES + 1,
        },
    ] {
        let r = Editor::create(&Arc::new(b"<svg/>".to_vec()), limits);
        assert_eq!(r.error, EditError::Limit);
        assert_eq!(r.diagnostic, vexel_xml::XmlDiagnostic::default());
    }
    let token_limit = (64 * 1024 * 1024) / std::mem::size_of::<vexel_xml::XmlToken>();
    assert!(token_limit < 1000000);
    let mut source = b"<svg>".to_vec();
    for _ in 0..token_limit {
        source.extend_from_slice(b"<a/>");
    }
    source.extend_from_slice(b"</svg>");
    let r = Editor::create(&Arc::new(source), EditLimits::default());
    assert!(r.editor.is_none());
    assert_eq!(r.error, EditError::Limit);
    assert_eq!(r.diagnostic.category, vexel_xml::XmlError::OutputExhausted);
    let mut deep = Vec::new();
    for _ in 0..256 {
        deep.extend_from_slice(b"<g>");
    }
    for _ in 0..256 {
        deep.extend_from_slice(b"</g>");
    }
    let edit = create(&deep);
    let h = edit.model().node_at(255).unwrap();
    let mut out = [0xa5; 16];
    assert_eq!(
        edit.element_path(h, &mut out),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(out, [0xa5; 16]);
}
#[test]
fn doctype_unknown_metadata_quotes_and_caller_arc_copy_on_write_remain_exact() {
    let source=b"<?xml version='1.0'?>\r\n<!DOCTYPE svg [<!--keep-->]>\n<svg xmlns:i='urn:i'><i:unknown i:metadata='keep'><rect id='r' x='1'/></i:unknown></svg>\n";
    let mut owner = Arc::new(source.to_vec());
    let mut edit = Editor::create(&owner, EditLimits::default())
        .editor
        .unwrap();
    Arc::make_mut(&mut owner)[0] = b'!';
    assert_eq!(bytes(&edit), source);
    set(&mut edit, b"r", b"x", b"2");
    let expected = source.windows(5).position(|w| w == b"x='1'").unwrap() + 3;
    let mut oracle = source.to_vec();
    oracle[expected] = b'2';
    assert_eq!(bytes(&edit), oracle);
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
}
#[test]
fn labels_noop_and_unsaved_marker_preserve_identity_and_cursors() {
    let mut edit = create(b"<svg x='0'/>");
    let root = edit.root().unwrap();
    edit.set_attribute(root, b"x", b"0").unwrap();
    assert_eq!(edit.history_count(), 0);
    assert_eq!(edit.root(), Some(root));
    edit.mark_unsaved();
    assert!(edit.is_dirty());
    assert_eq!(edit.history_saved_cursor(), None);
    edit.mark_saved();
    assert!(!edit.is_dirty());
    for (action, label) in [
        (HistoryAction::EditAttribute, "Edit attribute"),
        (HistoryAction::AddAttribute, "Add attribute"),
        (HistoryAction::RemoveAttribute, "Remove attribute"),
        (HistoryAction::EditText, "Edit text"),
        (HistoryAction::Delete, "Delete"),
        (HistoryAction::Duplicate, "Duplicate"),
        (HistoryAction::Move, "Move"),
        (HistoryAction::Group, "Group"),
        (HistoryAction::Ungroup, "Ungroup"),
        (HistoryAction::AddRectangle, "Add rectangle"),
        (HistoryAction::AddCircle, "Add circle"),
        (HistoryAction::AddEllipse, "Add ellipse"),
        (HistoryAction::AddLine, "Add line"),
        (HistoryAction::AddPath, "Add path"),
        (HistoryAction::AddGroup, "Add group"),
        (HistoryAction::AddText, "Add text"),
        (HistoryAction::AddShape, "Add shape"),
        (HistoryAction::EditObjects, "Edit objects"),
        (HistoryAction::EditPath, "Edit path"),
        (HistoryAction::Resize, "Resize"),
        (HistoryAction::Rotate, "Rotate"),
    ] {
        assert_eq!(action.label(), label);
        assert!(label.len() < HISTORY_LABEL_CAPACITY);
    }
    edit.set_attribute(root, b"x", b"1").unwrap();
    edit.history_relabel_latest(HistoryAction::Resize);
    assert_eq!(edit.history_entry(0).unwrap().label, "Resize");
    assert_eq!(edit.history_entry(0).unwrap().state_cursor, 1);
    edit.mark_saved();
    assert!(edit.history_entry(0).unwrap().is_saved);
}
#[test]
fn cached_slice_text_path_and_view_queries_do_not_allocate() {
    let edit = create(b"<svg><text id='t'>A&amp;B</text></svg>");
    let h = id(&edit, b"t");
    let attribute = edit.attribute_at(h, 0).unwrap();
    let span = edit.source_span(h).unwrap();
    let mut output = [0; 64];
    allocation_gate(Some(0));
    assert!(edit.root().is_some());
    assert_eq!(edit.tag_name(h), Some(&b"text"[..]));
    assert!(edit.span_equals(attribute.value, b"t"));
    assert_eq!(edit.source_span(h), Some(span));
    assert_eq!(edit.attribute_count(h), 1);
    assert!(edit.attribute_at(h, 0).is_some());
    assert_eq!(edit.text_content(h, &mut output), Ok(3));
    assert_eq!(
        edit.element_path(h, &mut output),
        Ok(b"/svg[1]/text[1]".len())
    );
    assert_eq!(edit.decoded_span(attribute.value, &mut output), Ok(1));
    allocation_gate(None);
}
#[test]
fn raw_decoder_matches_c_numeric_rules_while_parser_checks_xml_char() {
    let mut output = [0xa5; 8];
    assert_eq!(decode_bytes(b"&#0;", &mut output), Ok(1));
    assert_eq!(output[0], 0);
    assert_eq!(decode_bytes(b"&#xD800;", &mut output), Ok(3));
    assert_eq!(&output[..3], &[0xed, 0xa0, 0x80]);
    for raw in [
        b"&#;".as_slice(),
        b"&#x;",
        b"&unknown;",
        b"&#+1;",
        b"&#4294967296;",
    ] {
        let before = output;
        assert_eq!(decode_bytes(raw, &mut output), Err(EditError::MalformedXml));
        assert_eq!(output, before);
    }
    let mut edit = create(b"<svg><text id='t'/></svg>");
    let h = id(&edit, b"t");
    assert_eq!(edit.set_text_content(h, &[0]), Err(EditError::MalformedXml));
    assert!(edit.tag_name(h).is_some());
}
#[test]
fn public_decoded_id_lookup_and_internal_raw_collision_priority_match_c() {
    let mut edit = create(b"<svg><rect id='a&amp;'/><g id='layer'/></svg>");
    assert!(edit.find_id(b"a&").is_some());
    assert_eq!(edit.find_id(b"a&amp;"), None);
    let layer = id(&edit, b"layer");
    let before = bytes(&edit);
    assert_eq!(
        edit.insert_xml_fragment(layer, b"<circle id='a&amp;amp;'/>"),
        Err(EditError::DuplicateId)
    );
    assert_eq!(bytes(&edit), before);
    assert_eq!(edit.history_count(), 0);
    assert!(edit.tag_name(layer).is_some());
}
#[test]
fn mutation_stale_handles_reject_without_rebuilding_invalidated_cache() {
    let mut edit = create(b"<svg><rect id='r' x='0'/></svg>");
    let h = id(&edit, b"r");
    edit.set_attribute(h, b"x", b"1").unwrap();
    assert_eq!(edit.allocation_stats().cache_bytes, 0);
    allocation_gate(Some(0));
    assert_eq!(edit.delete_element(h), Err(EditError::StaleHandle));
    assert_eq!(edit.duplicate_element(h), Err(EditError::StaleHandle));
    assert_eq!(edit.set_text_content(h, b"x"), Err(EditError::StaleHandle));
    assert_eq!(edit.wrap_in_group(h), Err(EditError::StaleHandle));
    assert_eq!(
        edit.insert_template(h, TemplateKind::Circle, &[]),
        Err(EditError::StaleHandle)
    );
    allocation_gate(None);
    assert_eq!(edit.allocation_stats().cache_bytes, 0);
    assert_eq!(edit.history_count(), 1);
}
#[test]
fn fragment_and_template_caps_include_generated_id_and_parent_wrapper() {
    let mut edit = create(b"<svg/>");
    let root = edit.root().unwrap();
    let prefix = b"<path data='";
    let suffix = b"'/>";
    let mut large = prefix.to_vec();
    large.extend(vec![b'x'; MAX_FRAGMENT_BYTES - prefix.len() - suffix.len()]);
    large.extend_from_slice(suffix);
    assert_eq!(large.len(), MAX_FRAGMENT_BYTES);
    assert_eq!(
        edit.insert_xml_fragment(root, &large),
        Err(EditError::Limit)
    );
    assert!(!edit.is_dirty());
    assert_eq!(edit.root(), Some(root));
    let inserted = edit.insert_xml_fragment(root, b"<path/>").unwrap();
    assert_eq!(id(&edit, b"vexel-object-1"), inserted);
    let mut edit = create(b"<svg/>");
    let root = edit.root().unwrap();
    let prefix = b"<path id=\"supplied\" data=\"";
    let suffix = b"\"/>";
    let body = vec![b'x'; MAX_FRAGMENT_BYTES - prefix.len() - suffix.len()];
    let attrs = [
        TemplateAttribute {
            name: b"id",
            value: b"supplied",
        },
        TemplateAttribute {
            name: b"data",
            value: &body,
        },
    ];
    assert_eq!(
        edit.insert_template(root, TemplateKind::Path, &attrs),
        Err(EditError::Limit)
    );
    assert_eq!(bytes(&edit), b"<svg/>");
    let mut fragment = prefix.to_vec();
    fragment.extend_from_slice(&body);
    fragment.extend_from_slice(suffix);
    assert_eq!(fragment.len(), MAX_FRAGMENT_BYTES);
    edit.insert_xml_fragment(root, &fragment).unwrap();
    let mut oracle = b"<svg>".to_vec();
    oracle.extend_from_slice(&fragment);
    oracle.extend_from_slice(b"</svg>");
    assert_eq!(bytes(&edit), oracle);
}
#[test]
fn error_names_preserve_all_original_categories() {
    for (error, name) in [
        (EditError::Ok, "ok"),
        (EditError::InvalidArgument, "invalid_argument"),
        (EditError::StaleHandle, "stale_handle"),
        (EditError::NotFound, "not_found"),
        (EditError::DuplicateAttribute, "duplicate_attribute"),
        (EditError::DuplicateId, "duplicate_id"),
        (EditError::InvalidName, "invalid_name"),
        (EditError::MalformedXml, "malformed_xml"),
        (EditError::RootDelete, "root_delete"),
        (EditError::InvalidOperation, "invalid_operation"),
        (EditError::Limit, "limit"),
        (EditError::HistoryLimit, "history_limit"),
        (EditError::OutOfMemory, "out_of_memory"),
        (EditError::SaveFailed, "save_failed"),
    ] {
        assert_eq!(error.as_str(), name);
    }
    let absent: Option<&EditSnapshot> = None;
    assert_eq!(absent.map_or(0, EditSnapshot::byte_length), 0);
}
