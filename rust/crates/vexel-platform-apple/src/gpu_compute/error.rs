//! Error types for the Metal compute runner.

use crate::NativeFault;

/// No usable Metal device (or queue) exists on this machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuUnavailable {
    pub reason: &'static str,
}

impl std::fmt::Display for GpuUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU unavailable: {}", self.reason)
    }
}

impl std::error::Error for GpuUnavailable {}

/// Compute runner failure.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GpuError {
    /// The request violated a documented limit; nothing reached Metal.
    Invalid(String),
    /// MSL compilation failed; carries the Metal compiler log text.
    Compile(String),
    /// Pipeline state creation failed (unknown entry point, resource limits).
    Pipeline(String),
    /// Buffer allocation failed.
    Alloc(String),
    /// The command buffer did not finish in time (Metal watchdog or wall clock).
    Timeout(String),
    /// The command buffer finished with an error status.
    Execution { code: u64, message: String },
    /// A Rust panic or Objective-C exception was caught at the boundary.
    Native(NativeFault),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(m) => write!(f, "invalid GPU request: {m}"),
            Self::Compile(m) => write!(f, "MSL compile failed: {m}"),
            Self::Pipeline(m) => write!(f, "GPU pipeline creation failed: {m}"),
            Self::Alloc(m) => write!(f, "GPU allocation failed: {m}"),
            Self::Timeout(m) => write!(f, "GPU timeout: {m}"),
            Self::Execution { code, message } => {
                write!(f, "GPU execution failed (code {code}): {message}")
            }
            Self::Native(fault) => write!(f, "native fault: {:?}", fault.kind()),
        }
    }
}

impl std::error::Error for GpuError {}

impl From<NativeFault> for GpuError {
    fn from(fault: NativeFault) -> Self {
        Self::Native(fault)
    }
}
