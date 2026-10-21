use super::*;

#[test]
fn element_queries_handle_nested_and_unidentified_nodes() {
    let doc = V1Document::from_source(br#"<svg width="20" height="20"><g id="group" fill="red"><g><rect id="box" fill="red" x="1" y="2" width="3" height="4"/></g><circle id="dot"/></g><rect id="other"/></svg>"#).unwrap();
    let info = doc.element_get("box").unwrap();
    assert_eq!(info.parent.as_deref(), Some("group"));
    assert_eq!(info.children, Vec::<String>::new());
    assert_eq!(info.bounds, Some((1.0, 2.0, 4.0, 6.0)));
    assert_eq!(
        doc.element_query(&Query {
            kind: Some(ElementKind::Rect),
            id_prefix: None,
            attribute: Some(("fill".into(), Some("red".into()))),
            limit: 20
        })
        .unwrap(),
        ["box"]
    );
    let (tree, truncated) = doc.tree_get(16, 10).unwrap();
    assert!(!truncated);
    assert_eq!(tree.tag, "svg");
}

#[test]
fn element_get_bounds_attributes_and_limits_are_bounded() {
    let long = "é".repeat(140);
    let source = format!("<svg width=\"20\" height=\"20\"><rect id=\"r\" title=\"{long}\" width=\"1\" height=\"2\"/></svg>");
    let doc = V1Document::from_source(source.as_bytes()).unwrap();
    let info = doc.element_get("r").unwrap();
    let title = &info
        .attributes
        .iter()
        .find(|(k, _)| k == "title")
        .unwrap()
        .1;
    assert!(title.len() <= 256);
    assert!(std::str::from_utf8(title.as_bytes()).is_ok());
    assert_eq!(
        doc.element_query(&Query {
            kind: None,
            id_prefix: None,
            attribute: None,
            limit: 0
        }),
        Err(V1DocumentError::InvalidGeometry)
    );
    assert_eq!(doc.tree_get(17, 1), Err(V1DocumentError::InvalidGeometry));
    let (_, truncated) = doc.tree_get(0, 1).unwrap();
    assert!(truncated);
}

#[test]
fn missing_ids_and_tree_node_limit_are_reported() {
    let doc =
        V1Document::from_source(br#"<svg width="20" height="20"><g><rect id="r"/></g></svg>"#)
            .unwrap();
    assert_eq!(
        doc.element_get("missing"),
        Err(V1DocumentError::MissingStableId)
    );
    let (_, truncated) = doc.tree_get(16, 1).unwrap();
    assert!(truncated);
}
