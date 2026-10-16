use super::path::{
    PathError, TempError, create_temp, open_regular_nofollow, parent_bytes, remove_temp,
    target_path,
};
use super::{
    AtomicFileReplaceError as Error, AtomicFileReplaceHooks as Hooks,
    AtomicFileReplaceResult as ResultState, write_all,
};
use std::fs::{self, File};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

const TEMP_PREFIX: &[u8] = b".vexel-save-";

fn result(error: Error) -> ResultState {
    ResultState {
        error,
        file_synced: false,
        renamed: false,
        directory_synced: false,
    }
}

fn io_result(error: io::Error) -> Error {
    if error.kind() == io::ErrorKind::NotFound {
        Error::NotFound
    } else {
        Error::Io
    }
}

fn cleanup_temp(file: Option<File>, path: &Path) {
    if let Some(file) = file {
        let _ = crate::sys::close_checked(file.into());
    }
    remove_temp(path);
}

/// Atomically replace an existing regular file with `bytes`.
///
/// The path is a Unix byte path and must be 1..=1023 bytes without NUL. The
/// existing target's mode is retained. Successful rename is the commit point;
/// a directory sync failure is reported with `renamed == true`.
pub fn atomic_file_replace(
    path: &[u8],
    bytes: &[u8],
    mut hooks: Option<&mut dyn Hooks>,
) -> ResultState {
    let target = match target_path(path) {
        Ok(path) => path,
        Err(PathError::Invalid) => return result(Error::Invalid),
        Err(PathError::Limit) => return result(Error::PathLimit),
    };
    let initial = match fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) => return result(io_result(error)),
    };
    if !initial.file_type().is_file() {
        return result(Error::NotRegular);
    }

    let original = match open_regular_nofollow(&target) {
        Ok(file) => file,
        Err(_) => return result(Error::Io),
    };
    let opened = match original.metadata() {
        Ok(metadata)
            if metadata.file_type().is_file()
                && metadata.dev() == initial.dev()
                && metadata.ino() == initial.ino() =>
        {
            metadata
        }
        _ => {
            let _ = crate::sys::close_checked(original.into());
            return result(Error::Io);
        }
    };
    let saved_mode = opened.permissions().mode() & 0o7777;
    let close_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_close(&original),
        None => Ok(()),
    };
    let original_close = crate::sys::close_checked(original.into());
    if close_gate.is_err() || original_close.is_err() {
        return result(Error::Io);
    }

    let directory_bytes = parent_bytes(path);
    let directory = Path::new(std::ffi::OsStr::from_bytes(directory_bytes));
    let (mut temporary_file, temporary_path) = match create_temp(directory_bytes, TEMP_PREFIX) {
        Ok(value) => value,
        Err(TempError::Limit) => return result(Error::PathLimit),
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

    if temporary_file
        .set_permissions(fs::Permissions::from_mode(saved_mode))
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

    let rename_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_rename(&temporary_path, &target),
        None => Ok(()),
    };
    let rename_result = rename_gate.and_then(|()| fs::rename(&temporary_path, &target));
    if rename_result.is_err() {
        remove_temp(&temporary_path);
        return state;
    }
    state.renamed = true;
    state.error = Error::Durability;

    let directory_gate = match hooks.as_deref_mut() {
        Some(hook) => hook.before_sync_directory(directory),
        None => Ok(()),
    };
    let directory_sync = directory_gate.and_then(|()| super::sync_directory(directory));
    if directory_sync.is_ok() {
        state.error = Error::Ok;
        state.directory_synced = true;
    }
    state
}
