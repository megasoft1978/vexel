#![forbid(unsafe_code)]
//! Native project save and crash-recovery file APIs.

mod file;
mod file_load;
mod recovery;

pub use file::{
    FileError, FileResult, project_file_error_name, project_file_save_as, project_file_save_encoded,
};
pub use file_load::{LoadFailure, LoadedProject, project_file_load};
pub use recovery::{
    RECOVERY_HEADER_BYTES, RECOVERY_MAX_BYTES, RECOVERY_MAX_PATH_BYTES, RECOVERY_VERSION,
    RecoveryError, RecoveryFailure, RecoveryInput, RecoveryResult, RecoveryView,
    project_recovery_decode, project_recovery_encode_into, project_recovery_encoded_size,
    project_recovery_error_name, project_recovery_save,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod load_tests;
