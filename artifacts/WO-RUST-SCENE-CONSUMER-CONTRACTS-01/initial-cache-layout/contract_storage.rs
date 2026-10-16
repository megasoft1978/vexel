use crate::memory::{add, bytes};
use crate::types::stopped;
use crate::*;
impl RasterScratch {
    /// Actual owned buffer capacities, including per-shape stroke grids.
    pub fn storage_bytes(&self, control: &mut dyn WorkControl) -> Result<usize> {
        control.checkpoint().map_err(stopped)?;
        let mut total = 0;
        macro_rules! count {
            ($values:expr) => {
                total = add(total, bytes($values)?)?;
            };
        }
        count!(&self.bounds.shapes);
        count!(&self.boundary.bits);
        count!(&self.scanlines.events);
        count!(&self.scanlines.nodes);
        count!(&self.scanlines.roots);
        count!(&self.scanlines.shape_events);
        count!(&self.scanlines.shape_nodes);
        count!(&self.rows.shape_offsets);
        count!(&self.rows.active_edges);
        count!(&self.rows.active_shapes);
        count!(&self.fill.eligible_shapes);
        count!(&self.fill.crossings);
        count!(&self.fill.intervals);
        count!(&self.fill.pixel_runs);
        count!(&self.local_edges);
        count!(&self.dash_seams);
        count!(&self.dash.segments);
        count!(&self.dash.joins);
        count!(&self.dash.shapes);
        count!(&self.center_samples);
        count!(&self.subpixel_samples);
        count!(&self.stroke_cost);
        for range in &self.dash.shapes {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            count!(&range.grid_cells);
            count!(&range.grid_refs);
        }
        Ok(total)
    }
}
impl TextCache {
    pub fn storage_bytes(&self, control: &mut dyn WorkControl) -> Result<usize> {
        control.checkpoint().map_err(stopped)?;
        let mut total = bytes(&self.entries)?;
        for entry in &self.entries {
            control.charge(WorkKind::Media, 1).map_err(stopped)?;
            total = add(total, bytes(&entry.text)?)?;
            total = add(total, bytes(&entry.family_spans)?)?;
            total = add(total, bytes(&entry.owned_families)?)?;
            total = add(total, entry.layout.storage_bytes()?)?;
            for family in &entry.owned_families {
                control.charge(WorkKind::Media, 1).map_err(stopped)?;
                total = add(total, bytes(family)?)?;
            }
        }
        Ok(total)
    }
}
impl PaintCache {
    pub fn storage_bytes(&self) -> Result<usize> {
        add(add(bytes(&self.entries)?, bytes(&self.slots)?)?, bytes(&self.stops)?)
    }
}
impl ActiveDefinitions {
    pub fn push(
        &mut self,
        kind: DecodeFlavor,
        element: ElementId,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        let (stack, cap) = match kind {
            DecodeFlavor::Normal => (&mut self.uses, 16),
            DecodeFlavor::Mask => (&mut self.masks, 8),
            DecodeFlavor::Pattern => (&mut self.patterns, 8),
            DecodeFlavor::Marker => (&mut self.markers, 8),
            DecodeFlavor::Clip { .. } => return Err(crate::types::invalid(0)),
        };
        if stack.len() >= cap {
            return Err(crate::types::limit(LimitKind::Depth));
        }
        for current in stack.iter() {
            control
                .charge(WorkKind::ReferenceProbe, 1)
                .map_err(stopped)?;
            if *current == element {
                return Err(crate::types::limit(LimitKind::References));
            }
        }
        stack.try_reserve_exact(1).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
        stack.push(element);
        Ok(())
    }
    pub fn pop(&mut self, kind: DecodeFlavor, element: ElementId) -> Result<()> {
        let stack = match kind {
            DecodeFlavor::Normal => &mut self.uses,
            DecodeFlavor::Mask => &mut self.masks,
            DecodeFlavor::Pattern => &mut self.patterns,
            DecodeFlavor::Marker => &mut self.markers,
            DecodeFlavor::Clip { .. } => return Err(crate::types::invalid(0)),
        };
        if stack.last() != Some(&element) {
            return Err(crate::types::invalid(0));
        }
        stack.pop();
        Ok(())
    }
    pub fn storage_bytes(&self) -> Result<usize> {
        add(
            add(bytes(&self.uses)?, bytes(&self.masks)?)?,
            add(bytes(&self.patterns)?, bytes(&self.markers)?)?,
        )
    }
}
