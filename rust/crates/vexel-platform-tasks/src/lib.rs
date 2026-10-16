#![forbid(unsafe_code)]
//! Bounded native task execution for the macOS platform layer.

mod queue;

pub use queue::{
    CancellationToken, QueueConfig, QueueError, SubmitError, TaskCallback, TaskOutput, TaskQueue,
    TaskResult,
};
