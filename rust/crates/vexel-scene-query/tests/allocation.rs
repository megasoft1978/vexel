use std::sync::Arc;
use vexel_scene::{AllocationTag, ChargeLedger, ErrorKind, LimitKind, Stop, WorkControl, WorkKind};
use vexel_scene_query::{AllocationTracking, SVG_ALLOC_TRACKER_CAPACITY, reserve_table};

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
struct StopAfter(usize);
impl WorkControl for StopAfter {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            self.0 -= 1;
            Ok(())
        }
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}
fn ledger(bytes: usize, leases: usize) -> ChargeLedger {
    ChargeLedger::new(&Arc::new(Vec::new()), bytes, leases).unwrap()
}
fn track<T: Copy>(
    tracking: &mut AllocationTracking,
    job: &mut ChargeLedger,
    tag: AllocationTag,
    capacity: usize,
    initial: &[T],
) -> vexel_scene_query::TrackedBuffer<T> {
    tracking
        .track_buffer(job, tag, capacity, initial, &mut Run)
        .unwrap()
}

#[test]
fn explicit_capacity_is_charged_and_released() {
    let mut job = ledger(1 << 20, 32);
    let mut tracking = AllocationTracking::tracking_start(&job);
    let baseline = job.stats().live_bytes;
    let mut values = track(
        &mut tracking,
        &mut job,
        AllocationTag::TokenArray,
        8,
        &[3u32, 5],
    );
    let stats = tracking.tracking_stop(&job).unwrap();
    assert!(stats.token_array_bytes >= 8 * std::mem::size_of::<u32>());
    assert!(stats.live_bytes >= stats.token_array_bytes + stats.source_bytes);
    assert!(stats.source_bytes == 0);
    let before_release = job.stats().live_bytes;
    tracking.release_buffer(&mut job, &mut values).unwrap();
    assert_eq!(
        job.stats().live_bytes,
        before_release - stats.token_array_bytes
    );
    assert!(job.stats().live_bytes > baseline); // retained ledger registry capacity
    assert_eq!(
        values.values(&tracking, &job).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
}

#[test]
fn calloc_adaptation_initializes_typed_elements_after_preallocation() {
    let mut job = ledger(1 << 20, 16);
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = tracking
        .track_filled_buffer(&mut job, AllocationTag::TokenArray, 4, 0u8, &mut Run)
        .unwrap();
    assert_eq!(values.values(&tracking, &job).unwrap(), &[0, 0, 0, 0]);
    assert_eq!(
        job.stats().token_array_bytes,
        values.capacity(&tracking, &job).unwrap()
    );
    tracking.release_buffer(&mut job, &mut values).unwrap();
}

#[test]
fn tracking_scope_enforces_all_records_and_rejects_foreign_release() {
    let mut job = ledger(16 << 20, SVG_ALLOC_TRACKER_CAPACITY);
    let mut owner = AllocationTracking::tracking_start(&job);
    let mut foreign = AllocationTracking::tracking_start(&job);
    let mut first = track(&mut owner, &mut job, AllocationTag::None, 1, &[()]);
    assert_eq!(
        foreign
            .release_buffer(&mut job, &mut first)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    owner.release_buffer(&mut job, &mut first).unwrap();
    let mut buffers: Vec<vexel_scene_query::TrackedBuffer<u8>> = Vec::new();
    buffers
        .try_reserve_exact(SVG_ALLOC_TRACKER_CAPACITY)
        .unwrap();
    for _ in 0..SVG_ALLOC_TRACKER_CAPACITY {
        buffers.push(track(&mut owner, &mut job, AllocationTag::None, 0, &[]));
    }
    assert_eq!(
        owner
            .track_buffer::<u8>(&mut job, AllocationTag::None, 0, &[], &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Nodes)
    );
    let before = job.stats();
    assert_eq!(
        reserve_table(
            &mut owner,
            &mut job,
            AllocationTag::None,
            &mut buffers[0],
            1,
            &mut Run
        )
        .unwrap_err()
        .kind,
        ErrorKind::Limit(LimitKind::Nodes)
    );
    assert_eq!(
        job.stats().total_allocated_bytes,
        before.total_allocated_bytes
    );
    owner.release_buffer(&mut job, &mut buffers[1]).unwrap();
    reserve_table(
        &mut owner,
        &mut job,
        AllocationTag::None,
        &mut buffers[0],
        1,
        &mut Run,
    )
    .unwrap(); // At 65,535 old records, replacement overlap reaches exactly 65,536.
    assert!(buffers[0].capacity(&owner, &job).unwrap() >= 1);
    for (index, buffer) in buffers.iter_mut().enumerate() {
        if index == 1 {
            continue;
        }
        owner.release_buffer(&mut job, buffer).unwrap();
    }
}

#[test]
fn replacement_keeps_old_and_new_charges_until_old_storage_drops() {
    let mut job = ledger(1 << 20, 32);
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = track(
        &mut tracking,
        &mut job,
        AllocationTag::Output,
        2,
        &[3u32, 5],
    );
    let old_bytes = job.stats().output_bytes;
    reserve_table(
        &mut tracking,
        &mut job,
        AllocationTag::Output,
        &mut values,
        64,
        &mut Run,
    )
    .unwrap();
    assert_eq!(values.values(&tracking, &job).unwrap(), &[3, 5]);
    let new_bytes = values.capacity(&tracking, &job).unwrap() * 4;
    assert!(new_bytes >= 66 * 4);
    assert!(job.stats().peak_live_bytes >= job.stats().source_bytes + old_bytes + new_bytes);
    assert_eq!(job.stats().output_bytes, new_bytes);
    tracking.release_buffer(&mut job, &mut values).unwrap();
    assert_eq!(job.stats().output_bytes, 0);
    assert_eq!(
        values.values(&tracking, &job).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        tracking
            .release_buffer(&mut job, &mut values)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
}

#[test]
fn budget_failure_preserves_old_values_and_honest_ledger_history() {
    let mut job = ledger(1 << 20, 64);
    let mut warm = Vec::new();
    for _ in 0..32 {
        warm.push(
            job.allocate_buffer::<u8>(AllocationTag::None, 0, &mut Run)
                .unwrap(),
        );
    }
    for buffer in &mut warm {
        buffer.release(&mut job).unwrap();
    }
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = track(
        &mut tracking,
        &mut job,
        AllocationTag::Output,
        512 << 10,
        &[11u8],
    );
    let before = job.stats();
    assert_eq!(
        reserve_table(
            &mut tracking,
            &mut job,
            AllocationTag::Output,
            &mut values,
            600 << 10,
            &mut Run
        )
        .unwrap_err()
        .kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert_eq!(values.values(&tracking, &job).unwrap(), &[11]);
    assert_eq!(job.stats(), before);
    tracking.release_buffer(&mut job, &mut values).unwrap();
}

#[test]
fn checked_overflow_and_native_reserve_failure_keep_old_identity() {
    let mut job = ledger(usize::MAX, 32);
    let mut warm = Vec::new();
    for _ in 0..8 {
        warm.push(
            job.allocate_buffer::<u8>(AllocationTag::None, 0, &mut Run)
                .unwrap(),
        );
    }
    for buffer in &mut warm {
        buffer.release(&mut job).unwrap();
    }
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = track(&mut tracking, &mut job, AllocationTag::Output, 4, &[6u8, 7]);
    let before = job.stats();
    let error = reserve_table(
        &mut tracking,
        &mut job,
        AllocationTag::Output,
        &mut values,
        isize::MAX as usize,
        &mut Run,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::OutOfMemory, "{error:?}"); // Vec CapacityOverflow before payload allocation
    assert_eq!(values.values(&tracking, &job).unwrap(), &[6, 7]);
    assert_eq!(job.stats().output_bytes, before.output_bytes);
    assert_eq!(job.stats().live_bytes, before.live_bytes);
    assert!(job.stats().total_allocated_bytes >= before.total_allocated_bytes);
    tracking.release_buffer(&mut job, &mut values).unwrap();

    let mut arithmetic_job = ledger(1 << 20, 32);
    let mut arithmetic_tracking = AllocationTracking::tracking_start(&arithmetic_job);
    let mut empty: vexel_scene_query::TrackedBuffer<u64> = track(
        &mut arithmetic_tracking,
        &mut arithmetic_job,
        AllocationTag::Output,
        0,
        &[],
    );
    assert_eq!(
        reserve_table(
            &mut arithmetic_tracking,
            &mut arithmetic_job,
            AllocationTag::Output,
            &mut empty,
            usize::MAX,
            &mut Run
        )
        .unwrap_err()
        .kind,
        ErrorKind::Limit(LimitKind::Arithmetic)
    );
    assert!(arithmetic_job.stats().overflow);
    assert!(
        empty
            .values(&arithmetic_tracking, &arithmetic_job)
            .unwrap()
            .is_empty()
    );
    arithmetic_tracking
        .release_buffer(&mut arithmetic_job, &mut empty)
        .unwrap();
}

#[test]
fn replacement_rejects_same_source_wrong_ledger_and_wrong_tracker() {
    let source = Arc::new(b"same source".to_vec());
    let mut job = ChargeLedger::new(&source, 1 << 20, 32).unwrap();
    let mut other_job = ChargeLedger::new(&source, 1 << 20, 32).unwrap();
    let empty_source = Arc::new(Vec::new());
    let mut wrong_source_job = ChargeLedger::new(&empty_source, 1 << 20, 32).unwrap();
    let mut owner = AllocationTracking::tracking_start(&job);
    let mut foreign = AllocationTracking::tracking_start(&job);
    let mut values = track(&mut owner, &mut job, AllocationTag::Output, 8, &[2u8, 4]);
    let before = job.stats();
    assert_eq!(
        reserve_table(
            &mut owner,
            &mut other_job,
            AllocationTag::Output,
            &mut values,
            8,
            &mut Run
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        reserve_table(
            &mut owner,
            &mut wrong_source_job,
            AllocationTag::Output,
            &mut values,
            8,
            &mut Run
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        foreign
            .release_buffer(&mut job, &mut values)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(values.values(&owner, &job).unwrap(), &[2, 4]);
    assert_eq!(job.stats(), before);
    assert_eq!(other_job.stats().total_allocated_bytes, 0);
    owner.release_buffer(&mut job, &mut values).unwrap();
}

#[test]
fn replacement_cancellation_releases_new_lease_but_retains_physical_history() {
    let mut job = ledger(1 << 20, 32);
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = track(
        &mut tracking,
        &mut job,
        AllocationTag::Output,
        8,
        &[9u32, 8],
    );
    let before = job.stats();
    assert_eq!(
        reserve_table(
            &mut tracking,
            &mut job,
            AllocationTag::Output,
            &mut values,
            64,
            &mut StopAfter(1)
        )
        .unwrap_err()
        .kind,
        ErrorKind::Canceled
    );
    assert_eq!(values.values(&tracking, &job).unwrap(), &[9, 8]);
    assert_eq!(job.stats().output_bytes, before.output_bytes);
    assert!(job.stats().total_allocated_bytes > before.total_allocated_bytes);
    tracking.release_buffer(&mut job, &mut values).unwrap();
}

#[test]
fn construction_cancel_releases_buffer_without_erasing_physical_history() {
    let mut job = ledger(1 << 20, 32);
    let mut tracking = AllocationTracking::tracking_start(&job);
    let before = job.stats();
    assert_eq!(
        tracking
            .track_buffer(
                &mut job,
                AllocationTag::Output,
                1024,
                &[1u8],
                &mut StopAfter(1)
            )
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(tracking.tracking_stop(&job).unwrap().output_bytes, 0);
    assert!(job.stats().total_allocated_bytes >= before.total_allocated_bytes + 1024);
}
