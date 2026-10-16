//! Clip geometry, queries and safe scene mutation adapters.
use vexel_base::WorkControl;
use vexel_geometry::{affine_map_point, affine_valid};
use vexel_scene::*;

pub const MAX_CLIP_CHILDREN: usize = 64;
pub const MAX_CLIP_DEPTH: usize = 256;
pub const MAX_CLIP_REFERENCE_BYTES: usize = 64 * 1024;
pub const MAX_CLIP_JOB_REFERENCE_BYTES: usize = 512 * 1024;
pub const CLIP_WRAPPER_BUFFER_BYTES: usize = 192;
pub const CLIP_WRAPPER_LIMIT_BYTES: usize = 4096;
pub const CLIP_WRAPPER_WORST_BYTES: usize = 155;
pub const MAX_CLIP_EDGES: usize = 65536;
pub const MAX_CLIP_CLONE_DEPTH: usize = 64;
pub const MAX_CLIP_CLONE_BYTES: usize = 64 * 1024 * 1024;
const _: () = assert!(CLIP_WRAPPER_BUFFER_BYTES <= CLIP_WRAPPER_LIMIT_BYTES);
const _: () = assert!(CLIP_WRAPPER_WORST_BYTES <= CLIP_WRAPPER_BUFFER_BYTES);

pub fn create_rect(
    points: [Point; 4],
    user_inverse: Affine,
    owner_node: Option<TransformId>,
    builder: &mut SceneBuilder,
    scene: SceneId,
    control: &mut dyn WorkControl,
) -> Result<ClipId> {
    if !affine_valid(user_inverse) || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Err(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: 0,
        });
    }
    if 4 > MAX_CLIP_EDGES {
        return Err(limit());
    }
    let mut edges = Vec::new();
    edges.try_reserve_exact(4).map_err(|_| oom())?;
    for i in 0..4 {
        edges.push(Edge {
            a: points[i],
            b: points[(i + 1) % 4],
        });
    }
    let mut shapes = Vec::new();
    shapes.try_reserve_exact(1).map_err(|_| oom())?;
    shapes.push(ClipChild {
        edges: TableRange { first: 0, count: 4 },
        evenodd: false,
        filled: true,
    });
    let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) =
        (points[0].x, points[0].y, points[0].x, points[0].y);
    for p in points.iter().skip(1) {
        lo_x = lo_x.min(p.x);
        lo_y = lo_y.min(p.y);
        hi_x = hi_x.max(p.x);
        hi_y = hi_y.max(p.y);
    }
    builder.append_clip(
        scene,
        Clip {
            geometry: ClipGeometry { edges, shapes },
            parent: None,
            owner_node,
            owner_before_suffix: true,
            object_bounding_box: false,
            bounds: [lo_x, lo_y, hi_x, hi_y],
            user_inverse,
        },
        control,
    )
}

