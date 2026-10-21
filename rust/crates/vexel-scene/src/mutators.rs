use crate::builder::room;
use crate::memory::{add, bytes};
use crate::types::{invalid, limit, stopped};
use crate::*;
impl SceneBuilder {
    pub fn append_transform(
        &mut self,
        scene: SceneId,
        value: TransformFrame,
        control: &mut dyn WorkControl,
    ) -> Result<TransformId> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::transform(self, scene, &value, control)?;
        if self.scenes[i].transforms.len() >= self.limits.transforms {
            return Err(limit(LimitKind::Transforms));
        }
        let nested = 0;
        self.pending(nested)?;
        let serial = self.serial()?;
        room!(self, i, transforms, 1, self.limits.transforms, nested);
        room!(
            self,
            i,
            transforms_serials,
            1,
            self.limits.transforms,
            nested
        );
        control.checkpoint().map_err(stopped)?;
        self.retain(nested)?;
        let id = TransformId {
            owner: self.owner,
            scene: scene.slot,
            slot: self.scenes[i].transforms.len() as u32,
            serial,
        };
        self.scenes[i].transforms.push(value);
        self.scenes[i].transforms_serials.push(serial);
        Ok(id)
    }
    pub fn append_clip(
        &mut self,
        scene: SceneId,
        value: Clip,
        control: &mut dyn WorkControl,
    ) -> Result<ClipId> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::clip(self, scene, &value, control)?;
        if self.scenes[i].clips.len() >= self.limits.clips {
            return Err(limit(LimitKind::Clips));
        }
        let nested = add(
            bytes(&value.geometry.edges)?,
            bytes(&value.geometry.shapes)?,
        )?;
        self.pending(nested)?;
        let serial = self.serial()?;
        room!(self, i, clips, 1, self.limits.clips, nested);
        room!(self, i, clips_serials, 1, self.limits.clips, nested);
        control.checkpoint().map_err(stopped)?;
        self.retain(nested)?;
        let id = ClipId {
            owner: self.owner,
            scene: scene.slot,
            slot: self.scenes[i].clips.len() as u32,
            serial,
        };
        self.scenes[i].clips.push(value);
        self.scenes[i].clips_serials.push(serial);
        Ok(id)
    }
    pub fn append_pattern(
        &mut self,
        scene: SceneId,
        value: Pattern,
        control: &mut dyn WorkControl,
    ) -> Result<PatternId> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::pattern(self, scene, &value, control)?;
        if self.scenes[i].patterns.len() >= self.limits.patterns {
            return Err(limit(LimitKind::Patterns));
        }
        let nested = value.tile.allocated_byte_count();
        self.pending(nested)?;
        let serial = self.serial()?;
        room!(self, i, patterns, 1, self.limits.patterns, nested);
        room!(self, i, patterns_serials, 1, self.limits.patterns, nested);
        control.checkpoint().map_err(stopped)?;
        self.retain(nested)?;
        let id = PatternId {
            owner: self.owner,
            scene: scene.slot,
            slot: self.scenes[i].patterns.len() as u32,
            serial,
        };
        self.scenes[i].patterns.push(value);
        self.scenes[i].patterns_serials.push(serial);
        Ok(id)
    }
    pub fn append_mask(
        &mut self,
        scene: SceneId,
        value: Mask,
        control: &mut dyn WorkControl,
    ) -> Result<MaskId> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::mask(self, scene, &value, control)?;
        if self.scenes[i].masks.len() >= self.limits.masks {
            return Err(limit(LimitKind::Masks));
        }
        let nested = 0;
        self.pending(nested)?;
        let serial = self.serial()?;
        room!(self, i, masks, 1, self.limits.masks, nested);
        room!(self, i, masks_serials, 1, self.limits.masks, nested);
        control.checkpoint().map_err(stopped)?;
        self.retain(nested)?;
        let id = MaskId {
            owner: self.owner,
            scene: scene.slot,
            slot: self.scenes[i].masks.len() as u32,
            serial,
        };
        self.scenes[i].masks.push(value);
        self.scenes[i].masks_serials.push(serial);
        Ok(id)
    }
    pub fn append_group(
        &mut self,
        scene: SceneId,
        value: OpacityGroup,
        control: &mut dyn WorkControl,
    ) -> Result<GroupId> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        self.cover_open_ancestors(i, &value)?;
        crate::validate::group(self, scene, &value, self.scenes[i].groups.len(), control)?;
        if self.scenes[i].groups.len() >= self.limits.groups {
            return Err(limit(LimitKind::Nodes));
        }
        let nested = 0;
        self.pending(nested)?;
        let serial = self.serial()?;
        room!(self, i, groups, 1, self.limits.groups, nested);
        room!(self, i, groups_serials, 1, self.limits.groups, nested);
        control.checkpoint().map_err(stopped)?;
        self.retain(nested)?;
        let id = GroupId {
            owner: self.owner,
            scene: scene.slot,
            slot: self.scenes[i].groups.len() as u32,
            serial,
        };
        self.scenes[i].groups.push(value);
        self.scenes[i].groups_serials.push(serial);
        Ok(id)
    }
    pub fn append_image(
        &mut self,
        scene: SceneId,
        value: Image,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::image(&value, control)?;
        let pixels = self.scenes[i].image_pixels;
        if add(
            pixels,
            (value.pixels.width as usize) * (value.pixels.height as usize),
        )? > 67_108_864
        {
            return Err(limit(LimitKind::Images));
        }
        let nested = bytes(&value.pixels.rgba)?;
        self.pending(nested)?;
        room!(self, i, images, 1, self.limits.shapes, nested);
        control.checkpoint().map_err(stopped)?;
        self.retain(nested)?;
        let index = self.scenes[i].images.len();
        self.scenes[i].image_pixels = add(
            pixels,
            (value.pixels.width as usize) * (value.pixels.height as usize),
        )?;
        self.scenes[i].images.push(value);
        Ok(index)
    }
    pub fn append_stops(
        &mut self,
        scene: SceneId,
        values: &[PaintStop],
        control: &mut dyn WorkControl,
    ) -> Result<TableRange> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        if values.len() > 256 {
            return Err(limit(LimitKind::PaintStops));
        }
        crate::validate::stops(values, control)?;
        let cap = self.limits.stops;
        if add(self.scenes[i].stops.len(), values.len())? > cap {
            return Err(limit(LimitKind::PaintStops));
        }
        room!(self, i, stops, values.len(), cap, 0);
        let mark = self.checkpoint_internal(scene)?;
        let first = self.scenes[i].stops.len();
        for value in values {
            if let Err(stop) = control.charge(WorkKind::PaintVisit, 1) {
                self.rollback_internal(mark)?;
                return Err(stopped(stop));
            }
            self.scenes[i].stops.push(*value);
        }
        Ok(TableRange {
            first,
            count: values.len(),
        })
    }
    pub fn append_dashes(
        &mut self,
        scene: SceneId,
        values: &[f64],
        control: &mut dyn WorkControl,
    ) -> Result<TableRange> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        if values.len() > 16 {
            return Err(limit(LimitKind::DashCache));
        }
        for value in values {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if !value.is_finite() || *value < 0.0 || *value > 1e6 {
                return Err(invalid(0));
            }
        }
        let cap = self
            .limits
            .shapes
            .checked_mul(16)
            .ok_or_else(crate::memory::arithmetic)?;
        room!(self, i, dash_values, values.len(), cap, 0);
        let mark = self.checkpoint_internal(scene)?;
        let first = self.scenes[i].dash_values.len();
        for value in values {
            if let Err(stop) = control.charge(WorkKind::StrokeTest, 1) {
                self.rollback_internal(mark)?;
                return Err(stopped(stop));
            }
            self.scenes[i].dash_values.push(*value);
        }
        Ok(TableRange {
            first,
            count: values.len(),
        })
    }
    pub fn append_fill_paint(
        &mut self,
        scene: SceneId,
        value: Paint,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::paint(self, scene, &value, control)?;
        room!(self, i, fill_paints, 1, self.limits.shapes, 0);
        control.checkpoint().map_err(stopped)?;
        let index = self.scenes[i].fill_paints.len();
        self.scenes[i].fill_paints.push(value);
        Ok(index)
    }
    pub fn append_stroke_paint(
        &mut self,
        scene: SceneId,
        value: Paint,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::paint(self, scene, &value, control)?;
        room!(self, i, stroke_paints, 1, self.limits.shapes, 0);
        control.checkpoint().map_err(stopped)?;
        let index = self.scenes[i].stroke_paints.len();
        self.scenes[i].stroke_paints.push(value);
        Ok(index)
    }
    pub fn append_filter(
        &mut self,
        scene: SceneId,
        mut value: Filter,
        primitives: &[FilterPrimitive],
        control: &mut dyn WorkControl,
    ) -> Result<FilterId> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::filter(value, primitives, control)?;
        for p in primitives {
            if p.source_offset > self.source.len() {
                return Err(invalid(p.source_offset));
            }
        }
        if self.scenes[i].filters.len() >= self.limits.filters {
            return Err(limit(LimitKind::Filters));
        }
        let serial = self.serial()?;
        room!(self, i, filters, 1, self.limits.filters, 0);
        room!(self, i, filters_serials, 1, self.limits.filters, 0);
        room!(self, i, filter_primitives, primitives.len(), 65536, 0);
        let mark = self.checkpoint_internal(scene)?;
        value.primitives = TableRange {
            first: self.scenes[i].filter_primitives.len(),
            count: primitives.len(),
        };
        for primitive in primitives {
            if let Err(stop) = control.charge(WorkKind::FilterPixel, 1) {
                self.rollback_internal(mark)?;
                return Err(stopped(stop));
            }
            self.scenes[i].filter_primitives.push(*primitive);
        }
        let id = FilterId {
            owner: self.owner,
            scene: scene.slot,
            slot: self.scenes[i].filters.len() as u32,
            serial,
        };
        self.scenes[i].filters.push(value);
        self.scenes[i].filters_serials.push(serial);
        Ok(id)
    }
}
