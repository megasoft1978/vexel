use crate::types::{invalid, limit, stopped};
use crate::*;
use vexel_geometry::affine_valid;
pub(crate) fn charge(control: &mut dyn WorkControl, kind: WorkKind) -> Result<()> {
    control.charge(kind, 1).map_err(stopped)
}
fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}
fn coord(value: f64) -> bool {
    value.is_finite() && value.abs() <= 1e6
}
fn region(values: &[f64; 4]) -> bool {
    finite(values) && values.iter().all(|v| v.abs() <= 1e6) && values[2] >= 0.0 && values[3] >= 0.0
}
fn span(source: &[u8], value: Span) -> Result<()> {
    if value.slice(source).is_none() {
        Err(invalid(value.start))
    } else {
        Ok(())
    }
}
fn source_offset(source: &[u8], offset: usize) -> Result<()> {
    if offset > source.len() {
        Err(invalid(offset))
    } else {
        Ok(())
    }
}
fn edge(value: Edge) -> Result<()> {
    if [value.a.x, value.a.y, value.b.x, value.b.y]
        .into_iter()
        .all(coord)
    {
        Ok(())
    } else {
        Err(invalid(0))
    }
}
pub(crate) fn viewport(value: Viewport) -> Result<()> {
    if value.width == 0
        || value.height == 0
        || !coord(value.view_x)
        || !coord(value.view_y)
        || !coord(value.view_width)
        || !coord(value.view_height)
        || value.view_width <= 0.0
        || value.view_height <= 0.0
        || !finite(&[value.aspect.align_x, value.aspect.align_y])
        || !(0.0..=1.0).contains(&value.aspect.align_x)
        || !(0.0..=1.0).contains(&value.aspect.align_y)
    {
        Err(Diagnostic {
            kind: ErrorKind::Viewport,
            offset: 0,
        })
    } else {
        Ok(())
    }
}
pub(crate) fn transform(
    builder: &SceneBuilder,
    id: SceneId,
    value: &TransformFrame,
    control: &mut dyn WorkControl,
) -> Result<()> {
    charge(control, WorkKind::Transform)?;
    source_offset(&builder.source, value.source_offset)?;
    if ![value.attribute, value.suffix, value.motion]
        .into_iter()
        .all(affine_valid)
    {
        return Err(invalid(value.source_offset));
    }
    if let Some(parent) = value.parent {
        if builder.traversal_scene(id)?.transform(parent).is_none() {
            return Err(invalid(value.source_offset));
        }
    }
    Ok(())
}
pub(crate) fn stops(values: &[PaintStop], control: &mut dyn WorkControl) -> Result<()> {
    let mut previous = 0.0;
    for stop in values {
        charge(control, WorkKind::PaintVisit)?;
        if !stop.offset.is_finite() || stop.offset < previous || stop.offset > 1.0 {
            return Err(invalid(0));
        }
        previous = stop.offset;
    }
    Ok(())
}
pub(crate) fn paint(
    builder: &SceneBuilder,
    id: SceneId,
    value: &Paint,
    control: &mut dyn WorkControl,
) -> Result<()> {
    paint_view(builder.traversal_scene(id)?, value, control)
}
pub(crate) fn paint_view<V: ViewportRead, A: Copy>(
    scene: SceneView<'_, V, A>,
    value: &Paint,
    control: &mut dyn WorkControl,
) -> Result<()> {
    charge(control, WorkKind::PaintVisit)?;
    value.stops.require(scene.stops.len(), 0)?;
    if value.stops.count > 256 {
        return Err(limit(LimitKind::PaintStops));
    }
    stops(
        value.stops.slice(scene.stops).ok_or_else(|| invalid(0))?,
        control,
    )?;
    if !finite(&value.coordinates)
        || !region(&value.bounds)
        || ![
            value.user_inverse,
            value.gradient_inverse,
            value.pattern_inverse,
        ]
        .into_iter()
        .all(affine_valid)
        || !value.pattern_opacity.is_finite()
        || !(0.0..=1.0).contains(&value.pattern_opacity)
    {
        return Err(invalid(0));
    }
    if value.coordinates.iter().any(|v| v.abs() > 1e6) {
        return Err(invalid(0));
    }
    if value.kind == PaintKind::Radial && value.coordinates[2] < 0.0 {
        return Err(invalid(0));
    }
    if let Some(pattern) = value.pattern {
        if scene.pattern(pattern).is_none() {
            return Err(invalid(0));
        }
    }
    if value.kind == PaintKind::Pattern && value.pattern.is_none() {
        return Err(invalid(0));
    }
    Ok(())
}
pub(crate) fn shape_attributes(
    builder: &SceneBuilder,
    id: SceneId,
    value: &Shape,
    control: &mut dyn WorkControl,
) -> Result<()> {
    shape_attributes_view(builder.traversal_scene(id)?, value, control)
}
pub(crate) fn shape_attributes_view<V: ViewportRead, A: Copy>(
    scene: SceneView<'_, V, A>,
    value: &Shape,
    control: &mut dyn WorkControl,
) -> Result<()> {
    charge(control, WorkKind::TokenVisit)?;
    span(scene.source(), value.source.element)?;
    if !affine_valid(value.source.transform) || !affine_valid(value.stroke_inverse) {
        return Err(invalid(value.source.element.start));
    }
    if let Some(node) = value.source.transform_node {
        if scene.transform(node).is_none() {
            return Err(invalid(0));
        }
    }
    if let Some(clip) = value.clip {
        if scene.clip(clip).is_none() {
            return Err(invalid(0));
        }
    }
    for (index, length) in [
        (value.fill_paint, scene.fill_paints.len()),
        (value.stroke_paint, scene.stroke_paints.len()),
        (value.image, scene.images.len()),
    ] {
        if index.is_some_and(|i| i >= length) {
            return Err(invalid(0));
        }
    }
    let stroke = value.stroke;
    if !stroke.width.is_finite()
        || !(0.0..=1024.0).contains(&stroke.width)
        || !stroke.miter_limit.is_finite()
        || !(1.0..=64.0).contains(&stroke.miter_limit)
        || !coord(stroke.dash_offset)
        || stroke.paint_order > 5
    {
        return Err(invalid(0));
    }
    stroke.dash_values.require(scene.dash_values.len(), 0)?;
    if stroke.dash_values.count > 16 {
        return Err(limit(LimitKind::DashCache));
    }
    Ok(())
}
pub(crate) fn path(
    source: &[u8],
    value: &vexel_svg_path::PathDocument,
    source_range: Span,
    control: &mut dyn WorkControl,
) -> Result<()> {
    for &e in &value.edges {
        charge(control, WorkKind::Edge)?;
        edge(e)?;
    }
    let mut end = 0;
    let mut overlap = false;
    for subpath in &value.subpaths {
        charge(control, WorkKind::TokenVisit)?;
        let range = TableRange {
            first: subpath.first_edge,
            count: subpath.edge_count,
        };
        range.require(value.edges.len(), 0)?;
        overlap |= range.first < end;
        end = range.end().ok_or_else(|| invalid(0))?;
    }
    if overlap {
        crate::path_topology::canonical(
            &value.subpaths,
            TableRange {
                first: 0,
                count: value.edges.len(),
            },
            &value.edges,
            source_range,
            value.segments.iter().copied(),
            control,
        )?;
    }
    for segment in &value.segments {
        charge(control, WorkKind::TokenVisit)?;
        segment_valid(source, *segment, value.edges.len(), value.numbers.len())?;
    }
    for number in &value.numbers {
        charge(control, WorkKind::SourceRead)?;
        span(source, number.source)?;
        if !number.value.is_finite() {
            return Err(invalid(number.source.start));
        }
    }
    Ok(())
}
fn segment_valid(
    source: &[u8],
    value: vexel_svg_path::PathSegment,
    edges: usize,
    numbers: usize,
) -> Result<()> {
    span(source, value.source)?;
    TableRange {
        first: value.first_edge,
        count: value.edge_count,
    }
    .require(edges, value.source.start)?;
    TableRange {
        first: value.first_number,
        count: value.number_count,
    }
    .require(numbers, value.source.start)?;
    let count = match value.command.to_ascii_uppercase() {
        b'M' | b'L' | b'T' => 2,
        b'H' | b'V' => 1,
        b'C' => 6,
        b'S' | b'Q' => 4,
        b'A' => 7,
        b'Z' => 0,
        _ => return Err(invalid(value.source.start)),
    };
    if value.number_count != count
        || (value.command.eq_ignore_ascii_case(&b'M') && value.edge_count != 0)
    {
        return Err(invalid(value.source.start));
    }
    if matches!(
        value.command.to_ascii_uppercase(),
        b'L' | b'H' | b'V' | b'Z'
    ) && value.edge_count > 1
    {
        return Err(invalid(value.source.start));
    }
    if let Some(offset) = value.command_offset {
        if offset < value.source.start
            || offset >= value.source.end
            || source.get(offset).map(u8::to_ascii_uppercase)
                != Some(value.command.to_ascii_uppercase())
        {
            return Err(invalid(offset));
        }
    }
    Ok(())
}
pub(crate) fn image(value: &Image, control: &mut dyn WorkControl) -> Result<()> {
    charge(control, WorkKind::Media)?;
    value.pixels.validate(0)?;
    if value.pixels.width > 16384
        || value.pixels.height > 16384
        || (value.pixels.width as usize) * (value.pixels.height as usize) > 67_108_864
    {
        return Err(limit(LimitKind::Images));
    }
    for row in value.pixels.rgba[..value.pixels.byte_count].chunks_exact(value.pixels.row_stride) {
        for pixel in row[..value.pixels.width as usize * 4].chunks_exact(4) {
            charge(control, WorkKind::Media)?;
            if pixel[..3].iter().any(|v| *v > pixel[3]) {
                return Err(invalid(0));
            }
        }
    }
    if ![
        value.x,
        value.y,
        value.width,
        value.height,
        value.draw_x,
        value.draw_y,
        value.draw_width,
        value.draw_height,
    ]
    .into_iter()
    .all(coord)
        || value.width <= 0.0
        || value.height <= 0.0
        || value.draw_width < 0.0
        || value.draw_height < 0.0
    {
        return Err(invalid(0));
    }
    Ok(())
}
pub(crate) fn clip(
    builder: &SceneBuilder,
    id: SceneId,
    value: &Clip,
    control: &mut dyn WorkControl,
) -> Result<()> {
    clip_view(builder.traversal_scene(id)?, value, control)
}
pub(crate) fn clip_view<V: ViewportRead, A: Copy>(
    scene: SceneView<'_, V, A>,
    value: &Clip,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if !region(&value.bounds) || !affine_valid(value.user_inverse) {
        return Err(invalid(0));
    }
    if value.geometry.edges.len() > 65536 || value.geometry.shapes.len() > 64 {
        return Err(limit(LimitKind::Clips));
    }
    if let Some(node) = value.owner_node {
        if scene.transform(node).is_none() {
            return Err(invalid(0));
        }
    }
    let mut parent = value.parent;
    let mut depth = 1;
    while let Some(id) = parent {
        charge(control, WorkKind::ClipChain)?;
        let clip = scene.clip(id).ok_or_else(|| invalid(0))?;
        parent = clip.parent;
        depth += 1;
        if depth > 256 {
            return Err(limit(LimitKind::Clips));
        }
    }
    for &e in &value.geometry.edges {
        charge(control, WorkKind::ClipEdge)?;
        edge(e)?;
    }
    for child in &value.geometry.shapes {
        charge(control, WorkKind::ClipChild)?;
        child.edges.require(value.geometry.edges.len(), 0)?;
    }
    Ok(())
}
pub(crate) fn pattern(
    _builder: &SceneBuilder,
    _id: SceneId,
    value: &Pattern,
    control: &mut dyn WorkControl,
) -> Result<()> {
    charge(control, WorkKind::PaintVisit)?;
    if (value.tile.width() as usize) * (value.tile.height() as usize) > 1_048_576
        || value.tile.allocated_byte_count() > 16 * 1024 * 1024
    {
        return Err(limit(LimitKind::Patterns));
    }
    if !coord(value.tile_width)
        || !coord(value.tile_height)
        || value.tile_width <= 0.0
        || value.tile_height <= 0.0
        || !affine_valid(value.content_transform)
    {
        return Err(invalid(0));
    }
    Ok(())
}
pub(crate) fn mask(
    builder: &SceneBuilder,
    id: SceneId,
    value: &Mask,
    control: &mut dyn WorkControl,
) -> Result<()> {
    charge(control, WorkKind::ClipChild)?;
    let child = builder.index(value.content)?;
    if value.content == id
        || !builder.scenes[child].sealed
        || !region(&value.region)
        || !affine_valid(value.region_transform)
    {
        return Err(invalid(0));
    }
    source_offset(&builder.source, value.source_offset)
}
pub(crate) fn filter(
    value: Filter,
    primitives: &[FilterPrimitive],
    control: &mut dyn WorkControl,
) -> Result<()> {
    if primitives.len() > 16 {
        return Err(limit(LimitKind::Filters));
    }
    if !region(&value.region) {
        return Err(invalid(0));
    }
    for (index, p) in primitives.iter().enumerate() {
        charge(control, WorkKind::FilterPixel)?;
        if p.merge_count > 8 || !finite(&p.values) {
            return Err(invalid(p.source_offset));
        }
        for input in [p.input, p.input2]
            .into_iter()
            .chain(p.merge_inputs[..p.merge_count as usize].iter().copied())
        {
            if let FilterInput::Result(result) = input {
                if result >= index {
                    return Err(invalid(p.source_offset));
                }
            }
        }
        let mode_cap = match p.kind {
            FilterKind::Blend => 4,
            FilterKind::Composite => 6,
            FilterKind::ColorMatrix => 3,
            _ => 0,
        };
        if p.mode > mode_cap {
            return Err(invalid(p.source_offset));
        }
        if matches!(p.kind, FilterKind::Offset | FilterKind::DropShadow)
            && (p.values[2].abs() > 4096.0 || p.values[3].abs() > 4096.0)
        {
            return Err(invalid(p.source_offset));
        }
        if p.kind == FilterKind::ColorMatrix && p.mode == 1 && !(0.0..=1.0).contains(&p.values[0]) {
            return Err(invalid(p.source_offset));
        }
        if matches!(p.kind, FilterKind::GaussianBlur | FilterKind::DropShadow)
            && (!(0.0..=32.0).contains(&p.values[0]) || !(0.0..=32.0).contains(&p.values[1]))
        {
            return Err(invalid(p.source_offset));
        }
    }
    Ok(())
}
pub(crate) fn group(
    builder: &SceneBuilder,
    id: SceneId,
    value: &OpacityGroup,
    index: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let scene = builder.traversal_scene(id)?;
    value.shapes.require(scene.shapes.len(), 0)?;
    if !value.opacity.is_finite() || !(0.0..=1.0).contains(&value.opacity) || value.blend_mode > 4 {
        return Err(invalid(0));
    }
    if let Some(filter) = value.filter {
        if scene.filter(filter).is_none() {
            return Err(invalid(0));
        }
    }
    if let Some(mask) = value.mask {
        if scene.mask(mask).is_none() {
            return Err(invalid(0));
        }
    }
    let mut parent = value.parent;
    let mut depth = 1;
    while let Some(id) = parent {
        charge(control, WorkKind::ClipChain)?;
        let group = scene.group(id).ok_or_else(|| invalid(0))?;
        if value.shapes.first < group.shapes.first || value.shapes.end() > group.shapes.end() {
            return Err(invalid(0));
        }
        parent = group.parent;
        depth += 1;
        if depth > 64 {
            return Err(limit(LimitKind::Depth));
        }
    }
    group_order(scene, value, index, control)
}
/// Groups are depth-first painter ranges. The previous group's ancestry is a
/// bounded stack encoded by parent handles; no quadratic sibling search.
fn group_order<V: ViewportRead, A: Copy>(
    scene: SceneView<'_, V, A>,
    value: &OpacityGroup,
    index: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if index == 0 {
        return if value.parent.is_none() {
            Ok(())
        } else {
            Err(invalid(0))
        };
    }
    let mut cursor = index - 1;
    for _ in 0..64 {
        charge(control, WorkKind::ClipChain)?;
        if value.parent.is_some_and(|parent| parent.index() == cursor) {
            return Ok(());
        }
        let previous = scene.groups.get(cursor).ok_or_else(|| invalid(0))?;
        if previous.parent == value.parent {
            return if value.shapes.first >= previous.shapes.end().ok_or_else(|| invalid(0))? {
                Ok(())
            } else {
                Err(invalid(0))
            };
        }
        let parent = previous.parent.ok_or_else(|| invalid(0))?;
        if parent.index() >= cursor || scene.group(parent).is_none() {
            return Err(invalid(0));
        }
        cursor = parent.index();
    }
    Err(limit(LimitKind::Depth))
}
pub(crate) fn scene(
    builder: &SceneBuilder,
    id: SceneId,
    control: &mut dyn WorkControl,
) -> Result<()> {
    control.checkpoint().map_err(stopped)?;
    let i = builder.index(id)?;
    let data = &builder.scenes[i];
    data.validate_publications(control)?;
    let view = builder.traversal_scene(id)?;
    if let Some(value) = view.viewport { viewport(value)?; }
    else if id != builder.root_id() { return Err(invalid(0)); }
    if view.edges.len() != view.edge_colors.len() || view.edges.len() != view.edge_shape_ids.len() {
        return Err(invalid(0));
    }
    for (records, serials) in [
        (data.shapes.len(), data.shapes_serials.len()),
        (data.transforms.len(), data.transforms_serials.len()),
        (data.clips.len(), data.clips_serials.len()),
        (data.patterns.len(), data.patterns_serials.len()),
        (data.masks.len(), data.masks_serials.len()),
        (data.filters.len(), data.filters_serials.len()),
        (data.groups.len(), data.groups_serials.len()),
    ] {
        if records != serials {
            return Err(invalid(0));
        }
    }
    for (index, &e) in view.edges.iter().enumerate() {
        charge(control, WorkKind::Edge)?;
        edge(e)?;
        let shape = view
            .edge_shape(index)
            .and_then(|id| view.shape(id))
            .ok_or_else(|| invalid(0))?;
        if index < shape.edges.first || index >= shape.edges.end().ok_or_else(|| invalid(0))? {
            return Err(invalid(0));
        }
    }
    for (shape_index, shape) in view.shapes.iter().enumerate() {
        shape_attributes(builder, id, shape, control)?;
        shape.edges.require(view.edges.len(), 0)?;
        shape.subpaths.require(view.path_subpaths.len(), 0)?;
        let mut end = shape.edges.first;
        let mut overlap = false;
        for subpath in shape
            .subpaths
            .slice(view.path_subpaths)
            .ok_or_else(|| invalid(0))?
        {
            charge(control, WorkKind::TokenVisit)?;
            let range = TableRange {
                first: subpath.first_edge,
                count: subpath.edge_count,
            };
            range.require(view.edges.len(), 0)?;
            if range.first < shape.edges.first || range.end() > shape.edges.end() {
                return Err(invalid(0));
            }
            overlap |= range.first < end;
            end = range.end().ok_or_else(|| invalid(0))?;
        }
        if overlap {
            // Preserve each original shape association; foreign segments cannot prove overlap.
            let paths = shape
                .subpaths
                .slice(view.path_subpaths)
                .ok_or_else(|| invalid(0))?;
            let segments =
                crate::path_topology::scene_segments(view.path_segments, shape_index, control)?;
            crate::path_topology::canonical(
                paths,
                shape.edges,
                view.edges,
                shape.source.element,
                segments.iter().map(|s| s.path),
                control,
            )?;
        }
    }
    let mut previous_shape = 0;
    for segment in view.path_segments {
        charge(control, WorkKind::TokenVisit)?;
        if segment.shape.index() < previous_shape {
            return Err(invalid(0));
        }
        previous_shape = segment.shape.index();
        segment_valid(
            &builder.source,
            segment.path,
            view.edges.len(),
            view.path_numbers.len(),
        )?;
        let shape = view.shape(segment.shape).ok_or_else(|| invalid(0))?;
        let range = TableRange {
            first: segment.path.first_edge,
            count: segment.path.edge_count,
        };
        if range.first < shape.edges.first || range.end() > shape.edges.end() {
            return Err(invalid(0));
        }
    }
    for number in view.path_numbers {
        charge(control, WorkKind::SourceRead)?;
        span(&builder.source, number.source)?;
        if !number.value.is_finite() {
            return Err(invalid(number.source.start));
        }
    }
    for (index, value) in view.transforms.iter().enumerate() {
        transform(builder, id, value, control)?;
        if value.parent.is_some_and(|p| p.index() >= index) {
            return Err(invalid(value.source_offset));
        }
    }
    let mut pixels = 0usize;
    for value in view.images {
        image(value, control)?;
        pixels = crate::memory::add(
            pixels,
            (value.pixels.width as usize) * (value.pixels.height as usize),
        )?;
        if pixels > 67_108_864 {
            return Err(limit(LimitKind::Images));
        }
    }
    if pixels != data.image_pixels {
        return Err(invalid(0));
    }
    for (index, value) in view.clips.iter().enumerate() {
        clip(builder, id, value, control)?;
        if value.parent.is_some_and(|p| p.index() >= index) {
            return Err(invalid(0));
        }
    }
    for value in view.patterns {
        pattern(builder, id, value, control)?;
    }
    for value in view.masks {
        mask(builder, id, value, control)?;
    }
    for value in view.fill_paints.iter().chain(view.stroke_paints) {
        paint(builder, id, value, control)?;
    }
    for value in view.filters {
        charge(control, WorkKind::FilterPixel)?;
        value.primitives.require(view.filter_primitives.len(), 0)?;
        let primitives = value
            .primitives
            .slice(view.filter_primitives)
            .ok_or_else(|| invalid(0))?;
        filter(*value, primitives, control)?;
        for primitive in primitives {
            charge(control, WorkKind::SourceRead)?;
            source_offset(&builder.source, primitive.source_offset)?;
        }
    }
    for (index, value) in view.groups.iter().enumerate() {
        group(builder, id, value, index, control)?;
        if value.parent.is_some_and(|p| p.index() >= index) {
            return Err(invalid(0));
        }
    }
    for value in view.dash_values {
        charge(control, WorkKind::StrokeTest)?;
        if !coord(*value) || *value < 0.0 {
            return Err(invalid(0));
        }
    }
    // Unreferenced stops and primitives are still retained user input and must validate.
    for value in view.stops {
        charge(control, WorkKind::PaintVisit)?;
        if !value.offset.is_finite() || !(0.0..=1.0).contains(&value.offset) {
            return Err(invalid(0));
        }
    }
    Ok(())
}
