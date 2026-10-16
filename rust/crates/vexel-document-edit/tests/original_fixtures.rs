//! Direct fixture/assertion port of tests/edit/test_document_edit.c.
mod common;
use common::*;
use vexel_document_edit::*;
#[test]
fn test_generated_object_ids() {
    let source = b"<svg><rect id='vexel-object-1'/><g id='layer'/></svg>";
    let mut edit = create(source);
    let layer = id(&edit, b"layer");
    let inserted = edit
        .insert_template(layer, TemplateKind::Path, &[])
        .unwrap();
    assert_eq!(id(&edit, b"vexel-object-2"), inserted);
    assert!(contains(&bytes(&edit), b"<path id=\"vexel-object-2\"/>"));
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
    edit.redo().unwrap();
    id(&edit, b"vexel-object-2");
    let before = bytes(&edit);
    let layer = id(&edit, b"layer");
    let inserted = edit
        .insert_xml_fragment(layer, b"<g><path d='M0 0L1 1'/></g>")
        .unwrap();
    assert_eq!(id(&edit, b"vexel-object-3"), inserted);
    assert!(contains(
        &bytes(&edit),
        b"<g id=\"vexel-object-3\"><path d='M0 0L1 1'/></g>"
    ));
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), before);
    assert!(!contains(&bytes(&edit), b"vexel-object-3"));
    edit.redo().unwrap();
    id(&edit, b"vexel-object-3");
    let layer = id(&edit, b"layer");
    edit.insert_xml_fragment(layer, b"<circle id='artist-id'/>")
        .unwrap();
    id(&edit, b"artist-id");
    assert!(contains(&bytes(&edit), b"<circle id='artist-id'/>"));
    let count = edit.history_count();
    let layer = id(&edit, b"layer");
    assert_eq!(
        edit.insert_xml_fragment(layer, b"<ellipse id='artist-id'/>"),
        Err(EditError::DuplicateId)
    );
    assert_eq!(edit.history_count(), count);
    let layer = id(&edit, b"layer");
    edit.insert_xml_fragment(
        layer,
        b"<defs><metadata><title>keep</title></metadata></defs>",
    )
    .unwrap();
    let current = bytes(&edit);
    assert!(contains(
        &current,
        b"<defs><metadata><title>keep</title></metadata></defs>"
    ));
    assert!(!contains(&current, b"defs id="));
    assert!(!contains(&current, b"metadata id="));
}
#[test]
fn test_queries_and_attributes() {
    let source=b"\xef\xbb\xbf<?xml version='1.0'?>\r\n<svg xmlns='urn:svg'\txmlns:x='urn:x'>\r\n\t<!--preserve--><g id='root'><text id='txt'>A&amp;B<![CDATA[<c>]]></text><rect id='one' x='1'/><circle id='two'/></g>\r\n</svg>\r\n";
    let mut edit = create(source);
    assert!(!edit.is_dirty());
    edit.root().unwrap();
    let rect = id(&edit, b"one");
    let text = id(&edit, b"txt");
    assert_eq!(edit.tag_name(rect), Some(&b"rect"[..]));
    assert_eq!(edit.attribute_count(rect), 2);
    let attribute = edit.attribute_at(rect, 1).unwrap();
    assert_eq!(attribute.quote, b'\'');
    let x = attribute.value;
    assert_eq!(edit.span_bytes(x), Some(&b"1"[..]));
    let mut decoded = [0; 64];
    let n = edit.text_content(text, &mut decoded).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&decoded[..n], b"A&B<c>");
    let mut path = [0; 128];
    let n = edit.element_path(rect, &mut path).unwrap();
    assert_eq!(&path[..n], b"/svg[1]/g[1]/rect[1]");
    let rect_span = edit.source_span(rect).unwrap();
    assert_eq!(edit.element_at_offset(rect_span.offset + 1), Some(rect));
    assert_eq!(
        edit.set_attribute(rect, b"x", &[1]),
        Err(EditError::MalformedXml)
    );
    assert_eq!(bytes(&edit), source);
    edit.set_attribute(rect, b"x", b"<&'\"").unwrap();
    assert!(edit.is_dirty());
    assert_eq!(
        edit.set_attribute(rect, b"x", b"<&'\""),
        Err(EditError::StaleHandle)
    );
    let current = bytes(&edit);
    assert!(contains(&current, b"x='&lt;&amp;&apos;\"'"));
    assert_eq!(&current[..x.offset], &source[..x.offset]);
    let suffix = x.offset + b"&lt;&amp;&apos;\"".len();
    assert_eq!(&current[suffix..], &source[x.offset + x.length..]);
    valid(&current);
    for needle in [
        b"\xef\xbb\xbf<?xml version='1.0'?>\r\n".as_slice(),
        b"\t<!--preserve-->",
        b"xmlns:x='urn:x'",
    ] {
        assert!(contains(&current, needle));
    }
    let rect = id(&edit, b"one");
    edit.add_attribute(rect, b"class", b"shape&mark").unwrap();
    let rect = id(&edit, b"one");
    assert_eq!(
        edit.add_attribute(rect, b"class", b"x"),
        Err(EditError::DuplicateAttribute)
    );
    assert_eq!(
        edit.add_attribute(rect, b"bad name", b"x"),
        Err(EditError::InvalidName)
    );
    edit.remove_attribute(rect, b"x").unwrap();
    let rect = id(&edit, b"one");
    assert_eq!(
        edit.remove_attribute(rect, b"missing"),
        Err(EditError::NotFound)
    );
    let text = id(&edit, b"txt");
    edit.set_text_content(text, b"hello <&>").unwrap();
    let text = id(&edit, b"txt");
    let n = edit.text_content(text, &mut decoded).unwrap();
    assert_eq!(&decoded[..n], b"hello <&>");
    let current = bytes(&edit);
    valid(&current);
    assert!(contains(&current, b"hello &lt;&amp;&gt;"));
    assert!(edit.history_count() >= 3);
}
#[test]
fn test_structural_operations() {
    let source =
        b"<svg><g id='layer'><rect id='a'/><!--gap--><circle id='b'/><ellipse id='c'/></g></svg>";
    let mut edit = create(source);
    let (a, b) = (id(&edit, b"a"), id(&edit, b"b"));
    edit.reorder_sibling(a, b, false).unwrap();
    valid(&bytes(&edit));
    assert!(contains(
        &bytes(&edit),
        b"<!--gap--><circle id='b'/><rect id='a'/><ellipse"
    ));
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
    edit.redo().unwrap();
    valid(&bytes(&edit));
    edit.undo().unwrap();
    let a = id(&edit, b"a");
    let duplicated = edit.duplicate_element(a).unwrap();
    assert!(edit.tag_name(duplicated).is_some());
    let a = id(&edit, b"a");
    let group = edit.wrap_in_group(a).unwrap();
    assert_eq!(edit.tag_name(group), Some(&b"g"[..]));
    edit.ungroup(group).unwrap();
    edit.undo().unwrap();
    valid(&bytes(&edit));
    edit.undo().unwrap();
    valid(&bytes(&edit));
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
    let b = id(&edit, b"b");
    edit.delete_element(b).unwrap();
    assert_eq!(edit.find_id(b"b"), None);
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
    let root = edit.root().unwrap();
    assert_eq!(edit.delete_element(root), Err(EditError::RootDelete));
    let layer = id(&edit, b"layer");
    let inserted = edit
        .insert_template(
            layer,
            TemplateKind::Path,
            &[TemplateAttribute {
                name: b"width",
                value: b"50",
            }],
        )
        .unwrap();
    assert_eq!(edit.tag_name(inserted), Some(&b"path"[..]));
    assert!(contains(
        &bytes(&edit),
        b"<path width=\"50\" id=\"vexel-object-1\"/>"
    ));
    edit.undo().unwrap();
    for (kind, name) in [
        (TemplateKind::Rect, b"rect".as_slice()),
        (TemplateKind::Circle, b"circle"),
        (TemplateKind::Ellipse, b"ellipse"),
        (TemplateKind::Line, b"line"),
        (TemplateKind::Path, b"path"),
        (TemplateKind::Group, b"g"),
        (TemplateKind::Text, b"text"),
    ] {
        let layer = id(&edit, b"layer");
        let h = edit.insert_template(layer, kind, &[]).unwrap();
        assert_eq!(edit.tag_name(h), Some(name));
        edit.undo().unwrap();
    }
    let layer = id(&edit, b"layer");
    let h = edit
        .insert_template(
            layer,
            TemplateKind::Text,
            &[TemplateAttribute {
                name: b"id",
                value: b"new-text",
            }],
        )
        .unwrap();
    edit.set_text_content(h, b"A<&>").unwrap();
    assert!(contains(
        &bytes(&edit),
        b"<text id=\"new-text\">A&lt;&amp;&gt;</text>"
    ));
    edit.undo().unwrap();
    edit.undo().unwrap();
    let mut edit = create(b"<svg/>");
    let root = edit.root().unwrap();
    edit.insert_template(root, TemplateKind::Circle, &[])
        .unwrap();
    assert_eq!(bytes(&edit), b"<svg><circle id=\"vexel-object-1\"/></svg>");
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), b"<svg/>");
}
#[test]
fn test_empty_text_and_xml_fragment() {
    let mut edit = create(b"<svg><text id='empty'/></svg>");
    let h = id(&edit, b"empty");
    edit.set_text_content(h, b"inserted").unwrap();
    let h = id(&edit, b"empty");
    edit.set_text_content(h, b"").unwrap();
    let h = id(&edit, b"empty");
    let mut out = [0; 16];
    assert_eq!(edit.text_content(h, &mut out), Ok(0));
    let mut edit = create(b"<svg/>");
    let root = edit.root().unwrap();
    let h = edit
        .insert_xml_fragment(
            root,
            b"<!--lead--><g id='fragment'><text id='nested'>old</text></g><!--tail-->",
        )
        .unwrap();
    assert_eq!(edit.tag_name(h), Some(&b"g"[..]));
    let nested = id(&edit, b"nested");
    edit.set_text_content(nested, b"changed & safe").unwrap();
    valid(&bytes(&edit));
    assert!(contains(
        &bytes(&edit),
        b"<text id='nested'>changed &amp; safe</text>"
    ));
    edit.undo().unwrap();
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), b"<svg/>");
    let root = edit.root().unwrap();
    assert_eq!(
        edit.insert_xml_fragment(root, b"<g><text></g>"),
        Err(EditError::MalformedXml)
    );
    assert_eq!(
        edit.insert_xml_fragment(root, b"<rect/><circle/>"),
        Err(EditError::InvalidOperation)
    );
    assert_eq!(
        edit.insert_xml_fragment(root, &vec![b'x'; XML_FRAGMENT_LIMIT + 1]),
        Err(EditError::Limit)
    );
    assert!(!edit.is_dirty());
    assert_eq!(edit.byte_length(), 6);
}
#[test]
fn test_replace_xml_region() {
    let source=b"<svg><!--keep--><g id='group'>  <rect id='a'/><!--middle--><ellipse id='b'/></g><path id='tail'/></svg>";
    let mut edit = create(source);
    let (group, a, b) = (id(&edit, b"group"), id(&edit, b"a"), id(&edit, b"b"));
    let (sa, sb) = (edit.source_span(a).unwrap(), edit.source_span(b).unwrap());
    let region = SourceSpan {
        offset: sa.offset,
        length: sb.offset + sb.length - sa.offset,
    };
    edit.replace_xml_region(group, region, b"<circle id='replacement'/>")
        .unwrap();
    assert_eq!(edit.history_count(), 1);
    let current = bytes(&edit);
    valid(&current);
    assert!(contains(
        &current,
        b"<g id='group'>  <circle id='replacement'/></g>"
    ));
    assert!(contains(&current, b"<!--keep-->"));
    assert!(contains(&current, b"<path id='tail'/>"));
    assert!(!contains(&current, b"id='a'"));
    assert!(!contains(&current, b"id='b'"));
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
    let group = id(&edit, b"group");
    assert_eq!(
        edit.replace_xml_region(group, region, b"<circle><rect></circle>"),
        Err(EditError::MalformedXml)
    );
    assert!(!edit.is_dirty());
    assert_eq!(edit.history_count(), 1);
}
#[test]
fn test_replace_source_span() {
    let source = b"<svg><path d=\"M 1 2 L 3 4\"/></svg>";
    let mut edit = create(source);
    let offset = b"<svg><path d=\"M ".len();
    edit.replace_source_span(SourceSpan { offset, length: 1 }, b"10")
        .unwrap();
    assert_eq!(edit.history_count(), 1);
    assert_eq!(bytes(&edit), b"<svg><path d=\"M 10 2 L 3 4\"/></svg>");
    assert_eq!(
        edit.replace_source_span(SourceSpan { offset, length: 2 }, b"<"),
        Err(EditError::MalformedXml)
    );
    assert_eq!(edit.history_count(), 1);
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), source);
}
#[test]
fn test_insert_text_element() {
    let mut edit = create(b"<svg/>");
    let root = edit.root().unwrap();
    let mut attrs = [
        TemplateAttribute {
            name: b"x",
            value: b"12",
        },
        TemplateAttribute {
            name: b"fill",
            value: b"#ffffff",
        },
    ];
    edit.insert_text_element(root, &attrs, b"A<&>").unwrap();
    assert_eq!(edit.history_count(), 1);
    assert_eq!(
        bytes(&edit),
        b"<svg><text x=\"12\" fill=\"#ffffff\" id=\"vexel-object-1\">A&lt;&amp;&gt;</text></svg>"
    );
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), b"<svg/>");
    let root = edit.root().unwrap();
    attrs[1].name = attrs[0].name;
    assert_eq!(
        edit.insert_text_element(root, &attrs, b"duplicate"),
        Err(EditError::DuplicateAttribute)
    );
    assert_eq!(edit.history_count(), 1);
}
#[test]
fn test_mark_saved_checkpoint() {
    let mut edit = create(b"<svg><rect id='r' x='0'/></svg>");
    assert!(!edit.is_dirty());
    set(&mut edit, b"r", b"x", b"1");
    assert!(edit.is_dirty());
    edit.mark_saved();
    assert!(!edit.is_dirty());
    set(&mut edit, b"r", b"x", b"2");
    assert!(edit.is_dirty());
    edit.undo().unwrap();
    assert!(!edit.is_dirty());
    edit.redo().unwrap();
    assert!(edit.is_dirty());
    edit.mark_saved();
    assert!(!edit.is_dirty());
}
#[test]
fn test_history_bounds() {
    let source = b"<svg><rect x='0'/></svg>";
    let mut edit = Editor::create(
        &std::sync::Arc::new(source.to_vec()),
        EditLimits {
            history_commands: 2,
            history_bytes: 64,
        },
    )
    .editor
    .unwrap();
    for value in b'1'..=b'3' {
        edit.validate_replace(
            SourceSpan {
                offset: 5,
                length: 13,
            },
            14,
            1,
            &[value],
            false,
            true,
        )
        .unwrap();
        edit.commit_replace(14, 1, &[value], 1, false, HistoryAction::EditAttribute)
            .unwrap();
    }
    assert_eq!((edit.history_count(), edit.history_cursor()), (2, 2));
    edit.undo().unwrap();
    edit.undo().unwrap();
    assert_eq!(edit.undo(), Err(EditError::InvalidOperation));
    assert!(contains(&bytes(&edit), b"x='1'"));
    edit.redo().unwrap();
    edit.redo().unwrap();
    assert!(contains(&bytes(&edit), b"x='3'"));
    edit.undo().unwrap();
    edit.validate_replace(
        SourceSpan {
            offset: 5,
            length: 13,
        },
        14,
        1,
        b"9",
        false,
        true,
    )
    .unwrap();
    edit.commit_replace(14, 1, b"9", 1, false, HistoryAction::EditAttribute)
        .unwrap();
    assert_eq!(edit.redo(), Err(EditError::InvalidOperation));
    let mut edit = Editor::create(
        &std::sync::Arc::new(source.to_vec()),
        EditLimits {
            history_commands: 10,
            history_bytes: 2,
        },
    )
    .editor
    .unwrap();
    assert_eq!(edit.find_id(b""), None);
    let rect = edit.model().first_child(edit.root().unwrap()).unwrap();
    edit.set_attribute(rect, b"x", b"1").unwrap();
    assert_eq!(edit.find_id(b"absent"), None);
    let rect = edit.model().first_child(edit.root().unwrap()).unwrap();
    assert_eq!(
        edit.set_attribute(rect, b"x", b"2"),
        Err(EditError::HistoryLimit)
    );
    assert!(contains(&bytes(&edit), b"x='1'"));
    assert_eq!(edit.history_count(), 1);
    assert_eq!(edit.history_bytes(), 2);
}
#[test]
fn test_invalid_source() {
    let result = Editor::create(
        &std::sync::Arc::new(b"<svg><g></svg>".to_vec()),
        EditLimits::default(),
    );
    assert!(result.editor.is_none());
    assert_eq!(result.error, EditError::MalformedXml);
    assert_eq!(
        result.diagnostic.category,
        vexel_xml::XmlError::MismatchedTag
    );
    assert_eq!(result.diagnostic.offset, 8);
}
