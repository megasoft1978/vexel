//! Device, pipeline compilation and dispatch.

use std::ptr::NonNull;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSError, NSString};
use objc2_metal::{
    MTLCommandBuffer, MTLCommandBufferStatus, MTLCommandEncoder, MTLCommandQueue,
    MTLCompileOptions, MTLComputeCommandEncoder, MTLComputePipelineState,
    MTLCreateSystemDefaultDevice, MTLDevice, MTLLibrary, MTLSize,
};

use super::buffer::Buffer;
use super::error::{GpuError, GpuUnavailable};
use crate::guard_native_call;

/// Wall-clock limit for one command buffer (Metal's own watchdog usually fires first).
pub const DISPATCH_TIMEOUT: Duration = Duration::from_secs(10);
/// Largest accepted MSL source.
pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
/// Metal's per-encoder buffer argument table size.
pub const MAX_BUFFERS_PER_DISPATCH: usize = 31;
/// Dispatches allowed in one command buffer.
pub const MAX_DISPATCHES: usize = 256;

const METAL_TIMEOUT_CODE: u64 = 2;

/// System default Metal device plus one command queue.
pub struct GpuDevice {
    pub(super) device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    max_buffer_length: u64,
    timeout: Duration,
}

impl std::fmt::Debug for GpuDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuDevice")
            .field("name", &self.name())
            .finish()
    }
}

/// A compiled compute pipeline.
pub struct Pipeline {
    state: Retained<ProtocolObject<dyn MTLComputePipelineState>>,
    max_threads: usize,
    thread_width: usize,
}

impl Pipeline {
    /// Largest threads-per-threadgroup this pipeline supports.
    pub fn max_threads_per_group(&self) -> usize {
        self.max_threads
    }

    /// SIMD width of the pipeline.
    pub fn thread_execution_width(&self) -> usize {
        self.thread_width
    }
}

/// One dispatch inside a command buffer. `grid` is the total thread count
/// (`dispatchThreads`, non-uniform groups allowed on Apple GPUs).
#[derive(Clone, Copy)]
pub struct Dispatch<'a> {
    pub pipeline: &'a Pipeline,
    /// Bound at argument indices 0.. in order.
    pub buffers: &'a [&'a Buffer],
    pub grid: (u32, u32, u32),
    pub threads_per_group: (u32, u32, u32),
}

/// Timing of one committed command buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    /// Commit-to-completion wall time as seen by the caller.
    pub wall: Duration,
    /// GPU execution time (`GPUEndTime - GPUStartTime`) if Metal reported it.
    pub gpu: Option<Duration>,
}

fn ns_error_text(e: &NSError) -> String {
    e.localizedDescription().to_string()
}

impl GpuDevice {
    /// Opens the system default device. Never panics when no GPU exists.
    pub fn new() -> Result<Self, GpuUnavailable> {
        let unavailable = |reason| GpuUnavailable { reason };
        let opened = guard_native_call(|| {
            let device = MTLCreateSystemDefaultDevice()?;
            let queue = device.newCommandQueue()?;
            let max_buffer_length = device.maxBufferLength() as u64;
            Some(Self {
                device,
                queue,
                max_buffer_length,
                timeout: DISPATCH_TIMEOUT,
            })
        })
        .map_err(|_| unavailable("native fault while opening the Metal device"))?;
        opened.ok_or_else(|| unavailable("no Metal device or command queue"))
    }

    /// Device name, e.g. "Apple M3".
    pub fn name(&self) -> String {
        guard_native_call(|| self.device.name().to_string()).unwrap_or_default()
    }

    /// Device limit on a single buffer.
    pub fn max_buffer_length(&self) -> u64 {
        self.max_buffer_length
    }

    /// Overrides the wall-clock completion timeout.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Compiles MSL `source` (fast math off) and builds the pipeline for `entry`.
    pub fn compile(&self, source: &str, entry: &str) -> Result<Pipeline, GpuError> {
        if source.is_empty() || source.len() > MAX_SOURCE_BYTES {
            return Err(GpuError::Invalid(format!(
                "MSL source must be 1..={MAX_SOURCE_BYTES} bytes"
            )));
        }
        if entry.is_empty() {
            return Err(GpuError::Invalid("empty entry point name".to_owned()));
        }
        guard_native_call(|| {
            let options = MTLCompileOptions::new();
            #[allow(deprecated)]
            options.setFastMathEnabled(false);
            let library = self
                .device
                .newLibraryWithSource_options_error(&NSString::from_str(source), Some(&options))
                .map_err(|e| GpuError::Compile(ns_error_text(&e)))?;
            let function = library
                .newFunctionWithName(&NSString::from_str(entry))
                .ok_or_else(|| GpuError::Pipeline(format!("entry point `{entry}` not found")))?;
            let state = self
                .device
                .newComputePipelineStateWithFunction_error(&function)
                .map_err(|e| GpuError::Pipeline(ns_error_text(&e)))?;
            Ok(Pipeline {
                max_threads: state.maxTotalThreadsPerThreadgroup(),
                thread_width: state.threadExecutionWidth(),
                state,
            })
        })?
    }

