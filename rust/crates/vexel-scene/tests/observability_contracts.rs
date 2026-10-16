use std::sync::Arc;
use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::*;
use vexel_xml::XmlSpan;

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
struct CancelAt(usize);
impl WorkControl for CancelAt {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            self.0 -= 1;
            Ok(())
        }
    }
}

#[test]
fn source_reference_hits_and_misses_expose_real_probe_deltas() {
    let bytes = Arc::new(b"<svg><g id=\"present\"/></svg>".to_vec());
    let doc = LexicalDocument::parse(bytes.clone(), 1 << 20, &mut Run).unwrap();
    let mut source = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut reference = vexel_svg_reference::ReferenceIndex::create(&bytes, doc.tokens()).unwrap();
    let before = source.reference_lookup_probes();
    let direct_before = reference.lookup_probes();
    let hit = source
        .find_reference(
            XmlSpan {
                offset: 12,
                length: 7,
            },
            &mut Run,
        )
        .unwrap();
    let after_hit = source.reference_lookup_probes();
    let direct_hit = reference.find(XmlSpan {
        offset: 12,
        length: 7,
    });
    assert_eq!(hit.map(|id| id.0), direct_hit);
    assert_eq!(
        after_hit - before,
        reference.lookup_probes() - direct_before
    );
    let before_miss = source.reference_lookup_probes();
    let direct_before_miss = reference.lookup_probes();
    let miss = source
        .find_reference(
            XmlSpan {
                offset: 0,
                length: 3,
            },
            &mut Run,
        )
        .unwrap();
    assert!(miss.is_none());
    let direct_miss = reference.find(XmlSpan {
        offset: 0,
        length: 3,
    });
    assert_eq!(miss.map(|id| id.0), direct_miss);
    assert_eq!(
        source.reference_lookup_probes() - before_miss,
        reference.lookup_probes() - direct_before_miss
    );
    assert_eq!(source.source(), bytes.as_slice());
}

#[test]
fn payload_history_survives_release_reuse_and_buffer_replacement() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 16).unwrap();
    let first = ledger.admit(AllocationTag::Output, 31).unwrap();
    ledger.release(first).unwrap();
    let again = ledger.admit(AllocationTag::Output, 9).unwrap();
    assert_eq!(
        ledger.allocation_history()[AllocationTag::Output as usize],
        40
    );
    ledger.release(again).unwrap();
    let mut buffer = ledger
        .allocate_buffer::<u8>(AllocationTag::ReferenceIndex, 16, &mut Run)
        .unwrap();
    let before = ledger.allocation_history();
    buffer
        .replace_capacity(&mut ledger, AllocationTag::ReferenceIndex, 32, &mut Run)
        .unwrap();
    let after = ledger.allocation_history();
    let replacement_capacity = buffer.capacity(&ledger).unwrap();
    assert!(replacement_capacity >= 32);
    assert_eq!(
        after[AllocationTag::ReferenceIndex as usize]
            - before[AllocationTag::ReferenceIndex as usize],
        replacement_capacity * std::mem::size_of::<u8>()
    );
    buffer.release(&mut ledger).unwrap();
    assert_eq!(ledger.allocation_history(), after);
    assert!(ledger.stats().live_bytes >= ledger.stats().source_bytes);
}

#[test]
fn failed_admission_and_preallocation_cancellation_add_no_payload_history() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 8, 4).unwrap();
    assert!(ledger.admit(AllocationTag::Output, 9).is_err());
    let before = ledger.allocation_history();
    assert!(
        ledger
            .allocate_buffer::<u8>(AllocationTag::Output, 1, &mut CancelAt(0))
            .is_err()
    );
    assert_eq!(ledger.allocation_history(), before);
    assert_eq!(
        ledger.allocation_history()[AllocationTag::Output as usize],
        0
    );
}

#[test]
fn physical_reserve_failure_does_not_record_payload() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, usize::MAX, 4).unwrap();
    let result =
        ledger.allocate_buffer::<u8>(AllocationTag::Output, isize::MAX as usize + 1, &mut Run);
    assert!(result.is_err());
    assert_eq!(
        ledger.allocation_history()[AllocationTag::Output as usize],
        0
    );
    assert_eq!(ledger.stats().output_bytes, 0);
}

#[test]
fn postreserve_cancellation_keeps_actual_history_and_registry_is_separate() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 8).unwrap();
    let metadata_before = ledger.allocation_history()[AllocationTag::None as usize];
    assert!(
        ledger
            .allocate_buffer::<u32>(AllocationTag::Output, 16, &mut CancelAt(1))
            .is_err()
    );
    let history = ledger.allocation_history();
    assert_eq!(
        history[AllocationTag::Output as usize],
        16 * std::mem::size_of::<u32>()
    );
    assert!(history[AllocationTag::None as usize] > metadata_before);
    assert_eq!(
        history[AllocationTag::Output as usize],
        16 * std::mem::size_of::<u32>()
    );
    assert!(ledger.stats().total_allocated_bytes >= history[AllocationTag::Output as usize]);
}

