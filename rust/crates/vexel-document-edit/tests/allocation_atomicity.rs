//! Deterministic significant-allocation failures and actual shared-owner charges.
mod common;
use common::*;
use std::sync::Arc;
use vexel_document_edit::*;
fn at_least_one_failure(mut operation: impl FnMut(&mut Editor) -> Result<(), EditError>) {
    let mut failures = 0;
    for gate in 0..150 {
        let mut edit = create(b"<svg><g id='layer'><rect id='a' x='0'/><circle id='b'/></g></svg>");
        set(&mut edit, b"a", b"x", b"1");
        edit.undo().unwrap();
        let handle = id(&edit, b"a");
        let original = bytes(&edit);
        let before = (
            edit.history_count(),
            edit.history_cursor(),
            edit.history_saved_cursor(),
            edit.is_dirty(),
            edit.history_bytes(),
        );
        allocation_gate(Some(gate));
        let result = operation(&mut edit);
        allocation_gate(None);
        match result {
            Err(error) => {
                assert_eq!(error, EditError::OutOfMemory, "gate {gate}");
                failures += 1;
                assert_eq!(bytes(&edit), original);
                assert_eq!(
                    (
                        edit.history_count(),
                        edit.history_cursor(),
                        edit.history_saved_cursor(),
                        edit.is_dirty(),
                        edit.history_bytes()
                    ),
                    before
                );
                assert_eq!(edit.tag_name(handle), Some(&b"rect"[..]));
            }
            Ok(()) => {
                assert!(failures > 0);
                return;
            }
        }
        let _ = handle;
    }
    panic!("operation never passed all significant allocations");
}
#[test]
fn failures_before_commit_preserve_source_history_saveflags_and_handles() {
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        e.set_attribute(h, b"x", b"123")
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        e.add_attribute(h, b"class", b"<&")
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        e.remove_attribute(h, b"x")
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        e.set_text_content(h, b"<&text")
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        e.delete_element(h)
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        e.duplicate_element(h).map(|_| ())
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        e.wrap_in_group(h).map(|_| ())
    });
    at_least_one_failure(|e| {
        let h = id(e, b"layer");
        e.insert_template(h, TemplateKind::Circle, &[]).map(|_| ())
    });
    at_least_one_failure(|e| {
        let h = id(e, b"layer");
        e.insert_xml_fragment(h, b"<g><path/></g>").map(|_| ())
    });
    at_least_one_failure(|e| {
        let h = id(e, b"layer");
        e.insert_text_element(h, &[], b"abc").map(|_| ())
    });
    at_least_one_failure(|e| {
        let (a, b) = (id(e, b"a"), id(e, b"b"));
        e.reorder_sibling(a, b, false)
    });
    at_least_one_failure(|e| {
        let (a, b) = (id(e, b"a"), id(e, b"b"));
        e.group_siblings(&[b, a]).map(|_| ())
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        let span = e.source_span(h).unwrap();
        let parent = id(e, b"layer");
        e.replace_xml_region(parent, span, b"<ellipse/>")
    });
    at_least_one_failure(|e| {
        let h = id(e, b"a");
        let a = e.attribute_at(h, 1).unwrap();
        e.replace_source_span(a.value, b"2")
    });
    at_least_one_failure(|e| e.redo());
    at_least_one_failure(|e| e.history_jump(1));
}
#[test]
fn blob_accounting_tracks_snapshots_and_live_pieces_after_eviction() {
    let mut edit = Editor::create(
        &Arc::new(b"<svg x='0'/>".to_vec()),
        EditLimits {
            history_commands: 2,
            history_bytes: 6,
        },
    )
    .editor
    .unwrap();
    let root = edit.root().unwrap();
    edit.set_attribute(root, b"x", b"long").unwrap();
    assert_eq!(edit.history_bytes(), 5);
    let snapshot = edit.snapshot().unwrap();
    let root = edit.root().unwrap();
    edit.set_attribute(root, b"x", b"b").unwrap();
    assert_eq!(edit.history_bytes(), 6, "snapshot retains replaced blob");
    let root = edit.root().unwrap();
    assert_eq!(
        edit.set_attribute(root, b"x", b"cc"),
        Err(EditError::HistoryLimit)
    );
    assert_eq!(edit.history_bytes(), 6);
    drop(snapshot);
    assert_eq!(edit.history_bytes(), 2);
    edit.set_attribute(root, b"x", b"cc").unwrap();
    assert_eq!(edit.history_bytes(), 3);
    let mut edit = Editor::create(
        &Arc::new(b"<svg/>".to_vec()),
        EditLimits {
            history_commands: 1,
            history_bytes: 64,
        },
    )
    .editor
    .unwrap();
    let root = edit.root().unwrap();
    edit.add_attribute(root, b"a", b"1").unwrap();
    let root = edit.root().unwrap();
    edit.add_attribute(root, b"b", b"2").unwrap();
    assert_eq!(edit.history_count(), 1);
    assert_eq!(
        edit.history_bytes(),
        12,
        "evicted command blob is still a live piece"
    );
    assert!(edit.allocation_stats().piece_bytes > 0);
    assert!(edit.allocation_stats().command_bytes > 0);
}
#[test]
fn create_cache_and_snapshot_gates_are_fallible_and_retryable() {
    allocation_gate(Some(0));
    let r = Editor::create(&Arc::new(b"<svg/>".to_vec()), EditLimits::default());
    allocation_gate(None);
    assert_eq!(r.error, EditError::OutOfMemory);
    assert!(r.editor.is_none());
    let mut edit = create(b"<svg x='0'/>");
    let root = edit.root().unwrap();
    edit.set_attribute(root, b"x", b"1").unwrap();
    assert_eq!(edit.allocation_stats().cache_bytes, 0);
    allocation_gate(Some(0));
    assert_eq!(edit.try_model().unwrap_err(), EditError::OutOfMemory);
    allocation_gate(None);
    assert!(edit.try_model().is_ok());
    allocation_gate(Some(0));
    assert_eq!(edit.snapshot().unwrap_err(), EditError::OutOfMemory);
    allocation_gate(None);
    assert!(edit.snapshot().is_ok());
}
#[test]
fn undo_and_multi_step_jump_reject_atomically() {
    for jump in [false, true] {
        let mut failures = 0;
        for gate in 0..150 {
            let mut edit = create(b"<svg a='0' b='0'/>");
            for i in 1..=5 {
                let root = edit.root().unwrap();
                edit.set_attribute(root, if i % 2 == 0 { b"a" } else { b"b" }, &[b'0' + i])
                    .unwrap();
            }
            edit.mark_saved();
            let root = edit.root().unwrap();
            let original = bytes(&edit);
            let cursor = edit.history_cursor();
            let saved = edit.history_saved_cursor();
            let charge = edit.history_bytes();
            allocation_gate(Some(gate));
            let result = if jump {
                edit.history_jump(0)
            } else {
                edit.undo()
            };
            allocation_gate(None);
            match result {
                Err(error) => {
                    assert_eq!(error, EditError::OutOfMemory);
                    failures += 1;
                    assert_eq!(bytes(&edit), original);
                    assert_eq!(edit.history_cursor(), cursor);
                    assert_eq!(edit.history_saved_cursor(), saved);
                    assert!(!edit.is_dirty());
                    assert_eq!(edit.history_bytes(), charge);
                    assert_eq!(edit.root(), Some(root));
                }
                Ok(()) => {
                    assert!(failures > 0);
                    break;
                }
            }
            assert!(gate < 149);
        }
    }
    at_least_one_failure(|e| {
        let layer = id(e, b"layer");
        e.ungroup(layer)
    });
    at_least_one_failure(|e| {
        let layer = id(e, b"layer");
        e.ungroup_siblings(&[layer])
    });
}
#[test]
fn command_table_growth_is_preflighted_without_mutating_history() {
    let mut failed = 0;
    for gate in 0..100 {
        let mut edit = create(b"<svg a='0' b='0'/>");
        for i in 1..=8 {
            let root = edit.root().unwrap();
            edit.set_attribute(root, if i % 2 == 0 { b"a" } else { b"b" }, &[b'0' + i])
                .unwrap();
        }
        let root = edit.root().unwrap();
        let before = bytes(&edit);
        let stats = edit.allocation_stats();
        allocation_gate(Some(gate));
        let result = edit.set_attribute(root, b"b", b"9");
        allocation_gate(None);
        match result {
            Err(e) => {
                assert_eq!(e, EditError::OutOfMemory);
                failed += 1;
                assert_eq!(bytes(&edit), before);
                assert_eq!((edit.history_count(), edit.history_cursor()), (8, 8));
                assert_eq!(edit.history_bytes(), stats.blob_bytes);
                assert_eq!(edit.allocation_stats().command_bytes, stats.command_bytes);
                assert_eq!(edit.root(), Some(root));
            }
            Ok(()) => {
                assert!(failed > 0);
                assert_eq!(edit.history_count(), 9);
                assert!(edit.allocation_stats().command_bytes > stats.command_bytes);
                return;
            }
        }
    }
    panic!("command growth never succeeded");
}
#[test]
fn redo_blobs_are_charged_before_branch_discard() {
    let mut edit = Editor::create(
        &Arc::new(b"<svg x='0'/>".to_vec()),
        EditLimits {
            history_commands: 10,
            history_bytes: 2,
        },
    )
    .editor
    .unwrap();
    let root = edit.root().unwrap();
    edit.set_attribute(root, b"x", b"1").unwrap();
    edit.undo().unwrap();
    let root = edit.root().unwrap();
    let before = bytes(&edit);
    assert_eq!(edit.history_bytes(), 2);
    assert_eq!(
        edit.set_attribute(root, b"x", b"2"),
        Err(EditError::HistoryLimit)
    );
    assert_eq!(bytes(&edit), before);
    assert_eq!((edit.history_count(), edit.history_cursor()), (1, 0));
    assert_eq!(edit.root(), Some(root));
    assert!(!edit.is_dirty());
    edit.redo().unwrap();
    assert_eq!(bytes(&edit), b"<svg x='1'/>");
}
