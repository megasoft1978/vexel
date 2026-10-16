use crate::*;
use std::sync::Arc;
#[test]
fn piece_capacity_rejection_and_state_id_exhaustion_precede_publication() {
    let owner = Arc::new(vec![b'x']);
    let mut pieces = crate::alloc::vector(MAX_PIECES).unwrap();
    for _ in 0..MAX_PIECES {
        pieces.push(crate::storage::Piece {
            owner: crate::storage::Buffer::Original(Arc::clone(&owner)),
            start: 0,
            len: 1,
        });
    }
    assert!(matches!(
        crate::storage::replace_copy(&pieces, MAX_PIECES, 0, 1, None),
        Err(EditError::OutOfMemory)
    ));
    let mut edit = Editor::create(&Arc::new(b"<svg x='0'/>".to_vec()), EditLimits::default())
        .editor
        .unwrap();
    let root = edit.root().unwrap();
    edit.next_state = u64::MAX;
    assert_eq!(
        edit.commit_replace(8, 1, b"1", 0, true, HistoryAction::EditAttribute),
        Err(EditError::Limit)
    );
    assert_eq!(edit.root(), Some(root));
    assert!(!edit.is_dirty());
    assert_eq!(edit.history_count(), 0);
    assert_eq!(edit.history_bytes(), 0);
}
#[test]
fn metadata_actual_capacity_and_read_range_guards() {
    let mut edit = Editor::create(
        &Arc::new(b"<svg x='0'/>".to_vec()),
        EditLimits {
            history_commands: 2,
            history_bytes: 64,
        },
    )
    .editor
    .unwrap();
    let root = edit.root().unwrap();
    edit.set_attribute(root, b"x", b"1").unwrap();
    assert_eq!(edit.history.capacity(), 2);
    assert_eq!(
        edit.allocation_stats().command_bytes,
        2 * std::mem::size_of::<crate::piece_table::Command>()
    );
    assert_eq!(
        edit.allocation_stats().piece_bytes,
        edit.pieces.capacity() * std::mem::size_of::<crate::storage::Piece>()
    );
    let mut output = [0xa5; 4];
    assert_eq!(
        edit.read_range(usize::MAX, &mut output),
        Err(EditError::InvalidArgument)
    );
    assert_eq!(output, [0xa5; 4]);
}
