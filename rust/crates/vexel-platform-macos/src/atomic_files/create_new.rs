use super::path::{
    create_temp, parent_bytes, remove_temp, target_path, validate_new_name, PathError, TempError,
};
use super::{
    write_all, AtomicFileCreateNewError as Error, AtomicFileCreateNewHooks as Hooks,
    AtomicFileCreateNewResult as ResultState,
};
use std::fs::{self, File};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const TEMP_PREFIX: &[u8] = b".vexel-new-";

fn result(error: Error) -> ResultState {
    ResultState {
        error,
        file_synced: false,
        published: false,
        directory_synced: false,
    }
}

fn cleanup_temp(file: Option<File>, path: &Path) {
    if let Some(file) = file {
        let _ = crate::sys::close_checked(file.into());
    }
    remove_temp(path);
}

/// Write and atomically publish a sibling file only if `path` does not exist.
///
/// The target is a Unix byte path of 1..=1023 bytes, without NUL and with a
/// nonempty final component other than `.` or `..`. A hard link is the atomic
/// no-replace commit point. Directory sync errors retain `published == true`.
pub fn atomic_file_create_new(
    path: &[u8],
    bytes: &[u8],
    mut hooks: Option<&mut dyn Hooks>,
) -> ResultState {
    let target = match target_path(path) {
        Ok(path) => path,
        Err(PathError::Invalid) => return result(Error::Invalid),
        Err(PathError::Limit) => return result(Error::PathLimit),
    };
    if let Err(error) = validate_new_name(path) {
        return result(match error {
            PathError::Invalid => Error::Invalid,
            PathError::Limit => Error::PathLimit,
        });
    }

    match fs::symlink_metadata(&target) {
        Ok(_) => return result(Error::Exists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return result(Error::Io),
    }

    let directory_bytes = parent_bytes(path);
    let directory = PathBuf::from(std::ffi::OsStr::from_bytes(directory_bytes));
    let (mut temporary_file, temporary_path) = match create_temp(directory_bytes, TEMP_PREFIX) {
        Ok(value) => value,
        Err(TempError::Limit) => return result(Error::Invalid),
        Err(TempError::Io) => return result(Error::Io),
    };

    if write_all(&mut temporary_file, bytes, |file, remaining| {
        match hooks.as_deref_mut() {
            Some(hook) => hook.before_write(file, remaining),
            None => Ok(remaining.len()),
        }
    })
    .is_err()
    {
        cleanup_temp(Some(temporary_file), &temporary_path);
        return result(Error::Io);
    }

    let sync_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_sync_file(&temporary_file),
        None => Ok(()),
    };
    let sync_result = sync_gate.and_then(|()| temporary_file.sync_all());
    if sync_result.is_err() {
        cleanup_temp(Some(temporary_file), &temporary_path);
        return result(Error::Io);
    }
    let mut state = result(Error::Io);
    state.file_synced = true;

    let close_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_close(&temporary_file),
        None => Ok(()),
    };
    let temp_close = crate::sys::close_checked(temporary_file.into());
    if close_gate.is_err() || temp_close.is_err() {
        remove_temp(&temporary_path);
        return state;
    }

    let publish_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_publish(&temporary_path, &target),
        None => Ok(()),
    };
    let publish_result = publish_gate.and_then(|()| fs::hard_link(&temporary_path, &target));
    if let Err(error) = publish_result {
        if error.kind() == io::ErrorKind::AlreadyExists {
            state.error = Error::Exists;
        }
        remove_temp(&temporary_path);
        return state;
    }

    state.published = true;
    state.error = Error::Io;
    let remove_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_remove_temporary(&temporary_path),
        None => Ok(()),
    };
    let remove_result = remove_gate.and_then(|()| fs::remove_file(&temporary_path));
    if remove_result.is_err() {
        remove_temp(&temporary_path);
        return state;
    }

    state.error = Error::Durability;
    let directory_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_sync_directory(&directory),
        None => Ok(()),
    };
    let directory_sync = directory_gate.and_then(|()| super::sync_directory(&directory));
    if directory_sync.is_ok() {
        state.error = Error::Ok;
        state.directory_synced = true;
    }
    state
}
