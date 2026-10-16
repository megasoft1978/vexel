#![forbid(unsafe_code)]
//! Bounded, owner-thread cache for rendered RGBA tiles.

use std::mem::size_of;
use vexel_raster::Tile;

/// C cache limit: 256 MiB.
pub const MAX_BYTES: usize = 256 * 1024 * 1024;
/// C cache limit: 65,536 slots.
pub const MAX_ENTRIES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheError {
    InvalidLimits,
    MetadataExceedsByteCap,
    AllocationFailed,
    InvalidTile,
    TileExceedsByteCap,
    AccountingOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub entry_count: usize,
    pub accounted_bytes: usize,
    pub byte_cap: usize,
    pub generation: u64,
    pub scale_bucket: u32,
}

#[derive(Debug)]
struct Entry {
    key: u64,
    tile: Tile,
    used_epoch: u64,
    used_tick: u64,
}

/// Single-owner cache. `get` borrows the tile from this cache; Rust prevents
/// mutation or destruction while that borrow remains live.
#[derive(Debug)]
pub struct TileCache {
    slots: Vec<Option<Entry>>,
    count: usize,
    payload_bytes: usize,
    metadata_bytes: usize,
    byte_cap: usize,
    generation: u64,
    scale_bucket: u32,
    clock_epoch: u64,
    clock: u64,
}

impl TileCache {
    pub fn new(
        byte_cap: usize,
        max_entries: usize,
        generation: u64,
        scale_bucket: u32,
    ) -> Result<Self, CacheError> {
        if byte_cap == 0 || byte_cap > MAX_BYTES || max_entries == 0 || max_entries > MAX_ENTRIES {
            return Err(CacheError::InvalidLimits);
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(max_entries)
            .map_err(|_| CacheError::AllocationFailed)?;
        let metadata_bytes = slots
            .capacity()
            .checked_mul(size_of::<Option<Entry>>())
            .ok_or(CacheError::AccountingOverflow)?;
        if metadata_bytes > byte_cap {
            return Err(CacheError::MetadataExceedsByteCap);
        }
        slots.resize_with(max_entries, || None);
        Ok(Self {
            slots,
            count: 0,
            payload_bytes: 0,
            metadata_bytes,
            byte_cap,
            generation,
            scale_bucket,
            clock_epoch: 0,
            clock: 0,
        })
    }

    pub fn get(&mut self, key: u64) -> Option<&Tile> {
        let index = self.find(key)?;
        let tick = self.next_tick();
        let entry = self.slots[index].as_mut()?;
        entry.used_epoch = self.clock_epoch;
        entry.used_tick = tick;
        Some(&entry.tile)
    }

    /// Stores an owned tile. An error returns that same tile to the caller.
    pub fn put(&mut self, key: u64, tile: Tile) -> Result<(), (CacheError, Tile)> {
        if !Self::valid_tile(&tile) {
            return Err((CacheError::InvalidTile, tile));
        }
        let charge = tile.allocated_byte_count();
        let available = match self.byte_cap.checked_sub(self.metadata_bytes) {
            Some(n) => n,
            None => return Err((CacheError::MetadataExceedsByteCap, tile)),
        };
        if charge > available {
            return Err((CacheError::TileExceedsByteCap, tile));
        }

        let mut slot = self.find(key);
        loop {
            let old_charge = slot
                .and_then(|i| self.slots[i].as_ref())
                .map_or(0, |e| e.tile.allocated_byte_count());
            let replacing = slot.is_some();
            let projected = self
                .accounted_bytes()
                .checked_sub(old_charge)
                .and_then(|n| n.checked_add(charge));
            if projected.map_or(false, |n| n <= self.byte_cap)
                && (replacing || self.count < self.slots.len())
            {
                break;
            }
            let victim = match self.lru_slot() {
                Some(i) => i,
                None => return Err((CacheError::AccountingOverflow, tile)),
            };
            if Some(victim) == slot {
                slot = None;
            }
            self.remove(victim);
        }
        if let Some(i) = slot {
            self.remove(i);
        }
        let index = slot.or_else(|| self.slots.iter().position(Option::is_none));
        let Some(index) = index else {
            return Err((CacheError::AccountingOverflow, tile));
        };
        let tick = self.next_tick();
        self.payload_bytes += charge;
        self.count += 1;
        self.slots[index] = Some(Entry {
            key,
            tile,
            used_epoch: self.clock_epoch,
            used_tick: tick,
        });
        Ok(())
    }

    pub fn set_epoch(&mut self, generation: u64, scale_bucket: u32) {
        if self.generation == generation && self.scale_bucket == scale_bucket {
            return;
        }
        self.clear();
        self.generation = generation;
        self.scale_bucket = scale_bucket;
        self.clock_epoch = 0;
        self.clock = 0;
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.count,
            accounted_bytes: self.accounted_bytes(),
            byte_cap: self.byte_cap,
            generation: self.generation,
            scale_bucket: self.scale_bucket,
        }
    }

