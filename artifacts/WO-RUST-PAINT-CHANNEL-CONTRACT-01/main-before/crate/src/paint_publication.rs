//! Source-derived template witnesses and scene-owned transactional stop sharing.
use crate::builder::room;
use crate::memory::{add, arithmetic};
use crate::types::{invalid, limit, stopped};
use crate::*;
use std::sync::Arc;

/// Constructed only from a canonical retained lexical source. A clone keeps
/// that allocation immutable, including after its lexical owner is dropped.
/// Instance inverse/bounds deliberately do not participate in template identity.
#[derive(Debug, Clone)]
pub struct GradientTemplateId {
    source: SourceOwner,
    key: TemplateKey,
    kind: PaintKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TemplateKey {
    definition: usize,
    viewport: [u64; 2],
}
#[derive(Debug)]
pub(crate) struct Publication {
    key: TemplateKey,
    pub(crate) order: usize,
    serial: u64,
    range: TableRange,
}
/// A checked graph publication witness, never a cache-local range. Scene and
/// insertion identity prevent reused slots from reviving rolled-back witnesses.
#[derive(Debug, Clone, Copy)]
pub struct PublishedStops {
    scene: SceneId,
    key: TemplateKey,
    serial: u64,
}
impl DecodeSource<'_> {
    pub fn gradient_template_id(
        &self,
        definition: ElementId,
        viewport: [f64; 2],
        control: &mut dyn WorkControl,
    ) -> Result<GradientTemplateId> {
        control.checkpoint().map_err(stopped)?;
        if !viewport.into_iter().all(|v| v.is_finite() && v > 0.) {
            return Err(invalid(0));
        }
        let token = self.tokens().get(definition.0).ok_or_else(|| invalid(0))?;
        self.element(definition, add(definition.0, 1)?, control)?;
        let name = token
            .local_name
            .slice(self.source())
            .ok_or_else(|| invalid(0))?;
        if name != b"linearGradient" && name != b"radialGradient" {
            return Err(invalid(token.source.offset));
        }
        control.checkpoint().map_err(stopped)?;
        Ok(GradientTemplateId {
            source: Arc::clone(self.source_owner()),
            kind: if name == b"linearGradient" {
                PaintKind::Linear
            } else {
                PaintKind::Radial
            },
            key: TemplateKey {
                definition: definition.0,
                viewport: viewport.map(f64::to_bits),
            },
        })
    }
}
impl GradientTemplateId {
    /// Equal retained allocation, actual definition token and viewport.
    pub fn same_template(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.source, &other.source) && self.key == other.key
    }
}
// Sorted metadata guarantees logarithmic lookup even for hostile keys. New
// insertion moves a bounded suffix; that byte work is charged before mutation.
fn lookup(
    values: &[Publication],
    key: TemplateKey,
    control: &mut dyn WorkControl,
) -> Result<std::result::Result<usize, usize>> {
    let (mut lo, mut hi) = (0, values.len());
    while lo < hi {
        control
            .charge(WorkKind::ReferenceProbe, 1)
            .map_err(stopped)?;
        let mid = lo + (hi - lo) / 2;
        match values[mid].key.cmp(&key) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => return Ok(Ok(mid)),
        }
    }
    Ok(Err(lo))
}
fn byte_work(mut bytes: usize, control: &mut dyn WorkControl) -> Result<()> {
    while bytes != 0 {
        let batch = bytes.min(256);
        control
            .charge(WorkKind::SourceRead, batch as u64)
            .map_err(stopped)?;
        bytes -= batch;
    }
    Ok(())
}
impl crate::model::SceneData {
    pub(crate) fn validate_publications(&self, control: &mut dyn WorkControl) -> Result<()> {
        if self.publications.len() > 32768 {
            return Err(limit(LimitKind::PaintVisits));
        }
        let mut previous = None;
        for entry in &self.publications {
            control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
            if previous.is_some_and(|key| key >= entry.key)
                || entry.order >= self.publications.len()
                || entry.serial == 0
            {
                return Err(invalid(0));
            }
            entry.range.require(self.stops.len(), 0)?;
            if entry.range.count > 256 {
                return Err(limit(LimitKind::PaintStops));
            }
            previous = Some(entry.key);
        }
        Ok(())
    }
}
impl SceneBuilder {
    /// Publish a template's local stop slice once into this scene. Conflicting
    /// payloads under one source/definition/viewport key are invalid arguments.
    /// Errors can retain unused capacity/history but never logical publications.
    pub fn publish_template_stops(
        &mut self,
        scene: SceneId,
        template: &GradientTemplateId,
        stops: &[PaintStop],
        control: &mut dyn WorkControl,
    ) -> Result<PublishedStops> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        if !Arc::ptr_eq(&self.source, &template.source) {
            return Err(invalid(0));
        }
        if stops.len() > 256 {
            return Err(limit(LimitKind::PaintStops));
        }
        crate::validate::stops(stops, control)?;
        let position = lookup(&self.scenes[i].publications, template.key, control)?;
        if let Ok(index) = position {
            let entry = &self.scenes[i].publications[index];
            let retained = entry
                .range
                .slice(&self.scenes[i].stops)
                .ok_or_else(|| invalid(0))?;
            if retained.len() != stops.len() {
                return Err(invalid(0));
            }
            for (a, b) in retained.iter().zip(stops) {
                control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
                if a != b {
                    return Err(invalid(0));
                }
            }
            control.checkpoint().map_err(stopped)?;
            return Ok(PublishedStops {
                scene,
                key: template.key,
                serial: entry.serial,
            });
        }
        let position = position.unwrap_err();
        let order = self.scenes[i].publications.len();
        if order >= 32768 {
            return Err(limit(LimitKind::PaintVisits));
        }
        if add(self.scenes[i].stops.len(), stops.len())? > self.limits.stops {
            return Err(limit(LimitKind::PaintStops));
        }
        // Covers reserve transfer and insertion shift, as well as stop copy.
        let transfers = if order == self.scenes[i].publications.capacity() {
            order
        } else {
            0
        };
        byte_work(
            add(transfers, order - position)?
                .checked_mul(std::mem::size_of::<Publication>())
                .ok_or_else(arithmetic)?,
            control,
        )?;
        if add(self.scenes[i].stops.len(), stops.len())? > self.scenes[i].stops.capacity() {
            byte_work(
                self.scenes[i]
                    .stops
                    .len()
                    .checked_mul(std::mem::size_of::<PaintStop>())
                    .ok_or_else(arithmetic)?,
                control,
            )?;
        }
        byte_work(std::mem::size_of_val(stops), control)?;
        let serial = self.serial()?;
        room!(self, i, publications, 1, 32768, 0);
        room!(self, i, stops, stops.len(), self.limits.stops, 0);
        control.checkpoint().map_err(stopped)?;
        let range = TableRange {
            first: self.scenes[i].stops.len(),
            count: stops.len(),
        };
        self.scenes[i].stops.extend_from_slice(stops);
        self.scenes[i].publications.insert(
            position,
            Publication {
                key: template.key,
                order,
                serial,
                range,
            },
        );
        Ok(PublishedStops {
            scene,
            key: template.key,
            serial,
        })
    }
    /// Check exact scene/insertion ownership before obtaining a graph range.
    /// The returned plain range is a snapshot; retain the witness across drafts.
    pub fn published_stop_range(
        &self,
        scene: SceneId,
        published: &PublishedStops,
        control: &mut dyn WorkControl,
    ) -> Result<TableRange> {
        let i = self.index(scene)?;
        control.checkpoint().map_err(stopped)?;
        if scene != published.scene {
            return Err(invalid(0));
        }
        let index = lookup(&self.scenes[i].publications, published.key, control)?
            .map_err(|_| invalid(0))?;
        let entry = &self.scenes[i].publications[index];
        if entry.serial != published.serial {
            return Err(invalid(0));
        }
        entry.range.require(self.scenes[i].stops.len(), 0)?;
        Ok(entry.range)
    }
    /// Atomic complete resolver handoff. `ResolvedPaint.paint.stops` indexes
    /// only its owned local vector. The resolver returns its selected template
    /// explicitly; DECODE need not re-resolve URLs or reconstruct identity.
    pub fn append_resolved_fill(
        &mut self,
        scene: SceneId,
        resolved: &ResolvedPaint,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        self.append_local(
            scene,
            resolved.paint,
            &resolved.stops,
            resolved.template.as_ref(),
            false,
            control,
        )
    }
    pub fn append_resolved_stroke(
        &mut self,
        scene: SceneId,
        resolved: &ResolvedPaint,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        self.append_local(
            scene,
            resolved.paint,
            &resolved.stops,
            resolved.template.as_ref(),
            true,
            control,
        )
    }
    /// Borrow a real cached template without cloning its stop vector. Instance
    /// paint bounds/inverses remain independent; paint.stops is cache-local.
    pub fn append_template_fill(
        &mut self,
        scene: SceneId,
        template: &GradientTemplateId,
        paint: Paint,
        local_stops: &[PaintStop],
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        self.append_local(scene, paint, local_stops, Some(template), false, control)
    }
    pub fn append_template_stroke(
        &mut self,
        scene: SceneId,
        template: &GradientTemplateId,
        paint: Paint,
        local_stops: &[PaintStop],
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        self.append_local(scene, paint, local_stops, Some(template), true, control)
    }
    fn append_local(
        &mut self,
        scene: SceneId,
        local_paint: Paint,
        local_stops: &[PaintStop],
        template: Option<&GradientTemplateId>,
        stroke: bool,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        if local_stops.len() > 256 {
            return Err(limit(LimitKind::PaintStops));
        }
        let gradient = matches!(local_paint.kind, PaintKind::Linear | PaintKind::Radial);
        if gradient != template.is_some() || template.is_some_and(|t| t.kind != local_paint.kind) {
            return Err(invalid(0));
        }
        let mut view = self.scene(scene)?;
        view.stops = local_stops;
        crate::validate::paint_view(view, &local_paint, control)?;
        let stops = local_paint
            .stops
            .slice(local_stops)
            .ok_or_else(|| invalid(0))?;
        let mark = self.checkpoint_internal(scene)?;
        let result = (|| {
            let mut paint = local_paint;
            paint.stops = if let Some(template) = template {
                let published = self.publish_template_stops(scene, template, stops, control)?;
                self.published_stop_range(scene, &published, control)?
            } else {
                self.append_stops(scene, stops, control)?
            };
            if stroke {
                self.append_stroke_paint(scene, paint, control)
            } else {
                self.append_fill_paint(scene, paint, control)
            }
        })();
        if result.is_err() {
            let i = self.index(scene)?;
            // append_stops may have already cleaned its own failed suffix.
            // Do not invalidate ordinary marks when nothing was published.
            if self.scenes[i].lengths() != mark.lengths
                || self.scenes[i].publications.len() != mark.publications
            {
                let mut mark = mark;
                mark.epoch = self.scenes[i].epoch;
                self.rollback_internal(mark)?;
            }
        }
        result
    }
}

#[cfg(test)]
#[path = "paint_publication_tests.rs"]
mod tests;
