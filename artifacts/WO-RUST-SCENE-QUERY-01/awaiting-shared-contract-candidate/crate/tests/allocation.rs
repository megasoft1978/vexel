use std::sync::Arc;
use vexel_scene::{AllocationTag, ChargeLedger};
use vexel_scene_query::{AllocationTracking, SVG_ALLOC_TRACKER_CAPACITY, reserve_table};

fn ledger() -> ChargeLedger {
    let source = Arc::new(b"source".to_vec());
    ChargeLedger::new(&source, 1 << 20, 32).unwrap()
}

#[test]
fn explicit_capacity_is_charged_and_released() {
    let mut job = ledger();
    let mut tracking = AllocationTracking::tracking_start(&job);
    let baseline = job.stats().live_bytes;
    let mut values = Vec::<u32>::new();
    values.try_reserve_exact(8).unwrap();
    let mut lease = tracking
        .track_buffer(&mut job, AllocationTag::TokenArray, values)
        .unwrap();
    let stats = tracking.tracking_stop(&mut job).unwrap();
    assert!(stats.token_array_bytes >= 8 * std::mem::size_of::<u32>());
    assert!(stats.live_bytes >= stats.token_array_bytes + stats.source_bytes);
    assert!(stats.source_bytes > 0);
    let before_release = job.stats().live_bytes;
    tracking.release_buffer(&mut job, &mut lease).unwrap();
    assert_eq!(
        job.stats().live_bytes,
        before_release - stats.token_array_bytes
    );
    assert!(job.stats().live_bytes > baseline); // retained lease registry capacity is charged
}

#[test]
fn tracking_scope_enforces_record_cap_and_rejects_foreign_release() {
    let source = Arc::new(Vec::new());
    let mut job =
        ChargeLedger::new(&source, 16 * 1024 * 1024, SVG_ALLOC_TRACKER_CAPACITY + 2).unwrap();
    let mut owner = AllocationTracking::tracking_start(&job);
    let mut foreign = AllocationTracking::tracking_start(&job);
    let mut first = owner
        .track_buffer(&mut job, AllocationTag::None, Vec::<()>::with_capacity(1))
        .unwrap();
    assert_eq!(
        foreign
            .release_buffer(&mut job, &mut first)
            .unwrap_err()
            .kind,
        vexel_scene::ErrorKind::InvalidArgument
    );
    owner.release_buffer(&mut job, &mut first).unwrap();
    let mut leases: Vec<_> = Vec::new();
    leases
        .try_reserve_exact(SVG_ALLOC_TRACKER_CAPACITY)
        .unwrap();
    for _ in 0..SVG_ALLOC_TRACKER_CAPACITY {
        leases.push(
            owner
                .track_buffer(&mut job, AllocationTag::None, Vec::<()>::new())
                .unwrap(),
        );
    }
    let error = owner
        .track_buffer(&mut job, AllocationTag::None, Vec::<()>::new())
        .unwrap_err();
    assert_eq!(
        error.kind,
        vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Nodes)
    );
    for lease in &mut leases {
        owner.release_buffer(&mut job, lease).unwrap();
    }
}

#[test]
fn reserve_table_tracks_old_new_overlap_and_releases_old_capacity() {
    let mut job = ledger();
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = tracking
        .track_buffer(&mut job, AllocationTag::Output, vec![3u32, 5])
        .unwrap();
    let before = job.stats().output_bytes;
    reserve_table(
        &mut tracking,
        &mut job,
        AllocationTag::Output,
        &mut values,
        64,
    )
    .unwrap();
    assert_eq!(values.as_slice(), &[3, 5]);
    assert!(values.capacity() >= 66);
    let new_bytes = values.capacity() * std::mem::size_of::<u32>();
    assert!(job.stats().peak_live_bytes >= before + new_bytes);
    assert_eq!(job.stats().output_bytes, new_bytes);
    tracking.release_buffer(&mut job, &mut values).unwrap();
    assert_eq!(job.stats().output_bytes, 0);
    assert_eq!(
        tracking
            .release_buffer(&mut job, &mut values)
            .unwrap_err()
            .kind,
        vexel_scene::ErrorKind::InvalidArgument
    );
}

