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
    let lease = tracking
        .track_capacity(&mut job, AllocationTag::TokenArray, &values)
        .unwrap();
    let stats = tracking.tracking_stop(&job);
    assert!(stats.token_array_bytes >= 8 * std::mem::size_of::<u32>());
    assert!(stats.live_bytes >= stats.token_array_bytes + stats.source_bytes);
    assert!(stats.source_bytes > 0);
    let before_release = job.stats().live_bytes;
    tracking.release(&mut job, &lease).unwrap();
    assert_eq!(
        job.stats().live_bytes,
        before_release - stats.token_array_bytes
    );
    assert!(job.stats().live_bytes > baseline); // retained lease registry capacity is charged
}

#[test]
fn tracking_scope_enforces_record_cap_and_rejects_foreign_release() {
    let source = Arc::new(Vec::new());
    let mut job = ChargeLedger::new(&source, 16 * 1024 * 1024, SVG_ALLOC_TRACKER_CAPACITY).unwrap();
    let mut owner = AllocationTracking::tracking_start(&job);
    let mut foreign = AllocationTracking::tracking_start(&job);
    let values = Vec::<()>::with_capacity(1);
    let first = owner
        .track_capacity(&mut job, AllocationTag::None, &values)
        .unwrap();
    assert_eq!(
        foreign.release(&mut job, &first).unwrap_err().kind,
        vexel_scene::ErrorKind::InvalidArgument
    );
    owner.release(&mut job, &first).unwrap();
    let mut leases = Vec::new();
    leases
        .try_reserve_exact(SVG_ALLOC_TRACKER_CAPACITY)
        .unwrap();
    for _ in 0..SVG_ALLOC_TRACKER_CAPACITY {
        leases.push(
            owner
                .track_capacity(&mut job, AllocationTag::None, &values)
                .unwrap(),
        );
    }
    let error = owner
        .track_capacity(&mut job, AllocationTag::None, &values)
        .unwrap_err();
    assert_eq!(
        error.kind,
        vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Nodes)
    );
    for lease in &leases {
        owner.release(&mut job, lease).unwrap();
    }
}

#[test]
fn reserve_table_charges_replacement_before_publishing() {
    let mut job = ledger();
    let mut values = vec![3u32, 5];
    let old_bytes = values.capacity() * std::mem::size_of::<u32>();
    let old_lease = job.admit_vec(AllocationTag::Output, &values).unwrap();
    let lease = reserve_table(&mut job, AllocationTag::Output, &mut values, 64)
        .unwrap()
        .unwrap();
    assert_eq!(&values[..2], &[3, 5]);
    assert!(values.capacity() >= 66);
    let new_bytes = values.capacity() * std::mem::size_of::<u32>();
    assert_eq!(job.stats().output_bytes, old_bytes + new_bytes);
    job.release(old_lease).unwrap();
    assert_eq!(job.stats().output_bytes, new_bytes);
    job.release(lease).unwrap();
    assert_eq!(job.stats().output_bytes, 0);
}

#[test]
fn reserve_table_rejects_budget_without_losing_values() {
    let mut values = vec![11u8];
    let source = Arc::new(Vec::new());
    let old_bytes = values.capacity() * std::mem::size_of::<u8>();
    let mut job = ChargeLedger::new(&source, old_bytes + 512, 4).unwrap();
    let old = job.admit_vec(AllocationTag::Output, &values).unwrap();
    assert!(reserve_table(&mut job, AllocationTag::Output, &mut values, 1024).is_err());
    assert_eq!(values, vec![11]);
    job.release(old).unwrap();
}

#[test]
fn reserve_table_reports_size_arithmetic_before_reserving() {
    let mut job = ledger();
    let mut values = Vec::<u64>::new();
    let error =
        reserve_table(&mut job, AllocationTag::Output, &mut values, usize::MAX).unwrap_err();
    assert_eq!(
        error.kind,
        vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic)
    );
    assert!(values.is_empty());
}
