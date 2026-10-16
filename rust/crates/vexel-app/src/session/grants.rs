//! Launch-time MCP filesystem grants.
//!
//! Grants are an owner decision made out of band (`--grant-dir` on the command
//! line, or a constructor in tests). No MCP tool can add one, and none change
//! at runtime. The explicit directories are joined with the existing
//! `VEXEL_MCP_ALLOWED_DIRS` value and the `~/.config/vexel/mcp-allowed-dirs.txt`
//! file into the one [`AccessPolicy`] used by every path-taking tool and by
//! the GIF export job.

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use vexel_mcp_access::{ALLOWED_DIRS_CAP, AccessPolicy, PATH_CAP};

/// Why a `--grant-dir` value was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrantError {
    /// Missing, unreadable, empty, NUL, or too long.
    NotFound,
    NotDirectory,
    /// The filesystem root or the home directory itself.
    TooBroad,
    /// Contains `:` (the grant list separator) or would overflow the list cap.
    Unrepresentable,
}

impl GrantError {
    pub fn message(self) -> &'static str {
        match self {
            Self::NotFound => "--grant-dir needs an existing directory",
            Self::NotDirectory => "--grant-dir path is not a directory",
            Self::TooBroad => "--grant-dir must not be / or the home directory",
            Self::Unrepresentable => "--grant-dir path cannot be used as a grant",
        }
    }
}

/// Explicit grant directories, canonical and fixed at launch.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct McpGrants {
    dirs: Vec<PathBuf>,
}

impl McpGrants {
    /// No explicit grants: only the environment variable and config file apply.
    pub fn none() -> Self {
        Self::default()
    }

    /// Validates and adds one directory (canonicalized).
    pub fn add(&mut self, path: &Path, home: Option<&Path>) -> Result<(), GrantError> {
        let canonical = checked_directory(path, home)?;
        let joined = self
            .dirs
            .iter()
            .map(|d| d.as_os_str().len() + 1)
            .sum::<usize>()
            + canonical.as_os_str().len();
        if joined >= ALLOWED_DIRS_CAP {
            return Err(GrantError::Unrepresentable);
        }
        if !self.dirs.contains(&canonical) {
            self.dirs.push(canonical);
        }
        Ok(())
    }

    /// Grants for the given directories, each validated like `--grant-dir`.
    pub fn from_dirs<P: AsRef<Path>>(
        dirs: impl IntoIterator<Item = P>,
        home: Option<&Path>,
    ) -> Result<Self, GrantError> {
        let mut grants = Self::none();
        for dir in dirs {
            grants.add(dir.as_ref(), home)?;
        }
        Ok(grants)
    }

    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// The policy for this process: explicit grants plus the environment
    /// variable and the `HOME` config file.
    pub fn policy(&self) -> AccessPolicy {
        self.policy_with(
            std::env::var_os("VEXEL_MCP_ALLOWED_DIRS"),
            std::env::var_os("HOME").map(PathBuf::from),
        )
    }

    /// [`McpGrants::policy`] with explicit environment inputs. The explicit
    /// directories come first; the environment value is appended unless the
    /// combined list would exceed the policy's list cap (explicit grants win).
    pub fn policy_with(&self, env_dirs: Option<OsString>, home: Option<PathBuf>) -> AccessPolicy {
        let mut joined: Vec<u8> = Vec::new();
        for dir in &self.dirs {
            if !joined.is_empty() {
                joined.push(b':');
            }
            joined.extend_from_slice(dir.as_os_str().as_bytes());
        }
        if let Some(env) = env_dirs.filter(|e| !e.is_empty()) {
            if joined.is_empty() {
                return AccessPolicy::new(Some(env), home);
            }
            if joined.len() + 1 + env.len() < ALLOWED_DIRS_CAP {
                joined.push(b':');
                joined.extend_from_slice(env.as_bytes());
            }
        }
        let allowed = (!joined.is_empty()).then(|| OsString::from_vec(joined));
        AccessPolicy::new(allowed, home)
    }
}

fn checked_directory(path: &Path, home: Option<&Path>) -> Result<PathBuf, GrantError> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.is_empty() || bytes.contains(&0) || bytes.len() >= PATH_CAP {
        return Err(GrantError::NotFound);
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| GrantError::NotFound)?;
    let metadata = std::fs::metadata(&canonical).map_err(|_| GrantError::NotFound)?;
    if !metadata.is_dir() {
        return Err(GrantError::NotDirectory);
    }
    let text = canonical.as_os_str().as_bytes();
    if text.len() >= PATH_CAP || text.contains(&b':') {
        return Err(GrantError::Unrepresentable);
    }
    let home = home.and_then(|h| std::fs::canonicalize(h).ok());
    if canonical == Path::new("/") || home.as_deref() == Some(canonical.as_path()) {
        return Err(GrantError::TooBroad);
    }
    Ok(canonical)
}