#[test]
fn none_tagged_caller_payload_is_added_after_registry_growth() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 8).unwrap();
    let before = ledger.allocation_history()[AllocationTag::None as usize];
    let lease = ledger.admit(AllocationTag::None, 7).unwrap();
    let after = ledger.allocation_history()[AllocationTag::None as usize];
    assert!(after >= before + 7);
    assert_eq!(
        ledger.stats().live_bytes,
        7 + ledger.stats().source_bytes + (after - before - 7)
    );
    ledger.release(lease).unwrap();
    assert_eq!(
        ledger.allocation_history()[AllocationTag::None as usize],
        after
    );
}

#[test]
fn cancellation_after_copying_replacement_preserves_original_and_counts_replacement() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 8).unwrap();
    let mut buffer = ledger
        .allocate_buffer::<u32>(AllocationTag::Output, 4, &mut Run)
        .unwrap();
    buffer.append(&ledger, &[11, 22, 33], &mut Run).unwrap();
    let original_capacity = buffer.capacity(&ledger).unwrap();
    let original = buffer.values(&ledger).unwrap().to_vec();
    let before = ledger.allocation_history();
    assert!(
        buffer
            .replace_capacity(&mut ledger, AllocationTag::Output, 32, &mut CancelAt(3))
            .is_err()
    );
    assert_eq!(buffer.capacity(&ledger).unwrap(), original_capacity);
    assert_eq!(buffer.values(&ledger).unwrap(), original);
    let after = ledger.allocation_history();
    assert_eq!(
        after[AllocationTag::Output as usize] - before[AllocationTag::Output as usize],
        32 * std::mem::size_of::<u32>()
    );
    buffer.release(&mut ledger).unwrap();
    assert_eq!(ledger.allocation_history(), after);
}

#[test]
fn reading_observability_does_not_change_allocation_or_limit_results() {
    let source = Arc::new(Vec::new());
    let mut observed = ChargeLedger::new(&source, 256, 8).unwrap();
    let mut plain = ChargeLedger::new(&source, 256, 8).unwrap();
    let mut left = observed
        .allocate_buffer::<u32>(AllocationTag::Output, 8, &mut Run)
        .unwrap();
    let mut right = plain
        .allocate_buffer::<u32>(AllocationTag::Output, 8, &mut Run)
        .unwrap();
    for _ in 0..4 {
        let _ = observed.stats();
        let _ = observed.allocation_history();
    }
    assert_eq!(left.capacity(&observed), right.capacity(&plain));
    assert_eq!(observed.stats(), plain.stats());
    assert_eq!(observed.allocation_history(), plain.allocation_history());
    assert_eq!(
        observed
            .admit(AllocationTag::ClipCopy, 1024)
            .unwrap_err()
            .kind,
        plain.admit(AllocationTag::ClipCopy, 1024).unwrap_err().kind
    );
    left.release(&mut observed).unwrap();
    right.release(&mut plain).unwrap();
    assert_eq!(observed.stats(), plain.stats());
}

#[test]
fn repeated_registry_growth_keeps_none_history_equal_to_retained_metadata_plus_payload() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
    let first = ledger.admit(AllocationTag::None, 7).unwrap();
    let initial = ledger.allocation_history()[0];
    let mut grew = false;
    for _ in 0..64 {
        let before = ledger.stats();
        let history = ledger.allocation_history();
        let lease = ledger.admit(AllocationTag::None, 11).unwrap();
        let after = ledger.stats();
        assert_eq!(ledger.allocation_history()[0], after.live_bytes);
        if ledger.allocation_history()[0] - history[0] > 11 {
            assert!(
                after.total_allocated_bytes - before.total_allocated_bytes
                    > ledger.allocation_history()[0] - history[0]
            );
            let history = ledger.allocation_history();
            ledger.release(lease).unwrap();
            ledger.release(first).unwrap();
            assert_eq!(ledger.allocation_history(), history);
            let before = ledger.stats();
            let reused = ledger.admit(AllocationTag::None, 13).unwrap();
            assert_eq!(ledger.allocation_history()[0], history[0] + 13);
            assert_eq!(
                ledger.stats().total_allocated_bytes,
                before.total_allocated_bytes + 13
            );
            ledger.release(reused).unwrap();
            assert_eq!(ledger.allocation_history()[0], history[0] + 13);
            grew = true;
            break;
        }
    }
    assert!(grew, "second registry growth was exercised");
    assert!(ledger.allocation_history()[0] > initial);
}
