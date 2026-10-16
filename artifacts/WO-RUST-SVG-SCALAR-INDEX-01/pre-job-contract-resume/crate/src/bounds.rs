use crate::util::*;
use vexel_geometry::Point;
use vexel_scene::{
    BoundaryMask, BoundsSet, ErrorKind, KernelResources, LimitKind, Linecap, Linejoin, PixelBounds,
    Result, SceneView, ShapeBounds, ViewMapping, WorkControl, WorkKind,
};

#[derive(Clone, Copy, Default)]
struct GeomBounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    present: bool,
    invalid: bool,
}

impl GeomBounds {
    fn include(&mut self, p: Point) {
        if !p.x.is_finite() || !p.y.is_finite() {
            self.invalid = true;
            return;
        }
        if !self.present {
            self.left = p.x;
            self.right = p.x;
            self.top = p.y;
            self.bottom = p.y;
            self.present = true;
        } else {
            self.left = self.left.min(p.x);
            self.right = self.right.max(p.x);
            self.top = self.top.min(p.y);
            self.bottom = self.bottom.max(p.y);
        }
    }
    fn merge(&mut self, other: Self) {
        if other.present {
            if !self.present {
                *self = other;
            } else {
                self.left = self.left.min(other.left);
                self.right = self.right.max(other.right);
                self.top = self.top.min(other.top);
                self.bottom = self.bottom.max(other.bottom);
                self.invalid |= other.invalid;
            }
        }
        self.invalid |= other.invalid;
    }
}

fn validate_map(map: ViewMapping) -> Result<()> {
    if !map.scale_x.is_finite()
        || map.scale_x <= 0.0
        || !map.scale_y.is_finite()
        || map.scale_y <= 0.0
        || !map.offset_x.is_finite()
        || !map.offset_y.is_finite()
    {
        return Err(invalid(0));
    }
    Ok(())
}

fn full_tile(view: SceneView<'_>) -> ShapeBounds {
    ShapeBounds {
        device_left: 0.0,
        device_top: 0.0,
        device_right: view.viewport.width as f64,
        device_bottom: view.viewport.height as f64,
        pixels: PixelBounds {
            x_first: 0,
            x_last: view.viewport.width.saturating_sub(1),
            y_first: 0,
            y_last: view.viewport.height.saturating_sub(1),
            visible: true,
        },
    }
}

