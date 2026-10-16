use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

fn queue(capacity: usize, workers: usize) -> TaskQueue {
    TaskQueue::new(QueueConfig { capacity, workers }).unwrap()
}

fn enqueue<F>(queue: &TaskQueue, id: u64, generation: u64, callback: F) -> Result<(), SubmitError>
where
    F: for<'a> FnOnce(CancellationToken<'a>) -> TaskOutput + Send + 'static,
{
    queue.submit(
        id,
        generation,
        TaskCallback::new((), move |_, token| callback(token)),
    )
}

fn enqueue_owned<C, F>(
    queue: &TaskQueue,
    id: u64,
    generation: u64,
    context: C,
    callback: F,
) -> Result<(), SubmitError>
where
    C: Send + 'static,
    F: for<'a> FnOnce(&mut C, CancellationToken<'a>) -> TaskOutput + Send + 'static,
{
    queue.submit(id, generation, TaskCallback::new(context, callback))
}

fn receive<T>(receiver: &mpsc::Receiver<T>) -> T {
    receiver
        .recv_timeout(Duration::from_secs(3))
        .expect("worker signal timed out")
}

struct CountDrop(Arc<AtomicUsize>);

impl Drop for CountDrop {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct DropSignal(mpsc::Sender<()>);

impl Drop for DropSignal {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

struct FailingGate {
    calls: usize,
    fail_at: usize,
}

impl AllocationGate for FailingGate {
    fn permit(&mut self, _requested_bytes: usize) -> Result<(), BaseStatus> {
        let call = self.calls;
        self.calls += 1;
        if call == self.fail_at {
            Err(BaseStatus::OutOfMemory)
        } else {
            Ok(())
        }
    }
}

#[test]
fn configuration_and_fallible_bounded_allocations() {
    for config in [
        QueueConfig {
            capacity: 0,
            workers: 1,
        },
        QueueConfig {
            capacity: 65,
            workers: 1,
        },
        QueueConfig {
            capacity: 1,
            workers: 0,
        },
        QueueConfig {
            capacity: 1,
            workers: 5,
        },
    ] {
        assert_eq!(TaskQueue::new(config).unwrap_err(), QueueError::Invalid);
    }
    for fail_at in 0..5 {
        let mut gate = FailingGate { calls: 0, fail_at };
        assert_eq!(
            TaskQueue::with_allocation_gate(
                QueueConfig {
                    capacity: 2,
                    workers: 1
                },
                &mut gate
            )
            .err()
            .unwrap(),
            QueueError::NoMemory
        );
        assert_eq!(gate.calls, fail_at + 1);
    }
    let q = queue(64, 4);
    assert_eq!(q.capacity(), 64);
    q.shutdown();
}

#[test]
fn worker_start_failure_stops_and_joins_started_workers() {
    let mut gate = FailingGate {
        calls: 0,
        fail_at: usize::MAX,
    };
    let mut starts = 0;
    let result = TaskQueue::with_allocation_gate_and_spawn(
        QueueConfig {
            capacity: 2,
            workers: 2,
        },
        &mut gate,
        |shared| {
            if starts == 1 {
                starts += 1;
                Err(std::io::Error::other("injected worker-start failure"))
            } else {
                starts += 1;
                spawn_worker(shared)
            }
        },
    );
    assert_eq!(result.err().unwrap(), QueueError::NoMemory);
    assert_eq!(starts, 2);
}

#[test]
fn status_priority_duplicate_full_stale_and_stopped() {
    let q = queue(2, 1);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    enqueue(&q, 1, 1, move |_| {
        entered_tx.send(()).unwrap();
        receive(&release_rx);
        TaskOutput::success(1)
    })
    .unwrap();
    receive(&entered_rx);
    enqueue(&q, 2, 1, |_| TaskOutput::success(2)).unwrap();
    let duplicate = enqueue(&q, 2, 2, |_| TaskOutput::success(0)).unwrap_err();
    assert_eq!(duplicate.status, QueueError::Duplicate);
    assert_eq!(
        enqueue(&q, 3, 1, |_| TaskOutput::success(0))
            .unwrap_err()
            .status,
        QueueError::Full
    );
    assert_eq!(
        enqueue(&q, 3, 2, |_| TaskOutput::success(0))
            .unwrap_err()
            .status,
        QueueError::Stale
    );
    release_tx.send(()).unwrap();
    q.shutdown();
    assert_eq!(
        enqueue(&q, 0, 1, |_| TaskOutput::success(0))
            .unwrap_err()
            .status,
        QueueError::Invalid
    );
    assert_eq!(
        enqueue(&q, 5, 1, |_| TaskOutput::success(0))
            .unwrap_err()
            .status,
        QueueError::Stopped
    );
    assert_eq!(q.cancel(99), Err(QueueError::NotFound));
}

#[test]
fn rejected_submission_returns_callback_and_context() {
    let q = queue(1, 1);
    let drops = Arc::new(AtomicUsize::new(0));
    let capture = Arc::clone(&drops);
    enqueue(&q, 1, 1, move |_| {
        let _keep = capture;
        TaskOutput::success(1)
    })
    .unwrap();
    let capture = CountDrop(Arc::clone(&drops));
    let error = enqueue(&q, 1, 1, move |_| {
        let _keep = capture;
        TaskOutput::success(2)
    })
    .unwrap_err();
    assert_eq!(error.status, QueueError::Duplicate);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(error.callback);
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    let context = Arc::new(String::from("owned"));
    let callback_context = Arc::clone(&context);
    let shared_error = q
        .submit(
            1,
            1,
            TaskCallback::new((), move |_, _| {
                assert_eq!(callback_context.as_str(), "owned");
                TaskOutput::success(3)
            }),
        )
        .unwrap_err();
    assert_eq!(shared_error.status, QueueError::Duplicate);
    drop(shared_error.callback);
    assert_eq!(Arc::strong_count(&context), 1);
    q.shutdown();
}

struct ReentrantDrop(TaskQueue, Arc<AtomicUsize>, Arc<Mutex<Option<TaskResult>>>);

impl Drop for ReentrantDrop {
    fn drop(&mut self) {
        *lock(&self.2) = self.0.poll();
        self.1.fetch_add(1, Ordering::SeqCst);
    }
}

struct ReentrantShutdown(TaskQueue);

impl Drop for ReentrantShutdown {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}

#[test]
fn queued_cancel_drops_owned_context_once_outside_queue_lock() {
    let q = queue(2, 1);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (returned_tx, returned_rx) = mpsc::channel();
    enqueue(&q, 10, 1, move |_| {
        entered_tx.send(()).unwrap();
        receive(&release_rx);
        returned_tx.send(()).unwrap();
        TaskOutput::success(10)
    })
    .unwrap();
    receive(&entered_rx);
    let drops = Arc::new(AtomicUsize::new(0));
    let reentrant_result = Arc::new(Mutex::new(None));
    let guard = ReentrantDrop(q.clone(), Arc::clone(&drops), Arc::clone(&reentrant_result));
    enqueue_owned(&q, 11, 1, guard, |_, _| TaskOutput::success(11)).unwrap();
    assert_eq!(q.cancel(11), Ok(()));
    release_tx.send(()).unwrap();
    receive(&returned_rx);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while drops.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < deadline {
        thread::yield_now();
    }
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        lock(&reentrant_result)
            .as_ref()
            .map(|result| (result.id, result.value)),
        Some((10, 10))
    );
    q.shutdown();
}

#[test]
fn running_cancel_and_generation_advance_suppress_publication() {
    for advance in [false, true] {
        let q = queue(2, 1);
        let (started_tx, started_rx) = mpsc::channel();
        enqueue(&q, 20, 1, move |token| {
            started_tx.send(()).unwrap();
            while !token.is_cancelled() {
                thread::yield_now();
            }
            TaskOutput::success(20)
        })
        .unwrap();
        receive(&started_rx);
        if advance {
            assert_eq!(q.advance_generation(2), Ok(()));
        } else {
            assert_eq!(q.cancel(20), Ok(()));
        }
        q.shutdown();
        assert_eq!(q.poll(), None);
    }
    let q = queue(1, 1);
    assert_eq!(q.advance_generation(2), Ok(()));
    assert_eq!(q.advance_generation(2), Err(QueueError::Stale));
    assert_eq!(
        enqueue(&q, 21, 1, |_| TaskOutput::success(0))
            .unwrap_err()
            .status,
        QueueError::Stale
    );
    let shared = Arc::new(42);
    enqueue_owned(&q, 22, 2, Arc::clone(&shared), |value, _| {
        TaskOutput::success(**value)
    })
    .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let result = loop {
        if let Some(result) = q.poll() {
            break result;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "shared-context completion timed out"
        );
        thread::yield_now();
    };
    assert_eq!((result.id, result.value), (22, 42));
    q.shutdown();
}

#[test]
fn cancellation_after_running_transition_still_invokes_callback_without_publication() {
    let q = queue(1, 1);
    let (gate_entered_rx, gate_resume_tx) = arm_callback_entry_gate(23);
    let (observed_tx, observed_rx) = mpsc::channel();
    let (dropped_tx, dropped_rx) = mpsc::channel();
    enqueue_owned(&q, 23, 1, DropSignal(dropped_tx), move |_, token| {
        observed_tx.send(token.is_cancelled()).unwrap();
        TaskOutput::success(23)
    })
    .unwrap();

    // The test gate only opens after the worker has changed the state to Running
    // and released the queue mutex, but before callback entry.
    receive(&gate_entered_rx);
    assert_eq!(q.cancel(23), Ok(()));
    gate_resume_tx.send(()).unwrap();

    assert!(
        receive(&observed_rx),
        "callback must observe the intervening cancellation"
    );
    receive(&dropped_rx);
    assert_eq!(
        q.poll(),
        None,
        "canceled callback result must not be published"
    );
    q.shutdown();
}

#[test]
fn callback_failure_and_panic_are_retired_as_results() {
    let q = queue(3, 2);
    enqueue(&q, 30, 1, |_| TaskOutput::failure(-1)).unwrap();
    enqueue(&q, 31, 1, |_| -> TaskOutput { panic!("callback panic") }).unwrap();
    let mut results = Vec::new();
    for _ in 0..2 {
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while results.len() < 2 && std::time::Instant::now() < deadline {
            if let Some(result) = q.poll() {
                results.push(result);
            } else {
                thread::yield_now();
            }
        }
    }
    results.sort_by_key(|result| result.id);
    assert_eq!(
        results,
        vec![
            TaskResult {
                id: 30,
                generation: 1,
                succeeded: false,
                value: -1
            },
            TaskResult {
                id: 31,
                generation: 1,
                succeeded: false,
                value: 0
            },
        ]
    );
    q.shutdown();
}

#[test]
fn completion_releases_id_only_after_poll() {
    let q = queue(1, 1);
    let cleanup_count = Arc::new(AtomicUsize::new(0));
    let cleanup = CountDrop(Arc::clone(&cleanup_count));
    enqueue_owned(&q, 40, 1, cleanup, |_, _| TaskOutput::success(44)).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let result = loop {
        if let Some(result) = q.poll() {
            break result;
        }
        assert!(std::time::Instant::now() < deadline, "completion timed out");
        thread::yield_now();
    };
    assert_eq!(
        result,
        TaskResult {
            id: 40,
            generation: 1,
            succeeded: true,
            value: 44
        }
    );
    assert_eq!(cleanup_count.load(Ordering::SeqCst), 1);
    enqueue(&q, 40, 1, |_| TaskOutput::success(45)).unwrap();
    q.shutdown();
}

#[test]
fn shutdown_races_safely_with_cloned_api_calls_and_final_drop_joins() {
    let q = queue(8, 2);
    let producer_queue = q.clone();
    let (started_tx, started_rx) = mpsc::channel();
    let producer = thread::spawn(move || {
        started_tx.send(()).unwrap();
        for id in 1..=10_000 {
            let _ = enqueue(&producer_queue, id, 1, |_| TaskOutput::success(0));
        }
    });
    receive(&started_rx);
    q.shutdown();
    producer.join().unwrap();
    assert_eq!(
        enqueue(&q, 50_000, 1, |_| TaskOutput::success(0))
            .unwrap_err()
            .status,
        QueueError::Stopped
    );

    let q = queue(1, 1);
    let (started_tx, started_rx) = mpsc::channel();
    enqueue(&q, 60, 1, move |token| {
        started_tx.send(()).unwrap();
        while !token.is_cancelled() {
            thread::yield_now();
        }
        TaskOutput::success(60)
    })
    .unwrap();
    receive(&started_rx);
    drop(q); // Final owner requests cancellation and joins before returning.
}

#[test]
fn cleanup_can_request_shutdown_while_another_thread_joins() {
    let q = queue(1, 1);
    let (started_tx, started_rx) = mpsc::channel();
    let context = ReentrantShutdown(q.clone());
    enqueue_owned(&q, 70, 1, context, move |_, token| {
        started_tx.send(()).unwrap();
        while !token.is_cancelled() {
            thread::yield_now();
        }
        TaskOutput::failure(70)
    })
    .unwrap();
    receive(&started_rx);
    q.shutdown();
}
