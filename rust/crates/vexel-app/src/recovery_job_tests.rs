use super::{RecoveryJob, RecoveryJobResult};
use crate::project_session::SessionError;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

fn result(revision: u64) -> RecoveryJobResult {
    RecoveryJobResult {
        epoch: 7,
        revision,
        published: true,
        durability_uncertain: false,
        error: None,
    }
}

fn wait_for_idle(job: &RecoveryJob) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let state = job.shared.state.lock().expect("worker state lock");
        if !state.active && state.pending.is_none() {
            return;
        }
        drop(state);
        assert!(Instant::now() < deadline, "worker did not become idle");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn latest_pending_job_replaces_older_work_and_result_slot_is_bounded() {
    let job = RecoveryJob::spawn_actor().expect("spawn worker");
    let (started_tx, started_rx) = mpsc::channel();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let task_gate = Arc::clone(&gate);
    assert!(job.submit_injected(move || {
        started_tx.send(()).expect("notify started");
        let (lock, changed) = &*task_gate;
        let mut released = lock.lock().expect("gate lock");
        while !*released {
            released = changed.wait(released).expect("gate wait");
        }
        result(1)
    }));
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("first job started");

    let ran_old = Arc::new(Mutex::new(false));
    let old_marker = Arc::clone(&ran_old);
    assert!(job.submit_injected(move || {
        *old_marker.lock().expect("old marker") = true;
        result(2)
    }));
    assert!(job.submit_injected(|| result(3)));
    {
        let (lock, changed) = &*gate;
        *lock.lock().expect("gate lock") = true;
        changed.notify_all();
    }
    wait_for_idle(&job);

    assert!(!*ran_old.lock().expect("old marker"));
    assert_eq!(job.poll_result(), Some(result(3)));
    assert_eq!(job.poll_result(), None);
}

#[test]
fn pause_clears_pending_and_waits_until_active_publication_finishes() {
    let job = Arc::new(RecoveryJob::spawn_actor().expect("spawn worker"));
    let (started_tx, started_rx) = mpsc::channel();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let task_gate = Arc::clone(&gate);
    assert!(job.submit_injected(move || {
        started_tx.send(()).expect("notify started");
        let (lock, changed) = &*task_gate;
        let mut released = lock.lock().expect("gate lock");
        while !*released {
            released = changed.wait(released).expect("gate wait");
        }
        result(1)
    }));
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("active publisher started");
    assert!(job.submit_injected(|| result(2)));

    let (cleared_tx, cleared_rx) = mpsc::channel();
    let (paused_tx, paused_rx) = mpsc::channel();
    let pause_job = Arc::clone(&job);
    let pause_thread = thread::spawn(move || {
        let guard = pause_job.pause_test(|| {
            cleared_tx.send(()).expect("notify paused and cleared");
        });
        paused_tx.send(guard).expect("return pause guard");
    });
    cleared_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("pause cleared pending work");
    {
        let state = job.shared.state.lock().expect("worker state lock");
        assert!(state.paused);
        assert!(state.pending.is_none());
        assert!(state.active);
    }
    assert!(paused_rx.try_recv().is_err());
    {
        let (lock, changed) = &*gate;
        *lock.lock().expect("gate lock") = true;
        changed.notify_all();
    }
    let guard = paused_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("pause acknowledges after publication");
    pause_thread.join().expect("pause thread");
    assert!(!job.submit_injected(|| result(3)));
    assert_eq!(job.poll_result(), Some(result(1)));
    drop(guard);
    assert!(job.submit_injected(|| result(4)));
    wait_for_idle(&job);
    assert_eq!(job.poll_result(), Some(result(4)));
}

#[test]
fn shutdown_drops_pending_and_joins_active_task() {
    let mut job = RecoveryJob::spawn_actor().expect("spawn worker");
    let (started_tx, started_rx) = mpsc::channel();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let task_gate = Arc::clone(&gate);
    assert!(job.submit_injected(move || {
        started_tx.send(()).expect("notify started");
        let (lock, changed) = &*task_gate;
        let mut released = lock.lock().expect("gate lock");
        while !*released {
            released = changed.wait(released).expect("gate wait");
        }
        result(1)
    }));
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("active publisher started");
    let ran_pending = Arc::new(Mutex::new(false));
    let pending_marker = Arc::clone(&ran_pending);
    assert!(job.submit_injected(move || {
        *pending_marker.lock().expect("pending marker") = true;
        result(2)
    }));

    let (stopping_tx, stopping_rx) = mpsc::channel();
    let (stopped_tx, stopped_rx) = mpsc::channel();
    let shared = Arc::clone(&job.shared);
    let shutdown_thread = thread::spawn(move || {
        job.shutdown_test(|| {
            stopping_tx.send(()).expect("notify stopping and cleared");
        });
        stopped_tx.send(job).expect("return stopped worker");
    });
    stopping_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("shutdown recorded stopping state");
    {
        let state = shared.state.lock().expect("worker state lock");
        assert!(state.stopping);
        assert!(state.pending.is_none());
        assert!(state.active);
    }
    assert!(stopped_rx.try_recv().is_err());
    {
        let (lock, changed) = &*gate;
        *lock.lock().expect("gate lock") = true;
        changed.notify_all();
    }
    let stopped = stopped_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("shutdown joined active publisher");
    shutdown_thread.join().expect("shutdown thread");
    assert!(!*ran_pending.lock().expect("pending marker"));
    assert!(!stopped.submit_injected(|| result(3)));
    assert_eq!(stopped.poll_result(), Some(result(1)));
}

#[test]
fn publication_errors_remain_typed_in_the_bounded_result() {
    let job = RecoveryJob::spawn_actor().expect("spawn worker");
    assert!(job.submit_injected(|| RecoveryJobResult {
        epoch: 2,
        revision: 9,
        published: false,
        durability_uncertain: false,
        error: Some(SessionError::RecoveryUnavailable),
    }));
    wait_for_idle(&job);
    assert_eq!(
        job.poll_result(),
        Some(RecoveryJobResult {
            epoch: 2,
            revision: 9,
            published: false,
            durability_uncertain: false,
            error: Some(SessionError::RecoveryUnavailable),
        })
    );
}
