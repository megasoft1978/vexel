use super::*;

fn doc(body: &str) -> V1Document {
    V1Document::from_source(
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\">{body}</svg>"
        )
        .as_bytes(),
    )
    .unwrap()
}
fn source(d: &V1Document) -> String {
    String::from_utf8(d.source().unwrap()).unwrap()
}
fn roundtrip(d: &mut V1Document, operation: impl FnOnce(&mut V1Document)) {
    let before = d.source().unwrap();
    let history = d.history_count();
    operation(d);
    assert_eq!(d.history_count(), history + 1);
    let after = d.source().unwrap();
    d.undo().unwrap();
    assert_eq!(d.source().unwrap(), before);
    d.redo().unwrap();
    assert_eq!(d.source().unwrap(), after);
}

#[test]
fn group_handles_any_selection_order_and_undo_redo() {
    let mut d = doc("<rect id='a'/><circle id='b'/><rect id='c'/>");
    roundtrip(&mut d, |d| {
        let id = d.group(&["c".into(), "a".into()]).unwrap();
        assert_eq!(id, "vexel-object-1");
    });
    let s = source(&d);
    assert!(
        s.contains("<g id=\"vexel-object-1\"><rect id='a'/><rect id='c'/></g><circle id='b'/>")
            || s.contains("<g id=\"vexel-object-1\"><rect id='a'/><rect id='c'/></g>")
    );
}

#[test]
fn group_rejects_invalid_selections_atomically() {
    let mut d = doc("<rect id='a'/><g id='g'><rect id='b'/></g>");
    let before = d.source().unwrap();
    assert_eq!(d.group(&[]), Err(V1DocumentError::Unsupported));
    assert_eq!(
        d.group(&["a".into(), "b".into()]),
        Err(V1DocumentError::Unsupported)
    );
    assert_eq!(
        d.group(&["missing".into()]),
        Err(V1DocumentError::MissingStableId)
    );
    assert_eq!(d.source().unwrap(), before);
    assert_eq!(d.history_count(), 0);
}

#[test]
fn ungroup_rejects_transforms_and_roundtrips_plain_group() {
    let mut d = doc("<g id='g'><rect id='a'/><ellipse id='b'/></g>");
    let mut transformed = doc("<g id='g' transform='scale(2)'><rect id='a'/></g>");
    assert_eq!(
        transformed.ungroup("g"),
        Err(V1DocumentError::UnsupportedTransform)
    );
    roundtrip(&mut d, |d| {
        assert_eq!(d.ungroup("g").unwrap(), vec!["a", "b"])
    });
    assert!(!source(&d).contains("<g id='g'"));
    assert_eq!(d.ungroup("absent"), Err(V1DocumentError::MissingStableId));
}

#[test]
fn duplicate_generates_unique_ids_for_root_and_descendants() {
    let mut d = doc("<g id='g'><rect id='child'/><circle id='other'/></g>");
    roundtrip(&mut d, |d| {
        assert_eq!(d.duplicate("g").unwrap(), "vexel-object-1")
    });
    let s = source(&d);
    assert!(
        s.contains("id='vexel-object-1'")
            && s.contains("id='vexel-object-2'")
            && s.contains("id='vexel-object-3'")
    );
    assert_eq!(
        d.duplicate("missing"),
        Err(V1DocumentError::MissingStableId)
    );
}

#[test]
fn reorder_is_one_entry_and_edge_moves_are_noops() {
    let mut d = doc("<rect id='a'/><rect id='b'/><rect id='c'/>");
    roundtrip(&mut d, |d| d.reorder("a", Reorder::Front).unwrap());
    assert!(source(&d).find("id='a'").unwrap() > source(&d).find("id='c'").unwrap());
    let count = d.history_count();
    d.reorder("a", Reorder::Front).unwrap();
    assert_eq!(d.history_count(), count);
    assert_eq!(
        d.reorder("absent", Reorder::Back),
        Err(V1DocumentError::MissingStableId)
    );
}

#[test]
fn group_bounds_and_translate_cover_children_and_hit_returns_child() {
    let mut d=doc("<g id='g'><rect id='a' x='10' y='20' width='5' height='6'/><rect id='b' x='30' y='40' width='2' height='3'/></g>");
    assert_eq!(d.geometry_bounds("g").unwrap(), (10.0, 20.0, 32.0, 43.0));
    assert_eq!(d.hit_test(11.0, 21.0).as_deref(), Some("a"));
    roundtrip(&mut d, |d| d.translate("g", 5.0, 7.0).unwrap());
    assert_eq!(d.geometry_bounds("g").unwrap(), (15.0, 27.0, 37.0, 50.0));
}
