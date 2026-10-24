//! Parallel frame rendering with in-order delivery. Worker threads render
//! frames ahead of a single consumer (the encoder or writer) that sees them
//! in index order, so output is identical to a serial loop. At most
//! `workers + 2` rendered frames exist at once, and the first error (by frame
//! index) stops everything, as the serial loop would.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};

/// Frame workers for an export of `count` frames. The rasterizer already
/// splits one frame into row bands, so a few frames in flight keep the
/// remaining cores busy while the consumer encodes; more only add memory.
pub(crate) fn worker_count(count: usize) -> usize {
    let forced = FORCED_WORKERS.load(Ordering::Relaxed);
    let auto = std::thread::available_parallelism().map_or(1, |n| n.get()) / 2;
    let k = if forced > 0 { forced } else { auto };
    k.clamp(1, 8).min(count.max(1))
}

/// Tuning and test hook: a nonzero value replaces the automatic worker count.
pub(crate) static FORCED_WORKERS: AtomicUsize = AtomicUsize::new(0);

/// First failure of [`render_ordered`], in frame order.
#[derive(Debug, PartialEq)]
pub(crate) enum OrderedError<R, C> {
    Render(R),
    Consume(C),
}

struct State<T, R> {
    ready: HashMap<usize, Result<T, R>>,
    next_claim: usize,
    consumed: usize,
    abort: bool,
}

/// Sets `abort` if a worker unwinds so the consumer cannot wait forever.
struct PanicGuard<'a, T, R> {
    state: &'a Mutex<State<T, R>>,
    wake: &'a Condvar,
}

impl<T, R> Drop for PanicGuard<'_, T, R> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            lock(self.state).abort = true;
            self.wake.notify_all();
        }
    }
}

fn lock<T, R>(state: &Mutex<State<T, R>>) -> MutexGuard<'_, State<T, R>> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// Renders frames `0..count` with `render` on `workers` threads and passes
/// each result to `consume` on the calling thread in index order.
pub(crate) fn render_ordered<T: Send, R: Send, C>(
    count: usize,
    workers: usize,
    render: impl Fn(usize) -> Result<T, R> + Sync,
    mut consume: impl FnMut(usize, T) -> Result<(), C>,
) -> Result<(), OrderedError<R, C>> {
    let workers = workers.clamp(1, count.max(1));
    if workers == 1 {
        for index in 0..count {
            let item = render(index).map_err(OrderedError::Render)?;
            consume(index, item).map_err(OrderedError::Consume)?;
        }
        return Ok(());
    }
    let window = workers + 2;
    let state = Mutex::new(State {
        ready: HashMap::new(),
        next_claim: 0,
        consumed: 0,
        abort: false,
    });
    let wake = Condvar::new();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let _guard = PanicGuard {
                    state: &state,
                    wake: &wake,
                };
                loop {
                    let index = {
                        let mut s = lock(&state);
                        while !s.abort
                            && s.next_claim < count
                            && s.next_claim >= s.consumed + window
                        {
                            s = wake.wait(s).unwrap_or_else(|e| e.into_inner());
                        }
                        if s.abort || s.next_claim >= count {
                            return;
                        }
                        s.next_claim += 1;
                        s.next_claim - 1
                    };
                    let item = render(index);
                    let failed = item.is_err();
                    let mut s = lock(&state);
                    s.ready.insert(index, item);
                    drop(s);
                    wake.notify_all();
                    if failed {
                        return;
                    }
                }
            });
        }
        let mut outcome = Ok(());
        for index in 0..count {
            let item = {
                let mut s = lock(&state);
                loop {
                    if let Some(item) = s.ready.remove(&index) {
                        break Some(item);
                    }
                    if s.abort {
                        break None;
                    }
                    s = wake.wait(s).unwrap_or_else(|e| e.into_inner());
                }
            };
            let result = match item {
                Some(Ok(item)) => consume(index, item).map_err(OrderedError::Consume),
                Some(Err(e)) => Err(OrderedError::Render(e)),
                None => break,
            };
            if let Err(e) = result {
                outcome = Err(e);
                break;
            }
            lock(&state).consumed = index + 1;
            wake.notify_all();
        }
        lock(&state).abort = true;
        wake.notify_all();
        outcome
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivers_in_order_for_any_worker_count() {
        for workers in [1, 2, 5, 8] {
            let mut seen = Vec::new();
            render_ordered(
                40,
                workers,
                |i| {
                    std::thread::sleep(std::time::Duration::from_micros(((40 - i) * 30) as u64));
                    Ok::<_, ()>(i * 2)
                },
                |i, v| {
                    seen.push((i, v));
                    Ok::<(), ()>(())
                },
            )
            .unwrap();
            assert_eq!(seen, (0..40).map(|i| (i, i * 2)).collect::<Vec<_>>());
        }
    }

    #[test]
    fn first_error_by_index_wins_and_stops_consuming() {
        for workers in [1, 4] {
            let mut seen = 0;
            let result = render_ordered(
                30,
                workers,
                |i| if i == 7 || i == 20 { Err(i) } else { Ok(i) },
                |_, _| {
                    seen += 1;
                    Ok::<(), ()>(())
                },
            );
            assert_eq!(result, Err(OrderedError::Render(7)));
            assert_eq!(seen, 7);
            let result = render_ordered(30, workers, Ok::<_, ()>, |i, _| {
                if i == 3 { Err("sink") } else { Ok(()) }
            });
            assert_eq!(result, Err(OrderedError::Consume("sink")));
        }
    }

    #[test]
    fn frames_in_flight_are_bounded() {
        let live = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        render_ordered(
            60,
            3,
            |_| {
                let now = live.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                Ok::<_, ()>(())
            },
            |_, ()| {
                std::thread::sleep(std::time::Duration::from_micros(200));
                live.fetch_sub(1, Ordering::SeqCst);
                Ok::<(), ()>(())
            },
        )
        .unwrap();
        assert!(peak.load(Ordering::SeqCst) <= 5);
    }
}
