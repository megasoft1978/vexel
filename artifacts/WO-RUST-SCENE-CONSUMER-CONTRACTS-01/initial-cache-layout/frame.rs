//! Copy-on-change evaluated tables. The immutable scene/source remains borrowed;
//! memory statistics below describe additional owned frame storage only.
use crate::memory::{add, arithmetic, bytes};
use crate::types::{invalid, limit, stopped};
use crate::*;
#[derive(Debug)]
pub struct EvaluatedFrame<'a> {
    base: SceneView<'a>,
    byte_limit: usize,
    edges: Option<Vec<Edge>>,
    colors: Option<Vec<[u8; 4]>>,
    shapes: Option<Vec<Shape>>,
    fill_paints: Option<Vec<Paint>>,
    stroke_paints: Option<Vec<Paint>>,
    transforms: Option<Vec<TransformFrame>>,
    clips: Option<Vec<Clip>>,
    stats: MemoryStats,
}
#[derive(Debug)]
pub struct FrameBuilder<'a> {
    frame: EvaluatedFrame<'a>,
    byte_limit: usize,
}
impl<'a> EvaluatedFrame<'a> {
    pub fn view(&self) -> SceneView<'_> {
        self.base.overlay(
            self.edges.as_deref(),
            self.colors.as_deref(),
            self.shapes.as_deref(),
            self.fill_paints.as_deref(),
            self.stroke_paints.as_deref(),
            self.transforms.as_deref(),
            self.clips.as_deref(),
        )
    }
    pub fn base(&self) -> SceneView<'a> {
        self.base
    }
    pub fn memory_stats(&self) -> MemoryStats {
        self.stats
    }
}
impl<'a> FrameBuilder<'a> {
    pub fn new(base: SceneView<'a>, byte_limit: usize) -> Result<Self> {
        if base.scene(base.id).is_none() {
            return Err(invalid(0));
        }
        Ok(Self {
            frame: EvaluatedFrame {
                base,
                byte_limit,
                edges: None,
                colors: None,
                shapes: None,
                fill_paints: None,
                stroke_paints: None,
                transforms: None,
                clips: None,
                stats: Default::default(),
            },
            byte_limit,
        })
    }
    fn incoming(&mut self, new: usize, old: usize) -> Result<()> {
        let peak = add(self.frame.stats.live_bytes, new)?;
        if peak > self.byte_limit {
            return Err(limit(LimitKind::ReservedBytes));
        }
        let live = add(self.frame.stats.live_bytes - old, new)?;
        let history = add(self.frame.stats.total_allocated_bytes, new)?;
        let count = self
            .frame
            .stats
            .allocation_count
            .checked_add(usize::from(new != 0))
            .ok_or_else(arithmetic)?;
        self.frame.stats.live_bytes = live;
        self.frame.stats.peak_live_bytes = self.frame.stats.peak_live_bytes.max(peak);
        self.frame.stats.total_allocated_bytes = history;
        self.frame.stats.allocation_count = count;
        Ok(())
    }
    fn copy<T: Copy>(&mut self, source: &[T], control: &mut dyn WorkControl) -> Result<Vec<T>> {
        control.checkpoint().map_err(stopped)?;
        let requested = source
            .len()
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(arithmetic)?;
        if add(self.frame.stats.live_bytes, requested)? > self.byte_limit {
            return Err(limit(LimitKind::ReservedBytes));
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(source.len())
            .map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: 0,
            })?;
        let actual = bytes(&values)?;
        self.incoming(actual, 0)?;
        for item in source {
            if let Err(stop) = control.charge(WorkKind::AnimationTransform, 1) {
                self.frame.stats.live_bytes -= actual;
                return Err(stopped(stop));
            }
            values.push(*item);
        }
        Ok(values)
    }
    pub fn replace_edges(
        &mut self,
        values: Vec<Edge>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        if values.len() != self.frame.base.edges.len() {
            return Err(invalid(0));
        }
        for value in &values {
            control.charge(WorkKind::Edge, 1).map_err(stopped)?;
            if ![value.a.x, value.a.y, value.b.x, value.b.y]
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 1e6)
            {
                return Err(invalid(0));
            }
        }
        let old = self
            .frame
            .edges
            .as_ref()
            .map(bytes)
            .transpose()?
            .unwrap_or(0);
        self.incoming(bytes(&values)?, old)?;
        self.frame.edges = Some(values);
        Ok(())
    }
    pub fn replace_colors(
        &mut self,
        values: Vec<[u8; 4]>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        if values.len() != self.frame.base.edge_colors.len() {
            return Err(invalid(0));
        }
        let old = self
            .frame
            .colors
            .as_ref()
            .map(bytes)
            .transpose()?
            .unwrap_or(0);
        self.incoming(bytes(&values)?, old)?;
        self.frame.colors = Some(values);
        Ok(())
    }
    pub fn set_shape(
        &mut self,
        id: ShapeId,
        value: Shape,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        let old = self.frame.base.shape(id).ok_or_else(|| invalid(0))?;
        if old.edges != value.edges
            || old.subpaths != value.subpaths
            || old.source.element != value.source.element
        {
            return Err(invalid(0));
        }
        crate::validate::shape_attributes_view(self.frame.view(), &value, control)?;
        if self.frame.shapes.is_none() {
            self.frame.shapes = Some(self.copy(self.frame.base.shapes, control)?);
        }
        self.frame.shapes.as_mut().ok_or_else(|| invalid(0))?[id.index()] = value;
        Ok(())
    }
    pub fn set_transform(
        &mut self,
        id: TransformId,
        value: TransformFrame,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        let old = self.frame.base.transform(id).ok_or_else(|| invalid(0))?;
        if old.parent != value.parent
            || old.source_offset != value.source_offset
            || ![value.attribute, value.suffix, value.motion]
                .into_iter()
                .all(vexel_geometry::affine_valid)
        {
            return Err(invalid(0));
        }
        if self.frame.transforms.is_none() {
            self.frame.transforms = Some(self.copy(self.frame.base.transforms, control)?);
        }
        self.frame.transforms.as_mut().ok_or_else(|| invalid(0))?[id.index()] = value;
        Ok(())
    }
    pub fn set_fill_paint(
        &mut self,
        index: usize,
        value: Paint,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.set_paint(index, value, true, control)
    }
    pub fn set_stroke_paint(
        &mut self,
        index: usize,
        value: Paint,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.set_paint(index, value, false, control)
    }
    fn set_paint(
        &mut self,
        index: usize,
        value: Paint,
        fill: bool,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        let source = if fill {
            self.frame.base.fill_paints
        } else {
            self.frame.base.stroke_paints
        };
        if index >= source.len() {
            return Err(invalid(0));
        }
        crate::validate::paint_view(self.frame.view(), &value, control)?;
        if fill {
            if self.frame.fill_paints.is_none() {
                self.frame.fill_paints = Some(self.copy(source, control)?);
            }
            self.frame.fill_paints.as_mut().ok_or_else(|| invalid(0))?[index] = value;
        } else {
            if self.frame.stroke_paints.is_none() {
                self.frame.stroke_paints = Some(self.copy(source, control)?);
            }
            self.frame
                .stroke_paints
                .as_mut()
                .ok_or_else(|| invalid(0))?[index] = value;
        }
        Ok(())
    }
    pub fn set_clip(
        &mut self,
        id: ClipId,
        value: Clip,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        let original = self.frame.base.clip(id).ok_or_else(|| invalid(0))?;
        if original.parent != value.parent {
            return Err(invalid(0));
        }
        crate::validate::clip_view(self.frame.view(), &value, control)?;
        let new = add(
            bytes(&value.geometry.edges)?,
            bytes(&value.geometry.shapes)?,
        )?;
        if let Some(clips) = &self.frame.clips {
            let old = add(
                bytes(&clips[id.index()].geometry.edges)?,
                bytes(&clips[id.index()].geometry.shapes)?,
            )?;
            let size = clip_bytes(clips, control)?;
            if add(size - old, new)? > 64 * 1024 * 1024 {
                return Err(limit(LimitKind::Animation));
            }
            self.incoming(new, old)?;
            self.frame.clips.as_mut().ok_or_else(|| invalid(0))?[id.index()] = value;
            return Ok(());
        }
        let before = self.frame.stats.live_bytes;
        let result = self.copy_clips(id, value, new, control);
        match result {
            Ok(values) => {
                self.frame.clips = Some(values);
                Ok(())
            }
            Err(error) => {
                self.frame.stats.live_bytes = before;
                Err(error)
            }
        }
    }
    fn copy_clips(
        &mut self,
        id: ClipId,
        value: Clip,
        nested: usize,
        control: &mut dyn WorkControl,
    ) -> Result<Vec<Clip>> {
        let mut result = Vec::new();
        let count = self.frame.base.clips.len();
        let minimum = count
            .checked_mul(std::mem::size_of::<Clip>())
            .ok_or_else(arithmetic)?;
        if add(self.frame.stats.live_bytes, add(minimum, nested)?)? > self.byte_limit {
            return Err(limit(LimitKind::ReservedBytes));
        }
        result.try_reserve_exact(count).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
        self.incoming(add(bytes(&result)?, nested)?, 0)?;
        let mut replacement = Some(value);
        for (index, source) in self.frame.base.clips.iter().enumerate() {
            control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
            if index == id.index() {
                result.push(replacement.take().ok_or_else(|| invalid(0))?);
            } else {
                let edges = self.copy(&source.geometry.edges, control)?;
                let shapes = self.copy(&source.geometry.shapes, control)?;
                result.push(Clip {
                    geometry: ClipGeometry { edges, shapes },
                    parent: source.parent,
                    owner_node: source.owner_node,
                    owner_before_suffix: source.owner_before_suffix,
                    object_bounding_box: source.object_bounding_box,
                    bounds: source.bounds,
                    user_inverse: source.user_inverse,
                });
            }
        }
        if clip_bytes(&result, control)? > 64 * 1024 * 1024 {
            return Err(limit(LimitKind::Animation));
        }
        Ok(result)
    }
    pub fn finish(self, control: &mut dyn WorkControl) -> Result<EvaluatedFrame<'a>> {
        control.checkpoint().map_err(stopped)?;
        let view = self.frame.view();
        for shape in view.shapes {
            crate::validate::shape_attributes_view(view, shape, control)?;
        }
        for paint in view.fill_paints.iter().chain(view.stroke_paints) {
            crate::validate::paint_view(view, paint, control)?;
        }
        for clip in view.clips {
            crate::validate::clip_view(view, clip, control)?;
        }
        let measured = self.frame.owned_bytes(control)?;
        if measured != self.frame.stats.live_bytes {
            return Err(invalid(0));
        }
        Ok(self.frame)
    }
}
impl EvaluatedFrame<'_> {
    fn owned_bytes(&self, control: &mut dyn WorkControl) -> Result<usize> {
        let mut total = 0;
        macro_rules! include {
            ($field:ident) => {
                if let Some(values) = &self.$field {
                    total = add(total, bytes(values)?)?;
                }
            };
        }
        include!(edges);
        include!(colors);
        include!(shapes);
        include!(fill_paints);
        include!(stroke_paints);
        include!(transforms);
        include!(clips);
        if let Some(clips) = &self.clips {
            for clip in clips {
                control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
                total = add(
                    total,
                    add(bytes(&clip.geometry.edges)?, bytes(&clip.geometry.shapes)?)?,
                )?;
            }
        }
        Ok(total)
    }
}

fn clip_bytes(values: &Vec<Clip>, control: &mut dyn WorkControl) -> Result<usize> {
    let mut total = bytes(values)?;
    for clip in values {
        control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
        total = add(
            total,
            add(bytes(&clip.geometry.edges)?, bytes(&clip.geometry.shapes)?)?,
        )?;
    }
    Ok(total)
}
