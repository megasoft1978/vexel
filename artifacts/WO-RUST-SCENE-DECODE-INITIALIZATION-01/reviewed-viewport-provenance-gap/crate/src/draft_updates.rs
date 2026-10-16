use crate::builder::room;
use crate::memory::{add, bytes};
use crate::types::{invalid, limit, stopped};
use crate::*;
impl SceneBuilder {
    pub fn finalize_shape(
        &mut self,
        token: &DraftToken,
        id: ShapeId,
        value: Shape,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        self.draft_slot(token, 3, id.index(), self.scenes[i].shapes.len())?;
        let old = self
            .traversal_scene(token.mark.scene)?
            .shape(id)
            .ok_or_else(|| invalid(0))?;
        if old.edges != value.edges
            || old.subpaths != value.subpaths
            || old.source.element != value.source.element
        {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)?;
        crate::validate::shape_attributes(self, token.mark.scene, &value, control)?;
        self.scenes[i].shapes[id.index()] = value;
        Ok(())
    }
    pub fn finalize_shape_colors(
        &mut self,
        token: &DraftToken,
        id: ShapeId,
        colors: &[[u8; 4]],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        self.draft_slot(token, 3, id.index(), self.scenes[i].shapes.len())?;
        let shape = self
            .traversal_scene(token.mark.scene)?
            .shape(id)
            .ok_or_else(|| invalid(0))?;
        let range = shape.edges;
        if range.count != colors.len() || range.first < token.mark.lengths[1] {
            return Err(invalid(0));
        }
        range.require(self.scenes[i].edge_colors.len(), 0)?;
        control.checkpoint().map_err(stopped)?;
        for _ in colors {
            control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
        }
        self.scenes[i].edge_colors[range.first..range.end().ok_or_else(|| invalid(0))?]
            .copy_from_slice(colors);
        Ok(())
    }
    pub fn finalize_transform(
        &mut self,
        token: &DraftToken,
        id: TransformId,
        value: TransformFrame,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        self.draft_slot(token, 7, id.index(), self.scenes[i].transforms.len())?;
        let old = self
            .traversal_scene(token.mark.scene)?
            .transform(id)
            .ok_or_else(|| invalid(0))?;
        if old.parent != value.parent || old.source_offset != value.source_offset {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)?;
        crate::validate::transform(self, token.mark.scene, &value, control)?;
        self.scenes[i].transforms[id.index()] = value;
        Ok(())
    }
    pub fn finalize_clip(
        &mut self,
        token: &DraftToken,
        id: ClipId,
        value: Clip,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        self.draft_slot(token, 18, id.index(), self.scenes[i].clips.len())?;
        let old = self
            .traversal_scene(token.mark.scene)?
            .clip(id)
            .ok_or_else(|| invalid(0))?;
        if old.parent != value.parent {
            return Err(invalid(0));
        }
        let old_bytes = add(bytes(&old.geometry.edges)?, bytes(&old.geometry.shapes)?)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::clip(self, token.mark.scene, &value, control)?;
        let new = add(
            bytes(&value.geometry.edges)?,
            bytes(&value.geometry.shapes)?,
        )?;
        self.pending(new)?;
        self.retain(new)?;
        self.scenes[i].clips[id.index()] = value;
        self.stats.live_bytes -= old_bytes;
        Ok(())
    }
    pub fn finalize_mask(
        &mut self,
        token: &DraftToken,
        id: MaskId,
        value: Mask,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        self.draft_slot(token, 12, id.index(), self.scenes[i].masks.len())?;
        self.traversal_scene(token.mark.scene)?
            .mask(id)
            .ok_or_else(|| invalid(0))?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::mask(self, token.mark.scene, &value, control)?;
        self.scenes[i].masks[id.index()] = value;
        Ok(())
    }
    pub fn finalize_group(
        &mut self,
        token: &DraftToken,
        id: GroupId,
        value: OpacityGroup,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        self.draft_slot(token, 9, id.index(), self.scenes[i].groups.len())?;
        let old = self
            .traversal_scene(token.mark.scene)?
            .group(id)
            .ok_or_else(|| invalid(0))?;
        if old.parent != value.parent || old.shapes.first != value.shapes.first {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)?;
        crate::validate::group(self, token.mark.scene, &value, id.index(), control)?;
        self.scenes[i].groups[id.index()] = value;
        Ok(())
    }
    pub fn finalize_fill_paint(
        &mut self,
        token: &DraftToken,
        index: usize,
        value: Paint,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.finalize_paint(token, index, value, true, control)
    }
    pub fn finalize_stroke_paint(
        &mut self,
        token: &DraftToken,
        index: usize,
        value: Paint,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.finalize_paint(token, index, value, false, control)
    }
    fn finalize_paint(
        &mut self,
        token: &DraftToken,
        index: usize,
        value: Paint,
        fill: bool,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        let (table, len) = if fill {
            (14, self.scenes[i].fill_paints.len())
        } else {
            (15, self.scenes[i].stroke_paints.len())
        };
        self.draft_slot(token, table, index, len)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::paint(self, token.mark.scene, &value, control)?;
        if fill {
            self.scenes[i].fill_paints[index] = value;
        } else {
            self.scenes[i].stroke_paints[index] = value;
        }
        Ok(())
    }
    pub fn finalize_filter(
        &mut self,
        token: &DraftToken,
        id: FilterId,
        mut value: Filter,
        primitives: &[FilterPrimitive],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(token)?;
        self.draft_slot(token, 10, id.index(), self.scenes[i].filters.len())?;
        self.traversal_scene(token.mark.scene)?
            .filter(id)
            .ok_or_else(|| invalid(0))?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::filter(value, primitives, control)?;
        for p in primitives {
            control.charge(WorkKind::FilterPixel, 1).map_err(stopped)?;
            if p.source_offset > self.source.len() {
                return Err(invalid(p.source_offset));
            }
        }
        if add(self.scenes[i].filter_primitives.len(), primitives.len())? > 65536 {
            return Err(limit(LimitKind::Filters));
        }
        room!(self, i, filter_primitives, primitives.len(), 65536, 0);
        control.checkpoint().map_err(stopped)?;
        value.primitives = TableRange {
            first: self.scenes[i].filter_primitives.len(),
            count: primitives.len(),
        };
        self.scenes[i]
            .filter_primitives
            .extend_from_slice(primitives);
        self.scenes[i].filters[id.index()] = value;
        Ok(())
    }
}
