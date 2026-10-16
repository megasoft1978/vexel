use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

fn entropy_for(value: u128) -> [u8; 16] {
    value.to_be_bytes()
}

fn start(store: &TaskStore, owner: u64, now: u64, value: u128) -> TaskId {
    let mut random = Some(entropy_for(value));
    store
        .start_with_entropy(owner, now, |output| {
            *output = random.take().expect("one entropy request");
            Ok(())
        })
        .unwrap()
}

#[test]
fn lifecycle_ownership_progress_cancel_and_finish_preserve_original_values() {
    let store = TaskStore::new().unwrap();
    let first = start(&store, 7, 100, 1);
    let second = start(&store, 7, 101, 2);
    assert_eq!(first.to_string(), "00000000000000000000000000000001");
    assert_ne!(first, second);
    let entropy_called = std::cell::Cell::new(false);
    assert_eq!(
        store
            .start_with_entropy(7, 102, |_| {
                entropy_called.set(true);
                Ok(())
            })
            .unwrap_err(),
        TaskStoreError::ActiveLimit
    );
    assert!(!entropy_called.get());
    assert_eq!(
        store.get(8, &first, 103, 32).unwrap_err(),
        TaskStoreError::NotFound
    );
    assert!(store.update(7, &first, 104, 400, b"Rendering 1/2").is_ok());
    assert_eq!(
        store.update(7, &first, 105, 300, b"Regressed"),
        Err(TaskStoreError::InvalidProgress)
    );
    assert_eq!(
        store.update(7, &first, 105, 1001, b"Out of range"),
        Err(TaskStoreError::InvalidProgress)
    );
    store.request_cancel(7, &first, 106).unwrap();
    assert!(store.cancel_requested(7, &first).unwrap());
    store
        .finish(
            7,
            &first,
            107,
            TaskStatus::Cancelled,
            b"Export cancelled",
            &[],
            false,
        )
        .unwrap();
    assert!(store.cancel_requested(7, &first).unwrap());
    let (snapshot, result) = store.get(7, &first, 108, 0).unwrap();
    assert_eq!(snapshot.status, TaskStatus::Cancelled);
    assert_eq!(snapshot.progress_permille, 400);
    assert_eq!(snapshot.message.as_str(), Some("Export cancelled"));
    assert!(result.bytes.is_empty());
    assert_eq!(
        store.finish(
            7,
            &first,
            109,
            TaskStatus::Failed,
            b"duplicate finish",
            &[],
            true
        ),
        Err(TaskStoreError::NotFound)
    );
}

#[test]
fn retained_eviction_expiration_and_status_names_match_c_contract() {
    let store = TaskStore::new().unwrap();
    let mut ids = Vec::new();
    for index in 0..RETAINED_LIMIT as u128 {
        let id = start(&store, 3, 100 + index as u64, index + 10);
        store
            .finish(
                3,
                &id,
                200 + index as u64,
                TaskStatus::Completed,
                b"Export complete",
                b"{\"saved\":true}",
                false,
            )
            .unwrap();
        ids.push(id);
    }
    let new_id = start(&store, 3, 300, 99);
    assert_eq!(
        store.get(3, &ids[0], 301, RESULT_LIMIT).unwrap_err(),
        TaskStoreError::NotFound
    );
    store
        .finish(
            3,
            &new_id,
            302,
            TaskStatus::Completed,
            b"Export complete",
            b"{\"saved\":true}",
            false,
        )
        .unwrap();
    let (snapshot, result) = store.get(3, &ids[1], 303, 32).unwrap();
    assert_eq!(snapshot.status, TaskStatus::Completed);
    assert_eq!(snapshot.result_length, 14);
    assert_eq!(result.bytes, b"{\"saved\":true}");
    assert_eq!(
        store
            .get(3, &ids[1], 303 + TASK_TTL_MS, RESULT_LIMIT)
            .unwrap_err(),
        TaskStoreError::NotFound
    );
    assert_eq!(TaskStatus::Working.name(), "working");
    assert_eq!(TaskStatus::Completed.name(), "completed");
    assert_eq!(TaskStatus::Cancelled.name(), "cancelled");
    assert_eq!(TaskStatus::Failed.name(), "failed");
}

