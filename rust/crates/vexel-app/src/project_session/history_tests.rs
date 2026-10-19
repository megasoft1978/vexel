use super::{FAIL_NEXT_RESERVATION, Journal, JournalError, MAX_STATES, PayloadSnapshot};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot {
    identity: usize,
    bytes: usize,
}

impl Snapshot {
    fn new(identity: usize, bytes: usize) -> Self {
        Self { identity, bytes }
    }
}

impl PayloadSnapshot for Snapshot {
    fn identity(&self) -> usize {
        self.identity
    }

    fn retained_bytes(&self) -> usize {
        self.bytes
    }
}

fn journal(scene: u64, payload: Snapshot) -> Journal<Snapshot> {
    Journal::try_new(scene, payload).unwrap()
}

#[test]
fn scene_payload_scene_undo_redo_preserves_global_chronology() {
    let original = Snapshot::new(1, 32);
    let changed = Snapshot::new(2, 48);
    let mut journal = journal(10, original.clone());
    let scene_b = journal
        .prepare_scene()
        .unwrap()
        .finish(11, |scene| [10, 11, 12].contains(&scene))
        .unwrap();
    let payload_b = journal.prepare_payload(changed.clone()).unwrap().commit();
    let scene_c = journal
        .prepare_scene()
        .unwrap()
        .finish(12, |scene| [10, 11, 12].contains(&scene))
        .unwrap();

    let target = journal.undo_target(&[10, 11, 12]).unwrap();
    assert_eq!(target.scene_state_id(), 11);
    assert_eq!(target.payload(), &changed);
    journal.commit_undo(target.app_state_id()).unwrap();
    let target = journal.undo_target(&[10, 11, 12]).unwrap();
    assert_eq!(target.scene_state_id(), 11);
    assert_eq!(target.payload(), &original);
    journal.commit_undo(target.app_state_id()).unwrap();
    let target = journal.undo_target(&[10, 11, 12]).unwrap();
    assert_eq!(target.scene_state_id(), 10);
    journal.commit_undo(target.app_state_id()).unwrap();

    for expected in [scene_b, payload_b, scene_c] {
        let target = journal.redo_target(&[10, 11, 12]).unwrap();
        assert_eq!(target.app_state_id(), expected);
        journal.commit_redo(expected).unwrap();
    }
    assert!(journal.redo_target(&[10, 11, 12]).is_none());
}

#[test]
fn payload_branch_discards_redo_and_never_reuses_identity() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    let abandoned = journal
        .prepare_payload(Snapshot::new(2, 8))
        .unwrap()
        .commit();
    let target = journal.undo_target(&[1]).unwrap();
    journal.commit_undo(target.app_state_id()).unwrap();
    let branch = journal
        .prepare_payload(Snapshot::new(3, 8))
        .unwrap()
        .commit();
    assert!(branch > abandoned);
    assert!(journal.redo_target(&[1]).is_none());
}

#[test]
fn scene_coalescing_replaces_only_scene_tail_and_payload_is_hard_boundary() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    assert_eq!(
        journal
            .prepare_scene()
            .unwrap()
            .finish(2, |scene| [1, 2].contains(&scene)),
        Some(2)
    );
    let coalesced = journal
        .prepare_scene()
        .unwrap()
        .finish(3, |scene| [1, 3].contains(&scene))
        .unwrap();
    assert_eq!(journal.state_count(), 2);
    assert_eq!(journal.current().scene_state_id(), 3);
    assert_eq!(journal.current().app_state_id(), coalesced);
    journal
        .prepare_payload(Snapshot::new(2, 8))
        .unwrap()
        .commit();
    journal
        .prepare_scene()
        .unwrap()
        .finish(4, |scene| [1, 3, 4].contains(&scene))
        .unwrap();
    assert_eq!(journal.state_count(), 4);
}

