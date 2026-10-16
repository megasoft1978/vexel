//! Bounded, caller-configured work accounting and cancellation.
//!
//! Charges count completed work and are intentionally not rolled back when a
//! caller later rolls back a higher-level source transaction. Callers should
//! charge/checkpoint in batches of at most 256 work units.

use std::sync::atomic::{AtomicBool, Ordering};

/// Categories shared by incremental parsers, renderers, and related workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum WorkKind {
    SourceRead,
    TokenVisit,
    ReferenceProbe,
    Transform,
    Expansion,
    Edge,
    PaintVisit,
    ClipChain,
    ClipChild,
    ClipEdge,
    ShapePixel,
    ShapeSample,
    FillEdgeTest,
    FillIntervalTest,
    StrokeTest,
    ScanlineEdgeEntry,
    ShapeRowVisit,
    BoundaryMark,
    FilterPixel,
    AnimationTransform,
    Media,
}

impl WorkKind {
    pub const ALL: [Self; 21] = [
        Self::SourceRead, Self::TokenVisit, Self::ReferenceProbe, Self::Transform,
        Self::Expansion, Self::Edge, Self::PaintVisit, Self::ClipChain, Self::ClipChild,
        Self::ClipEdge, Self::ShapePixel, Self::ShapeSample, Self::FillEdgeTest,
        Self::FillIntervalTest, Self::StrokeTest, Self::ScanlineEdgeEntry,
        Self::ShapeRowVisit, Self::BoundaryMark, Self::FilterPixel,
        Self::AnimationTransform, Self::Media,
    ];

    const fn index(self) -> usize { self as usize }
}

/// The first reason a work-control job stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop { Canceled, Limit, Overflow }

/// Error returned by a controlled operation, preserving its original error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledError<E> {
    Original(E),
    Canceled,
    Limit,
    Overflow,
    OutOfMemory,
}

impl<E> From<Stop> for ControlledError<E> {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::Canceled => Self::Canceled,
            Stop::Limit => Self::Limit,
            Stop::Overflow => Self::Overflow,
        }
    }
}

/// A real result shape for work performed by a controlled worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledStep<T, E> {
    Pending { charged_units: u64 },
    Complete(T),
    Failed(ControlledError<E>),
}

/// Object-safe boundary for bounded work and cancellation checks.
pub trait WorkControl {
    fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop>;
    fn checkpoint(&mut self) -> Result<(), Stop>;
}

/// Fixed-size per-kind caps. Unconfigured kinds default to `u64::MAX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkLimits([u64; WorkKind::ALL.len()]);

impl WorkLimits {
    /// Create a policy with no per-kind cap below the `u64` counter maximum.
    pub const fn unlimited() -> Self { Self([u64::MAX; WorkKind::ALL.len()]) }

    /// Set one kind's cap, returning the updated fixed-size policy.
    pub const fn with_limit(mut self, kind: WorkKind, limit: u64) -> Self {
        self.0[kind.index()] = limit;
        self
    }

    const fn get(self, kind: WorkKind) -> u64 { self.0[kind.index()] }
}

impl Default for WorkLimits {
    fn default() -> Self { Self::unlimited() }
}

/// Immutable snapshot of successfully accepted work units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkStats {
    total_units: u64,
    per_kind: [u64; WorkKind::ALL.len()],
}

impl WorkStats {
    pub const fn total_units(&self) -> u64 { self.total_units }
    pub const fn units(&self, kind: WorkKind) -> u64 { self.per_kind[kind.index()] }
}

/// Safe budget and cancellation state borrowing the caller's real cancel flag.
pub struct BudgetControl<'a> {
    cancel: &'a AtomicBool,
    total_limit: u64,
    kind_limits: WorkLimits,
    stats: WorkStats,
    stopped: Option<Stop>,
}

impl<'a> BudgetControl<'a> {
    /// Construct with an explicit total limit and unlimited per-kind limits.
    pub const fn new(cancel: &'a AtomicBool, total_limit: u64) -> Self {
        Self::with_limits(cancel, total_limit, WorkLimits::unlimited())
    }

    /// Construct with explicit total and per-kind policies.
    pub const fn with_limits(cancel: &'a AtomicBool, total_limit: u64, kind_limits: WorkLimits) -> Self {
        Self {
            cancel,
            total_limit,
            kind_limits,
            stats: WorkStats { total_units: 0, per_kind: [0; WorkKind::ALL.len()] },
            stopped: None,
        }
    }

    /// Snapshot accepted counters. Failed charges never change this value.
    pub const fn stats(&self) -> WorkStats { self.stats }

    fn stop(&mut self, reason: Stop) -> Stop {
        *self.stopped.get_or_insert(reason)
    }

    fn cancellation_requested(&self) -> bool {
        // Cancellation is a signal only; callers must synchronize unrelated
        // shared data independently. Acquire pairs with conventional Release stores.
        self.cancel.load(Ordering::Acquire)
    }
}

impl WorkControl for BudgetControl<'_> {
    fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop> {
        if let Some(stop) = self.stopped { return Err(stop); }
        if self.cancellation_requested() { return Err(self.stop(Stop::Canceled)); }

        let Some(total) = self.stats.total_units.checked_add(units) else {
            return Err(self.stop(Stop::Overflow));
        };
        let current_kind = self.stats.per_kind[kind.index()];
        let Some(per_kind) = current_kind.checked_add(units) else {
            return Err(self.stop(Stop::Overflow));
        };
        if total > self.total_limit || per_kind > self.kind_limits.get(kind) {
            return Err(self.stop(Stop::Limit));
        }
        self.stats.total_units = total;
        self.stats.per_kind[kind.index()] = per_kind;
        Ok(())
    }

    fn checkpoint(&mut self) -> Result<(), Stop> {
        if let Some(stop) = self.stopped { return Err(stop); }
        if self.cancellation_requested() { return Err(self.stop(Stop::Canceled)); }
        Ok(())
    }
}
