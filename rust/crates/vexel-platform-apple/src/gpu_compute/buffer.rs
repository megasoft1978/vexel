//! Shared-storage Metal buffers and a simple size-bucketed pool.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLResourceOptions};

use super::GpuDevice;
use super::error::GpuError;
use crate::guard_native_call;

/// Largest single buffer the runner will allocate.
pub const MAX_BUFFER_BYTES: usize = 256 * 1024 * 1024;

mod sealed {
    pub trait Sealed {}
}

/// Plain-old-data element types accepted by the typed buffer accessors.
/// Every bit pattern is valid and there is no padding.
pub trait Plain: Copy + sealed::Sealed {}
macro_rules! plain {
    ($($t:ty),*) => {$(impl sealed::Sealed for $t {} impl Plain for $t {})*};
}
plain!(u8, i8, u16, i16, u32, i32, u64, i64, f32);

/// A `MTLStorageModeShared` buffer.
///
/// Not `Send`/`Sync`: objc2-metal does not declare `MTLBuffer` thread-safe, so
/// buffers stay on the thread that created them (the dispatching worker).
pub struct Buffer {
    pub(super) raw: Retained<ProtocolObject<dyn MTLBuffer>>,
    len: usize,
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer").field("len", &self.len).finish()
    }
}

fn oob(what: &str, offset: usize, n: usize, len: usize) -> GpuError {
    GpuError::Invalid(format!(
        "{what} range {offset}+{n} exceeds buffer of {len} bytes"
    ))
}

impl Buffer {
    /// Allocates a zero-initialised buffer of `len` bytes (1..=[`MAX_BUFFER_BYTES`]).
    pub fn new(device: &GpuDevice, len: usize) -> Result<Self, GpuError> {
        if len == 0 || len > MAX_BUFFER_BYTES || len as u64 > device.max_buffer_length() {
            return Err(GpuError::Invalid(format!(
                "buffer size {len} outside 1..={MAX_BUFFER_BYTES} (device max {})",
                device.max_buffer_length()
            )));
        }
        let raw = guard_native_call(|| {
            device
                .device
                .newBufferWithLength_options(len, MTLResourceOptions::StorageModeShared)
        })?
        .ok_or_else(|| GpuError::Alloc(format!("newBuffer({len}) returned nil")))?;
        let buffer = Self { raw, len };
        // Shared buffers are documented zeroed on creation; make it explicit.
        buffer.fill(0)?;
        Ok(buffer)
    }

    /// Allocates a buffer holding a copy of `data`.
    pub fn from_slice(device: &GpuDevice, data: &[u8]) -> Result<Self, GpuError> {
        let buffer = Self::new(device, data.len())?;
        buffer.write(0, data)?;
        Ok(buffer)
    }

    /// Allocates a buffer holding a copy of typed `data`.
    pub fn from_typed<T: Plain>(device: &GpuDevice, data: &[T]) -> Result<Self, GpuError> {
        let buffer = Self::new(device, std::mem::size_of_val(data))?;
        buffer.write_typed(0, data)?;
        Ok(buffer)
    }

    /// Size in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Always false: zero-length buffers are rejected.
    pub fn is_empty(&self) -> bool {
        false
    }

    fn base(&self) -> *mut u8 {
        // SAFETY: `contents` is a plain property read; shared buffers are
        // CPU-mapped for their whole lifetime.
        let p: std::ptr::NonNull<c_void> = self.raw.contents();
        p.as_ptr().cast()
    }

    /// Copies `data` into the buffer at byte `offset`.
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<(), GpuError> {
        let end = offset.checked_add(data.len());
        if end.is_none_or(|e| e > self.len) {
            return Err(oob("write", offset, data.len(), self.len));
        }
        // SAFETY: range checked above; source and destination cannot overlap
        // (the buffer's storage is owned by Metal, `data` is a Rust slice).
        // No GPU work is in flight: dispatch waits for completion before returning.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.base().add(offset), data.len())
        };
        Ok(())
    }

    /// Copies `out.len()` bytes starting at byte `offset` into `out`.
    pub fn read(&self, offset: usize, out: &mut [u8]) -> Result<(), GpuError> {
        let end = offset.checked_add(out.len());
        if end.is_none_or(|e| e > self.len) {
            return Err(oob("read", offset, out.len(), self.len));
        }
        // SAFETY: range checked above; see `write` for aliasing and GPU idleness.
        unsafe {
            std::ptr::copy_nonoverlapping(self.base().add(offset), out.as_mut_ptr(), out.len())
        };
        Ok(())
    }

    /// Typed write at byte `offset`.
    pub fn write_typed<T: Plain>(&self, offset: usize, data: &[T]) -> Result<(), GpuError> {
        // SAFETY: `Plain` types have no padding, so viewing them as bytes is sound.
        let bytes = unsafe {
            std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), std::mem::size_of_val(data))
        };
        self.write(offset, bytes)
    }

    /// Typed read at byte `offset`.
    pub fn read_typed<T: Plain>(&self, offset: usize, out: &mut [T]) -> Result<(), GpuError> {
        // SAFETY: `Plain` types accept every bit pattern and have no padding.
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                out.as_mut_ptr().cast::<u8>(),
                std::mem::size_of_val(out),
            )
        };
        self.read(offset, bytes)
    }

    /// Sets every byte to `value`.
    pub fn fill(&self, value: u8) -> Result<(), GpuError> {
        // SAFETY: whole-buffer write inside the mapped range.
        unsafe { std::ptr::write_bytes(self.base(), value, self.len) };
        Ok(())
    }
}

/// Reuses buffers by power-of-two size class. Single-threaded by design
/// (buffers are not `Send`); cached bytes are capped.
#[derive(Debug, Default)]
pub struct BufferPool {
    free: Vec<(usize, Buffer)>,
    cached_bytes: usize,
}

/// Upper bound on bytes the pool keeps idle.
pub const MAX_POOL_CACHED_BYTES: usize = 128 * 1024 * 1024;

impl BufferPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes currently idle in the pool.
    pub fn cached_bytes(&self) -> usize {
        self.cached_bytes
    }

    /// Returns a zeroed buffer with `len() >= len` (rounded up to a power of two).
    pub fn acquire(&mut self, device: &GpuDevice, len: usize) -> Result<Buffer, GpuError> {
        let class = len
            .checked_next_power_of_two()
            .filter(|c| (1..=MAX_BUFFER_BYTES).contains(c))
            .ok_or_else(|| GpuError::Invalid(format!("pool request {len} out of range")))?;
        if let Some(i) = self.free.iter().position(|(c, _)| *c == class) {
            let (_, buffer) = self.free.swap_remove(i);
            self.cached_bytes -= class;
            buffer.fill(0)?;
            return Ok(buffer);
        }
        Buffer::new(device, class)
    }

    /// Returns a buffer to the pool, or drops it if the cache is full or the
    /// buffer was not pool-sized.
    pub fn release(&mut self, buffer: Buffer) {
        let class = buffer.len();
        if class.is_power_of_two() && self.cached_bytes + class <= MAX_POOL_CACHED_BYTES {
            self.cached_bytes += class;
            self.free.push((class, buffer));
        }
    }
}