#[test]
fn checkpoint_and_dirty_state_follow_app_identity_through_navigation() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    let saved = journal
        .prepare_payload(Snapshot::new(2, 8))
        .unwrap()
        .commit();
    journal.checkpoint();
    assert_eq!(journal.checkpoint_id(), saved);
    assert!(!journal.is_dirty());

    let later = journal
        .prepare_scene()
        .unwrap()
        .finish(2, |scene| [1, 2].contains(&scene))
        .unwrap();
    assert!(journal.is_dirty());
    journal.commit_undo(saved).unwrap();
    assert!(!journal.is_dirty());
    let branch = journal
        .prepare_payload(Snapshot::new(3, 8))
        .unwrap()
        .commit();
    assert!(branch > later);
    assert!(journal.is_dirty());
}

#[test]
fn dropped_and_noop_scene_admissions_preserve_redo_and_identity() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    let forward = journal
        .prepare_payload(Snapshot::new(2, 8))
        .unwrap()
        .commit();
    journal.commit_undo(1).unwrap();
    let before = journal.current().app_state_id();
    drop(journal.prepare_scene().unwrap());
    assert_eq!(journal.current().app_state_id(), before);
    assert_eq!(journal.prepare_scene().unwrap().finish(1, |_| true), None);
    assert_eq!(journal.current().app_state_id(), before);
    assert_eq!(journal.redo_target(&[1]).unwrap().app_state_id(), forward);
}

#[test]
fn partial_error_scene_observation_is_committed_and_unreachable_is_rejected() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    let edit_result: Result<(), &'static str> = Err("partial edit");
    let final_id = 2; // The caller reads this after the failed editor closure.
    let committed = journal
        .prepare_scene()
        .unwrap()
        .finish(final_id, |scene| scene == 2);
    assert_eq!(edit_result, Err("partial edit"));
    assert_eq!(committed, Some(2));
    assert_eq!(journal.current().scene_state_id(), 2);

    assert_eq!(
        journal
            .prepare_scene()
            .unwrap()
            .finish(3, |scene| scene != 3),
        None
    );
    assert_eq!(journal.current().scene_state_id(), 2);
}

#[test]
fn payload_admission_rejection_preserves_journal_and_candidate() {
    let candidate = Snapshot::new(2, vexel_project::MAX_BYTES);
    let saved_candidate = candidate.clone();
    let mut journal = journal(1, Snapshot::new(1, 1));
    let initial_id = journal.current().app_state_id();
    assert_eq!(
        journal.prepare_payload(candidate.clone()).err().unwrap(),
        JournalError::Capacity
    );
    assert_eq!(candidate, saved_candidate);
    assert_eq!(journal.current().app_state_id(), initial_id);
    assert_eq!(
        journal
            .prepare_payload(Snapshot::new(2, 1))
            .unwrap()
            .commit(),
        2
    );
}

#[test]
fn shared_snapshot_bytes_count_once_and_identity_size_conflicts_reject() {
    let shared = Snapshot::new(7, vexel_project::MAX_BYTES / 2);
    let mut journal = journal(1, shared.clone());
    journal.prepare_payload(shared.clone()).unwrap().commit();
    assert_eq!(journal.current().payload().identity(), 7);
    assert_eq!(
        journal
            .prepare_payload(Snapshot::new(7, shared.bytes + 1))
            .err()
            .unwrap(),
        JournalError::SnapshotIdentityConflict
    );
}

#[test]
fn exact_payload_memory_bound_is_admitted_and_one_byte_over_is_rejected() {
    let mut exact = journal(1, Snapshot::new(1, vexel_project::MAX_BYTES));
    exact.prepare_payload(Snapshot::new(2, 0)).unwrap().commit();
    let mut over = journal(1, Snapshot::new(1, 1));
    assert_eq!(
        over.prepare_payload(Snapshot::new(2, vexel_project::MAX_BYTES))
            .err()
            .unwrap(),
        JournalError::Capacity
    );
}

#[test]
fn scene_admission_fails_before_edit_when_id_cannot_advance() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    journal.next_app_state_id = u64::MAX;
    let mut edit_ran = false;
    let admission = journal.prepare_scene().map(|guard| {
        edit_ran = true;
        guard.finish(2, |_| true)
    });
    assert_eq!(admission, Err(JournalError::IdentityExhausted));
    assert!(!edit_ran);
}

