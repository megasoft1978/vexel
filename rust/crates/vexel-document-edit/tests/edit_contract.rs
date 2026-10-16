use std::sync::Arc;
use vexel_document_edit::{EditError, EditLimits, Editor, TemplateKind};

fn editor(source: &[u8]) -> Editor {
    let source = Arc::new(source.to_vec());
    Editor::create(&source, EditLimits::default())
        .editor
        .expect("valid source")
}

fn bytes(editor: &Editor) -> Vec<u8> {
    let mut output = vec![0; editor.byte_length()];
    let length = editor.materialize(&mut output).expect("materialize");
    output.truncate(length);
    output
}

#[test]
fn source_ownership_queries_edits_and_undo_preserve_bytes() {
    let input =
        Arc::new(b"<?xml version='1.0'?>\r\n<svg><!--keep--><rect id='r' x='1'/></svg>\n".to_vec());
    let mut editor = Editor::create(&input, EditLimits::default())
        .editor
        .unwrap();
    drop(input);
    let rect = editor.find_id(b"r").unwrap();
    assert_eq!(editor.tag_name(rect), Some(&b"rect"[..]));
    let attribute = editor.attribute_at(rect, 1).unwrap();
    assert_eq!(attribute.quote, b'\'');
    assert_eq!(editor.span_bytes(attribute.value), Some(&b"1"[..]));
    editor.set_attribute(rect, b"x", b"<&'\"").unwrap();
    assert_eq!(
        editor.tag_name(rect),
        None,
        "source-state changes invalidate old handles"
    );
    assert_eq!(editor.history_count(), 1);
    assert!(editor.is_dirty());
    let escaped = b"x='&lt;&amp;&apos;\"'";
    assert!(bytes(&editor).windows(escaped.len()).any(|w| w == escaped));
    editor.undo().unwrap();
    assert_eq!(
        bytes(&editor),
        b"<?xml version='1.0'?>\r\n<svg><!--keep--><rect id='r' x='1'/></svg>\n"
    );
    assert!(!editor.is_dirty());
    editor.redo().unwrap();
}

#[test]
fn rejected_parse_keeps_source_history_and_snapshot_is_stable() {
    let mut editor = editor(b"<svg><rect id='r'/></svg>");
    let rect = editor.find_id(b"r").unwrap();
    let snapshot = editor.snapshot().unwrap();
    let before = bytes(&editor);
    assert_eq!(
        editor.replace_source_span(
            vexel_document_edit::SourceSpan {
                offset: 6,
                length: 1
            },
            b"<"
        ),
        Err(EditError::MalformedXml)
    );
    assert_eq!(bytes(&editor), before);
    assert_eq!(editor.history_count(), 0);
    let mut output = vec![0; snapshot.byte_length()];
    assert_eq!(snapshot.materialize(&mut output).unwrap(), before.len());
    assert_eq!(output, before);
    editor
        .insert_template(rect, TemplateKind::Circle, &[])
        .unwrap();
    assert_eq!(output, before);
}

#[test]
fn defaults_and_history_branching_are_bounded() {
    let mut editor = Editor::create(
        &Arc::new(b"<svg><rect x='0'/></svg>".to_vec()),
        EditLimits {
            history_commands: 2,
            history_bytes: 64,
        },
    )
    .editor
    .unwrap();
    let root = editor.root().unwrap();
    let rect = editor.model().first_child(root).unwrap();
    editor.set_attribute(rect, b"x", b"1").unwrap();
    let root = editor.root().unwrap();
    editor.set_attribute(root, b"width", b"2").unwrap();
    let root = editor.root().unwrap();
    let rect = editor.model().first_child(root).unwrap();
    editor.set_attribute(rect, b"x", b"3").unwrap();
    assert_eq!(editor.history_count(), 2);
    assert_eq!(editor.history_cursor(), 2);
    editor.undo().unwrap();
    let root = editor.root().unwrap();
    let rect = editor.model().first_child(root).unwrap();
    editor.set_attribute(rect, b"x", b"9").unwrap();
    assert_eq!(editor.redo(), Err(EditError::InvalidOperation));
}

