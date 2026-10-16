#![forbid(unsafe_code)]
//! Owned immutable spatial index with the C reference's bounds and ordering rules.

pub const MAX_RECORDS: usize = 1_000_000;
pub const MAX_BYTES: usize = 192 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub id: u64,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialError {
    Invalid,
    Limit,
    Overflow,
    OutOfMemory,
    InsufficientCapacity { required: usize },
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    bounds: Bounds,
    morton: u64,
    source_index: u32,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    bounds: Bounds,
    left: u32,
    right: u32,
    entry: u32,
    leaf: bool,
}

/// Owned tree. Sharing `&SpatialIndex` across threads is safe; queries use private scratch.
#[derive(Debug)]
pub struct SpatialIndex {
    entries: Vec<Entry>,
    nodes: Vec<Node>,
    root: Option<u32>,
}

fn valid(bounds: Bounds) -> bool {
    bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width >= 0.0
        && bounds.height >= 0.0
        && (bounds.x + bounds.width).is_finite()
        && (bounds.y + bounds.height).is_finite()
}

fn intersects(a: Bounds, b: Bounds) -> bool {
    a.x <= b.x + b.width && a.x + a.width >= b.x && a.y <= b.y + b.height && a.y + a.height >= b.y
}

fn combine(a: Bounds, b: Bounds) -> Bounds {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let right = (a.x + a.width).max(b.x + b.width);
    let bottom = (a.y + a.height).max(b.y + b.height);
    Bounds {
        x,
        y,
        width: right - x,
        height: bottom - y,
    }
}

fn center(origin: f64, extent: f64) -> f64 {
    origin * 0.5 + (origin + extent) * 0.5
}

fn quantize(value: f64, minimum: f64, maximum: f64) -> u32 {
    let range = maximum * 0.5 - minimum * 0.5;
    if range <= 0.0 {
        return 0;
    }
    let normalized = (value * 0.5 - minimum * 0.5) / range;
    if normalized <= 0.0 {
        return 0;
    }
    if normalized >= 1.0 {
        return u32::MAX;
    }
    (normalized * f64::from(u32::MAX)) as u32
}

