use crate::memory::{add, bytes};
use crate::*;
/// Mutable storage is private to the builder. Native objects never enter this graph.
#[derive(Debug)]
pub(crate) struct SceneData {
    pub(crate) publications: Vec<crate::paint_publication::Publication>,
    pub(crate) image_pixels: usize,
    pub(crate) serial: u64,
    pub(crate) sealed: bool,
    pub(crate) epoch: u64,
    pub(crate) viewport: Viewport,
    pub(crate) edges: Vec<Edge>,
    pub(crate) edge_colors: Vec<[u8; 4]>,
    pub(crate) edge_shape_ids: Vec<u32>,
    pub(crate) shapes: Vec<Shape>,
    pub(crate) path_subpaths: Vec<vexel_svg_path::PathSubpath>,
    pub(crate) path_segments: Vec<ScenePathSegment>,
    pub(crate) path_numbers: Vec<vexel_svg_path::PathNumber>,
    pub(crate) transforms: Vec<TransformFrame>,
    pub(crate) images: Vec<Image>,
    pub(crate) groups: Vec<OpacityGroup>,
    pub(crate) filters: Vec<Filter>,
    pub(crate) filter_primitives: Vec<FilterPrimitive>,
    pub(crate) masks: Vec<Mask>,
    pub(crate) patterns: Vec<Pattern>,
    pub(crate) fill_paints: Vec<Paint>,
    pub(crate) stroke_paints: Vec<Paint>,
    pub(crate) dash_values: Vec<f64>,
    pub(crate) stops: Vec<PaintStop>,
    pub(crate) clips: Vec<Clip>,
    pub(crate) shapes_serials: Vec<u64>,
    pub(crate) transforms_serials: Vec<u64>,
    pub(crate) clips_serials: Vec<u64>,
    pub(crate) patterns_serials: Vec<u64>,
    pub(crate) masks_serials: Vec<u64>,
    pub(crate) filters_serials: Vec<u64>,
    pub(crate) groups_serials: Vec<u64>,
}
impl SceneData {
    pub(crate) fn new(serial: u64, viewport: Viewport) -> Self {
        Self {
            publications: Vec::new(),
            image_pixels: 0,
            serial,
            sealed: false,
            epoch: 0,
            viewport,
            edges: Vec::new(),
            edge_colors: Vec::new(),
            edge_shape_ids: Vec::new(),
            shapes: Vec::new(),
            path_subpaths: Vec::new(),
            path_segments: Vec::new(),
            path_numbers: Vec::new(),
            transforms: Vec::new(),
            images: Vec::new(),
            groups: Vec::new(),
            filters: Vec::new(),
            filter_primitives: Vec::new(),
            masks: Vec::new(),
            patterns: Vec::new(),
            fill_paints: Vec::new(),
            stroke_paints: Vec::new(),
            dash_values: Vec::new(),
            stops: Vec::new(),
            clips: Vec::new(),
            shapes_serials: Vec::new(),
            transforms_serials: Vec::new(),
            clips_serials: Vec::new(),
            patterns_serials: Vec::new(),
            masks_serials: Vec::new(),
            filters_serials: Vec::new(),
            groups_serials: Vec::new(),
        }
    }
    pub(crate) fn lengths(&self) -> [usize; 26] {
        [
            self.edges.len(),
            self.edge_colors.len(),
            self.edge_shape_ids.len(),
            self.shapes.len(),
            self.path_subpaths.len(),
            self.path_segments.len(),
            self.path_numbers.len(),
            self.transforms.len(),
            self.images.len(),
            self.groups.len(),
            self.filters.len(),
            self.filter_primitives.len(),
            self.masks.len(),
            self.patterns.len(),
            self.fill_paints.len(),
            self.stroke_paints.len(),
            self.dash_values.len(),
            self.stops.len(),
            self.clips.len(),
            self.shapes_serials.len(),
            self.transforms_serials.len(),
            self.clips_serials.len(),
            self.patterns_serials.len(),
            self.masks_serials.len(),
            self.filters_serials.len(),
            self.groups_serials.len(),
        ]
    }
    pub(crate) fn truncate(&mut self, lengths: [usize; 26]) {
        self.edges.truncate(lengths[0]);
        self.edge_colors.truncate(lengths[1]);
        self.edge_shape_ids.truncate(lengths[2]);
        self.shapes.truncate(lengths[3]);
        self.path_subpaths.truncate(lengths[4]);
        self.path_segments.truncate(lengths[5]);
        self.path_numbers.truncate(lengths[6]);
        self.transforms.truncate(lengths[7]);
        self.images.truncate(lengths[8]);
        self.groups.truncate(lengths[9]);
        self.filters.truncate(lengths[10]);
        self.filter_primitives.truncate(lengths[11]);
        self.masks.truncate(lengths[12]);
        self.patterns.truncate(lengths[13]);
        self.fill_paints.truncate(lengths[14]);
        self.stroke_paints.truncate(lengths[15]);
        self.dash_values.truncate(lengths[16]);
        self.stops.truncate(lengths[17]);
        self.clips.truncate(lengths[18]);
        self.shapes_serials.truncate(lengths[19]);
        self.transforms_serials.truncate(lengths[20]);
        self.clips_serials.truncate(lengths[21]);
        self.patterns_serials.truncate(lengths[22]);
        self.masks_serials.truncate(lengths[23]);
        self.filters_serials.truncate(lengths[24]);
        self.groups_serials.truncate(lengths[25]);
    }
    pub(crate) fn storage_bytes(&self) -> Result<usize> {
        self.storage_bytes_controlled(&mut crate::builder::NoStop)
    }
    pub(crate) fn storage_bytes_controlled(&self, control: &mut dyn WorkControl) -> Result<usize> {
        let mut total = bytes(&self.publications)?;
        total = add(total, bytes(&self.edges)?)?;
        total = add(total, bytes(&self.edge_colors)?)?;
        total = add(total, bytes(&self.edge_shape_ids)?)?;
        total = add(total, bytes(&self.shapes)?)?;
        total = add(total, bytes(&self.path_subpaths)?)?;
        total = add(total, bytes(&self.path_segments)?)?;
        total = add(total, bytes(&self.path_numbers)?)?;
        total = add(total, bytes(&self.transforms)?)?;
        total = add(total, bytes(&self.images)?)?;
        total = add(total, bytes(&self.groups)?)?;
        total = add(total, bytes(&self.filters)?)?;
        total = add(total, bytes(&self.filter_primitives)?)?;
        total = add(total, bytes(&self.masks)?)?;
        total = add(total, bytes(&self.patterns)?)?;
        total = add(total, bytes(&self.fill_paints)?)?;
        total = add(total, bytes(&self.stroke_paints)?)?;
        total = add(total, bytes(&self.dash_values)?)?;
        total = add(total, bytes(&self.stops)?)?;
        total = add(total, bytes(&self.clips)?)?;
        total = add(total, bytes(&self.shapes_serials)?)?;
        total = add(total, bytes(&self.transforms_serials)?)?;
        total = add(total, bytes(&self.clips_serials)?)?;
        total = add(total, bytes(&self.patterns_serials)?)?;
        total = add(total, bytes(&self.masks_serials)?)?;
        total = add(total, bytes(&self.filters_serials)?)?;
        total = add(total, bytes(&self.groups_serials)?)?;
        for image in &self.images {
            control
                .charge(WorkKind::Media, 1)
                .map_err(crate::types::stopped)?;
            total = add(total, bytes(&image.pixels.rgba)?)?;
        }
        for clip in &self.clips {
            control
                .charge(WorkKind::ClipChild, 1)
                .map_err(crate::types::stopped)?;
            total = add(
                total,
                add(bytes(&clip.geometry.edges)?, bytes(&clip.geometry.shapes)?)?,
            )?;
        }
        for pattern in &self.patterns {
            control
                .charge(WorkKind::PaintVisit, 1)
                .map_err(crate::types::stopped)?;
            total = add(total, pattern.tile.allocated_byte_count())?;
        }
        Ok(total)
    }
}
/// Borrowed immutable scene tables. Every range is validated before graph publication.
#[derive(Debug, Clone, Copy)]
pub struct SceneView<'a> {
    scenes: &'a [SceneData],
    source: &'a [u8],
    animation: AnimationView<'a>,
    ignored: [Ignored; 4],
    pub id: SceneId,
    pub viewport: Viewport,
    pub edges: &'a [Edge],
    pub edge_colors: &'a [[u8; 4]],
    pub edge_shape_ids: &'a [u32],
    pub shapes: &'a [Shape],
    pub path_subpaths: &'a [vexel_svg_path::PathSubpath],
    pub path_segments: &'a [ScenePathSegment],
    pub path_numbers: &'a [vexel_svg_path::PathNumber],
    pub transforms: &'a [TransformFrame],
    pub images: &'a [Image],
    pub groups: &'a [OpacityGroup],
    pub filters: &'a [Filter],
    pub filter_primitives: &'a [FilterPrimitive],
    pub masks: &'a [Mask],
    pub patterns: &'a [Pattern],
    pub fill_paints: &'a [Paint],
    pub stroke_paints: &'a [Paint],
    pub dash_values: &'a [f64],
    pub stops: &'a [PaintStop],
    pub clips: &'a [Clip],
    serials: [&'a [u64]; 7],
}
impl SceneData {
    pub(crate) fn view<'a>(
        &'a self,
        owner: u64,
        slot: usize,
        scenes: &'a [SceneData],
        source: &'a [u8],
        animation: AnimationView<'a>,
        ignored: [Ignored; 4],
    ) -> SceneView<'a> {
        SceneView {
            scenes,
            source,
            animation,
            ignored,
            id: SceneId {
                owner,
                slot: slot as u32,
                serial: self.serial,
            },
            viewport: self.viewport,
            edges: &self.edges,
            edge_colors: &self.edge_colors,
            edge_shape_ids: &self.edge_shape_ids,
            shapes: &self.shapes,
            path_subpaths: &self.path_subpaths,
            path_segments: &self.path_segments,
            path_numbers: &self.path_numbers,
            transforms: &self.transforms,
            images: &self.images,
            groups: &self.groups,
            filters: &self.filters,
            filter_primitives: &self.filter_primitives,
            masks: &self.masks,
            patterns: &self.patterns,
            fill_paints: &self.fill_paints,
            stroke_paints: &self.stroke_paints,
            dash_values: &self.dash_values,
            stops: &self.stops,
            clips: &self.clips,
            serials: [
                &self.shapes_serials,
                &self.transforms_serials,
                &self.clips_serials,
                &self.patterns_serials,
                &self.masks_serials,
                &self.filters_serials,
                &self.groups_serials,
            ],
        }
    }
}
/// Complete immutable table view, including checked child lookup and source context.
pub type ViewTables<'a> = SceneView<'a>;
macro_rules! view_handle {
    ($name:ident, $kind:ident, $field:ident, $serial:expr) => {
        /// Retrieve an existing checked handle from this exact immutable view.
        /// Enumeration includes records with no edges and preserves insertion
        /// serials, so rollback/reused slots cannot authorize stale handles.
        pub fn $name(&self, index: usize) -> Option<$kind> {
            self.$field.get(index)?;
            Some($kind {
                owner: self.id.owner,
                scene: self.id.slot,
                slot: u32::try_from(index).ok()?,
                serial: *self.serials[$serial].get(index)?,
            })
        }
    };
}
impl<'a> SceneView<'a> {
    view_handle!(shape_id, ShapeId, shapes, 0);
    view_handle!(transform_id, TransformId, transforms, 1);
    view_handle!(clip_id, ClipId, clips, 2);
    view_handle!(pattern_id, PatternId, patterns, 3);
    view_handle!(mask_id, MaskId, masks, 4);
    view_handle!(filter_id, FilterId, filters, 5);
    view_handle!(group_id, GroupId, groups, 6);
    pub(crate) fn overlay<'b>(
        &'b self,
        edges: Option<&'b [Edge]>,
        colors: Option<&'b [[u8; 4]]>,
        shapes: Option<&'b [Shape]>,
        fills: Option<&'b [Paint]>,
        strokes: Option<&'b [Paint]>,
        transforms: Option<&'b [TransformFrame]>,
        clips: Option<&'b [Clip]>,
    ) -> SceneView<'b> {
        let mut view: SceneView<'b> = *self;
        if let Some(v) = edges {
            view.edges = v;
        }
        if let Some(v) = colors {
            view.edge_colors = v;
        }
        if let Some(v) = shapes {
            view.shapes = v;
        }
        if let Some(v) = fills {
            view.fill_paints = v;
        }
        if let Some(v) = strokes {
            view.stroke_paints = v;
        }
        if let Some(v) = transforms {
            view.transforms = v;
        }
        if let Some(v) = clips {
            view.clips = v;
        }
        view
    }

    pub fn tables(&self) -> ViewTables<'a> {
        *self
    }
    pub fn source(&self) -> &'a [u8] {
        self.source
    }
    pub fn animation(&self) -> AnimationView<'a> {
        self.animation
    }
    pub fn ignored(&self, kind: IgnoredKind) -> Ignored {
        self.ignored[kind as usize]
    }
    pub fn scene(&self, id: SceneId) -> Option<SceneView<'a>> {
        let scene = self.scenes.get(id.index())?;
        if id.owner != self.id.owner || id.serial != scene.serial || !scene.sealed {
            return None;
        }
        if id == self.id {
            return Some(*self);
        }
        Some(scene.view(
            self.id.owner,
            id.index(),
            self.scenes,
            self.source,
            self.animation,
            self.ignored,
        ))
    }
    pub fn fill_paint(&self, id: ShapeId) -> Option<&'a Paint> {
        self.fill_paints.get(self.shape(id)?.fill_paint?)
    }
    pub fn stroke_paint(&self, id: ShapeId) -> Option<&'a Paint> {
        self.stroke_paints.get(self.shape(id)?.stroke_paint?)
    }
    pub fn shape_dashes(&self, id: ShapeId) -> Option<&'a [f64]> {
        self.shape(id)?.stroke.dash_values.slice(self.dash_values)
    }

    pub fn shape(&self, id: ShapeId) -> Option<&'a Shape> {
        if id.owner != self.id.owner
            || id.scene != self.id.slot
            || self.serials[0].get(id.index()) != Some(&id.serial)
        {
            return None;
        }
        self.shapes.get(id.index())
    }
    pub fn transform(&self, id: TransformId) -> Option<&'a TransformFrame> {
        if id.owner != self.id.owner
            || id.scene != self.id.slot
            || self.serials[1].get(id.index()) != Some(&id.serial)
        {
            return None;
        }
        self.transforms.get(id.index())
    }
    pub fn clip(&self, id: ClipId) -> Option<&'a Clip> {
        if id.owner != self.id.owner
            || id.scene != self.id.slot
            || self.serials[2].get(id.index()) != Some(&id.serial)
        {
            return None;
        }
        self.clips.get(id.index())
    }
    pub fn pattern(&self, id: PatternId) -> Option<&'a Pattern> {
        if id.owner != self.id.owner
            || id.scene != self.id.slot
            || self.serials[3].get(id.index()) != Some(&id.serial)
        {
            return None;
        }
        self.patterns.get(id.index())
    }
    pub fn mask(&self, id: MaskId) -> Option<&'a Mask> {
        if id.owner != self.id.owner
            || id.scene != self.id.slot
            || self.serials[4].get(id.index()) != Some(&id.serial)
        {
            return None;
        }
        self.masks.get(id.index())
    }
    pub fn filter(&self, id: FilterId) -> Option<&'a Filter> {
        if id.owner != self.id.owner
            || id.scene != self.id.slot
            || self.serials[5].get(id.index()) != Some(&id.serial)
        {
            return None;
        }
        self.filters.get(id.index())
    }
    pub fn group(&self, id: GroupId) -> Option<&'a OpacityGroup> {
        if id.owner != self.id.owner
            || id.scene != self.id.slot
            || self.serials[6].get(id.index()) != Some(&id.serial)
        {
            return None;
        }
        self.groups.get(id.index())
    }
    pub fn edge_shape(&self, edge: usize) -> Option<ShapeId> {
        let slot = *self.edge_shape_ids.get(edge)?;
        Some(ShapeId {
            owner: self.id.owner,
            scene: self.id.slot,
            slot,
            serial: *self.serials[0].get(slot as usize)?,
        })
    }
}
/// Immutable, source-coherent scene graph suitable for worker ownership transfer.
#[derive(Debug)]
pub struct SceneGraph {
    pub(crate) owner: u64,
    pub(crate) source: SourceOwner,
    pub(crate) generation: u64,
    pub(crate) root: SceneId,
    pub(crate) scenes: Vec<SceneData>,
    pub(crate) animation: OwnedAnimation,
    pub(crate) ignored: [Ignored; 4],
    pub(crate) stats: MemoryStats,
}
impl SceneGraph {
    pub fn scene_id(&self, index: usize) -> Option<SceneId> {
        let scene = self.scenes.get(index)?;
        Some(SceneId {
            owner: self.owner,
            slot: u32::try_from(index).ok()?,
            serial: scene.serial,
        })
    }
    pub fn source_owner(&self) -> &SourceOwner {
        &self.source
    }
    pub fn source(&self) -> &[u8] {
        self.source.as_slice()
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn root(&self) -> SceneView<'_> {
        self.scene(self.root).expect("validated root")
    }
    pub fn scene(&self, id: SceneId) -> Option<SceneView<'_>> {
        let scene = self.scenes.get(id.index())?;
        if id.owner != self.owner || id.serial != scene.serial {
            return None;
        }
        Some(scene.view(
            self.owner,
            id.index(),
            &self.scenes,
            &self.source,
            self.animation.view(),
            self.ignored,
        ))
    }
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }
    pub fn animation(&self) -> AnimationView<'_> {
        self.animation.view()
    }
    pub fn ignored(&self, kind: IgnoredKind) -> Ignored {
        self.ignored[kind as usize]
    }
    pub fn memory_stats(&self) -> MemoryStats {
        self.stats
    }
}
