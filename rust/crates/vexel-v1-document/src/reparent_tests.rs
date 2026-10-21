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

#[test]
fn moves_into_a_group_at_an_index_in_one_undoable_edit() {
    let mut d = doc("<rect id='a'/><g id='g'><rect id='x'/><rect id='y'/></g><rect id='c'/>");
    let before = d.source().unwrap();
    d.reparent("a", "g", Some(1)).unwrap();
    assert_eq!(d.history_count(), 1);
    assert!(source(&d).contains("<g id='g'><rect id='x'/><rect id='a'/><rect id='y'/></g>"));
    d.undo().unwrap();
    assert_eq!(d.source().unwrap(), before);
}

#[test]
fn appends_by_default_and_moves_back_to_root() {
    let mut d = doc("<rect id='a'/><g id='g'><rect id='x'/></g>");
    d.reparent("a", "g", None).unwrap();
    assert!(source(&d).contains("<g id='g'><rect id='x'/><rect id='a'/></g>"));
    d.reparent("x", "root", Some(0)).unwrap();
    assert!(source(&d).contains("><rect id='x'/><g id='g'><rect id='a'/></g></svg>"));
}

#[test]
fn fills_empty_self_closing_and_open_groups() {
    let mut d = doc("<rect id='a'/><rect id='b'/><g id='e'/><g id='f' ></g>");
    d.reparent("a", "e", None).unwrap();
    d.reparent("b", "f", None).unwrap();
    let s = source(&d);
    assert!(s.contains("<g id='e'><rect id='a'/></g>"), "{s}");
    assert!(s.contains("<g id='f' ><rect id='b'/></g>"), "{s}");
}

#[test]
fn rejects_cycles_non_groups_and_missing_ids_without_changes() {
    let mut d = doc("<g id='g'><g id='h'/></g><rect id='r'/>");
    let before = d.source().unwrap();
    assert_eq!(
        d.reparent("g", "h", None),
        Err(V1DocumentError::Unsupported)
    );
    assert_eq!(
        d.reparent("g", "g", None),
        Err(V1DocumentError::Unsupported)
    );
    assert_eq!(
        d.reparent("h", "r", None),
        Err(V1DocumentError::Unsupported)
    );
    assert_eq!(
        d.reparent("nope", "g", None),
        Err(V1DocumentError::MissingStableId)
    );
    assert_eq!(d.source().unwrap(), before);
    assert_eq!(d.history_count(), 0);
}

#[test]
fn same_position_is_a_no_op() {
    let mut d = doc("<rect id='a'/><rect id='b'/>");
    d.reparent("b", "root", None).unwrap();
    d.reparent("a", "root", Some(0)).unwrap();
    assert_eq!(d.history_count(), 0);
}