fn stroke_padding(view: SceneView<'_>, shape: usize) -> Option<(f64, f64)> {
    let stroke = &view.shapes.get(shape)?.stroke;
    if stroke.rgba[3] == 0 || stroke.width <= 0.0 {
        return Some((0.0, 0.0));
    }
    let inv = view.shapes[shape].stroke_inverse;
    let det = inv.a * inv.d - inv.b * inv.c;
    let a = inv.d / det;
    let b = -inv.c / det;
    let c = -inv.b / det;
    let d = inv.a / det;
    let radius = stroke.width * 0.5;
    let join = if stroke.join == Linejoin::Miter {
        stroke.miter_limit
    } else {
        1.0
    };
    let cap = if !view.shapes[shape].stroke_closed && stroke.cap == Linecap::Square {
        std::f64::consts::SQRT_2
    } else {
        1.0
    };
    let extent = radius * join.max(cap);
    let x = extent * (a.hypot(b));
    let y = extent * (c.hypot(d));
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

fn map_bounds(view: SceneView<'_>, map: ViewMapping, b: GeomBounds) -> ShapeBounds {
    if !b.present && !b.invalid {
        return ShapeBounds::default();
    }
    if b.invalid {
        return full_tile(view);
    }
    let left = map.offset_x + (b.left - view.viewport.view_x) * map.scale_x;
    let right = map.offset_x + (b.right - view.viewport.view_x) * map.scale_x;
    let top = map.offset_y + (b.top - view.viewport.view_y) * map.scale_y;
    let bottom = map.offset_y + (b.bottom - view.viewport.view_y) * map.scale_y;
    if !left.is_finite() || !right.is_finite() || !top.is_finite() || !bottom.is_finite() {
        return full_tile(view);
    }
    let ex = left.abs().max(right.abs()).max(1.0) * f64::EPSILON * 8.0;
    let ey = top.abs().max(bottom.abs()).max(1.0) * f64::EPSILON * 8.0;
    let (left, right, top, bottom) = (left - ex, right + ex, top - ey, bottom + ey);
    let xr = pixel_range(left, right, view.viewport.width);
    let yr = pixel_range(top, bottom, view.viewport.height);
    let pixels = match (xr, yr) {
        (Some((x_first, x_last)), Some((y_first, y_last))) => PixelBounds {
            x_first,
            x_last,
            y_first,
            y_last,
            visible: true,
        },
        _ => PixelBounds::default(),
    };
    ShapeBounds {
        device_left: left,
        device_top: top,
        device_right: right,
        device_bottom: bottom,
        pixels,
    }
}

/// Build conservative per-shape pixel bounds from all filled and stroked scalar edges.
pub fn build_bounds(
    view: SceneView<'_>,
    map: ViewMapping,
    _resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<BoundsSet> {
    validate_map(map)?;
    if view.viewport.width == 0 || view.viewport.height == 0 {
        return Err(invalid(0));
    }
    let mut fills = Vec::new();
    let mut strokes = Vec::new();
    let mut shapes = Vec::new();
    let mut pixel_count = 0usize;
    reserve(&mut fills, view.shapes.len())?;
    reserve(&mut strokes, view.shapes.len())?;
    reserve(&mut shapes, view.shapes.len())?;
    fills.resize(view.shapes.len(), GeomBounds::default());
    strokes.resize(view.shapes.len(), GeomBounds::default());
    for (i, edge) in view.edges.iter().enumerate() {
        poll(control, WorkKind::Edge)?;
        if let Some(id) = view.edge_shape(i) {
            let index = id.index();
            if view.shapes.get(index).is_some_and(|s| s.filled) {
                fills[index].include(edge.a);
                fills[index].include(edge.b);
            }
        }
    }
    for (i, shape) in view.shapes.iter().enumerate() {
        poll(control, WorkKind::ShapeRowVisit)?;
        let Some(edges) = shape.edges.slice(view.edges) else {
            return Err(invalid(shape.source.element.start));
        };
        if shape.stroke.rgba[3] != 0 && shape.stroke.width > 0.0 && !edges.is_empty() {
            for edge in edges {
                poll(control, WorkKind::Edge)?;
                strokes[i].include(edge.a);
                strokes[i].include(edge.b);
            }
            match stroke_padding(view, i) {
                Some((px, py)) if strokes[i].present && !strokes[i].invalid => {
                    strokes[i].left -= px;
                    strokes[i].right += px;
                    strokes[i].top -= py;
                    strokes[i].bottom += py;
                }
                _ => strokes[i].invalid = true,
            }
        }
        fills[i].merge(strokes[i]);
        let b = map_bounds(view, map, fills[i]);
        if b.pixels.visible {
            let w = (b.pixels.x_last - b.pixels.x_first) as usize + 1;
            let h = (b.pixels.y_last - b.pixels.y_first) as usize + 1;
            pixel_count = add(pixel_count, mul(w, h)?)?;
        }
        shapes.push(b);
    }
    Ok(BoundsSet {
        shapes,
        pixel_count,
    })
}

fn checked_area(b: PixelBounds) -> Result<usize> {
    if !b.visible {
        return Ok(0);
    }
    mul(
        (b.x_last - b.x_first) as usize + 1,
        (b.y_last - b.y_first) as usize + 1,
    )
}

fn add_work(work: &mut usize, amount: usize) -> Result<()> {
    *work = add(*work, amount)?;
    if *work > BOUNDARY_WORK_LIMIT {
        return Err(limit(LimitKind::Work));
    }
    Ok(())
}

fn mark_box(
    mask: &mut BoundaryMask,
    b: PixelBounds,
    width: u32,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if !b.visible {
        return Ok(());
    }
    for y in b.y_first..=b.y_last {
        for x in b.x_first..=b.x_last {
            poll(control, WorkKind::BoundaryMark)?;
            let index = (y as usize)
                .checked_mul(width as usize)
                .and_then(|v| v.checked_add(x as usize))
                .ok_or_else(|| limit(LimitKind::Arithmetic))?;
            mark_pixel(mask, index);
        }
    }
    Ok(())
}

fn mark_pixel(mask: &mut BoundaryMask, index: usize) {
    let bit = 1u8 << (index & 7);
    let slot = &mut mask.bits[index >> 3];
    if *slot & bit == 0 {
        mask.marked_pixel_count += 1;
    }
    *slot |= bit;
}

fn publish_boundary_additions(
    boundary: &mut BoundaryMask,
    additions: &BoundaryMask,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut merged = Vec::new();
    reserve(&mut merged, boundary.bits.len())?;
    merged.extend_from_slice(&boundary.bits);
    let mut marked = boundary.marked_pixel_count;
    for (i, byte) in merged.iter_mut().enumerate() {
        if i & 255 == 0 {
            poll(control, WorkKind::BoundaryMark)?;
        }
        let extra = additions.bits.get(i).copied().unwrap_or(0) & !*byte;
        marked = add(marked, extra.count_ones() as usize)?;
        *byte |= additions.bits.get(i).copied().unwrap_or(0);
    }
    boundary.bits = merged;
    boundary.marked_pixel_count = marked;
    Ok(())
}

fn full_boundary_additions(pixels: usize, control: &mut dyn WorkControl) -> Result<BoundaryMask> {
    let bytes = add(pixels, 7)? / 8;
    let mut mask = BoundaryMask::default();
    reserve(&mut mask.bits, bytes)?;
    mask.bits.resize(bytes, 0);
    mask.bit_count = pixels;
    for i in 0..pixels {
        if i & 255 == 0 {
            poll(control, WorkKind::BoundaryMark)?;
        }
        mark_pixel(&mut mask, i);
    }
    Ok(mask)
}

fn edge_radius(view: SceneView<'_>, shape: usize, miter: bool) -> Option<f64> {
    let s = &view.shapes[shape];
    let stroke = &s.stroke;
    if stroke.rgba[3] == 0 || stroke.width <= 0.0 {
        return Some(0.0);
    }
    let inv = s.stroke_inverse;
    let det = inv.a * inv.d - inv.b * inv.c;
    let a = inv.d / det;
    let b = -inv.c / det;
    let c = -inv.b / det;
    let d = inv.a / det;
    let scale = (a * a + b * b + c * c + d * d).sqrt();
    let join = if miter && stroke.join == Linejoin::Miter {
        stroke.miter_limit
    } else {
        1.0
    };
    let cap = if !s.stroke_closed && stroke.cap == Linecap::Square {
        std::f64::consts::SQRT_2
    } else {
        1.0
    };
    let r = stroke.width * 0.5 * scale * join.max(cap);
    r.is_finite().then_some(r)
}

fn point_box(view: SceneView<'_>, map: ViewMapping, p: Point, r: f64) -> Option<PixelBounds> {
    let x = map.offset_x + (p.x - view.viewport.view_x) * map.scale_x;
    let y = map.offset_y + (p.y - view.viewport.view_y) * map.scale_y;
    let px = r * map.scale_x;
    let py = r * map.scale_y;
    let eps = x.abs().max(y.abs()).max(1.0) * f64::EPSILON * 8.0;
    if ![x, y, px, py, eps].iter().all(|v| v.is_finite()) {
        return None;
    }
    match (
        boundary_pixel_range(x - px - eps, x + px + eps, view.viewport.width),
        boundary_pixel_range(y - py - eps, y + py + eps, view.viewport.height),
    ) {
        (Some((x_first, x_last)), Some((y_first, y_last))) => Some(PixelBounds {
            x_first,
            x_last,
            y_first,
            y_last,
            visible: true,
        }),
        _ => Some(PixelBounds::default()),
    }
}

fn edge_box(
    view: SceneView<'_>,
    map: ViewMapping,
    edge: vexel_geometry::Edge,
    r: f64,
) -> PixelBounds {
    let x1 = map.offset_x + (edge.a.x - view.viewport.view_x) * map.scale_x;
    let x2 = map.offset_x + (edge.b.x - view.viewport.view_x) * map.scale_x;
    let y1 = map.offset_y + (edge.a.y - view.viewport.view_y) * map.scale_y;
    let y2 = map.offset_y + (edge.b.y - view.viewport.view_y) * map.scale_y;
    let px = r * map.scale_x;
    let py = r * map.scale_y;
    let eps = x1.abs().max(x2.abs()).max(y1.abs()).max(y2.abs()).max(1.0) * f64::EPSILON * 8.0;
    if ![x1, x2, y1, y2, px, py, eps].iter().all(|v| v.is_finite()) {
        return PixelBounds {
            x_first: 0,
            x_last: view.viewport.width - 1,
            y_first: 0,
            y_last: view.viewport.height - 1,
            visible: true,
        };
    }
    match (
        boundary_pixel_range(
            x1.min(x2) - px - eps,
            x1.max(x2) + px + eps,
            view.viewport.width,
        ),
        boundary_pixel_range(
            y1.min(y2) - py - eps,
            y1.max(y2) + py + eps,
            view.viewport.height,
        ),
    ) {
        (Some((x_first, x_last)), Some((y_first, y_last))) => PixelBounds {
            x_first,
            x_last,
            y_first,
            y_last,
            visible: true,
        },
        _ => PixelBounds::default(),
    }
}

/// Mark scalar edge neighborhoods and conservative miter join neighborhoods.
pub fn mark_boundaries(
    view: SceneView<'_>,
    map: ViewMapping,
    _resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<BoundaryMask> {
    validate_map(map)?;
    let pixels = mul(view.viewport.width as usize, view.viewport.height as usize)?;
    let bytes = add(pixels, 7)? / 8;
    let mut mask = BoundaryMask::default();
    reserve(&mut mask.bits, bytes)?;
    mask.bits.resize(bytes, 0);
    mask.bit_count = pixels;
    let mut boxes = Vec::new();
    reserve(&mut boxes, view.edges.len())?;
    let mut work = 0;
    for (i, edge) in view.edges.iter().enumerate() {
        poll(control, WorkKind::Edge)?;
        let shape = view.edge_shape(i).ok_or_else(|| invalid(0))?.index();
        let radius = edge_radius(view, shape, false).unwrap_or(f64::NAN);
        let b = if radius.is_finite() {
            edge_box(view, map, *edge, radius)
        } else {
            PixelBounds {
                x_first: 0,
                x_last: view.viewport.width - 1,
                y_first: 0,
                y_last: view.viewport.height - 1,
                visible: true,
            }
        };
        add_work(&mut work, checked_area(b)?)?;
        boxes.push(b);
        let s = &view.shapes[shape];
        if s.stroke.rgba[3] != 0 && s.stroke.width > 0.0 && s.stroke.join == Linejoin::Miter {
            let jr = edge_radius(view, shape, true).unwrap_or(f64::NAN);
            for p in [edge.a, edge.b] {
                poll(control, WorkKind::BoundaryMark)?;
                let jb = point_box(view, map, p, jr).unwrap_or(PixelBounds {
                    x_first: 0,
                    x_last: view.viewport.width - 1,
                    y_first: 0,
                    y_last: view.viewport.height - 1,
                    visible: true,
                });
                add_work(&mut work, checked_area(jb)?)?;
            }
        }
    }
    for (i, edge) in view.edges.iter().enumerate() {
        poll(control, WorkKind::Edge)?;
        mark_box(&mut mask, boxes[i], view.viewport.width, control)?;
        let shape = view.edge_shape(i).ok_or_else(|| invalid(0))?.index();
        let s = &view.shapes[shape];
        if s.stroke.rgba[3] != 0 && s.stroke.width > 0.0 && s.stroke.join == Linejoin::Miter {
            let r = edge_radius(view, shape, true).unwrap_or(f64::NAN);
            for p in [edge.a, edge.b] {
                poll(control, WorkKind::BoundaryMark)?;
                let b = point_box(view, map, p, r).unwrap_or(PixelBounds {
                    x_first: 0,
                    x_last: view.viewport.width - 1,
                    y_first: 0,
                    y_last: view.viewport.height - 1,
                    visible: true,
                });
                mark_box(&mut mask, b, view.viewport.width, control)?;
            }
        }
    }
    mask.mark_work = work;
    Ok(mask)
}

/// Add clip boundary neighborhoods; invalid inverse mappings mark the full mask.
pub fn mark_clip_boundaries(
    view: SceneView<'_>,
    map: ViewMapping,
    boundary: &mut BoundaryMask,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<()> {
    validate_map(map)?;
    let pixels = mul(view.viewport.width as usize, view.viewport.height as usize)?;
    let mask_bytes = add(boundary.bit_count, 7)? / 8;
    if boundary.bit_count != pixels || boundary.bits.len() < mask_bytes {
        return Err(invalid(0));
    }
    let mut total = boundary.mark_work;
    add_work(&mut total, mul(view.shapes.len(), 3)?)?;
    let mut clips = Vec::new();
    reserve(&mut clips, view.shapes.len())?;
    for shape in view.shapes {
        poll(control, WorkKind::ShapeRowVisit)?;
        if let Some(id) = shape.clip {
            clips.push(id);
        }
    }
    if clips.is_empty() {
        return Ok(());
    }
    let mut clip_box_counts = Vec::new();
    reserve(&mut clip_box_counts, clips.len())?;
    for clip in &clips {
        let c = resources.clip_work_counts(view, *clip, control)?;
        let visits = add(c.chain_elements, c.child_shapes)?;
        add_work(&mut total, add(c.chain_elements, mul(visits, 2)?)?)?;
    }
    let mut force_full = false;
    let mut all_boxes = Vec::new();
    // First traversal preflights edge neighborhoods; the resource traversal remains real and controlled.
    for &clip in &clips {
        let box_start = all_boxes.len();
        let mut edge_work = 0usize;
        let mut invalid_edge = false;
        let mut allocation_failed = false;
        let mut arithmetic_failed = false;
        let mut visitor = |edge: vexel_geometry::Edge| {
            let valid = [edge.a.x, edge.a.y, edge.b.x, edge.b.y]
                .iter()
                .all(|v| v.is_finite());
            if !valid {
                invalid_edge = true;
                edge_work = match edge_work.checked_add(2) {
                    Some(n) => n,
                    None => {
                        arithmetic_failed = true;
                        return false;
                    }
                };
                return true;
            }
            let x1 = map.offset_x + (edge.a.x - view.viewport.view_x) * map.scale_x;
            let x2 = map.offset_x + (edge.b.x - view.viewport.view_x) * map.scale_x;
            let y1 = map.offset_y + (edge.a.y - view.viewport.view_y) * map.scale_y;
            let y2 = map.offset_y + (edge.b.y - view.viewport.view_y) * map.scale_y;
            if ![x1, x2, y1, y2].iter().all(|v| v.is_finite()) {
                invalid_edge = true;
                edge_work = match edge_work.checked_add(2) {
                    Some(n) => n,
                    None => {
                        arithmetic_failed = true;
                        return false;
                    }
                };
                return true;
            }
            let eps =
                x1.abs().max(x2.abs()).max(y1.abs()).max(y2.abs()).max(1.0) * f64::EPSILON * 8.0;
            let xr = boundary_pixel_range(x1.min(x2) - eps, x1.max(x2) + eps, view.viewport.width);
            let yr = boundary_pixel_range(y1.min(y2) - eps, y1.max(y2) + eps, view.viewport.height);
            let b = match (xr, yr) {
                (Some((x_first, x_last)), Some((y_first, y_last))) => PixelBounds {
                    x_first,
                    x_last,
                    y_first,
                    y_last,
                    visible: true,
                },
                _ => PixelBounds::default(),
            };
            if all_boxes.try_reserve(1).is_err() {
                allocation_failed = true;
                return false;
            }
            all_boxes.push(b);
            match checked_area(b).and_then(|a| add(2, a)) {
                Ok(a) => match edge_work.checked_add(a) {
                    Some(n) => {
                        edge_work = n;
                        true
                    }
                    None => {
                        arithmetic_failed = true;
                        false
                    }
                },
                Err(_) => {
                    arithmetic_failed = true;
                    false
                }
            }
        };
        let valid = resources.clip_boundary_edges(view, clip, &mut visitor, control)?;
        if allocation_failed {
            return Err(oom());
        }
        if arithmetic_failed {
            return Err(limit(LimitKind::Arithmetic));
        }
        total = add(total, edge_work)?;
        if total > BOUNDARY_WORK_LIMIT {
            return Err(limit(LimitKind::Work));
        }
        if !valid || invalid_edge {
            force_full = true;
            break;
        }
        clip_box_counts.push(
            all_boxes
                .len()
                .checked_sub(box_start)
                .ok_or_else(|| limit(LimitKind::Arithmetic))?,
        );
    }
    if force_full {
        add_work(&mut total, pixels)?;
        let additions = full_boundary_additions(pixels, control)?;
        publish_boundary_additions(boundary, &additions, control)?;
        boundary.mark_work = total;
        return Ok(());
    }
    // The second controlled traversal validates the same real clip geometry before publishing marks.
    for &clip in &clips {
        let mut invalid_edge = false;
        let mut visitor = |edge: vexel_geometry::Edge| {
            let mapped = [edge.a.x, edge.a.y, edge.b.x, edge.b.y]
                .iter()
                .all(|v| v.is_finite())
                .then(|| {
                    [
                        map.offset_x + (edge.a.x - view.viewport.view_x) * map.scale_x,
                        map.offset_x + (edge.b.x - view.viewport.view_x) * map.scale_x,
                        map.offset_y + (edge.a.y - view.viewport.view_y) * map.scale_y,
                        map.offset_y + (edge.b.y - view.viewport.view_y) * map.scale_y,
                    ]
                });
            if mapped.is_none_or(|p| !p.iter().all(|v| v.is_finite())) {
                invalid_edge = true;
            }
            true
        };
        let valid = resources.clip_boundary_edges(view, clip, &mut visitor, control)?;
        if !valid || invalid_edge {
            force_full = true;
            break;
        }
    }
    if force_full {
        add_work(&mut total, pixels)?;
        let additions = full_boundary_additions(pixels, control)?;
        publish_boundary_additions(boundary, &additions, control)?;
        boundary.mark_work = total;
        return Ok(());
    }
    let mut additions = BoundaryMask::default();
    reserve(&mut additions.bits, mask_bytes)?;
    additions.bits.resize(mask_bytes, 0);
    additions.bit_count = pixels;
    let mut cursor = 0usize;
    for &edge_count in &clip_box_counts {
        let end = add(cursor, edge_count)?;
        if end > all_boxes.len() {
            return Err(invalid(0));
        }
        for b in &all_boxes[cursor..end] {
            mark_box(&mut additions, *b, view.viewport.width, control)?;
        }
        cursor = end;
    }
    if cursor != all_boxes.len() {
        return Err(invalid(0));
    }
    publish_boundary_additions(boundary, &additions, control)?;
    boundary.mark_work = total;
    control.checkpoint().map_err(|s| match s {
        vexel_scene::Stop::Canceled => vexel_scene::Diagnostic {
            kind: ErrorKind::Canceled,
            offset: 0,
        },
        vexel_scene::Stop::Limit => limit(LimitKind::Work),
        vexel_scene::Stop::Overflow => limit(LimitKind::Arithmetic),
    })?;
    Ok(())
}

/// Out-of-range masks are conservatively treated as boundary pixels.
pub fn boundary_test(mask: &BoundaryMask, pixel: usize) -> bool {
    if pixel >= mask.bit_count || pixel >> 3 >= mask.bits.len() {
        true
    } else {
        mask.bits[pixel >> 3] & (1 << (pixel & 7)) != 0
    }
}
