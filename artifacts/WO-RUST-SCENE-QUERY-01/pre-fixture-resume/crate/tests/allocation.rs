use std::sync::Arc;
use vexel_scene::{AllocationTag, ChargeLedger};
use vexel_scene_query::{AllocationTracking, reserve_table};

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
    tracking
        .track_capacity(&mut job, AllocationTag::TokenArray, &values)
        .unwrap();
    let stats = tracking.tracking_stop(&job);
    assert!(stats.token_array_bytes >= 8 * std::mem::size_of::<u32>());
    assert!(stats.live_bytes >= stats.token_array_bytes + stats.source_bytes);
    assert!(stats.source_bytes > 0);
    let before_release = job.stats().live_bytes;
    tracking.release_last(&mut job).unwrap();
    assert_eq!(
        job.stats().live_bytes,
        before_release - stats.token_array_bytes
    );
    assert!(job.stats().live_bytes > baseline); // retained lease registry capacity is charged
}

#[test]
fn reserve_table_charges_replacement_before_publishing() {
    let mut job = ledger();
    let mut values = vec![3u32, 5];
    let lease = reserve_table(&mut job, AllocationTag::Output, &mut values, 64).unwrap();
    assert_eq!(&values[..2], &[3, 5]);
    assert!(values.capacity() >= 66);
    assert!(job.stats().output_bytes > 0);
    job.release(lease).unwrap();
}

#[test]
fn reserve_table_rejects_budget_without_losing_values() {
    let source = Arc::new(Vec::new());
    let mut job = ChargeLedger::new(&source, 0, 4).unwrap();
    let mut values = vec![11u8];
    assert!(reserve_table(&mut job, AllocationTag::Output, &mut values, 1024).is_err());
    assert_eq!(values, vec![11]);
}
