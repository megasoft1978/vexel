use crate::util::*;
#[cfg(test)]
use vexel_scene::{BoundsSet, RowBuffers, ScanlineIndex};
use vexel_scene::{
    JobRasterScratch, LimitKind, RasterJob, Result, ScanlineEvent, ScanlineNode, SceneView,
    ShapeRowEvent, ViewMapping, WorkControl, WorkKind,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct JobScanlineState {
    pub shape_root: i32,
    pub event_cursor: usize,
    pub shape_event_cursor: usize,
    pub shape_count: usize,
    pub edge_count: usize,
    pub scanline_count: usize,
    pub listed_edge_count: usize,
}

const Y_OFFSETS: [f64; 5] = [0.125, 0.375, 0.5, 0.625, 0.875];

fn user_y(view: SceneView<'_>, slot: usize, map: ViewMapping) -> f64 {
    let pixel = slot / SCANLINE_VALUES_PER_ROW;
    let sample = slot % SCANLINE_VALUES_PER_ROW;
    view.viewport.view_y + ((pixel as f64 + Y_OFFSETS[sample]) - map.offset_y) / map.scale_y
}
#[cfg(test)]
fn first_at_or_above(view: SceneView<'_>, count: usize, target: f64, map: ViewMapping) -> usize {
    let mut low = 0;
    let mut high = count;
    while low < high {
        let mid = low + (high - low) / 2;
        if user_y(view, mid, map) < target {
            low = mid + 1
        } else {
            high = mid;
        }
    }
    low
}
fn first_controlled(
    view: SceneView<'_>,
    count: usize,
    target: f64,
    map: ViewMapping,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    let mut low = 0usize;
    let mut high = count;
    while low < high {
        poll(control, WorkKind::ScanlineEdgeEntry)?;
        let mid = low + (high - low) / 2;
        if user_y(view, mid, map) < target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    Ok(low)
}
fn event_cmp(a: &ScanlineEvent, b: &ScanlineEvent) -> std::cmp::Ordering {
    a.slot
        .cmp(&b.slot)
        .then(a.edge.cmp(&b.edge))
        .then(a.add.cmp(&b.add))
}

fn sift_events<T: Copy>(
    values: &mut [T],
    mut root: usize,
    count: usize,
    compare: fn(&T, &T) -> std::cmp::Ordering,
    control: &mut dyn WorkControl,
) -> Result<()> {
    while root < count / 2 {
        poll(control, WorkKind::ScanlineEdgeEntry)?;
        let mut child = root * 2 + 1;
        if child + 1 < count && compare(&values[child], &values[child + 1]).is_lt() {
            child += 1;
        }
        if !compare(&values[root], &values[child]).is_lt() {
            return Ok(());
        }
        values.swap(root, child);
        root = child;
    }
    Ok(())
}

fn heap_sort_events<T: Copy>(
    values: &mut [T],
    compare: fn(&T, &T) -> std::cmp::Ordering,
    control: &mut dyn WorkControl,
) -> Result<()> {
    for parent in (0..values.len() / 2).rev() {
        sift_events(values, parent, values.len(), compare, control)?;
    }
    for end in (1..values.len()).rev() {
        poll(control, WorkKind::ScanlineEdgeEntry)?;
        values.swap(0, end);
        sift_events(values, 0, end, compare, control)?;
    }
    Ok(())
}
fn shape_event_cmp(a: &ShapeRowEvent, b: &ShapeRowEvent) -> std::cmp::Ordering {
    a.row
        .cmp(&b.row)
        .then(a.add.cmp(&b.add))
        .then(a.shape.cmp(&b.shape))
}

/// Build and sort scanline edge and active-shape events.
#[cfg(test)]
pub fn create_scanlines(
    view: SceneView<'_>,
    bounds: &BoundsSet,
    map: ViewMapping,
    _resources: &dyn vexel_scene::KernelResources,
    control: &mut dyn WorkControl,
) -> Result<ScanlineIndex> {
    if !map.scale_y.is_finite()
        || map.scale_y <= 0.0
        || !map.offset_y.is_finite()
        || bounds.shapes.len() != view.shapes.len()
    {
        return Err(invalid(0));
    }
    let scanline_count = mul(view.viewport.height as usize, SCANLINE_VALUES_PER_ROW)?;
    if view.edges.len() > u32::MAX as usize
        || view.shapes.len() > u32::MAX as usize
        || view.edges.len() > i32::MAX as usize
        || view.shapes.len() > i32::MAX as usize
    {
        return Err(limit(LimitKind::Edges));
    }
    let mut index = ScanlineIndex::default();
    reserve(&mut index.events, mul(view.edges.len(), 2)?)?;
    reserve(&mut index.nodes, view.edges.len())?;
    reserve(&mut index.roots, view.shapes.len())?;
    reserve(&mut index.shape_events, mul(view.shapes.len(), 2)?)?;
    reserve(&mut index.shape_nodes, view.shapes.len())?;
    index.nodes.resize(
        view.edges.len(),
        ScanlineNode {
            left: -1,
            right: -1,
            height: 1,
        },
    );
    index.roots.resize(view.shapes.len(), -1);
    index.shape_nodes.resize(
        view.shapes.len(),
        ScanlineNode {
            left: -1,
            right: -1,
            height: 1,
        },
    );
    for (shape, b) in bounds.shapes.iter().enumerate() {
        poll(control, WorkKind::ShapeRowVisit)?;
        if !b.pixels.visible {
            continue;
        }
        if b.pixels.x_first > b.pixels.x_last
            || b.pixels.x_last >= view.viewport.width
            || b.pixels.y_first > b.pixels.y_last
            || b.pixels.y_last >= view.viewport.height
        {
            return Err(invalid(source_offset(view, shape)));
        }
        index.shape_events.push(ShapeRowEvent {
            row: b.pixels.y_first,
            shape: shape as u32,
            add: true,
        });
        index.shape_events.push(ShapeRowEvent {
            row: b.pixels.y_last.saturating_add(1),
            shape: shape as u32,
            add: false,
        });
    }
    for (i, edge) in view.edges.iter().enumerate() {
        poll(control, WorkKind::Edge)?;
        let Some(id) = view.edge_shape(i) else {
            continue;
        };
        let Some(shape) = view.shape(id) else {
            return Err(invalid(0));
        };
        if !shape.filled || edge.a.y.is_nan() || edge.b.y.is_nan() || edge.a.y == edge.b.y {
            continue;
        }
        let low = edge.a.y.min(edge.b.y);
        let high = edge.a.y.max(edge.b.y);
        let first = first_at_or_above(view, scanline_count, low, map);
        let mut end = first_at_or_above(view, scanline_count, high, map);
        if first >= end || first >= scanline_count {
            continue;
        }
        end = end.min(scanline_count);
        index.events.push(ScanlineEvent {
            slot: first,
            edge: i as u32,
            add: true,
        });
        index.events.push(ScanlineEvent {
            slot: end,
            edge: i as u32,
            add: false,
        });
    }
    heap_sort_events(&mut index.events, event_cmp, control)?;
    heap_sort_events(&mut index.shape_events, shape_event_cmp, control)?;
    index.shape_count = view.shapes.len();
    index.edge_count = view.edges.len();
    index.scanline_count = scanline_count;
    index.listed_edge_count = 0;
    Ok(index)
}

/// Job-owned scanline event/index construction. All event, node and root
/// capacity is admitted by the RasterJob later used by rows and output.
pub fn create_scanlines_job(
    view: SceneView<'_>,
    bounds: vexel_scene::JobBuffer<vexel_scene::ShapeBounds>,
    map: ViewMapping,
    job: &mut RasterJob,
    scratch: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<JobScanlineState> {
    if !map.scale_y.is_finite() || map.scale_y <= 0.0 || !map.offset_y.is_finite() {
        return Err(invalid(0));
    }
    job.validate_source(view.source_owner())?;
    scratch.validate(job, control)?;
    if job.slice(bounds)?.len() != view.shapes.len() {
        return Err(invalid(0));
    }
    if !job.slice(scratch.scanline_events)?.is_empty()
        || !job.slice(scratch.scanline_nodes)?.is_empty()
        || !job.slice(scratch.scanline_roots)?.is_empty()
        || !job.slice(scratch.shape_events)?.is_empty()
        || !job.slice(scratch.shape_nodes)?.is_empty()
    {
        return Err(invalid(0));
    }
    let n = view.shapes.len();
    let e = view.edges.len();
    if e > u32::MAX as usize
        || n > u32::MAX as usize
        || e > i32::MAX as usize
        || n > i32::MAX as usize
    {
        return Err(limit(LimitKind::Edges));
    }
    let slots = mul(view.viewport.height as usize, SCANLINE_VALUES_PER_ROW)?;
    job.reserve(scratch.scanline_events, mul(e, 2)?, control)?;
    job.reserve(scratch.scanline_nodes, e, control)?;
    job.reserve(scratch.scanline_roots, n, control)?;
    job.reserve(scratch.shape_events, mul(n, 2)?, control)?;
    job.reserve(scratch.shape_nodes, n, control)?;
    let build = (|| {
        for _ in 0..e {
            job.extend(
                scratch.scanline_nodes,
                &[ScanlineNode {
                    left: -1,
                    right: -1,
                    height: 1,
                }],
                control,
            )?;
        }
        for _ in 0..n {
            job.extend(scratch.scanline_roots, &[-1], control)?;
            job.extend(
                scratch.shape_nodes,
                &[ScanlineNode {
                    left: -1,
                    right: -1,
                    height: 1,
                }],
                control,
            )?;
        }
        for shape in 0..n {
            let b = *job.slice(bounds)?.get(shape).ok_or_else(|| invalid(0))?;
            poll(control, WorkKind::ShapeRowVisit)?;
            if !b.pixels.visible {
                continue;
            }
            if b.pixels.x_first > b.pixels.x_last
                || b.pixels.x_last >= view.viewport.width
                || b.pixels.y_first > b.pixels.y_last
                || b.pixels.y_last >= view.viewport.height
            {
                return Err(invalid(source_offset(view, shape)));
            }
            job.extend(
                scratch.shape_events,
                &[ShapeRowEvent {
                    row: b.pixels.y_first,
                    shape: shape as u32,
                    add: true,
                }],
                control,
            )?;
            job.extend(
                scratch.shape_events,
                &[ShapeRowEvent {
                    row: b.pixels.y_last.saturating_add(1),
                    shape: shape as u32,
                    add: false,
                }],
                control,
            )?;
        }
        for (i, edge) in view.edges.iter().enumerate() {
            poll(control, WorkKind::Edge)?;
            let Some(id) = view.edge_shape(i) else {
                continue;
            };
            let Some(shape) = view.shape(id) else {
                return Err(invalid(0));
            };
            if !shape.filled || edge.a.y.is_nan() || edge.b.y.is_nan() || edge.a.y == edge.b.y {
                continue;
            }
            let first = first_controlled(view, slots, edge.a.y.min(edge.b.y), map, control)?;
            let end =
                first_controlled(view, slots, edge.a.y.max(edge.b.y), map, control)?.min(slots);
            if first >= end || first >= slots {
                continue;
            }
            job.extend(
                scratch.scanline_events,
                &[ScanlineEvent {
                    slot: first,
                    edge: i as u32,
                    add: true,
                }],
                control,
            )?;
            job.extend(
                scratch.scanline_events,
                &[ScanlineEvent {
                    slot: end,
                    edge: i as u32,
                    add: false,
                }],
                control,
            )?;
        }
        heap_sort_events(job.slice_mut(scratch.scanline_events)?, event_cmp, control)?;
        heap_sort_events(
            job.slice_mut(scratch.shape_events)?,
            shape_event_cmp,
            control,
        )?;
        Ok(())
    })();
    if let Err(error) = build {
        job.truncate(scratch.scanline_events, 0)?;
        job.truncate(scratch.scanline_nodes, 0)?;
        job.truncate(scratch.scanline_roots, 0)?;
        job.truncate(scratch.shape_events, 0)?;
        job.truncate(scratch.shape_nodes, 0)?;
        return Err(error);
    }
    Ok(JobScanlineState {
        shape_root: -1,
        event_cursor: 0,
        shape_event_cursor: 0,
        shape_count: n,
        edge_count: e,
        scanline_count: slots,
        listed_edge_count: 0,
    })
}

#[cfg(test)]
pub fn reset_scanlines(index: &mut ScanlineIndex) {
    index.event_cursor = 0;
    index.shape_event_cursor = 0;
    index.listed_edge_count = 0;
    index.shape_root = -1;
    index.roots.fill(-1);
}

/// Reserve full admitted row capacities once. Call before repeated build_row calls.
#[cfg(test)]
pub fn prepare_rows(index: &ScanlineIndex, rows: &mut RowBuffers) -> Result<()> {
    let stride = add(index.shape_count, 1)?;
    let offsets = mul(stride, SCANLINE_VALUES_PER_ROW)?;
    let extra = offsets.saturating_sub(rows.shape_offsets.capacity());
    reserve(&mut rows.shape_offsets, extra)?;
    let edge_capacity = mul(index.edge_count, SCANLINE_VALUES_PER_ROW)?;
    let extra = edge_capacity.saturating_sub(rows.active_edges.capacity());
    reserve(&mut rows.active_edges, extra)?;
    let extra = index
        .shape_count
        .saturating_sub(rows.active_shapes.capacity());
    reserve(&mut rows.active_shapes, extra)?;
    rows.shape_offsets.resize(offsets, 0);
    rows.active_edges.clear();
    rows.active_shapes.clear();
    Ok(())
}

pub(crate) fn height(nodes: &[ScanlineNode], node: i32) -> i32 {
    if node < 0 {
        0
    } else {
        nodes[node as usize].height
    }
}
fn update(nodes: &mut [ScanlineNode], n: i32) {
    let i = n as usize;
    nodes[i].height = 1 + height(nodes, nodes[i].left).max(height(nodes, nodes[i].right));
}
pub(crate) fn rotate_left(nodes: &mut [ScanlineNode], root: i32) -> i32 {
    let p = nodes[root as usize].right;
    nodes[root as usize].right = nodes[p as usize].left;
    nodes[p as usize].left = root;
    update(nodes, root);
    update(nodes, p);
    p
}
pub(crate) fn rotate_right(nodes: &mut [ScanlineNode], root: i32) -> i32 {
    let p = nodes[root as usize].left;
    nodes[root as usize].left = nodes[p as usize].right;
    nodes[p as usize].right = root;
    update(nodes, root);
    update(nodes, p);
    p
}
pub(crate) fn rebalance(nodes: &mut [ScanlineNode], root: i32) -> i32 {
    if root < 0 {
        return root;
    }
    update(nodes, root);
    let i = root as usize;
    let balance = height(nodes, nodes[i].left) - height(nodes, nodes[i].right);
    if balance > 1 {
        let l = nodes[i].left;
        if height(nodes, nodes[l as usize].left) < height(nodes, nodes[l as usize].right) {
            nodes[i].left = rotate_left(nodes, l);
        }
        rotate_right(nodes, root)
    } else if balance < -1 {
        let r = nodes[i].right;
        if height(nodes, nodes[r as usize].right) < height(nodes, nodes[r as usize].left) {
            nodes[i].right = rotate_right(nodes, r);
        }
        rotate_left(nodes, root)
    } else {
        root
    }
}
#[cfg(test)]
pub(super) fn insert(nodes: &mut [ScanlineNode], root: i32, key: i32, depth: i32) -> Option<i32> {
    if depth >= SCANLINE_MAX_TREE_DEPTH {
        return None;
    }
    if root < 0 {
        return Some(key);
    }
    let i = root as usize;
    if key < root {
        nodes[i].left = insert(nodes, nodes[i].left, key, depth + 1)?;
    } else if key > root {
        nodes[i].right = insert(nodes, nodes[i].right, key, depth + 1)?;
    } else {
        return None;
    }
    Some(rebalance(nodes, root))
}
#[cfg(test)]
pub(super) fn remove(nodes: &mut [ScanlineNode], root: i32, key: i32, depth: i32) -> Option<i32> {
    if root < 0 || depth >= SCANLINE_MAX_TREE_DEPTH {
        return None;
    }
    let i = root as usize;
    if key < root {
        nodes[i].left = remove(nodes, nodes[i].left, key, depth + 1)?;
    } else if key > root {
        nodes[i].right = remove(nodes, nodes[i].right, key, depth + 1)?;
    } else {
        let l = nodes[i].left;
        let r = nodes[i].right;
        if l < 0 {
            return Some(r);
        }
        if r < 0 {
            return Some(l);
        }
        if height(nodes, l) > height(nodes, r) {
            let new = rotate_right(nodes, root);
            let ri = new as usize;
            nodes[ri].right = remove(nodes, nodes[ri].right, key, depth + 1)?;
            return Some(rebalance(nodes, new));
        } else {
            let new = rotate_left(nodes, root);
            let ni = new as usize;
            nodes[ni].left = remove(nodes, nodes[ni].left, key, depth + 1)?;
            return Some(rebalance(nodes, new));
        }
    }
    Some(rebalance(nodes, root))
}
#[cfg(test)]
pub(super) fn append_order(
    nodes: &[ScanlineNode],
    root: i32,
    out: &mut Vec<u32>,
    capacity: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut stack = [-1i32; SCANLINE_MAX_TREE_DEPTH as usize];
    let mut depth = 0usize;
    let mut current = root;
    while current >= 0 || depth != 0 {
        while current >= 0 {
            if depth >= stack.len() {
                return Err(limit(LimitKind::Work));
            }
            stack[depth] = current;
            depth += 1;
            current = nodes[current as usize].left;
        }
        depth -= 1;
        current = stack[depth];
        if out.len() >= capacity {
            return Err(limit(LimitKind::Scratch));
        }
        poll(control, WorkKind::ScanlineEdgeEntry)?;
        out.push(current as u32);
        current = nodes[current as usize].right;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn validate_index(index: &ScanlineIndex) -> Result<()> {
    if index.event_cursor > index.events.len()
        || index.shape_event_cursor > index.shape_events.len()
        || index.nodes.len() != index.edge_count
        || index.roots.len() != index.shape_count
        || index.shape_nodes.len() != index.shape_count
        || index.shape_root < -1
        || index.shape_root as usize >= index.shape_count && index.shape_root >= 0
    {
        return Err(invalid(0));
    }
    let valid_link = |n: i32, len: usize| n == -1 || (n >= 0 && (n as usize) < len);
    for n in &index.nodes {
        if !valid_link(n.left, index.edge_count)
            || !valid_link(n.right, index.edge_count)
            || n.height < 1
            || n.height > SCANLINE_MAX_TREE_DEPTH
        {
            return Err(invalid(0));
        }
    }
    for n in &index.shape_nodes {
        if !valid_link(n.left, index.shape_count)
            || !valid_link(n.right, index.shape_count)
            || n.height < 1
            || n.height > SCANLINE_MAX_TREE_DEPTH
        {
            return Err(invalid(0));
        }
    }
    if index
        .roots
        .iter()
        .any(|&r| !valid_link(r, index.edge_count))
    {
        return Err(invalid(0));
    }
    for e in &index.events {
        if e.edge as usize >= index.edge_count || e.slot > index.scanline_count {
            return Err(invalid(0));
        }
    }
    for e in &index.shape_events {
        if e.shape as usize >= index.shape_count
            || e.row as usize > index.scanline_count / SCANLINE_VALUES_PER_ROW
        {
            return Err(invalid(0));
        }
    }
    Ok(())
}

/// Update active AVL trees and populate five sorted edge rows plus painter-order shapes.
#[cfg(test)]
pub fn build_row(
    index: &mut ScanlineIndex,
    view: SceneView<'_>,
    pixel_y: u32,
    rows: &mut RowBuffers,
    _resources: &dyn vexel_scene::KernelResources,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    validate_index(index)?;
    if pixel_y >= view.viewport.height
        || index.shape_count != view.shapes.len()
        || index.edge_count != view.edges.len()
        || index.scanline_count != mul(view.viewport.height as usize, SCANLINE_VALUES_PER_ROW)?
    {
        return Err(invalid(0));
    }
    let stride = add(index.shape_count, 1)?;
    let offsets = mul(stride, SCANLINE_VALUES_PER_ROW)?;
    let edge_capacity = mul(index.edge_count, SCANLINE_VALUES_PER_ROW)?;
    if rows.shape_offsets.len() < offsets
        || rows.shape_offsets.capacity() < offsets
        || rows.active_edges.capacity() < edge_capacity
        || rows.active_shapes.capacity() < index.shape_count
    {
        return Err(limit(LimitKind::Scratch));
    }
    rows.active_edges.clear();
    rows.active_shapes.clear();
    while index.shape_event_cursor < index.shape_events.len()
        && index.shape_events[index.shape_event_cursor].row <= pixel_y
    {
        poll(control, WorkKind::ShapeRowVisit)?;
        let event = index.shape_events[index.shape_event_cursor];
        index.shape_event_cursor += 1;
        let shape = event.shape as usize;
        if event.add {
            index.shape_nodes[shape] = ScanlineNode {
                left: -1,
                right: -1,
                height: 1,
            };
        }
        index.shape_root = if event.add {
            insert(
                &mut index.shape_nodes,
                index.shape_root,
                event.shape as i32,
                0,
            )
        } else {
            remove(
                &mut index.shape_nodes,
                index.shape_root,
                event.shape as i32,
                0,
            )
        }
        .ok_or_else(|| limit(LimitKind::Work))?;
    }
    append_order(
        &index.shape_nodes,
        index.shape_root,
        &mut rows.active_shapes,
        index.shape_count,
        control,
    )?;
    let mut cursor = 0usize;
    for sample in 0..SCANLINE_VALUES_PER_ROW {
        let slot = (pixel_y as usize) * SCANLINE_VALUES_PER_ROW + sample;
        while index.event_cursor < index.events.len()
            && index.events[index.event_cursor].slot <= slot
        {
            poll(control, WorkKind::ScanlineEdgeEntry)?;
            let e = index.events[index.event_cursor];
            index.event_cursor += 1;
            let shape = view
                .edge_shape(e.edge as usize)
                .ok_or_else(|| invalid(0))?
                .index();
            let edge = e.edge as i32;
            if e.add {
                index.nodes[e.edge as usize] = ScanlineNode {
                    left: -1,
                    right: -1,
                    height: 1,
                };
            }
            index.roots[shape] = if e.add {
                insert(&mut index.nodes, index.roots[shape], edge, 0)
            } else {
                remove(&mut index.nodes, index.roots[shape], edge, 0)
            }
            .ok_or_else(|| limit(LimitKind::Work))?;
        }
        for &shape in &rows.active_shapes {
            let s = shape as usize;
            rows.shape_offsets[sample * stride + s] = cursor;
            append_order(
                &index.nodes,
                index.roots[s],
                &mut rows.active_edges,
                edge_capacity,
                control,
            )?;
            rows.shape_offsets[sample * stride + s + 1] = rows.active_edges.len();
            cursor = rows.active_edges.len();
        }
    }
    index.listed_edge_count = add(index.listed_edge_count, cursor)?;
    Ok(rows.active_shapes.len())
}
