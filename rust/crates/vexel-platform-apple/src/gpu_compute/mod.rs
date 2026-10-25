//! Thin Metal compute runner with no Vexel paint semantics.
//!
//! [`GpuDevice`] owns the system default device and one command queue;
//! [`GpuDevice::compile`] builds a [`Pipeline`] from MSL source (fast math
//! off); [`Buffer`]s are shared-storage; [`GpuDevice::dispatch`] and
//! [`GpuDevice::dispatch_sequence`] encode, commit and block the calling
//! (worker) thread until the GPU finishes, returning [`Timing`].
//!
//! Thread safety: `GpuDevice` and `Pipeline` are `Send + Sync` (objc2-metal
//! declares `MTLDevice`, `MTLCommandQueue` and `MTLComputePipelineState`
//! thread-safe; checked by static assertions below). `Buffer` is neither:
//! objc2-metal gives `MTLBuffer` no such guarantee, so buffers and
//! [`BufferPool`] stay on one thread. Every native call runs under
//! [`guard_native_call`](crate::guard_native_call).

mod buffer;
mod device;
mod error;
#[cfg(test)]
mod tests;

pub use buffer::{Buffer, BufferPool, MAX_BUFFER_BYTES, MAX_POOL_CACHED_BYTES, Plain};
pub use device::{
    DISPATCH_TIMEOUT, Dispatch, GpuDevice, MAX_BUFFERS_PER_DISPATCH, MAX_DISPATCHES,
    MAX_SOURCE_BYTES, Pipeline, Timing,
};
pub use error::{GpuError, GpuUnavailable};

const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<GpuDevice>();
    send_sync::<Pipeline>();
    send_sync::<GpuError>();
    send_sync::<Timing>();
};
