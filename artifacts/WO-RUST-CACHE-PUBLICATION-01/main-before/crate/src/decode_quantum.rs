//! Soft scheduling boundary between bounded synchronous operations. This is not
//! interruption/resumption inside an operation and promises no main-thread latency.
use crate::{types::invalid, *};
#[derive(Debug, PartialEq, Eq)]
pub enum AtomicStep<T, P = ()> {
    Complete(T),
    Boundary(P),
}
/// One step's cumulative actual work, including atomic overshoot and failed calls.
/// Once at the boundary, no further operation starts. A new quantum continues
/// from the caller's retained engine state, never replays a completed operation.
pub struct DecodeQuantum {
    requested: u64,
    work: RenderWorkStats,
}
impl DecodeQuantum {
    pub fn new(requested: u64) -> Result<Self> {
        if requested == 0 {
            return Err(invalid(0));
        }
        Ok(Self {
            requested,
            work: RenderWorkStats::default(),
        })
    }
    pub fn work(&self) -> RenderWorkStats {
        self.work
    }
    pub fn overshoot(&self) -> u64 {
        self.work.total_work.saturating_sub(self.requested)
    }
    /// Exactly one existing bounded shared atomic operation per call. Lower
    /// adapters must use their own original bounded atomic primitive and pass
    /// this control to every actual loop. No invented token-only progress.
    pub fn atomic<T>(
        &mut self,
        control: &mut dyn WorkControl,
        operation: impl FnOnce(&mut dyn WorkControl) -> Result<T>,
    ) -> Result<AtomicStep<T>> {
        if self.work.total_work >= self.requested {
            return Ok(AtomicStep::Boundary(()));
        }
        let mut meter = QuantumControl {
            work: &mut self.work,
            inner: control,
        };
        meter.checkpoint().map_err(crate::types::stopped)?;
        operation(&mut meter).map(AtomicStep::Complete)
    }
    pub fn compute_style<'src>(
        &mut self,
        source: &mut DecodeSource<'src>,
        element: ElementId,
        parent: Option<&CssPaint<'src>>,
        control: &mut dyn WorkControl,
    ) -> Result<AtomicStep<CssPaint<'src>>> {
        self.atomic(control, |c| source.compute_style(element, parent, c))
    }
    pub fn find_reference(
        &mut self,
        source: &mut DecodeSource<'_>,
        value: vexel_xml::XmlSpan,
        control: &mut dyn WorkControl,
    ) -> Result<AtomicStep<Option<ElementId>>> {
        self.atomic(control, |c| source.find_reference(value, c))
    }
    pub fn append_shape(
        &mut self,
        builder: &mut SceneBuilder,
        scene: SceneId,
        input: ShapeInput,
        control: &mut dyn WorkControl,
    ) -> Result<AtomicStep<ShapeId, ShapeInput>> {
        if self.work.total_work >= self.requested {
            return Ok(AtomicStep::Boundary(input));
        }
        let mut meter = QuantumControl {
            work: &mut self.work,
            inner: control,
        };
        meter.checkpoint().map_err(crate::types::stopped)?;
        builder
            .append_shape(scene, input, &mut meter)
            .map(AtomicStep::Complete)
    }
    pub fn commit_request(
        &mut self,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        request: RequestToken,
        control: &mut dyn WorkControl,
    ) -> Result<AtomicStep<()>> {
        self.atomic(control, |c| builder.commit_request(draft, request, c))
    }
    pub fn commit_draft(
        &mut self,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<AtomicStep<()>> {
        self.atomic(control, |c| builder.commit_draft(draft, c))
    }
}
struct QuantumControl<'a> {
    work: &'a mut RenderWorkStats,
    inner: &'a mut dyn WorkControl,
}
impl WorkControl for QuantumControl<'_> {
    fn charge(&mut self, kind: WorkKind, units: u64) -> std::result::Result<(), Stop> {
        // Count reported work at the same call site even if the outer policy stops.
        // Overflow is detected before a provider can perform unreportable work.
        self.work.charge(kind, units).map_err(|_| Stop::Overflow)?;
        self.inner.charge(kind, units)
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        self.inner.checkpoint()
    }
}
