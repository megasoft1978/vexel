//! Actual five-sample AVL rows in admitted job storage.
use crate::job_storage::{initialize, restore, save};
use crate::scanline::{JobScanlineState, height, rebalance, rotate_left, rotate_right};
use crate::util::*;
use vexel_scene::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JobRowsState {
    pub edge_count: usize,
    pub shape_count: usize,
    pub next_y: u32,
}

pub fn prepare_rows_job(
    index: &JobScanlineState,
    job: &mut RasterJob,
    scratch: &JobRasterScratch,
    backup: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<JobRowsState> {
    scratch.validate(job, control)?;
    backup.validate(job, control)?;
    if scratch.scanline_nodes == backup.scanline_nodes
        || scratch.scanline_roots == backup.scanline_roots
        || scratch.shape_nodes == backup.shape_nodes
        || scratch.shape_offsets == backup.shape_offsets
        || scratch.active_edges == backup.active_edges
        || scratch.active_shapes == backup.active_shapes
    {
        return Err(invalid(0));
    }
    let offsets = mul(add(index.shape_count, 1)?, 5)?;
    let edges = mul(index.edge_count, 5)?;
    let node = ScanlineNode {
        left: -1,
        right: -1,
        height: 1,
    };
    initialize(job, scratch.shape_offsets, offsets, 0, control)?;
    initialize(job, scratch.active_edges, edges, 0, control)?;
    initialize(job, scratch.active_shapes, index.shape_count, 0, control)?;
    initialize(job, backup.scanline_nodes, index.edge_count, node, control)?;
    initialize(job, backup.scanline_roots, index.shape_count, -1, control)?;
    initialize(job, backup.shape_nodes, index.shape_count, node, control)?;
    initialize(job, backup.shape_offsets, offsets, 0, control)?;
    initialize(job, backup.active_edges, edges, 0, control)?;
    initialize(job, backup.active_shapes, index.shape_count, 0, control)?;
    Ok(JobRowsState::default())
}

pub fn reset_scanlines_job(
    index: &mut JobScanlineState,
    rows: &mut JobRowsState,
    job: &mut RasterJob,
    scratch: &JobRasterScratch,
    backup: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<()> {
    save(job, scratch.scanline_roots, backup.scanline_roots, control)?;
    let result = (|| {
        for i in 0..index.shape_count {
            poll(control, WorkKind::ShapeRowVisit)?;
            job.slice_mut(scratch.scanline_roots)?[i] = -1;
        }
        Ok(())
    })();
    if result.is_err() {
        restore(job, backup.scanline_roots, scratch.scanline_roots)?;
        return result;
    }
    index.shape_root = -1;
    index.event_cursor = 0;
    index.shape_event_cursor = 0;
    index.listed_edge_count = 0;
    *rows = JobRowsState::default();
    Ok(())
}

fn insert(
    nodes: &mut [ScanlineNode],
    root: i32,
    key: i32,
    depth: i32,
    control: &mut dyn WorkControl,
) -> Result<i32> {
    poll(control, WorkKind::ScanlineEdgeEntry)?;
    if depth >= SCANLINE_MAX_TREE_DEPTH {
        return Err(limit(LimitKind::Work));
    }
    if root < 0 {
        return Ok(key);
    }
    let i = root as usize;
    if key < root {
        nodes[i].left = insert(nodes, nodes[i].left, key, depth + 1, control)?;
    } else if key > root {
        nodes[i].right = insert(nodes, nodes[i].right, key, depth + 1, control)?;
    } else {
        return Err(limit(LimitKind::Work));
    }
    Ok(rebalance(nodes, root))
}
fn remove(
    nodes: &mut [ScanlineNode],
    root: i32,
    key: i32,
    depth: i32,
    control: &mut dyn WorkControl,
) -> Result<i32> {
    poll(control, WorkKind::ScanlineEdgeEntry)?;
    if root < 0 || depth >= SCANLINE_MAX_TREE_DEPTH {
        return Err(limit(LimitKind::Work));
    }
    let i = root as usize;
    if key < root {
        nodes[i].left = remove(nodes, nodes[i].left, key, depth + 1, control)?;
    } else if key > root {
        nodes[i].right = remove(nodes, nodes[i].right, key, depth + 1, control)?;
    } else {
        let (l, r) = (nodes[i].left, nodes[i].right);
        if l < 0 {
            return Ok(r);
        }
        if r < 0 {
            return Ok(l);
        }
        if height(nodes, l) > height(nodes, r) {
            let n = rotate_right(nodes, root);
            let j = n as usize;
            nodes[j].right = remove(nodes, nodes[j].right, key, depth + 1, control)?;
            return Ok(rebalance(nodes, n));
        }
        let n = rotate_left(nodes, root);
        let j = n as usize;
        nodes[j].left = remove(nodes, nodes[j].left, key, depth + 1, control)?;
        return Ok(rebalance(nodes, n));
    }
    Ok(rebalance(nodes, root))
}
fn append(
    job: &mut RasterJob,
    nodes: JobBuffer<ScanlineNode>,
    root: i32,
    out: JobBuffer<u32>,
    cursor: &mut usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut stack = [-1; 64];
    let mut depth = 0usize;
    let mut current = root;
    while current >= 0 || depth != 0 {
        while current >= 0 {
            poll(control, WorkKind::ScanlineEdgeEntry)?;
            if depth >= 64 {
                return Err(limit(LimitKind::Work));
            }
            stack[depth] = current;
            depth += 1;
            current = job
                .slice(nodes)?
                .get(current as usize)
                .ok_or_else(|| invalid(0))?
                .left;
        }
        depth -= 1;
        current = stack[depth];
        poll(control, WorkKind::ScanlineEdgeEntry)?;
        let dst = job
            .slice_mut(out)?
            .get_mut(*cursor)
            .ok_or_else(|| limit(LimitKind::Scratch))?;
        *dst = current as u32;
        *cursor = add(*cursor, 1)?;
        current = job.slice(nodes)?[current as usize].right;
    }
    Ok(())
}

pub fn validate_scanlines_job(
    index: &JobScanlineState,
    job: &RasterJob,
    s: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let valid = |n: i32, len: usize| n == -1 || n >= 0 && (n as usize) < len;
    if job.slice(s.scanline_nodes)?.len() != index.edge_count
        || job.slice(s.scanline_roots)?.len() != index.shape_count
        || job.slice(s.shape_nodes)?.len() != index.shape_count
        || !valid(index.shape_root, index.shape_count)
        || index.event_cursor > job.slice(s.scanline_events)?.len()
        || index.shape_event_cursor > job.slice(s.shape_events)?.len()
    {
        return Err(invalid(0));
    }
    for (h, len) in [
        (s.scanline_nodes, index.edge_count),
        (s.shape_nodes, index.shape_count),
    ] {
        for n in job.slice(h)? {
            poll(control, WorkKind::ScanlineEdgeEntry)?;
            if !valid(n.left, len) || !valid(n.right, len) || n.height < 1 || n.height > 64 {
                return Err(invalid(0));
            }
        }
    }
    for &root in job.slice(s.scanline_roots)? {
        poll(control, WorkKind::ShapeRowVisit)?;
        if !valid(root, index.edge_count) {
            return Err(invalid(0));
        }
    }
    for e in job.slice(s.scanline_events)? {
        poll(control, WorkKind::ScanlineEdgeEntry)?;
        if e.edge as usize >= index.edge_count || e.slot > index.scanline_count {
            return Err(invalid(0));
        }
    }
    for e in job.slice(s.shape_events)? {
        poll(control, WorkKind::ShapeRowVisit)?;
        if e.shape as usize >= index.shape_count || e.row as usize > index.scanline_count / 5 {
            return Err(invalid(0));
        }
    }
    Ok(())
}

pub fn build_row_job(
    index: &mut JobScanlineState,
    rows: &mut JobRowsState,
    view: SceneView<'_>,
    y: u32,
    job: &mut RasterJob,
    s: &JobRasterScratch,
    backup: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    job.validate_source(view.source_owner())?;
    validate_scanlines_job(index, job, s, control)?;
    if (rows.next_y != 0 && y.saturating_add(1) < rows.next_y)
        || y >= view.viewport.height
        || index.shape_count != view.shapes.len()
        || index.edge_count != view.edges.len()
        || index.scanline_count != mul(view.viewport.height as usize, 5)?
    {
        return Err(invalid(0));
    }
    macro_rules! copies {
        ($fun:ident) => {{
            $fun(job, s.scanline_nodes, backup.scanline_nodes, control)?;
            $fun(job, s.scanline_roots, backup.scanline_roots, control)?;
            $fun(job, s.shape_nodes, backup.shape_nodes, control)?;
            $fun(job, s.shape_offsets, backup.shape_offsets, control)?;
            $fun(job, s.active_edges, backup.active_edges, control)?;
            $fun(job, s.active_shapes, backup.active_shapes, control)?;
        }};
    }
    if s.scanline_nodes == backup.scanline_nodes
        || s.scanline_roots == backup.scanline_roots
        || s.shape_nodes == backup.shape_nodes
        || s.shape_offsets == backup.shape_offsets
        || s.active_edges == backup.active_edges
        || s.active_shapes == backup.active_shapes
    {
        return Err(invalid(0));
    }
    if job.slice(s.shape_offsets)?.len() < mul(add(index.shape_count, 1)?, 5)?
        || job.slice(s.active_edges)?.len() < mul(index.edge_count, 5)?
        || job.slice(s.active_shapes)?.len() < index.shape_count
    {
        return Err(limit(LimitKind::Scratch));
    }
    copies!(save);
    let mut next = *index;
    let mut next_rows = *rows;
    let result = (|| {
        while next.shape_event_cursor < job.slice(s.shape_events)?.len() {
            let e = job.slice(s.shape_events)?[next.shape_event_cursor];
            if e.row > y {
                break;
            }
            poll(control, WorkKind::ShapeRowVisit)?;
            next.shape_event_cursor += 1;
            let nodes = job.slice_mut(s.shape_nodes)?;
            if e.add {
                nodes[e.shape as usize] = ScanlineNode {
                    left: -1,
                    right: -1,
                    height: 1,
                };
            }
            next.shape_root = if e.add {
                insert(nodes, next.shape_root, e.shape as i32, 0, control)?
            } else {
                remove(nodes, next.shape_root, e.shape as i32, 0, control)?
            };
        }
        let mut shapes = 0usize;
        append(
            job,
            s.shape_nodes,
            next.shape_root,
            s.active_shapes,
            &mut shapes,
            control,
        )?;
        let stride = add(next.shape_count, 1)?;
        let mut cursor = 0usize;
        for sample in 0..5 {
            let slot = add(mul(y as usize, 5)?, sample)?;
            while next.event_cursor < job.slice(s.scanline_events)?.len() {
                let e = job.slice(s.scanline_events)?[next.event_cursor];
                if e.slot > slot {
                    break;
                }
                poll(control, WorkKind::ScanlineEdgeEntry)?;
                next.event_cursor += 1;
                let shape = view
                    .edge_shape(e.edge as usize)
                    .ok_or_else(|| invalid(0))?
                    .index();
                let root = job.slice(s.scanline_roots)?[shape];
                let nodes = job.slice_mut(s.scanline_nodes)?;
                if e.add {
                    nodes[e.edge as usize] = ScanlineNode {
                        left: -1,
                        right: -1,
                        height: 1,
                    };
                }
                let root = if e.add {
                    insert(nodes, root, e.edge as i32, 0, control)?
                } else {
                    remove(nodes, root, e.edge as i32, 0, control)?
                };
                job.slice_mut(s.scanline_roots)?[shape] = root;
            }
            for active in 0..shapes {
                poll(control, WorkKind::ShapeRowVisit)?;
                let shape = job.slice(s.active_shapes)?[active] as usize;
                job.slice_mut(s.shape_offsets)?[sample * stride + shape] = cursor;
                let root = job.slice(s.scanline_roots)?[shape];
                append(
                    job,
                    s.scanline_nodes,
                    root,
                    s.active_edges,
                    &mut cursor,
                    control,
                )?;
                job.slice_mut(s.shape_offsets)?[sample * stride + shape + 1] = cursor;
            }
        }
        next.listed_edge_count = add(next.listed_edge_count, cursor)?;
        next_rows = JobRowsState {
            edge_count: cursor,
            shape_count: shapes,
            next_y: y + 1,
        };
        Ok(shapes)
    })();
    if result.is_err() {
        restore(job, backup.scanline_nodes, s.scanline_nodes)?;
        restore(job, backup.scanline_roots, s.scanline_roots)?;
        restore(job, backup.shape_nodes, s.shape_nodes)?;
        restore(job, backup.shape_offsets, s.shape_offsets)?;
        restore(job, backup.active_edges, s.active_edges)?;
        restore(job, backup.active_shapes, s.active_shapes)?;
    } else {
        *index = next;
        *rows = next_rows;
    }
    result
}
