use vexel_base::{
    bytes_equal, bytes_sub, mut_bytes_sub, size_add, size_mul, AllocationGate, Arena,
    ArenaConfig, BaseStatus, Span,
};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Default)]
struct FailingGate(Rc<Cell<(usize, usize)>>);

impl FailingGate {
    fn fail_on(&self, call: usize) {
        let (attempts, _) = self.0.get();
        self.0.set((attempts, call));
    }
    fn fail_next(&self) {
        let (attempts, _) = self.0.get();
        self.0.set((attempts, attempts + 1));
    }
}

impl AllocationGate for FailingGate {
    fn permit(&mut self, _requested_bytes: usize) -> Result<(), BaseStatus> {
        let (attempts, fail_on) = self.0.get();
        let call = attempts + 1;
        self.0.set((call, fail_on));
        if fail_on != 0 && call == fail_on {
            return Err(BaseStatus::OutOfMemory);
        }
        Ok(())
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn span_valid_empty_bounds_and_overflow() {
    assert_eq!(Span::from_offset_length(1, 2).and_then(|span| span.slice(b"abcd")), Some(&b"bc"[..]));
    assert_eq!(Span::from_offset_length(4, 0).and_then(|span| span.slice(b"abcd")), Some(&b""[..]));
    assert_eq!(Span { start: 3, end: 2 }.slice(b"abcd"), None);
    assert_eq!(Span { start: 0, end: 5 }.slice(b"abcd"), None);
    assert_eq!(Span::from_offset_length(usize::MAX, 1), None);
}

#[test]
fn arithmetic_matches_original_assertions_and_randomized_cases() {
    assert_eq!(size_add(0, 0), Some(0));
    assert_eq!(size_add(usize::MAX, 0), Some(usize::MAX));
    assert_eq!(size_add(usize::MAX, 1), None);
    assert_eq!(size_mul(0, usize::MAX), Some(0));
    assert_eq!(size_mul(usize::MAX, 1), Some(usize::MAX));
    assert_eq!(size_mul(usize::MAX, 2), None);

    let mut seed = 0x4d59_5df4_d0f3_3173;
    for _ in 0..100_000 {
        let a = next_random(&mut seed) as usize;
        let b = next_random(&mut seed) as usize;
        assert_eq!(size_add(a, b), a.checked_add(b));
        assert_eq!(size_mul(a, b), a.checked_mul(b));
    }
}

#[test]
fn immutable_and_mutable_slices_preserve_original_assertions() {
    let mut bytes = [0u8; 64];
    let mut copy = [0u8; 64];
    for i in 0..bytes.len() {
        bytes[i] = (i * 17) as u8;
        copy[i] = bytes[i];
    }
    assert!(bytes_equal(&bytes, &copy));
    assert!(bytes_equal(&[], &[]));
    assert!(!bytes_equal(&bytes, &copy[..63]));
    copy[7] ^= 1;
    assert!(!bytes_equal(&bytes, &copy));

    let mut seed = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..10_000 {
        let offset = (next_random(&mut seed) as usize) % (bytes.len() + 1);
        let length = (next_random(&mut seed) as usize) % (bytes.len() - offset + 1);
        assert_eq!(bytes_sub(&bytes, offset, length), Some(&bytes[offset..offset + length]));
        let sub = mut_bytes_sub(&mut bytes, offset, length).unwrap();
        assert_eq!(sub.len(), length);
    }
    assert_eq!(bytes_sub(&bytes, usize::MAX, 1), None);
    assert_eq!(bytes_sub(&bytes, 0, usize::MAX), None);
    assert_eq!(bytes_sub(&bytes[..1], 0, 2), None);
    assert_eq!(bytes_sub(&bytes, bytes.len(), 0), Some(&[][..]));
    assert_eq!(mut_bytes_sub(&mut bytes, 65, 0), None);
}

#[test]
fn arena_alignment_reset_stats_and_limit() {
    assert_eq!(Arena::new(ArenaConfig { initial_chunk_bytes: 0, capacity_limit: 8 }).err(), Some(BaseStatus::Invalid));
    let config = ArenaConfig { initial_chunk_bytes: 16, capacity_limit: 96 };
    let mut arena = Arena::new(config).unwrap();
    assert_eq!(arena.stats().chunk_count, 0);
    assert_eq!(arena.stats().payload_capacity, 0);
    assert_eq!(arena.stats().used_bytes, 0);
    assert_eq!(arena.push(0, 4096).unwrap(), None);
    assert_eq!(arena.push(1, 0), Err(BaseStatus::Invalid));
    assert_eq!(arena.push(1, 3), Err(BaseStatus::Invalid));
    assert_eq!(arena.push(1, 8192), Err(BaseStatus::Invalid));

    let mut handles = Vec::new();
    for alignment in [1, 2, 4, 8, 16, 64, 256, 4096] {
        let handle = arena.push(1, alignment).unwrap().unwrap();
        assert!(arena.handle_is_aligned(handle, alignment));
        handles.push(handle);
    }
    let before = arena.stats();
    assert!(before.chunk_count >= 1 && before.payload_capacity <= 96);
    assert!(before.used_bytes <= before.payload_capacity);
    assert_eq!(arena.push(48, 1), Err(BaseStatus::Limit));

    arena.reset();
    let after = arena.stats();
    assert_eq!(after.chunk_count, before.chunk_count);
    assert_eq!(after.payload_capacity, before.payload_capacity);
    assert_eq!(after.used_bytes, 0);
    assert!(arena.push(8, 8).unwrap().is_some());
    assert_eq!(arena.stats().chunk_count, before.chunk_count);
    assert_eq!(Arena::new(ArenaConfig { initial_chunk_bytes: 9, capacity_limit: 8 }).err(), Some(BaseStatus::Limit));
}

#[test]
fn arena_storage_handles_survive_growth_and_support_borrowed_access() {
    let mut arena = Arena::new(ArenaConfig { initial_chunk_bytes: 8, capacity_limit: 64 }).unwrap();
    let first = arena.push(8, 1).unwrap().unwrap();
    arena.get_mut(first).unwrap().copy_from(b"abcdefgh").unwrap();
    let _second = arena.push(8, 1).unwrap().unwrap();
    let mut copied = [0u8; 8];
    arena.get(first).unwrap().copy_to(&mut copied).unwrap();
    assert_eq!(&copied, b"abcdefgh");
    assert_eq!(first, first);
    let other = Arena::new(ArenaConfig { initial_chunk_bytes: 8, capacity_limit: 8 }).unwrap();
    assert!(other.get(first).is_none());
}

#[test]
fn allocator_failure_is_atomic_and_cleanup_is_owned() {
    let gate = FailingGate::default();
    let config = ArenaConfig { initial_chunk_bytes: 8, capacity_limit: 24 };
    gate.fail_on(1);
    let mut arena = Arena::with_allocation_gate(config, gate.clone()).unwrap();
    assert_eq!(arena.push(8, 1), Err(BaseStatus::OutOfMemory));
    assert_eq!(arena.stats().chunk_count, 0);

    gate.fail_on(0);
    let first = arena.push(8, 1).unwrap().unwrap();
    let before = arena.stats();
    // Reject the next chunk. The existing allocation and accounting stay live.
    gate.fail_next();
    assert_eq!(arena.push(8, 1), Err(BaseStatus::OutOfMemory));
    assert_eq!(arena.stats(), before);
    assert_eq!(arena.get(first).unwrap().len(), 8);
    gate.fail_on(0);
    assert!(arena.push(8, 1).unwrap().is_some());
    assert!(arena.push(8, 1).unwrap().is_some());
    assert_eq!(arena.stats().chunk_count, 3);
    assert_eq!(arena.stats().payload_capacity, 24);
    assert_eq!(arena.push(1, 1), Err(BaseStatus::Limit));
}

#[test]
fn arena_reports_overflow_without_partial_state_change() {
    let mut arena = Arena::new(ArenaConfig { initial_chunk_bytes: 1, capacity_limit: usize::MAX }).unwrap();
    let before = arena.stats();
    assert_eq!(arena.push(usize::MAX, 1), Err(BaseStatus::Overflow));
    assert_eq!(arena.stats(), before);
}
