//! Request-scoped shape capacity admission before geometry or media providers.
use crate::memory::{add, arithmetic};
use crate::types::{invalid, stopped};
use crate::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActiveShapeAdmission {
    pub(crate) active: bool,
    pub(crate) nonce: u64,
    pub(crate) draft_nonce: u64,
    pub(crate) request: RequestToken,
    pub(crate) source_offset: usize,
    pub(crate) geometry: Option<crate::shape_geometry::GeometryProof>,
}

/// Opaque one-use proof that this request reserved one real shape slot.
/// Dropping it leaves the builder uncommittable until request rollback.
#[derive(Debug)]
#[must_use = "cancel the admission or consume it with append_admitted_shape"]
pub struct ShapeAdmission {
    pub(crate) owner: u64,
    pub(crate) generation: u64,
    pub(crate) scene: SceneId,
    pub(crate) draft_nonce: u64,
    pub(crate) request: RequestToken,
    pub(crate) nonce: u64,
    pub(crate) registry_slot: usize,
    pub(crate) source_offset: usize,
    pub(crate) geometry: Option<crate::shape_geometry::GeometryProof>,
}

impl SceneBuilder {
    /// Reserve shape and insertion-serial capacity before geometry/provider work.
    /// The caller must consume or cancel this token; request rollback is the
    /// unconditional recovery path after an abandoned or panicking operation.
    pub fn admit_shape(
        &mut self,
        draft: &DraftToken,
        request: RequestToken,
        source_offset: usize,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeAdmission> {
        self.admit_shape_inner(draft, request, source_offset, None, control)
    }

    /// Admit a referenced path while retaining its canonical `<path d>` source
    /// independently from the instance's `Shape::source` host annotation.
    pub fn admit_shape_with_source(
        &mut self,
        draft: &DraftToken,
        request: RequestToken,
        source_offset: usize,
        geometry: CanonicalPathSource<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeAdmission> {
        if !geometry.belongs_to(self.source_owner()) {
            return Err(invalid(source_offset));
        }
        self.admit_shape_inner(draft, request, source_offset, Some(geometry.proof), control)
    }

    fn admit_shape_inner(
        &mut self,
        draft: &DraftToken,
        request: RequestToken,
        source_offset: usize,
        geometry: Option<crate::shape_geometry::GeometryProof>,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeAdmission> {
        let i = self.draft_index(draft)?;
        if self.request_identity() != Some(request)
            || !self.request_allows_draft(draft, request)
            || source_offset > self.source.len()
        {
            return Err(invalid(source_offset));
        }
        control.checkpoint().map_err(|stop| {
            let mut error = stopped(stop);
            error.offset = source_offset;
            error
        })?;
        let mut pending = 0usize;
        let mut pending_geometry = 0usize;
        let mut reusable = None;
        for (position, entry) in self.scenes[i].shape_admissions.iter().enumerate() {
            control
                .charge(WorkKind::TokenVisit, 1)
                .map_err(|stop| at_offset(stop, source_offset))?;
            if entry.active {
                pending = add(pending, 1)?;
                if entry.geometry.is_some() {
                    pending_geometry = add(pending_geometry, 1)?;
                }
            } else if reusable.is_none() {
                reusable = Some(position);
            }
        }
        let used = add(self.scenes[i].shapes.len(), pending)?;
        if used >= self.limits.shapes {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Shapes),
                offset: source_offset,
            });
        }
        let slot = u32::try_from(i).map_err(|_| arithmetic())?;
        let scene = self.scenes[i].serial;
        let nonce = self.serial()?;
        let admission = ActiveShapeAdmission {
            active: true,
            nonce,
            draft_nonce: draft.nonce,
            request,
            source_offset,
            geometry,
        };
        let registry_slot = reusable.unwrap_or(self.scenes[i].shape_admissions.len());
        let reserved = (|| -> Result<()> {
            if reusable.is_none() {
                room!(self, i, shape_admissions, 1, self.limits.shapes, 0);
            }
            if geometry.is_some() {
                room!(
                    self,
                    i,
                    geometry_sources,
                    add(pending_geometry, 1)?,
                    self.limits.shapes,
                    0
                );
            }
            room!(self, i, shapes, add(pending, 1)?, self.limits.shapes, 0);
            room!(
                self,
                i,
                shapes_serials,
                add(pending, 1)?,
                self.limits.shapes,
                0
            );
            Ok(())
        })();
        if let Err(mut error) = reserved {
            error.offset = source_offset;
            return Err(error);
        }
        if let Some(position) = reusable {
            self.scenes[i].shape_admissions[position] = admission;
        } else {
            self.scenes[i].shape_admissions.push(admission);
        }
        Ok(ShapeAdmission {
            owner: self.owner,
            generation: self.generation,
            scene: SceneId {
                owner: self.owner,
                slot,
                serial: scene,
            },
            draft_nonce: draft.nonce,
            request,
            nonce,
            registry_slot,
            source_offset,
            geometry,
        })
    }

    pub(crate) fn shape_admission_index(
        &self,
        draft: &DraftToken,
        token: &ShapeAdmission,
    ) -> Result<(usize, usize)> {
        if token.owner != self.owner
            || token.generation != self.generation
            || token.scene != draft.mark.scene
            || token.draft_nonce != draft.nonce
            || token.request
                != self
                    .request_identity()
                    .ok_or_else(|| invalid(token.source_offset))?
            || !self.request_allows_draft(draft, token.request)
        {
            return Err(invalid(token.source_offset));
        }
        let i = self.draft_index(draft)?;
        let position = token.registry_slot;
        let active = self.scenes[i]
            .shape_admissions
            .get(position)
            .filter(|active| {
                active.active
                    && active.nonce == token.nonce
                    && active.draft_nonce == token.draft_nonce
                    && active.request == token.request
                    && active.source_offset == token.source_offset
                    && active.geometry == token.geometry
            })
            .ok_or_else(|| invalid(token.source_offset))?;
        let _ = active;
        Ok((i, position))
    }

    /// Release an unused reservation. Its grown Vec capacities stay accounted.
    pub fn cancel_shape_admission(
        &mut self,
        draft: &DraftToken,
        token: &ShapeAdmission,
    ) -> Result<()> {
        let (i, position) = self.shape_admission_index(draft, token)?;
        self.scenes[i].shape_admissions[position].active = false;
        Ok(())
    }
}

fn at_offset(stop: Stop, offset: usize) -> Diagnostic {
    let mut error = stopped(stop);
    error.offset = offset;
    error
}