fn spread_bits(value: u32) -> u64 {
    let mut v = u64::from(value);
    v = (v | (v << 16)) & 0x0000_ffff_0000_ffff;
    v = (v | (v << 8)) & 0x00ff_00ff_00ff_00ff;
    v = (v | (v << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    v = (v | (v << 2)) & 0x3333_3333_3333_3333;
    v = (v | (v << 1)) & 0x5555_5555_5555_5555;
    v
}

fn allocation_budget(count: usize) -> Result<(usize, usize), SpatialError> {
    let nodes = count
        .checked_mul(2)
        .and_then(|n| n.checked_sub(1))
        .ok_or(SpatialError::Overflow)?;
    let entries_bytes = count
        .checked_mul(std::mem::size_of::<Entry>())
        .ok_or(SpatialError::Overflow)?;
    let nodes_bytes = nodes
        .checked_mul(std::mem::size_of::<Node>())
        .ok_or(SpatialError::Overflow)?;
    let ids_bytes = count
        .checked_mul(std::mem::size_of::<u64>())
        .ok_or(SpatialError::Overflow)?;
    let bytes = entries_bytes
        .checked_add(nodes_bytes)
        .and_then(|n| n.checked_add(ids_bytes))
        .ok_or(SpatialError::Overflow)?;
    if bytes > MAX_BYTES {
        return Err(SpatialError::Limit);
    }
    Ok((nodes, bytes))
}

impl SpatialIndex {
    /// Copy records and build a bounded immutable tree. Input IDs must be unique.
    pub fn new(records: &[Record]) -> Result<Self, SpatialError> {
        let count = records.len();
        if count > MAX_RECORDS {
            return Err(SpatialError::Limit);
        }
        if count == 0 {
            return Ok(Self {
                entries: Vec::new(),
                nodes: Vec::new(),
                root: None,
            });
        }
        let (node_count, _) = allocation_budget(count)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| SpatialError::OutOfMemory)?;
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(node_count)
            .map_err(|_| SpatialError::OutOfMemory)?;
        let mut ids = Vec::new();
        ids.try_reserve_exact(count)
            .map_err(|_| SpatialError::OutOfMemory)?;

        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        let (mut min_left, mut min_top) = (f64::INFINITY, f64::INFINITY);
        let (mut max_right, mut max_bottom) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (i, record) in records.iter().enumerate() {
            let b = record.bounds;
            if !valid(b) {
                return Err(SpatialError::Invalid);
            }
            ids.push(record.id);
            let right = b.x + b.width;
            let bottom = b.y + b.height;
            min_left = min_left.min(b.x);
            min_top = min_top.min(b.y);
            max_right = max_right.max(right);
            max_bottom = max_bottom.max(bottom);
            let cx = center(b.x, b.width);
            let cy = center(b.y, b.height);
            min_x = min_x.min(cx);
            min_y = min_y.min(cy);
            max_x = max_x.max(cx);
            max_y = max_y.max(cy);
            entries.push(Entry {
                bounds: b,
                morton: 0,
                source_index: u32::try_from(i).map_err(|_| SpatialError::Overflow)?,
            });
        }
        if !(max_right - min_left).is_finite() || !(max_bottom - min_top).is_finite() {
            return Err(SpatialError::Overflow);
        }
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SpatialError::Invalid);
        }
        for entry in &mut entries {
            let qx = quantize(center(entry.bounds.x, entry.bounds.width), min_x, max_x);
            let qy = quantize(center(entry.bounds.y, entry.bounds.height), min_y, max_y);
            entry.morton = spread_bits(qx) | (spread_bits(qy) << 1);
        }
        entries.sort_unstable_by_key(|entry| (entry.morton, entry.source_index));
        let mut index = Self {
            entries,
            nodes,
            root: None,
        };
        index.root = Some(index.build(0, count)?);
        Ok(index)
    }

    fn build(&mut self, first: usize, end: usize) -> Result<u32, SpatialError> {
        let node_index = u32::try_from(self.nodes.len()).map_err(|_| SpatialError::Overflow)?;
        // Reserve happened up front, so this push cannot allocate.
        self.nodes.push(Node {
            bounds: self.entries[first].bounds,
            left: 0,
            right: 0,
            entry: 0,
            leaf: false,
        });
        if end - first == 1 {
            self.nodes[node_index as usize].entry =
                u32::try_from(first).map_err(|_| SpatialError::Overflow)?;
            self.nodes[node_index as usize].leaf = true;
            return Ok(node_index);
        }
        let middle = first + (end - first) / 2;
        let left = self.build(first, middle)?;
        let right = self.build(middle, end)?;
        let bounds = combine(
            self.nodes[left as usize].bounds,
            self.nodes[right as usize].bounds,
        );
        self.nodes[node_index as usize] = Node {
            bounds,
            left,
            right,
            entry: 0,
            leaf: false,
        };
        Ok(node_index)
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Write matches in original input order. On any error, output remains untouched.
    pub fn query(&self, viewport: Bounds, output: &mut [usize]) -> Result<usize, SpatialError> {
        if !valid(viewport) {
            return Err(SpatialError::Invalid);
        }
        let mut hits = Vec::new();
        if let Some(root) = self.root {
            self.collect(root, viewport, &mut hits)?;
        }
        hits.sort_unstable();
        if output.len() < hits.len() {
            return Err(SpatialError::InsufficientCapacity {
                required: hits.len(),
            });
        }
        output[..hits.len()].copy_from_slice(&hits);
        Ok(hits.len())
    }

    fn collect(
        &self,
        node_index: u32,
        query: Bounds,
        hits: &mut Vec<usize>,
    ) -> Result<(), SpatialError> {
        let node = self.nodes[node_index as usize];
        if !intersects(node.bounds, query) {
            return Ok(());
        }
        if node.leaf {
            hits.try_reserve(1).map_err(|_| SpatialError::OutOfMemory)?;
            hits.push(self.entries[node.entry as usize].source_index as usize);
        } else {
            self.collect(node.left, query, hits)?;
            self.collect(node.right, query, hits)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{SpatialError, allocation_budget};

    #[test]
    fn sizing_arithmetic_reports_overflow_before_allocation() {
        assert_eq!(allocation_budget(usize::MAX), Err(SpatialError::Overflow));
    }
}