#[test]
fn active_expiry_requests_cooperative_cancellation_and_saturates_time() {
    let store = TaskStore::new().unwrap();
    let id = start(&store, 1, u64::MAX - 10, 50);
    let (snapshot, _) = store.get(1, &id, u64::MAX - 1, 0).unwrap();
    assert_eq!(snapshot.expires_ms, u64::MAX);
    assert!(!store.cancel_requested(1, &id).unwrap());
    // Expiration is inclusive; at the saturated boundary the entry stays
    // active, records the cancellation request, and updates its timestamp.
    let (expired, _) = store.get(1, &id, u64::MAX, 0).unwrap();
    assert_eq!(expired.updated_ms, u64::MAX);
    assert_eq!(
        expired.message.as_str(),
        Some("Task reached its time limit; stopping")
    );
    assert!(store.cancel_requested(1, &id).unwrap());
}

#[test]
fn result_flags_zero_limit_capacity_and_opaque_copy_are_preserved() {
    let store = TaskStore::new().unwrap();
    let id = start(&store, 4, 10, 70);
    store
        .finish(
            4,
            &id,
            11,
            TaskStatus::Failed,
            b"render failed",
            b"not parsed as JSON",
            true,
        )
        .unwrap();
    assert_eq!(
        store.get(4, &id, 12, 0).unwrap_err(),
        TaskStoreError::ResultCapacity {
            length: 18,
            capacity: 0
        }
    );
    let (snapshot, result) = store.get(4, &id, 12, 18).unwrap();
    assert_eq!(snapshot.status, TaskStatus::Failed);
    assert!(snapshot.result_is_error);
    assert_eq!(result.bytes, b"not parsed as JSON");

    let empty = start(&store, 4, 20, 71);
    store
        .finish(4, &empty, 21, TaskStatus::Completed, b"empty", &[], true)
        .unwrap();
    let (snapshot, result) = store.get(4, &empty, 22, 0).unwrap();
    assert!(snapshot.result_is_error);
    assert_eq!(snapshot.result_length, 0);
    assert!(result.bytes.is_empty());

    let active = start(&store, 4, 30, 72);
    let too_large = vec![0; RESULT_LIMIT + 1];
    assert_eq!(
        store.finish(
            4,
            &active,
            31,
            TaskStatus::Failed,
            b"oversized",
            &too_large,
            false
        ),
        Err(TaskStoreError::ResultTooLarge {
            length: RESULT_LIMIT + 1,
            maximum: RESULT_LIMIT
        })
    );
    assert_eq!(
        store.finish(4, &active, 32, TaskStatus::Working, b"invalid", &[], false),
        Err(TaskStoreError::InvalidStatus)
    );
    assert_eq!(
        store.finish(
            4,
            &active,
            32,
            TaskStatus::Failed,
            &vec![b'm'; MESSAGE_LIMIT + 1],
            &[],
            false
        ),
        Err(TaskStoreError::MessageTooLong)
    );
    store
        .finish(
            4,
            &active,
            33,
            TaskStatus::Completed,
            b"at limit",
            &vec![0xA5; RESULT_LIMIT],
            false,
        )
        .unwrap();
    let (_, copy) = store.get(4, &active, 34, RESULT_LIMIT).unwrap();
    assert_eq!(copy.bytes.len(), RESULT_LIMIT);
    assert!(copy.bytes.iter().all(|byte| *byte == 0xA5));
}

