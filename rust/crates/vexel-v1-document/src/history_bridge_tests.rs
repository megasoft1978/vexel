use super::*;

#[test]
fn history_state_ids_follow_reachable_cursors() {
    let mut document = V1Document::new(100, 80).unwrap();
    let initial = document.history_state_id();
    assert_eq!(document.history_state_id_at(0), Some(initial));
    assert_eq!(document.history_state_id_at(1), None);

    document
        .add_rectangle(10.0, 10.0, 30.0, 30.0, "#abcdef")
        .unwrap();
    let edited = document.history_state_id();
    assert_ne!(edited, initial);
    assert_eq!(document.history_state_id_at(0), Some(initial));
    assert_eq!(document.history_state_id_at(1), Some(edited));
    assert_eq!(document.history_state_id_at(2), None);
}

#[test]
fn history_navigation_restores_reachable_source_states() {
    let mut document = V1Document::new(100, 80).unwrap();
    let initial = document.history_state_id();
    let initial_source = document.source().unwrap();
    document
        .add_rectangle(10.0, 10.0, 30.0, 30.0, "#abcdef")
        .unwrap();
    let edited = document.history_state_id();

    document.history_navigate(initial).unwrap();
    assert_eq!(document.history_state_id(), initial);
    assert_eq!(document.history_cursor(), 0);
    assert_eq!(document.source().unwrap(), initial_source);

    document.history_navigate(edited).unwrap();
    assert_eq!(document.history_state_id(), edited);
    assert_eq!(document.history_cursor(), 1);
}

#[test]
fn rejected_history_navigation_does_not_mutate_document() {
    let mut document = V1Document::new(100, 80).unwrap();
    document
        .add_rectangle(10.0, 10.0, 30.0, 30.0, "#abcdef")
        .unwrap();
    let state_id = document.history_state_id();
    let cursor = document.history_cursor();
    let source = document.source().unwrap();

    assert_eq!(
        document.history_navigate(u64::MAX),
        Err(V1DocumentError::Edit(EditError::InvalidArgument))
    );
    assert_eq!(document.history_state_id(), state_id);
    assert_eq!(document.history_cursor(), cursor);
    assert_eq!(document.source().unwrap(), source);
}

#[test]
fn history_boundary_preserves_the_current_history_state() {
    let mut document = V1Document::new(100, 80).unwrap();
    document
        .add_rectangle(10.0, 10.0, 30.0, 30.0, "#abcdef")
        .unwrap();
    let state_id = document.history_state_id();
    let cursor = document.history_cursor();

    document.history_boundary();

    assert_eq!(document.history_state_id(), state_id);
    assert_eq!(document.history_cursor(), cursor);
    assert_eq!(document.history_state_id_at(cursor), Some(state_id));
}