#[test]
fn structural_edits_keep_sibling_gaps_and_group_round_trip_exactly() {
    let source = b"<svg><g id='layer'><rect id='a'/><!--gap--><circle id='b'/></g></svg>";
    let mut editor = editor(source);
    let a = editor.find_id(b"a").unwrap();
    let b = editor.find_id(b"b").unwrap();
    editor.reorder_sibling(a, b, false).unwrap();
    assert_eq!(
        bytes(&editor),
        b"<svg><g id='layer'><!--gap--><circle id='b'/><rect id='a'/></g></svg>"
    );
    editor.undo().unwrap();
    assert_eq!(bytes(&editor), source);

    let a = editor.find_id(b"a").unwrap();
    let group = editor.wrap_in_group(a).unwrap();
    assert_eq!(editor.tag_name(group), Some(&b"g"[..]));
    editor.ungroup(group).unwrap();
    assert_eq!(bytes(&editor), source);
}

#[test]
fn fragment_validation_template_ids_and_history_labels_match_contract() {
    let mut editor = editor(b"<svg/>");
    let root = editor.root().unwrap();
    let inserted = editor
        .insert_template(root, TemplateKind::Path, &[])
        .unwrap();
    assert_eq!(editor.tag_name(inserted), Some(&b"path"[..]));
    assert_eq!(editor.find_id(b"vexel-object-1"), Some(inserted));
    assert_eq!(editor.history_entry(0).unwrap().label, "Add path");
    editor.undo().unwrap();
    assert_eq!(bytes(&editor), b"<svg/>");
    let root = editor.root().unwrap();
    assert_eq!(
        editor.insert_xml_fragment(root, b"<rect/><circle/>"),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(
        editor.insert_xml_fragment(root, b"<g><path></g>"),
        Err(EditError::MalformedXml)
    );
    assert!(!editor.is_dirty());
}

#[test]
fn text_paths_attributes_and_region_replacements_keep_legacy_byte_rules() {
    let source = b"\xef\xbb\xbf<?xml version='1.0'?>\r\n<svg xmlns='urn:s'><g id='g'><text id='t'>A&amp;B<![CDATA[<c>]]></text><rect id='r' x='1'/></g></svg>\n";
    let mut editor = editor(source);
    let text = editor.find_id(b"t").unwrap();
    let mut content = [0u8; 32];
    let length = editor.text_content(text, &mut content).unwrap();
    assert_eq!(&content[..length], b"A&B<c>");
    let rect = editor.find_id(b"r").unwrap();
    let mut path = [0u8; 64];
    let length = editor.element_path(rect, &mut path).unwrap();
    assert_eq!(&path[..length], b"/svg[1]/g[1]/rect[1]");
    let rect_span = editor.source_span(rect).unwrap();
    assert_eq!(editor.element_at_offset(rect_span.offset + 1), Some(rect));

    editor.add_attribute(rect, b"class", b"shape&mark").unwrap();
    assert_eq!(
        editor.add_attribute(editor.find_id(b"r").unwrap(), b"class", b"again"),
        Err(EditError::DuplicateAttribute)
    );
    editor
        .remove_attribute(editor.find_id(b"r").unwrap(), b"x")
        .unwrap();
    assert!(
        bytes(&editor)
            .windows(b"class=\"shape&amp;mark\"".len())
            .any(|w| w == b"class=\"shape&amp;mark\"")
    );

    let group = editor.find_id(b"g").unwrap();
    let rect = editor.find_id(b"r").unwrap();
    let span = editor.source_span(rect).unwrap();
    editor
        .replace_xml_region(group, span, b"<circle id='replacement'/>")
        .unwrap();
    assert!(editor.find_id(b"replacement").is_some());
    editor.undo().unwrap();
    assert_eq!(
        bytes(&editor)
            .windows(b"<rect id='r'".len())
            .any(|w| w == b"<rect id='r'"),
        true
    );
}

#[test]
fn history_coalesces_only_while_the_current_state_is_unsaved() {
    let mut editor = editor(b"<svg data-a='00'/>");
    let root = editor.root().unwrap();
    editor.set_attribute(root, b"data-a", b"01").unwrap();
    let root = editor.root().unwrap();
    editor.set_attribute(root, b"data-a", b"02").unwrap();
    assert_eq!(editor.history_count(), 1);
    editor.undo().unwrap();
    assert_eq!(bytes(&editor), b"<svg data-a='00'/>");
    let root = editor.root().unwrap();
    editor.set_attribute(root, b"data-a", b"03").unwrap();
    editor.mark_saved();
    let root = editor.root().unwrap();
    editor.set_attribute(root, b"data-a", b"04").unwrap();
    assert_eq!(editor.history_count(), 2);
    assert_eq!(editor.history_saved_cursor(), Some(1));
}
