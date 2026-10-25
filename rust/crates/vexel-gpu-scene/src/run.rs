//! Glue from an input scene to RGBA8 pixels through a [`ComputeBackend`].
//!
//! This crate stays safe code and free of Metal: the backend (implemented by
//! `vexel-platform-apple`'s runner through a small adapter in the app, and by
//! a test adapter in `tests/`) compiles the kernel source, binds the byte
//! buffers in order, dispatches and returns the output buffer. All calls happen
//! on one thread: Metal buffers are neither `Send` nor `Sync`.

use std::time::{Duration, Instant};

use crate::bytes::{
    edge_bytes, gradient_bytes, header_bytes, list_bytes, shape_bytes, stop_bytes, tile_bytes,
};
use crate::classify::classify;
use crate::encode::{Encoded, Encoder};
use crate::error::{Capability, GpuError, UnsupportedReason};
use crate::input::GpuScene;
use crate::layout::TILE_SIZE;

/// Entry point name of the rasterizer kernel.
pub const KERNEL_ENTRY: &str = "vx_raster";

/// Complete MSL source (standard header, frozen layout, kernel).
pub fn kernel_source() -> String {
    format!(
        "#include <metal_stdlib>\nusing namespace metal;\n{}\n{}",
        include_str!("../shaders/layout.h"),
        include_str!("../shaders/raster.metal")
    )
}

/// One kernel launch. `inputs[i]` is bound at buffer index `i` (never empty:
/// the glue pads empty arrays); the buffer at index `inputs.len()` is the
/// zero-initialised output, the size of `out`, written to `out`.
pub struct ComputeRequest<'a> {
    pub source: &'a str,
    pub entry: &'a str,
    pub inputs: &'a [&'a [u8]],
    pub grid: (u32, u32, u32),
    pub threads_per_group: (u32, u32, u32),
}

/// Timing a backend reports for one launch.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComputeStats {
    /// Kernel execution time measured by the GPU, when available.
    pub gpu: Option<Duration>,
    /// Submit-to-completion wall time.
    pub wall: Duration,
}

pub trait ComputeBackend {
    /// Runs the request and fills `out` from the output buffer. A backend may
    /// cache the compiled pipeline by `(source, entry)`.
    fn run(
        &mut self,
        request: &ComputeRequest<'_>,
        out: &mut [u8],
    ) -> Result<ComputeStats, GpuError>;
}

/// Why a frame was not rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    /// Render on the CPU instead; not a runtime failure.
    Unsupported(UnsupportedReason),
    /// Runtime failure; counts toward the fallback latch.
    Gpu(GpuError),
}

/// Pixel format of the returned buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Straight,
    Premultiplied,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderTimes {
    pub encode: Duration,
    pub backend: ComputeStats,
    pub total: Duration,
}

pub struct GpuRasterizer<B: ComputeBackend> {
    backend: B,
    encoder: Encoder,
    encoded: Encoded,
    source: String,
}

fn padded(bytes: Vec<u8>) -> Vec<u8> {
    if bytes.is_empty() { vec![0; 16] } else { bytes }
}

impl<B: ComputeBackend> GpuRasterizer<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            encoder: Encoder::default(),
            encoded: Encoded::default(),
            source: kernel_source(),
        }
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// The scene encoded by the last successful render (for tests).
    pub fn encoded(&self) -> &Encoded {
        &self.encoded
    }

    /// Renders to RGBA8, row-major, `width * height * 4` bytes.
    pub fn render(&mut self, scene: &GpuScene, output: Output) -> Result<Vec<u8>, RenderError> {
        self.render_timed(scene, output).map(|(px, _)| px)
    }

    pub fn render_timed(
        &mut self,
        scene: &GpuScene,
        output: Output,
    ) -> Result<(Vec<u8>, RenderTimes), RenderError> {
        let start = Instant::now();
        if let Capability::Unsupported(reason) = classify(scene) {
            return Err(RenderError::Unsupported(reason));
        }
        self.encoder
            .encode_into(scene, &mut self.encoded)
            .map_err(|e| match e {
                GpuError::Encode(reason) => RenderError::Unsupported(reason),
                other => RenderError::Gpu(other),
            })?;
        let encode = start.elapsed();
        let e = &self.encoded;
        let mode = u32::from(output == Output::Premultiplied).to_le_bytes();
        let buffers = [
            header_bytes(&e.header),
            padded(shape_bytes(&e.shapes)),
            padded(edge_bytes(&e.edges)),
            padded(tile_bytes(&e.tiles)),
            padded(list_bytes(&e.tile_list)),
            padded(gradient_bytes(&e.gradients)),
            padded(stop_bytes(&e.stops)),
        ];
        let mut inputs: Vec<&[u8]> = buffers.iter().map(Vec::as_slice).collect();
        inputs.push(&mode);
        let mut pixels = vec![0u8; e.header.width as usize * e.header.height as usize * 4];
        let request = ComputeRequest {
            source: &self.source,
            entry: KERNEL_ENTRY,
            inputs: &inputs,
            grid: (
                e.header.tiles_x * TILE_SIZE,
                e.header.tiles_y * TILE_SIZE,
                1,
            ),
            threads_per_group: (TILE_SIZE, TILE_SIZE, 1),
        };
        let backend = self
            .backend
            .run(&request, &mut pixels)
            .map_err(RenderError::Gpu)?;
        let times = RenderTimes {
            encode,
            backend,
            total: start.elapsed(),
        };
        Ok((pixels, times))
    }
}