    fn valid_tile(tile: &Tile) -> bool {
        tile.width() > 0
            && tile.height() > 0
            && tile.width() <= vexel_raster::MAX_AXIS
            && tile.height() <= vexel_raster::MAX_AXIS
            && tile
                .width()
                .checked_mul(tile.height())
                .and_then(|pixels| usize::try_from(pixels).ok()?.checked_mul(4))
                == Some(tile.byte_count())
    }

    fn accounted_bytes(&self) -> usize {
        self.metadata_bytes + self.payload_bytes
    }
    fn find(&self, key: u64) -> Option<usize> {
        self.slots
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|e| e.key == key))
    }

    fn lru_slot(&self) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|v| (i, v)))
            .min_by_key(|(_, e)| (e.used_epoch, e.used_tick))
            .map(|(i, _)| i)
    }

    fn remove(&mut self, index: usize) {
        if let Some(entry) = self.slots[index].take() {
            self.payload_bytes -= entry.tile.allocated_byte_count();
            self.count -= 1;
        }
    }

    fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
        self.count = 0;
        self.payload_bytes = 0;
    }

    fn next_tick(&mut self) -> u64 {
        if self.clock == u64::MAX {
            self.clock = 0;
            self.clock_epoch = self.clock_epoch.wrapping_add(1);
        }
        self.clock += 1;
        self.clock
    }

    #[cfg(test)]
    fn set_clock_for_test(&mut self, clock: u64) {
        self.clock = clock;
        if clock == u64::MAX {
            if let Some(index) = self
                .slots
                .iter()
                .enumerate()
                .filter_map(|(i, e)| e.as_ref().map(|v| (i, v)))
                .max_by_key(|(_, e)| (e.used_epoch, e.used_tick))
                .map(|(i, _)| i)
            {
                if let Some(entry) = self.slots[index].as_mut() {
                    entry.used_tick = clock;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, TileCache};
    use std::mem::size_of;
    use vexel_raster::Tile;

    #[test]
    fn c_contract_clock_rollover_order() {
        let probe = TileCache::new(4096, 2, 1, 1).unwrap();
        let metadata = probe.stats().accounted_bytes;
        let mut cache = TileCache::new(metadata + 8, 2, 1, 1).unwrap();
        let tile = |value| {
            let mut result = Tile::new(1, 1).unwrap();
            result.rgba_mut()[0] = value;
            result
        };
        cache.put(1, tile(1)).unwrap();
        cache.put(2, tile(2)).unwrap();
        assert!(cache.get(1).is_some());
        cache.put(3, tile(3)).unwrap();
        assert_eq!(cache.get(2), None);
        cache.set_clock_for_test(u64::MAX);
        assert!(cache.get(3).is_some());
        cache.put(4, tile(4)).unwrap();
        assert_eq!(cache.get(1), None);
        assert!(cache.get(3).is_some());
        assert!(cache.get(4).is_some());
    }

    #[test]
    fn equal_recency_evicts_lowest_slot_first() {
        let probe = TileCache::new(4096, 2, 1, 1).unwrap();
        let metadata = probe.stats().accounted_bytes;
        let mut cache = TileCache::new(metadata + 8, 2, 1, 1).unwrap();
        let make_tile = || Tile::new(1, 1).unwrap();
        cache.put(1, make_tile()).unwrap();
        cache.put(2, make_tile()).unwrap();
        for entry in cache.slots.iter_mut().flatten() {
            entry.used_epoch = 8;
            entry.used_tick = 9;
        }
        cache.put(3, make_tile()).unwrap();
        assert_eq!(cache.get(1), None);
        assert!(cache.get(2).is_some());
        assert!(cache.get(3).is_some());
    }

    #[test]
    fn metadata_charge_uses_actual_entry_layout_and_reserved_capacity() {
        let cache = TileCache::new(4096, 3, 0, 0).unwrap();
        assert_eq!(
            cache.metadata_bytes,
            cache.slots.capacity() * size_of::<Option<Entry>>()
        );
        assert!(cache.slots.capacity() >= cache.slots.len());
        assert_eq!(cache.stats().accounted_bytes, cache.metadata_bytes);
    }
}

impl Drop for TileCache {
    fn drop(&mut self) {
        self.clear();
    }
}
