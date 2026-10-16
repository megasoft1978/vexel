use vexel_raster::Tile;
use vexel_tile_cache::{CacheError, TileCache};

fn tile(byte: u8) -> Tile {
    let mut value = Tile::new(1, 1).unwrap();
    value
        .rgba_mut()
        .copy_from_slice(&[byte, byte.wrapping_add(1), byte.wrapping_add(2), 255]);
    value
}

fn cache_for(cap: usize, entries: usize) -> TileCache {
    TileCache::new(cap, entries, 4, 2).unwrap()
}

fn tile_with_capacity(capacity: usize, byte: u8) -> Tile {
    let mut rgba = Vec::with_capacity(capacity);
    rgba.extend_from_slice(&[byte, byte.wrapping_add(1), byte.wrapping_add(2), 255]);
    Tile::from_owned_rgba(1, 1, rgba).unwrap()
}

#[test]
fn validates_limits_and_charges_reserved_metadata() {
    assert_eq!(
        TileCache::new(0, 1, 0, 0).unwrap_err(),
        CacheError::InvalidLimits
    );
    assert_eq!(
        TileCache::new(257 * 1024 * 1024, 1, 0, 0).unwrap_err(),
        CacheError::InvalidLimits
    );
    assert_eq!(
        TileCache::new(1, 1, 0, 0).unwrap_err(),
        CacheError::MetadataExceedsByteCap
    );
    let cache = cache_for(4096, 3);
    assert_eq!(cache.stats().entry_count, 0);
    assert!(cache.stats().accounted_bytes > 0);
}

#[test]
fn spare_payload_capacity_is_charged_and_failed_put_returns_exact_allocation() {
    let metadata = cache_for(4096, 1).stats().accounted_bytes;
    let mut cache = cache_for(metadata + 4, 1);
    let candidate = tile_with_capacity(128, 41);
    let pointer = candidate.rgba().as_ptr();
    let capacity = candidate.allocated_byte_count();
    assert!(capacity > candidate.byte_count());

    let (error, returned) = cache.put(9, candidate).unwrap_err();
    assert_eq!(error, CacheError::TileExceedsByteCap);
    assert_eq!(returned.rgba(), &[41, 42, 43, 255]);
    assert_eq!(returned.rgba().as_ptr(), pointer);
    assert_eq!(returned.allocated_byte_count(), capacity);
    assert_eq!(cache.stats().accounted_bytes, metadata);
    assert_eq!(cache.stats().entry_count, 0);
}

#[test]
fn replacement_and_eviction_charge_reserved_payload_capacity() {
    let metadata = cache_for(4096, 2).stats().accounted_bytes;
    let large = tile_with_capacity(128, 1);
    let large_capacity = large.allocated_byte_count();
    let small = tile_with_capacity(8, 2);
    let small_capacity = small.allocated_byte_count();
    assert!(large_capacity > large.byte_count());
    let mut cache = cache_for(metadata + large_capacity + small_capacity, 2);

    cache.put(1, large).unwrap();
    cache.put(2, small).unwrap();
    assert_eq!(
        cache.stats().accounted_bytes,
        metadata + large_capacity + small_capacity
    );

    let replacement = tile_with_capacity(8, 3);
    assert_eq!(replacement.allocated_byte_count(), small_capacity);
    cache.put(1, replacement).unwrap();
    assert_eq!(cache.get(1).unwrap().rgba()[0], 3);
    assert_eq!(cache.get(2).unwrap().rgba()[0], 2);
    assert_eq!(cache.stats().accounted_bytes, metadata + 2 * small_capacity);

    let before = cache.stats().accounted_bytes;
    let third = tile_with_capacity(large_capacity, 4);
    let third_capacity = third.allocated_byte_count();
    cache.put(3, third).unwrap();
    assert!(cache.stats().accounted_bytes <= cache.stats().byte_cap);
    assert!(cache.stats().accounted_bytes >= metadata + third_capacity);
    assert!(cache.stats().accounted_bytes < before + third_capacity);
    assert_eq!(cache.stats().entry_count, 2);
    assert_eq!(cache.get(1), None);
    assert_eq!(cache.get(2).unwrap().rgba()[0], 2);
    assert_eq!(cache.get(3).unwrap().rgba()[0], 4);
}

#[test]
fn exact_byte_boundary_and_rejected_tile_preserve_caller_data() {
    let metadata = cache_for(4096, 2).stats().accounted_bytes;
    let mut cache = cache_for(metadata + 4, 2);
    cache.put(7, tile(9)).unwrap();
    assert_eq!(cache.stats().accounted_bytes, metadata + 4);
    assert_eq!(cache.get(7).unwrap().rgba(), &[9, 10, 11, 255]);

    let mut too_big = Tile::new(1, 2).unwrap();
    too_big
        .rgba_mut()
        .copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    let (error, returned) = cache.put(8, too_big).unwrap_err();
    assert_eq!(error, CacheError::TileExceedsByteCap);
    assert_eq!(returned.rgba(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(cache.get(8), None);
}

#[test]
fn replacement_does_not_evict_other_entries_and_updates_value() {
    let metadata = cache_for(4096, 2).stats().accounted_bytes;
    let mut cache = cache_for(metadata + 8, 2);
    cache.put(1, tile(1)).unwrap();
    cache.put(2, tile(2)).unwrap();
    cache.put(1, tile(8)).unwrap();
    assert_eq!(cache.stats().entry_count, 2);
    assert_eq!(cache.get(1).unwrap().rgba()[0], 8);
    assert_eq!(cache.get(2).unwrap().rgba()[0], 2);
}

#[test]
fn touched_entry_is_newest_lru_and_eviction_tie_uses_slot_order() {
    let metadata = cache_for(4096, 2).stats().accounted_bytes;
    let mut cache = cache_for(metadata + 8, 2);
    cache.put(10, tile(10)).unwrap();
    cache.put(20, tile(20)).unwrap();
    assert_eq!(cache.get(10).unwrap().rgba()[0], 10);
    cache.put(30, tile(30)).unwrap();
    assert_eq!(cache.get(10).unwrap().rgba()[0], 10);
    assert_eq!(cache.get(20), None);
    assert_eq!(cache.get(30).unwrap().rgba()[0], 30);
}

#[test]
fn count_cap_stats_epoch_invalidation_and_same_epoch_retention() {
    let metadata = cache_for(4096, 1).stats().accounted_bytes;
    let mut cache = cache_for(metadata + 8, 1);
    cache.put(1, tile(1)).unwrap();
    assert_eq!(cache.stats().entry_count, 1);
    cache.set_epoch(4, 2);
    assert!(cache.get(1).is_some());
    cache.set_epoch(5, 2);
    assert_eq!(cache.get(1), None);
    assert_eq!(
        cache.stats(),
        vexel_tile_cache::CacheStats {
            entry_count: 0,
            accounted_bytes: metadata,
            byte_cap: metadata + 8,
            generation: 5,
            scale_bucket: 2,
        }
    );
    cache.put(2, tile(2)).unwrap();
    cache.set_epoch(5, 3);
    assert_eq!(cache.get(2), None);
}

#[test]
fn entry_count_boundary_and_accounting_stay_bounded() {
    let metadata = cache_for(4096, 2).stats().accounted_bytes;
    let mut cache = cache_for(metadata + 8, 2);
    for key in 0..50 {
        cache.put(key, tile(key as u8)).unwrap();
        let stats = cache.stats();
        assert!(stats.entry_count <= 2);
        assert!(stats.accounted_bytes <= stats.byte_cap);
    }
    assert_eq!(cache.stats().entry_count, 2);
}