    /// Runs one dispatch to completion on the calling thread.
    pub fn dispatch(
        &self,
        pipeline: &Pipeline,
        buffers: &[&Buffer],
        grid: (u32, u32, u32),
        threads_per_group: (u32, u32, u32),
    ) -> Result<Timing, GpuError> {
        self.dispatch_sequence(&[Dispatch {
            pipeline,
            buffers,
            grid,
            threads_per_group,
        }])
    }

    /// Runs several dispatches, in order, inside one command buffer and one
    /// serial compute encoder (Metal inserts barriers between dispatches), then
    /// blocks until completion.
    pub fn dispatch_sequence(&self, dispatches: &[Dispatch<'_>]) -> Result<Timing, GpuError> {
        validate(dispatches)?;
        guard_native_call(|| self.run(dispatches))?
    }

    fn run(&self, dispatches: &[Dispatch<'_>]) -> Result<Timing, GpuError> {
        let command_buffer = self
            .queue
            .commandBuffer()
            .ok_or_else(|| GpuError::Alloc("commandBuffer returned nil".to_owned()))?;
        let encoder = command_buffer
            .computeCommandEncoder()
            .ok_or_else(|| GpuError::Alloc("computeCommandEncoder returned nil".to_owned()))?;
        for d in dispatches {
            encoder.setComputePipelineState(&d.pipeline.state);
            for (index, buffer) in d.buffers.iter().enumerate() {
                // SAFETY: index < 31 (validated); the buffer is a live
                // MTLBuffer retained by the encoder until completion.
                unsafe { encoder.setBuffer_offset_atIndex(Some(&buffer.raw), 0, index) };
            }
            let size = |(x, y, z): (u32, u32, u32)| MTLSize {
                width: x as usize,
                height: y as usize,
                depth: z as usize,
            };
            encoder.dispatchThreads_threadsPerThreadgroup(size(d.grid), size(d.threads_per_group));
        }
        encoder.endEncoding();

        let (tx, rx) = mpsc::channel::<()>();
        let handler = RcBlock::new(move |_: NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
            let _ = tx.send(());
        });
        let started = Instant::now();
        // SAFETY: the block pointer is valid for the call; Metal copies it.
        unsafe { command_buffer.addCompletedHandler(RcBlock::as_ptr(&handler)) };
        command_buffer.commit();
        if rx.recv_timeout(self.timeout).is_err() {
            return Err(GpuError::Timeout(format!(
                "command buffer incomplete after {:?}",
                self.timeout
            )));
        }
        let wall = started.elapsed();

        if command_buffer.status() == MTLCommandBufferStatus::Error {
            return Err(match command_buffer.error() {
                Some(e) if e.code() as u64 == METAL_TIMEOUT_CODE => {
                    GpuError::Timeout(ns_error_text(&e))
                }
                Some(e) => GpuError::Execution {
                    code: e.code() as u64,
                    message: ns_error_text(&e),
                },
                None => GpuError::Execution {
                    code: 0,
                    message: "unknown error".to_owned(),
                },
            });
        }
        let (start, end) = (command_buffer.GPUStartTime(), command_buffer.GPUEndTime());
        let gpu = (end > start && start > 0.0).then(|| Duration::from_secs_f64(end - start));
        Ok(Timing { wall, gpu })
    }
}

fn validate(dispatches: &[Dispatch<'_>]) -> Result<(), GpuError> {
    let bad = |m: String| Err(GpuError::Invalid(m));
    if dispatches.is_empty() || dispatches.len() > MAX_DISPATCHES {
        return bad(format!("dispatch count must be 1..={MAX_DISPATCHES}"));
    }
    for (i, d) in dispatches.iter().enumerate() {
        if d.buffers.len() > MAX_BUFFERS_PER_DISPATCH {
            return bad(format!(
                "dispatch {i}: more than {MAX_BUFFERS_PER_DISPATCH} buffers"
            ));
        }
        let (gx, gy, gz) = d.grid;
        let (tx, ty, tz) = d.threads_per_group;
        if [gx, gy, gz, tx, ty, tz].contains(&0) {
            return bad(format!("dispatch {i}: zero grid or group dimension"));
        }
        let per_group = u64::from(tx) * u64::from(ty) * u64::from(tz);
        if per_group > d.pipeline.max_threads as u64 {
            return bad(format!(
                "dispatch {i}: {per_group} threads per group exceeds pipeline max {}",
                d.pipeline.max_threads
            ));
        }
    }
    Ok(())
}