#[test]
fn reserve_table_rejects_budget_without_losing_values() {
    let source = Arc::new(Vec::new());
    let mut job = ChargeLedger::new(&source, 513, 4).unwrap();
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = tracking
        .track_buffer(&mut job, AllocationTag::Output, vec![11u8])
        .unwrap();
    let before = job.stats();
    assert!(
        reserve_table(
            &mut tracking,
            &mut job,
            AllocationTag::Output,
            &mut values,
            1024
        )
        .is_err()
    );
    assert_eq!(values.as_slice(), &[11]);
    assert_eq!(
        job.stats().total_allocated_bytes,
        before.total_allocated_bytes
    );
    tracking.release_buffer(&mut job, &mut values).unwrap();
}

#[test]
fn reserve_table_reports_size_arithmetic_before_reserving() {
    let mut job = ledger();
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = tracking
        .track_buffer(&mut job, AllocationTag::Output, Vec::<u64>::new())
        .unwrap();
    let error = reserve_table(
        &mut tracking,
        &mut job,
        AllocationTag::Output,
        &mut values,
        usize::MAX,
    )
    .unwrap_err();
    assert_eq!(
        error.kind,
        vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic)
    );
    assert!(values.is_empty());
    tracking.release_buffer(&mut job, &mut values).unwrap();
}

#[test]
fn replacement_cannot_bypass_tracker_record_cap() {
    let source = Arc::new(Vec::new());
    let mut job = ChargeLedger::new(&source, 16 << 20, SVG_ALLOC_TRACKER_CAPACITY + 1).unwrap();
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = tracking
        .track_buffer(&mut job, AllocationTag::None, vec![1u8])
        .unwrap();
    let mut leases: Vec<vexel_scene_query::TrackedBuffer<u8>> = Vec::new();
    leases
        .try_reserve_exact(SVG_ALLOC_TRACKER_CAPACITY - 1)
        .unwrap();
    for _ in 1..SVG_ALLOC_TRACKER_CAPACITY {
        leases.push(
            tracking
                .track_buffer(&mut job, AllocationTag::None, Vec::new())
                .unwrap(),
        );
    }
    let before = job.stats();
    let error =
        reserve_table(&mut tracking, &mut job, AllocationTag::None, &mut values, 8).unwrap_err();
    assert_eq!(
        error.kind,
        vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Nodes)
    );
    assert_eq!(values.as_slice(), &[1]);
    assert_eq!(
        job.stats().total_allocated_bytes,
        before.total_allocated_bytes
    );
    for lease in &mut leases {
        tracking.release_buffer(&mut job, lease).unwrap();
    }
    tracking.release_buffer(&mut job, &mut values).unwrap();
}

#[test]
fn tracker_rejects_source_mismatch_and_stale_or_wrong_scope_leases() {
    let mut job = ledger();
    let other_source = Arc::new(b"source".to_vec());
    let mut other_job = ChargeLedger::new(&other_source, 1 << 20, 32).unwrap();
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut lease = tracking
        .track_buffer(&mut job, AllocationTag::None, Vec::<u8>::new())
        .unwrap();
    assert_eq!(
        tracking
            .release_buffer(&mut other_job, &mut lease)
            .unwrap_err()
            .kind,
        vexel_scene::ErrorKind::InvalidArgument
    );
    let mut foreign = AllocationTracking::tracking_start(&job);
    assert_eq!(
        foreign
            .release_buffer(&mut job, &mut lease)
            .unwrap_err()
            .kind,
        vexel_scene::ErrorKind::InvalidArgument
    );
    tracking.release_buffer(&mut job, &mut lease).unwrap();
    assert_eq!(
        tracking
            .release_buffer(&mut job, &mut lease)
            .unwrap_err()
            .kind,
        vexel_scene::ErrorKind::InvalidArgument
    );
}

#[test]
fn replacement_rejects_a_different_ledger_for_the_same_source() {
    let source = Arc::new(b"same source".to_vec());
    let mut job = ChargeLedger::new(&source, 1 << 20, 16).unwrap();
    let mut other_job = ChargeLedger::new(&source, 1 << 20, 16).unwrap();
    let mut tracking = AllocationTracking::tracking_start(&job);
    let mut values = tracking
        .track_buffer(&mut job, AllocationTag::Output, vec![2u8, 4])
        .unwrap();
    let before = job.stats();
    let error = reserve_table(
        &mut tracking,
        &mut other_job,
        AllocationTag::Output,
        &mut values,
        8,
    )
    .unwrap_err();
    assert_eq!(error.kind, vexel_scene::ErrorKind::InvalidArgument);
    assert_eq!(values.as_slice(), &[2, 4]);
    assert_eq!(job.stats().live_bytes, before.live_bytes);
    assert_eq!(other_job.stats().total_allocated_bytes, 0);
    tracking.release_buffer(&mut job, &mut values).unwrap();
}