pub fn contains(view: SceneView<'_>, clip: ClipId, x: f64, y: f64) -> bool {
    let (mut next, mut depth) = (Some(clip), 0usize);
    while let Some(id) = next {
        if depth >= MAX_CLIP_DEPTH {
            return false;
        }
        let Some(c) = view.clip(id) else { return false };
        let Some(p) = affine_map_point(c.user_inverse, Point { x, y }) else {
            return false;
        };
        let (mut px, mut py) = (p.x, p.y);
        if c.object_bounding_box {
            let [x0, y0, x1, y1] = c.bounds;
            if x1 <= x0 || y1 <= y0 {
                return false;
            }
            px = (px - x0) / (x1 - x0);
            py = (py - y0) / (y1 - y0);
        }
        if !c.geometry.shapes.iter().any(|child| {
            child.filled && point_inside(&c.geometry.edges, child.edges, child.evenodd, px, py)
        }) {
            return false;
        }
        next = c.parent;
        depth += 1;
    }
    true
}
fn point_inside(edges: &[Edge], range: TableRange, evenodd: bool, x: f64, y: f64) -> bool {
    let Some(slice) = range.slice(edges) else {
        return false;
    };
    let (mut winding, mut parity) = (0i32, false);
    for e in slice {
        let up = e.a.y <= y && e.b.y > y;
        let down = e.b.y <= y && e.a.y > y;
        if !up && !down {
            continue;
        }
        let cross = e.a.x + (y - e.a.y) * (e.b.x - e.a.x) / (e.b.y - e.a.y);
        if cross > x {
            winding += if up { 1 } else { -1 };
            parity = !parity;
        }
    }
    if evenodd { parity } else { winding != 0 }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClipWorkCounts {
    pub chain_elements: usize,
    pub child_shapes: usize,
    pub edges: usize,
}
pub fn work_counts(view: SceneView<'_>, clip: ClipId) -> Result<ClipWorkCounts> {
    let mut out = ClipWorkCounts::default();
    let (mut next, mut depth) = (Some(clip), 0);
    while let Some(id) = next {
        if depth >= MAX_CLIP_DEPTH {
            return Err(limit());
        }
        let c = view.clip(id).ok_or_else(invalid)?;
        out.chain_elements = out.chain_elements.checked_add(1).ok_or_else(arithmetic)?;
        out.child_shapes = out
            .child_shapes
            .checked_add(c.geometry.shapes.len())
            .ok_or_else(arithmetic)?;
        out.edges = out
            .edges
            .checked_add(c.geometry.edges.len())
            .ok_or_else(arithmetic)?;
        next = c.parent;
        depth += 1;
    }
    Ok(out)
}
pub fn work_units(view: SceneView<'_>, clip: ClipId) -> Result<usize> {
    let c = work_counts(view, clip)?;
    c.chain_elements
        .checked_add(c.child_shapes)
        .and_then(|x| x.checked_add(c.edges))
        .ok_or_else(arithmetic)
}
pub fn local_bounds(edges: &[Edge], inverse: Affine) -> Option<[f64; 4]> {
    if edges.is_empty() {
        return None;
    }
    let first = affine_map_point(inverse, edges[0].a)?;
    let mut b = [first.x, first.y, first.x, first.y];
    for e in edges {
        for p in [e.a, e.b] {
            let q = affine_map_point(inverse, p)?;
            b[0] = b[0].min(q.x);
            b[1] = b[1].min(q.y);
            b[2] = b[2].max(q.x);
            b[3] = b[3].max(q.y);
        }
    }
    Some(b)
}

/// Visits document-space edges. `Ok(false)` means the visitor stopped; an invalid
/// mapping is reported separately so callers disable culling conservatively.
pub fn visit_boundary_edges(
    view: SceneView<'_>,
    clip: ClipId,
    mut visitor: impl FnMut(Edge) -> bool,
) -> Result<(bool, bool)> {
    let (mut id, mut valid, mut depth) = (Some(clip), true, 0);
    while let Some(cid) = id {
        if depth >= MAX_CLIP_DEPTH {
            return Err(limit());
        }
        let c = view.clip(cid).ok_or_else(invalid)?;
        let Some(local_to_doc) = inverse_affine(c.user_inverse) else {
            return Ok((true, false));
        };
        for child in &c.geometry.shapes {
            if !child.filled {
                continue;
            }
            let Some(edges) = child.edges.slice(&c.geometry.edges) else {
                return Err(invalid());
            };
            for mut edge in edges.iter().copied() {
                for p in [&mut edge.a, &mut edge.b] {
                    let mut q = *p;
                    if c.object_bounding_box {
                        q.x = c.bounds[0] + q.x * (c.bounds[2] - c.bounds[0]);
                        q.y = c.bounds[1] + q.y * (c.bounds[3] - c.bounds[1]);
                    }
                    let Some(mapped) = affine_map_point(local_to_doc, q) else {
                        valid = false;
                        return Ok((true, valid));
                    };
                    *p = mapped;
                }
                if !visitor(edge) {
                    return Ok((false, valid));
                }
            }
        }
        id = c.parent;
        depth += 1;
    }
    Ok((true, valid))
}

/// Appends a per-shape child clip and finalizes each new shape inside the caller's
/// real draft. Existing clip records remain immutable and shared safely.
pub fn apply_group(
    builder: &mut SceneBuilder,
    token: &DraftToken,
    scene: SceneId,
    shape_ids: &[ShapeId],
    clip: ClipId,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if shape_ids.is_empty() {
        return Ok(());
    }
    if shape_ids.len() > MAX_CLIP_CLONE_BYTES / std::mem::size_of::<(ClipId, ClipId)>() {
        return Err(limit());
    }
    let mut root_cache = Vec::<(ClipId, ClipId)>::new();
    root_cache.try_reserve(shape_ids.len()).map_err(|_| oom())?;
    let mut copied_bytes = 0usize;
    for id in shape_ids {
        control.checkpoint().map_err(stopped)?;
        let old = builder
            .scene(scene)?
            .shape(*id)
            .copied()
            .ok_or_else(invalid)?;
        let appended = if let Some(old_root) = old.clip {
            if chain_contains(builder.scene(scene)?, old_root, clip, control)? {
                continue;
            }
            if let Some((_, copied_root)) =
                root_cache.iter().find(|(source, _)| *source == old_root)
            {
                *copied_root
            } else {
                let copied_root = append_clip_chain(
                    builder,
                    scene,
                    old_root,
                    Some(clip),
                    control,
                    &mut copied_bytes,
                )?;
                root_cache.push((old_root, copied_root));
                copied_root
            }
        } else {
            clip
        };
        let mut shape = old;
        shape.clip = Some(appended);
        builder.finalize_shape(token, *id, shape, control)?;
    }
    Ok(())
}
fn chain_contains(
    view: SceneView<'_>,
    root: ClipId,
    target: ClipId,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let mut next = Some(root);
    let mut depth = 0usize;
    while let Some(id) = next {
        control.checkpoint().map_err(stopped)?;
        if depth >= MAX_CLIP_CLONE_DEPTH {
            return Err(limit());
        }
        if id == target {
            return Ok(true);
        }
        let clip = view.clip(id).ok_or_else(invalid)?;
        control.charge(WorkKind::ClipChain, 1).map_err(stopped)?;
        next = clip.parent;
        depth += 1;
    }
    Ok(false)
}
fn append_clip_chain(
    builder: &mut SceneBuilder,
    scene: SceneId,
    root: ClipId,
    parent: Option<ClipId>,
    control: &mut dyn WorkControl,
    copied_bytes: &mut usize,
) -> Result<ClipId> {
    let mut chain = Vec::new();
    chain
        .try_reserve_exact(MAX_CLIP_CLONE_DEPTH)
        .map_err(|_| oom())?;
    let mut next = Some(root);
    while let Some(id) = next {
        control.checkpoint().map_err(stopped)?;
        if chain.len() >= MAX_CLIP_CLONE_DEPTH {
            return Err(limit());
        }
        let view = builder.scene(scene)?;
        let clip = view.clip(id).ok_or_else(invalid)?;
        let work = clip
            .geometry
            .edges
            .len()
            .checked_add(clip.geometry.shapes.len())
            .and_then(|n| n.checked_add(1))
            .ok_or_else(arithmetic)?;
        control
            .charge(
                WorkKind::ClipEdge,
                u64::try_from(work).map_err(|_| arithmetic())?,
            )
            .map_err(stopped)?;
        chain.push(id);
        next = clip.parent;
    }
    let mut parent = parent;
    let mut result = None;
    for id in chain.into_iter().rev() {
        let mut copy = {
            let view = builder.scene(scene)?;
            let source = view.clip(id).ok_or_else(invalid)?;
            clone_clip_geometry(source)?
        };
        copy.parent = parent;
        let actual = copy
            .geometry
            .edges
            .capacity()
            .checked_mul(std::mem::size_of::<Edge>())
            .and_then(|n| {
                copy.geometry
                    .shapes
                    .capacity()
                    .checked_mul(std::mem::size_of::<ClipChild>())
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or_else(arithmetic)?;
        *copied_bytes = copied_bytes
            .checked_add(actual)
            .filter(|n| *n <= MAX_CLIP_CLONE_BYTES)
            .ok_or_else(limit)?;
        let appended = builder.append_clip(scene, copy, control)?;
        parent = Some(appended);
        result = Some(appended);
    }
    result.ok_or_else(invalid)
}
fn clone_clip_geometry(c: &Clip) -> Result<Clip> {
    let mut edges = Vec::new();
    edges
        .try_reserve_exact(c.geometry.edges.len())
        .map_err(|_| oom())?;
    edges.extend_from_slice(&c.geometry.edges);
    let mut shapes = Vec::new();
    shapes
        .try_reserve_exact(c.geometry.shapes.len())
        .map_err(|_| oom())?;
    shapes.extend_from_slice(&c.geometry.shapes);
    Ok(Clip {
        geometry: ClipGeometry { edges, shapes },
        parent: None,
        owner_node: c.owner_node,
        owner_before_suffix: c.owner_before_suffix,
        object_bounding_box: c.object_bounding_box,
        bounds: c.bounds,
        user_inverse: c.user_inverse,
    })
}
fn invalid() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    }
}
fn limit() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Edges),
        offset: 0,
    }
}
fn arithmetic() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Arithmetic),
        offset: 0,
    }
}
fn oom() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: 0,
    }
}
fn stopped(s: vexel_base::Stop) -> Diagnostic {
    Diagnostic {
        kind: match s {
            vexel_base::Stop::Canceled => ErrorKind::Canceled,
            vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}

pub fn inverse_affine(m: Affine) -> Option<Affine> {
    if !affine_valid(m) {
        return None;
    }
    let det = m.a * m.d - m.b * m.c;
    if !det.is_finite() || det == 0.0 {
        return None;
    }
    let v = Affine {
        a: m.d / det,
        b: -m.b / det,
        c: -m.c / det,
        d: m.a / det,
        e: (m.c * m.f - m.d * m.e) / det,
        f: (m.b * m.e - m.a * m.f) / det,
    };
    affine_valid(v).then_some(v)
}

/// Maps one clip-local point into document coordinates, including the target's
/// object-bounding-box normalization when present.
pub fn local_to_document(view: SceneView<'_>, clip: ClipId, point: Point) -> Option<Point> {
    let c = view.clip(clip)?;
    let mut point = point;
    if c.object_bounding_box {
        let [x0, y0, x1, y1] = c.bounds;
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        point.x = x0 + point.x * (x1 - x0);
        point.y = y0 + point.y * (y1 - y0);
    }
    affine_map_point(inverse_affine(c.user_inverse)?, point)
}

#[derive(Debug, Default)]
pub struct ClipOverlay {
    entries: Vec<(ClipId, Clip)>,
}
impl ClipOverlay {
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn get(&self, id: ClipId) -> Option<&Clip> {
        self.entries
            .iter()
            .find_map(|(key, clip)| (*key == id).then_some(clip))
    }
    pub fn iter(&self) -> impl Iterator<Item = (ClipId, &Clip)> {
        self.entries.iter().map(|(id, clip)| (*id, clip))
    }
    pub fn allocated_bytes(&self) -> Option<usize> {
        let mut n = self
            .entries
            .capacity()
            .checked_mul(std::mem::size_of::<(ClipId, Clip)>())?;
        for (_, c) in &self.entries {
            n = n.checked_add(
                c.geometry
                    .edges
                    .capacity()
                    .checked_mul(std::mem::size_of::<Edge>())?,
            )?;
            n = n.checked_add(
                c.geometry
                    .shapes
                    .capacity()
                    .checked_mul(std::mem::size_of::<ClipChild>())?,
            )?;
        }
        Some(n)
    }
}
/// Copies requested clip graphs into owned, deduplicated Rust storage. IDs are
/// keys only; no pointer/refcount ownership crosses the overlay boundary.
pub fn clone_clips(
    view: SceneView<'_>,
    roots: &[ClipId],
    control: &mut dyn WorkControl,
) -> Result<ClipOverlay> {
    let mut out = ClipOverlay::default();
    let maximum_roots = MAX_CLIP_CLONE_BYTES / std::mem::size_of::<(ClipId, Clip)>();
    if roots.len() > maximum_roots {
        return Err(limit());
    }
    out.entries.try_reserve(roots.len()).map_err(|_| oom())?;
    if out
        .allocated_bytes()
        .is_none_or(|n| n > MAX_CLIP_CLONE_BYTES)
    {
        return Err(limit());
    }
    for root in roots {
        let (mut next, mut depth) = (Some(*root), 0);
        while let Some(id) = next {
            control.checkpoint().map_err(stopped)?;
            if out.entries.iter().any(|(seen, _)| seen == &id) {
                break;
            }
            if depth >= MAX_CLIP_CLONE_DEPTH {
                return Err(limit());
            }
            let c = view.clip(id).ok_or_else(invalid)?;
            let copy_bytes = c
                .geometry
                .edges
                .len()
                .checked_mul(std::mem::size_of::<Edge>())
                .and_then(|n| {
                    c.geometry
                        .shapes
                        .len()
                        .checked_mul(std::mem::size_of::<ClipChild>())
                        .and_then(|m| n.checked_add(m))
                })
                .ok_or_else(arithmetic)?;
            let current_bytes = out.allocated_bytes().ok_or_else(arithmetic)?;
            if current_bytes
                .checked_add(copy_bytes)
                .is_none_or(|n| n > MAX_CLIP_CLONE_BYTES)
            {
                return Err(limit());
            }
            let work = c
                .geometry
                .edges
                .len()
                .checked_add(c.geometry.shapes.len())
                .and_then(|n| n.checked_add(1))
                .ok_or_else(arithmetic)?;
            control
                .charge(
                    WorkKind::TokenVisit,
                    u64::try_from(work).map_err(|_| arithmetic())?,
                )
                .map_err(stopped)?;
            let mut edges = Vec::new();
            edges
                .try_reserve_exact(c.geometry.edges.len())
                .map_err(|_| oom())?;
            edges.extend_from_slice(&c.geometry.edges);
            let mut shapes = Vec::new();
            shapes
                .try_reserve_exact(c.geometry.shapes.len())
                .map_err(|_| oom())?;
            shapes.extend_from_slice(&c.geometry.shapes);
            let clone = Clip {
                geometry: ClipGeometry { edges, shapes },
                parent: c.parent,
                owner_node: c.owner_node,
                owner_before_suffix: c.owner_before_suffix,
                object_bounding_box: c.object_bounding_box,
                bounds: c.bounds,
                user_inverse: c.user_inverse,
            };
            out.entries.try_reserve(1).map_err(|_| oom())?;
            out.entries.push((id, clone));
            if out
                .allocated_bytes()
                .is_none_or(|n| n > MAX_CLIP_CLONE_BYTES)
            {
                return Err(limit());
            }
            next = c.parent;
            depth += 1;
        }
    }
    Ok(out)
}
/// Updates every matching owner on one chain. Bounds change only for valid
/// object-box bounds; an invalid inverse leaves the previous mapping intact.
pub fn update_owner(
    overlay: &mut ClipOverlay,
    id: ClipId,
    owner: TransformId,
    user_inverse: Affine,
    attribute_inverse: Affine,
    bounds: [f64; 4],
) -> Result<()> {
    let mut current = Some(id);
    let mut depth = 0;
    while let Some(key) = current {
        if depth >= MAX_CLIP_CLONE_DEPTH {
            return Err(limit());
        }
        let Some((_, clip)) = overlay.entries.iter_mut().find(|(k, _)| *k == key) else {
            return Err(invalid());
        };
        if clip.owner_node == Some(owner) {
            let candidate = if clip.owner_before_suffix {
                attribute_inverse
            } else {
                user_inverse
            };
            if affine_valid(candidate) {
                clip.user_inverse = candidate;
                if clip.object_bounding_box
                    && bounds.iter().all(|x| x.is_finite())
                    && bounds[2] >= bounds[0]
                    && bounds[3] >= bounds[1]
                {
                    clip.bounds = bounds;
                }
            }
        }
        current = clip.parent;
        depth += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect() -> Vec<Edge> {
        let p = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 10.0 },
            Point { x: 0.0, y: 10.0 },
        ];
        (0..4)
            .map(|i| Edge {
                a: p[i],
                b: p[(i + 1) % 4],
            })
            .collect()
    }
    #[test]
    fn clip_winding_center_inside_and_outside_predicate() {
        let edges = rect();
        assert!(point_inside(
            &edges,
            TableRange { first: 0, count: 4 },
            false,
            5.0,
            5.0
        ));
        assert!(!point_inside(
            &edges,
            TableRange { first: 0, count: 4 },
            false,
            15.0,
            5.0
        ));
    }
    #[test]
    fn evenodd_clip_rule_preserves_holes_and_nonzero_winding() {
        let mut edges = rect();
        edges.extend_from_slice(&[
            Edge {
                a: Point { x: 2.0, y: 2.0 },
                b: Point { x: 8.0, y: 2.0 },
            },
            Edge {
                a: Point { x: 8.0, y: 2.0 },
                b: Point { x: 8.0, y: 8.0 },
            },
            Edge {
                a: Point { x: 8.0, y: 8.0 },
                b: Point { x: 2.0, y: 8.0 },
            },
            Edge {
                a: Point { x: 2.0, y: 8.0 },
                b: Point { x: 2.0, y: 2.0 },
            },
        ]);
        let range = TableRange { first: 0, count: 8 };
        assert!(!point_inside(&edges, range, true, 5.0, 5.0));
        assert!(point_inside(&edges, range, false, 5.0, 5.0));
        assert!(point_inside(&edges, range, true, 1.0, 1.0));
    }
    #[test]
    fn clip_local_bounds_maps_every_endpoint_and_rejects_empty() {
        let edges = rect();
        let inverse = Affine {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 3.0,
            e: 4.0,
            f: -2.0,
        };
        assert_eq!(local_bounds(&edges, inverse), Some([4.0, -2.0, 24.0, 28.0]));
        assert_eq!(local_bounds(&[], inverse), None);
    }
    #[test]
    fn wrapper_admission_retains_extreme_viewbox_budget() {
        assert_eq!(CLIP_WRAPPER_WORST_BYTES, 155);
        assert!(CLIP_WRAPPER_WORST_BYTES < CLIP_WRAPPER_BUFFER_BYTES);
        assert!(CLIP_WRAPPER_BUFFER_BYTES <= CLIP_WRAPPER_LIMIT_BYTES);
    }
}
