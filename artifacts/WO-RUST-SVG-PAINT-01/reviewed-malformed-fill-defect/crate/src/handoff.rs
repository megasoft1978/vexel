//! Explicit cache borrowing, publication, and accounted owned compatibility.
use crate::cache::{invalid, limit, oom};
use crate::control::PaintControl;
use std::sync::Arc;
use vexel_base::{WorkControl, WorkKind};
use vexel_scene::{
    AllocationLease, AllocationTag, ChargeLedger, GradientTemplateId, LimitKind, Paint, PaintStop,
    ResolvedPaint, Result, SceneBuilder, SceneId, SourceOwner,
};

/// Temporary producer context with an independent cache lifetime. Unlike the
/// shared owned-result ResolveContext, these borrows need not have one lifetime.
pub struct CachedResolveContext<'src, 'cache, 'ctx> {
    pub shared: &'ctx mut vexel_scene::DecodeSource<'src>,
    pub cache: &'cache mut vexel_scene::PaintCache,
    pub ledger: &'ctx mut ChargeLedger,
}

/// The actual stop slice in the accepted shared cache. This result borrows
/// only the immutable cache entry, never the temporary resolver context.
#[derive(Debug)]
pub struct CachedPaint<'cache> {
    pub paint: Paint,
    pub template: GradientTemplateId,
    pub stops: &'cache [PaintStop],
    source: &'cache SourceOwner,
    offset: usize,
}
impl<'cache> CachedPaint<'cache> {
    pub(crate) fn new(
        paint: Paint,
        template: GradientTemplateId,
        stops: &'cache [PaintStop],
        source: &'cache SourceOwner,
        offset: usize,
    ) -> Self {
        Self {
            paint,
            template,
            stops,
            source,
            offset,
        }
    }
    /// Atomic real graph handoff. The scene builder owns sharing and rollback.
    pub fn append_fill(
        &self,
        builder: &mut SceneBuilder,
        scene: SceneId,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        builder.append_template_fill(scene, &self.template, self.paint, self.stops, control)
    }
    pub fn append_stroke(
        &self,
        builder: &mut SceneBuilder,
        scene: SceneId,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        builder.append_template_stroke(scene, &self.template, self.paint, self.stops, control)
    }
    /// Explicitly request a separate owned payload. Unlike cached publication,
    /// this copies stops and returns the actual-capacity lease for its lifetime.
    pub fn to_owned(
        &self,
        ledger: &mut ChargeLedger,
        work: &mut dyn WorkControl,
    ) -> Result<OwnedPaint> {
        if !Arc::ptr_eq(self.source, ledger.source_owner()) {
            return Err(invalid(self.offset));
        }
        let mut control = PaintControl::new(ledger, work);
        let result = (|| {
            if self.stops.len() > crate::MAX_GRADIENT_STOPS {
                return Err(limit(LimitKind::PaintStops, self.offset));
            }
            if self.stops.len() < 2
                || self.paint.stops
                    != (vexel_scene::TableRange {
                        first: 0,
                        count: self.stops.len(),
                    })
            {
                return Err(invalid(self.offset));
            }
            let mut previous = 0.0;
            for (index, stop) in self.stops.iter().enumerate() {
                control
                    .charge(WorkKind::PaintVisit, 1)
                    .map_err(|_| limit(LimitKind::Work, self.offset))?;
                if !stop.offset.is_finite()
                    || !(0.0..=1.0).contains(&stop.offset)
                    || (index != 0 && stop.offset < previous)
                {
                    return Err(invalid(self.offset));
                }
                previous = stop.offset;
            }
            control
                .checkpoint()
                .map_err(|_| limit(LimitKind::Work, self.offset))?;
            let requested = self
                .stops
                .len()
                .checked_mul(std::mem::size_of::<PaintStop>())
                .ok_or_else(|| limit(LimitKind::Arithmetic, self.offset))?;
            if requested > control.ledger.available() {
                return Err(limit(LimitKind::ReservedBytes, self.offset));
            }
            let mut stops = Vec::new();
            stops
                .try_reserve_exact(self.stops.len())
                .map_err(|_| oom(self.offset))?;
            let lease = control.ledger.admit_vec(AllocationTag::None, &stops)?;
            let copied = (|| {
                let mut bytes = requested;
                while bytes != 0 {
                    let batch = bytes.min(256);
                    control
                        .charge(WorkKind::SourceRead, batch as u64)
                        .map_err(|_| limit(LimitKind::Work, self.offset))?;
                    bytes -= batch;
                }
                control
                    .checkpoint()
                    .map_err(|_| limit(LimitKind::Work, self.offset))?;
                Ok(())
            })();
            if let Err(error) = copied {
                control.ledger.release(lease)?;
                return Err(error);
            }
            stops.extend_from_slice(self.stops);
            Ok(OwnedPaint {
                resolved: ResolvedPaint {
                    paint: self.paint,
                    stops,
                    template: Some(self.template.clone()),
                },
                lease,
            })
        })();
        control.result_at(result, self.offset)
    }
}

/// A deliberately copied compatibility payload plus its releasable job lease.
#[must_use = "release the owned capacity lease, transfer both parts, or retain it for the source job"]
#[derive(Debug)]
pub struct OwnedPaint {
    resolved: ResolvedPaint,
    lease: AllocationLease,
}
impl OwnedPaint {
    pub fn as_resolved(&self) -> &ResolvedPaint {
        &self.resolved
    }
    /// Consume after publication/use, dropping the allocation and releasing it.
    /// A wrong ledger returns the error and owner intact for a correct retry.
    pub fn release(
        self,
        ledger: &mut ChargeLedger,
    ) -> std::result::Result<(), (vexel_scene::Diagnostic, Self)> {
        match ledger.release(self.lease) {
            Ok(()) => Ok(()),
            Err(error) => Err((error, self)),
        }
    }
    /// Transfer both payload and lease to a caller that manages its lifetime.
    pub fn into_parts(self) -> (ResolvedPaint, AllocationLease) {
        (self.resolved, self.lease)
    }
    /// Compatibility for the frozen DecodeOperations owned return type. Its
    /// allocation remains charged for the entire job because that type cannot
    /// transport a lease or a cache borrow. Prefer cached graph publication.
    pub fn into_job_owned(self) -> ResolvedPaint {
        self.resolved
    }
}
