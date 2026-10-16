//! Read-only scene selection and explicit allocation admission.
#![forbid(unsafe_code)]

mod allocation;
mod selection;
pub use allocation::*;
pub use selection::*;
