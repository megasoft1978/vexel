use core::fmt;

/// Why a scene cannot take the GPU display route. The caller renders it on
/// the CPU instead; an unsupported scene is never an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedReason {
    Filter,
    Mask,
    PatternPaint,
    ImagePaint,
    Text,
    AnimationOverlay,
    /// Group nesting deeper than [`crate::layout::MAX_GROUP_DEPTH`].
    GroupTooDeep,
    /// Clip chain longer than [`crate::layout::MAX_CLIP_CHAIN`].
    ClipChainTooLong,
    TooManyShapes,
    TooManyEdges,
    TooManyTileEntries,
    TooManyGradientStops,
    /// Canvas larger than the GPU path's texture or buffer budget.
    CanvasTooLarge,
    /// Stroked shape; supported by a later lane.
    Strokes,
    /// Opacity group; supported by a later lane.
    Groups,
    /// Clipped shape; supported by a later lane.
    Clips,
    /// Unused since layout version 2 (the header carries the content
    /// rectangle); kept for compatibility.
    Letterbox,
    /// Non-finite or out-of-range coordinates, matrices or canvas size.
    InvalidScene,
}

/// Result of classifying a scene before any GPU work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Supported,
    Unsupported(UnsupportedReason),
}

impl Capability {
    pub fn is_supported(self) -> bool {
        matches!(self, Capability::Supported)
    }
}

/// Runtime failure of the GPU display route. Each one counts toward the
/// fallback latch ([`crate::policy::LATCH_ERROR_LIMIT`]) and the frame is
/// rendered on the CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuError {
    /// `VEXEL_GPU=0` or the latch tripped.
    Disabled,
    DeviceUnavailable,
    ShaderCompile,
    PipelineCreate,
    BufferAlloc,
    CommandFailed,
    Timeout,
    /// The scene was classified supported but encoding exceeded a budget.
    Encode(UnsupportedReason),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuError::Disabled => f.write_str("GPU rasterizer disabled"),
            GpuError::DeviceUnavailable => f.write_str("no Metal device"),
            GpuError::ShaderCompile => f.write_str("Metal shader compile failed"),
            GpuError::PipelineCreate => f.write_str("Metal pipeline creation failed"),
            GpuError::BufferAlloc => f.write_str("Metal buffer allocation failed"),
            GpuError::CommandFailed => f.write_str("Metal command buffer failed"),
            GpuError::Timeout => f.write_str("Metal command buffer timed out"),
            GpuError::Encode(reason) => write!(f, "scene encoding exceeded a budget: {reason:?}"),
        }
    }
}

impl std::error::Error for GpuError {}