#[test]
fn bounded_eviction_and_reachability_filter_preserve_current_state() {
    let mut journal = journal(0, Snapshot::new(1, 1));
    let mut scene = 0;
    while journal.state_count() < MAX_STATES {
        journal
            .prepare_payload(Snapshot::new(journal.next_app_state_id as usize + 1, 1))
            .unwrap()
            .commit();
        scene += 1;
        journal
            .prepare_scene()
            .unwrap()
            .finish(scene, |_| true)
            .unwrap();
    }
    journal
        .prepare_payload(Snapshot::new(usize::MAX, 1))
        .unwrap()
        .commit();
    assert_eq!(journal.state_count(), MAX_STATES);
    let current_scene = journal.current().scene_state_id();
    assert_eq!(
        journal.reconcile_reachable(&[current_scene]).unwrap(),
        MAX_STATES - 2
    );
    assert_eq!(journal.state_count(), 2);
    assert_eq!(journal.current().scene_state_id(), current_scene);
}

#[test]
fn stale_navigation_and_commit_ids_are_rejected_without_cursor_change() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    assert!(journal.undo_target(&[9]).is_none());
    assert_eq!(journal.commit_redo(99), Err(JournalError::ConcurrentChange));
    assert_eq!(journal.current().scene_state_id(), 1);
}

#[test]
fn independent_scene_edits_remain_separate_undo_steps() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    let first = journal
        .prepare_scene()
        .unwrap()
        .finish(2, |_| true)
        .unwrap();
    let second = journal
        .prepare_scene()
        .unwrap()
        .finish(3, |_| true)
        .unwrap();
    assert_eq!(journal.state_count(), 3);
    assert_eq!(
        journal.undo_target(&[1, 2, 3]).unwrap().app_state_id(),
        first
    );
    journal.commit_undo(first).unwrap();
    assert_eq!(journal.undo_target(&[1, 2, 3]).unwrap().app_state_id(), 1);
    assert_eq!(
        journal.redo_target(&[1, 2, 3]).unwrap().app_state_id(),
        second
    );
    let branch = journal
        .prepare_scene()
        .unwrap()
        .finish(4, |_| true)
        .unwrap();
    assert!(branch > second);
    assert_eq!(journal.state_count(), 3);
    assert!(journal.redo_target(&[1, 2, 4]).is_none());
    assert_eq!(
        journal.undo_target(&[1, 2, 4]).unwrap().app_state_id(),
        first
    );
}

#[test]
fn initialization_rejects_payload_over_byte_bound() {
    let payload = Snapshot::new(1, vexel_project::MAX_BYTES + 1);
    let before = payload.clone();
    assert_eq!(
        Journal::try_new(1, payload.clone()).unwrap_err(),
        JournalError::Capacity
    );
    assert_eq!(payload, before);
    let journal = journal(1, Snapshot::new(1, vexel_project::MAX_BYTES));
    assert_eq!(journal.current().app_state_id(), 1);
    assert_eq!(journal.next_app_state_id, 2);
}

// Include cursor, IDs, checkpoint and both branches in admission comparisons.
fn logical_state(journal: &Journal<Snapshot>) -> (usize, u64, u64, Vec<(u64, u64, Snapshot)>) {
    (
        journal.cursor,
        journal.next_app_state_id,
        journal.checkpoint_id,
        journal
            .states
            .iter()
            .map(|state| {
                (
                    state.app_state_id,
                    state.scene_state_id,
                    state.payload.clone(),
                )
            })
            .collect(),
    )
}

#[test]
fn initialization_reservation_failure_preserves_input_and_initial_identity() {
    let payload = Snapshot::new(1, 8);
    let before = payload.clone();
    FAIL_NEXT_RESERVATION.with(|fail| fail.set(true));
    assert_eq!(
        Journal::try_new(7, payload.clone()).unwrap_err(),
        JournalError::Capacity
    );
    assert_eq!(payload, before);
    let journal = journal(7, payload);
    assert_eq!(journal.cursor, 0);
    assert_eq!(journal.next_app_state_id, 2);
    assert_eq!(journal.checkpoint_id(), 1);
    assert_eq!(journal.current().app_state_id(), 1);
    assert_eq!(journal.current().scene_state_id(), 7);
    assert!(journal.redo_target(&[7]).is_none());
}

