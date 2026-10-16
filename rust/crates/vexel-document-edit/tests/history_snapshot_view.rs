//! Direct history, snapshot worker, and document_view pagination assertions.
mod common;
use common::*;
use std::sync::{Arc, Barrier};
use vexel_document_edit::*;
const HISTORY_SOURCE:&[u8]=b"<svg xmlns='http://www.w3.org/2000/svg' data-a='00' data-b='00'><rect id='shape' width='4' height='4'/></svg>";
fn root_attribute(edit: &mut Editor, name: &[u8], value: &[u8]) {
    let root = edit.root().unwrap();
    edit.set_attribute(root, name, value).unwrap();
}
#[test]
fn check_recent_entries_and_jump() {
    let mut edit = create(HISTORY_SOURCE);
    let mut states = vec![bytes(&edit)];
    for index in 0..25 {
        root_attribute(
            &mut edit,
            if index % 2 == 0 { b"data-a" } else { b"data-b" },
            &[b'0', b'1' + (index % 8) as u8],
        );
        states.push(bytes(&edit));
    }
    assert_eq!((edit.history_count(), edit.history_cursor()), (25, 25));
    assert_eq!(edit.history_saved_cursor(), Some(0));
    let first = edit.history_entry(5).unwrap();
    let newest = edit.history_entry(24).unwrap();
    assert_eq!(first.label, "Edit attribute");
    assert_eq!(newest.label, "Edit attribute");
    assert!(newest.is_current);
    assert!(!newest.is_saved);
    edit.history_jump(8).unwrap();
    assert_eq!(edit.history_cursor(), 8);
    assert_eq!(bytes(&edit), states[8]);
    root_attribute(&mut edit, b"data-a", b"09");
    assert_eq!((edit.history_count(), edit.history_cursor()), (9, 9));
    assert_eq!(edit.redo(), Err(EditError::InvalidOperation));
    assert_eq!(edit.history_jump(25), Err(EditError::InvalidArgument));
}
#[test]
fn check_coalescing_and_eviction() {
    let mut edit = create(HISTORY_SOURCE);
    for v in b'1'..=b'3' {
        root_attribute(&mut edit, b"data-a", &[b'0', v]);
    }
    assert_eq!(edit.history_count(), 1);
    let entry = edit.history_entry(0).unwrap();
    assert_eq!(entry.label, "Edit attribute");
    assert!(entry.is_current);
    edit.undo().unwrap();
    assert!(!edit.is_dirty());
    let mut edit = Editor::create(
        &Arc::new(HISTORY_SOURCE.to_vec()),
        EditLimits {
            history_commands: 3,
            history_bytes: 4096,
        },
    )
    .editor
    .unwrap();
    for index in 0..5 {
        root_attribute(
            &mut edit,
            if index % 2 == 0 { b"data-a" } else { b"data-b" },
            &[b'0', b'1' + index],
        );
    }
    assert_eq!((edit.history_count(), edit.history_cursor()), (3, 3));
    assert_eq!(edit.history_saved_cursor(), None);
    let entry = edit.history_entry(0).unwrap();
    assert_eq!(entry.label, "Edit attribute");
    assert!(!entry.is_current);
}
#[test]
fn test_snapshot_stability_and_worker_read() {
    let source=Arc::new(b"<svg xmlns='http://www.w3.org/2000/svg' data-a='old' data-b='zero'><rect id='shape' width='4' height='4'/></svg>".to_vec());
    let mut edit = Editor::create(&source, EditLimits::default())
        .editor
        .unwrap();
    root_attribute(&mut edit, b"data-a", b"captured");
    let snapshot = Arc::new(edit.snapshot().unwrap());
    let expected = bytes(&edit);
    let mut too_small = vec![0xa5; expected.len() - 1];
    assert_eq!(
        snapshot.materialize(&mut too_small),
        Err(EditError::InvalidArgument)
    );
    assert!(too_small.iter().all(|b| *b == 0xa5));
    let barrier = Arc::new(Barrier::new(2));
    let worker_snapshot = Arc::clone(&snapshot);
    let worker_barrier = Arc::clone(&barrier);
    let worker_expected = expected.clone();
    let worker = std::thread::spawn(move || {
        let mut out = vec![0; worker_expected.len()];
        for iteration in 0..1000 {
            if iteration < 60 {
                worker_barrier.wait();
            }
            assert_eq!(worker_snapshot.materialize(&mut out), Ok(out.len()));
            assert_eq!(out, worker_expected);
            if iteration < 60 {
                worker_barrier.wait();
            }
        }
    });
    for iteration in 0..60 {
        barrier.wait();
        root_attribute(&mut edit, b"data-b", format!("edit-{iteration}").as_bytes());
        barrier.wait();
    }
    worker.join().unwrap();
    edit.undo().unwrap();
    edit.redo().unwrap();
    let mut output = vec![0; expected.len()];
    snapshot.materialize(&mut output).unwrap();
    assert_eq!(output, expected);
    drop(edit);
    drop(source);
    snapshot.materialize(&mut output).unwrap();
    assert_eq!(
        output, expected,
        "stronger Rust snapshot ownership after editor destruction"
    );
}
#[test]
fn test_unedited_snapshot_and_output_capacity() {
    let source=b"<svg xmlns='http://www.w3.org/2000/svg' data-a='old' data-b='zero'><rect id='shape' width='4' height='4'/></svg>";
    let edit = create(source);
    let snapshot = edit.snapshot().unwrap();
    let mut output = vec![0; source.len()];
    assert_eq!(snapshot.materialize(&mut output), Ok(source.len()));
    assert_eq!(output, source);
}
#[test]
fn document_view_range_original_pagination() {
    let source=b"<?xml version='1.0'?>\r\n<svg xmlns='urn:svg' xmlns:i='urn:inkscape'><g i:label='keep'><rect id='one'/><i:unknown><path d='M0 0'/></i:unknown></g><circle/></svg>\n";
    let edit = create(source);
    let view = edit.model();
    let root = view.root().unwrap();
    let group = view.first_child(root).unwrap();
    assert_eq!(document_view_attribute_count(view, group), 1);
    assert_eq!(
        document_view_attribute_at(view, group, 0),
        Some((&b"i:label"[..], &b"keep"[..]))
    );
    let mut rows = [DocumentViewRow {
        handle: root,
        depth: 0,
    }; 2];
    let (count, next) = document_view_range(view, 0, view.node_count(), &mut rows).unwrap();
    assert_eq!((count, next), (2, 2));
    assert_eq!(rows[0].handle.slot, 0);
    assert_eq!(rows[1].depth, 1);
    assert_eq!(document_view_node_count(view), 6);
    assert_eq!(document_view_root(view), Some(root));
    assert_eq!(document_view_first_child(view, root), Some(group));
    assert_eq!(document_view_tag(view, group), Some(&b"g"[..]));
    assert!(document_view_next_sibling(view, group).is_some());
    assert_eq!(
        document_view_range(view, next, view.node_count(), &mut rows),
        Ok((2, 4))
    );
    assert_eq!(document_view_range(view, 4, 6, &mut []), Ok((0, 4)));
    assert_eq!(
        document_view_range(view, 6, 5, &mut rows),
        Err(EditError::InvalidArgument)
    );
}
#[test]
fn view_pagination_skips_dead_slots_without_extra_rows() {
    let source = Arc::new(b"<svg><g><rect/><path/></g><circle/></svg>".to_vec());
    let mut model = vexel_document::parse_owned_document(&source).unwrap();
    let root = model.root().unwrap();
    let group = model.first_child(root).unwrap();
    assert!(model.remove_subtree(group));
    let mut row = [DocumentViewRow {
        handle: root,
        depth: 99,
    }; 1];
    assert_eq!(
        document_view_range(&model, 0, model.node_count(), &mut row),
        Ok((1, 1))
    );
    assert_eq!(row[0].handle, root);
    assert_eq!(row[0].depth, 0);
    assert_eq!(
        document_view_range(&model, 1, model.node_count(), &mut row),
        Ok((1, 5))
    );
    assert_eq!(
        document_view_tag(&model, row[0].handle),
        Some(&b"circle"[..])
    );
    assert_eq!(row[0].depth, 1);
}