#[test]
fn message_boundaries_and_identifier_validation_are_strict() {
    assert!(TaskId::parse("0123456789abcdef0123456789abcdef").is_ok());
    for invalid in [
        "",
        "0123456789abcdef0123456789abcde",
        "0123456789abcdef0123456789abcdefg",
        "0123456789ABCDEF0123456789ABCDEF",
        "0123456789abcdef0123456789abcde!",
    ] {
        assert_eq!(TaskId::parse(invalid), Err(TaskStoreError::InvalidId));
    }
    let store = TaskStore::new().unwrap();
    let id = start(&store, 1, 0, 80);
    let max_message = "x".repeat(MESSAGE_LIMIT);
    store.update(1, &id, 1, 0, max_message.as_bytes()).unwrap();
    let (snapshot, _) = store.get(1, &id, 1, 0).unwrap();
    assert_eq!(snapshot.message.as_bytes(), max_message.as_bytes());
    assert_eq!(
        store.update(1, &id, 1, 0, &vec![b'x'; MESSAGE_LIMIT + 1]),
        Err(TaskStoreError::MessageTooLong)
    );
    store.update(1, &id, 1, 0, b"raw\xfftext\0ignored").unwrap();
    let (snapshot, _) = store.get(1, &id, 1, 0).unwrap();
    assert_eq!(snapshot.message.as_bytes(), b"raw\xfftext");
    assert_eq!(snapshot.message.as_str(), None);
}

#[test]
fn entropy_failure_and_repeated_collision_are_errors_without_fallback() {
    let store = TaskStore::new().unwrap();
    let before = start(&store, 1, 0, 90);
    let failed = store.start_with_entropy(1, 1, |bytes| {
        bytes.fill(0xCC);
        Err(SystemError::RandomStatus(-71))
    });
    assert_eq!(
        failed,
        Err(TaskStoreError::Entropy(SystemError::RandomStatus(-71)))
    );
    let repeated = entropy_for(90);
    let collision = store.start_with_entropy(1, 1, |bytes| {
        *bytes = repeated;
        Ok(())
    });
    assert_eq!(collision, Err(TaskStoreError::IdentifierCollision));
    assert_eq!(store.get(1, &before, 2, 0).unwrap().0.id, before);
    let callback_store = TaskStore::new().unwrap();
    let id = callback_store
        .start_with_entropy(2, 0, |_| {
            assert!(callback_store.entries.try_lock().is_ok());
            Ok(())
        })
        .unwrap();
    assert_eq!(callback_store.get(2, &id, 0, 0).unwrap().0.id, id);
}

#[test]
fn poisoned_mutex_is_reported_without_recovery() {
    let store = Arc::new(TaskStore::new().unwrap());
    let poisoner = Arc::clone(&store);
    assert!(
        std::thread::spawn(move || {
            let _guard = poisoner.entries.lock().unwrap();
            panic!("poison task store mutex");
        })
        .join()
        .is_err()
    );
    let id = TaskId::parse("00000000000000000000000000000001").unwrap();
    assert_eq!(
        store.cancel_requested(1, &id),
        Err(TaskStoreError::Poisoned)
    );
}

#[test]
fn concurrent_start_and_lifecycle_access_obey_capacity_and_consistency() {
    let store = Arc::new(TaskStore::new().unwrap());
    let barrier = Arc::new(Barrier::new(8));
    let sequence = Arc::new(AtomicUsize::new(1));
    let mut joins = Vec::new();
    for _ in 0..8 {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let sequence = Arc::clone(&sequence);
        joins.push(std::thread::spawn(move || {
            barrier.wait();
            let seed = sequence.fetch_add(1, Ordering::Relaxed) as u128;
            store.start_with_entropy(55, seed as u64, |bytes| {
                *bytes = entropy_for(seed);
                Ok(())
            })
        }));
    }
    let ids: Vec<_> = joins
        .into_iter()
        .filter_map(|join| join.join().unwrap().ok())
        .collect();
    assert_eq!(ids.len(), ACTIVE_LIMIT);
    for id in ids {
        let store_for_thread = Arc::clone(&store);
        std::thread::spawn(move || {
            store_for_thread
                .update(55, &id, 100, 700, b"Concurrent update")
                .unwrap();
            let (snapshot, result) = store_for_thread.get(55, &id, 101, 0).unwrap();
            assert_eq!(snapshot.progress_permille, 700);
            assert_eq!(result.bytes, b"");
        })
        .join()
        .unwrap();
    }
}