#[test]
fn admission_reservation_failures_preserve_cursor_ids_checkpoint_and_redo() {
    let mut journal = journal(1, Snapshot::new(1, 8));
    let saved = journal
        .prepare_payload(Snapshot::new(2, 8))
        .unwrap()
        .commit();
    journal.checkpoint();
    let redo = journal
        .prepare_payload(Snapshot::new(3, 8))
        .unwrap()
        .commit();
    journal.commit_undo(saved).unwrap();
    let before = logical_state(&journal);

    FAIL_NEXT_RESERVATION.with(|fail| fail.set(true));
    let mut edit_ran = false;
    let result = journal.prepare_scene().map(|admission| {
        edit_ran = true;
        admission.finish(2, |_| true)
    });
    assert_eq!(result, Err(JournalError::Capacity));
    assert!(!edit_ran);
    assert_eq!(logical_state(&journal), before);
    assert_eq!(journal.redo_target(&[1]).unwrap().app_state_id(), redo);

    let candidate = Snapshot::new(4, 8);
    let saved_candidate = candidate.clone();
    FAIL_NEXT_RESERVATION.with(|fail| fail.set(true));
    assert_eq!(
        journal.prepare_payload(candidate.clone()).err(),
        Some(JournalError::Capacity)
    );
    assert_eq!(candidate, saved_candidate);
    assert_eq!(logical_state(&journal), before);
    assert_eq!(journal.redo_target(&[1]).unwrap().app_state_id(), redo);
    assert_eq!(
        journal.prepare_payload(candidate).unwrap().commit(),
        redo + 1
    );
}

#[derive(Debug)]
struct CloneProbe {
    clones: std::rc::Rc<std::cell::Cell<usize>>,
    deny_clone: std::rc::Rc<std::cell::Cell<bool>>,
}

impl Clone for CloneProbe {
    fn clone(&self) -> Self {
        assert!(!self.deny_clone.get(), "payload cloned during finalization");
        self.clones.set(self.clones.get() + 1);
        Self {
            clones: self.clones.clone(),
            deny_clone: self.deny_clone.clone(),
        }
    }
}

impl PayloadSnapshot for CloneProbe {
    fn identity(&self) -> usize {
        std::rc::Rc::as_ptr(&self.clones) as usize
    }

    fn retained_bytes(&self) -> usize {
        1
    }
}

#[test]
fn scene_payload_clone_is_staged_before_mutation_and_finalization_never_clones() {
    let clones = std::rc::Rc::new(std::cell::Cell::new(0));
    let deny_clone = std::rc::Rc::new(std::cell::Cell::new(false));
    let mut journal = Journal::try_new(
        1,
        CloneProbe {
            clones: clones.clone(),
            deny_clone: deny_clone.clone(),
        },
    )
    .unwrap();
    let admission = journal.prepare_scene().unwrap();
    assert_eq!(clones.get(), 1);
    // Simulate publication of the editor mutation after admission succeeded.
    deny_clone.set(true);
    assert_eq!(admission.finish(2, |_| true), Some(2));
    assert_eq!(clones.get(), 1);
    deny_clone.set(false);

    let admission = journal.prepare_scene().unwrap();
    assert_eq!(clones.get(), 2);
    deny_clone.set(true);
    assert_eq!(
        admission.finish(3, |scene| [1, 3].contains(&scene)),
        Some(3)
    );
    assert_eq!(clones.get(), 2);
    deny_clone.set(false);

    let admission = journal
        .prepare_payload(CloneProbe {
            clones: clones.clone(),
            deny_clone: deny_clone.clone(),
        })
        .unwrap();
    let prepared_clones = clones.get();
    deny_clone.set(true);
    assert_eq!(admission.commit(), 4);
    assert_eq!(clones.get(), prepared_clones);
}
