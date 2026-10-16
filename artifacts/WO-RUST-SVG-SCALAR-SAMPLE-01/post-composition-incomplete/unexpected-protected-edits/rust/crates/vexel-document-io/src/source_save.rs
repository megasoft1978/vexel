use std::fs::File;
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use vexel_platform_macos::{
    AtomicFileCreateNewError, AtomicFileCreateNewHooks, AtomicFileCreateNewResult,
    AtomicFileReplaceError, AtomicFileReplaceHooks, AtomicFileReplaceResult,
    MAX_ATOMIC_FILE_PATH_BYTES, atomic_file_create_new, atomic_file_replace,
};
use vexel_xml::{
    MAX_INPUT_BYTES, XmlDiagnostic, XmlError, XmlLimits, XmlToken, tokenize_with_capacity,
};

/// Source-save failures, matching the C wrapper's public categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceSaveError {
    Ok,
    Invalid,
    InputLimit,
    PathLimit,
    Tokenizer,
    Destination,
    Durability,
}

impl SourceSaveError {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Invalid => "invalid",
            Self::InputLimit => "input_limit",
            Self::PathLimit => "path_limit",
            Self::Tokenizer => "tokenizer",
            Self::Destination => "destination",
            Self::Durability => "durability",
        }
    }
}

/// Safe observation and fault gates for source saving.
///
/// Returning success permits the native adapter operation to proceed; these
/// hooks cannot supply successful writes, syncs, closes, renames, or publishes.
pub trait SourceSaveHooks {
    fn before_write(&mut self, _file: &File, remaining: &[u8]) -> io::Result<usize> {
        Ok(remaining.len())
    }
    fn before_sync_file(&mut self, _file: &File) -> io::Result<()> {
        Ok(())
    }
    fn before_close(&mut self, _file: &File) -> io::Result<()> {
        Ok(())
    }
    fn before_rename(&mut self, _from: &Path, _to: &Path) -> io::Result<()> {
        Ok(())
    }
    fn before_publish(&mut self, _temporary: &Path, _target: &Path) -> io::Result<()> {
        Ok(())
    }
    fn before_remove_temporary(&mut self, _temporary: &Path) -> io::Result<()> {
        Ok(())
    }
    fn before_sync_directory(&mut self, _directory: &Path) -> io::Result<()> {
        Ok(())
    }
}

/// Owned bounded tokenizer output. Token spans are valid only as long as the
/// source borrowed for this result remains alive.
#[derive(Debug, Eq, PartialEq)]
pub struct SourceSaveResult<'source> {
    pub error: SourceSaveError,
    pub parse_diagnostic: XmlDiagnostic,
    pub tokens: Vec<XmlToken>,
    pub file_result: AtomicFileReplaceResult,
    _source: PhantomData<&'source [u8]>,
}

impl SourceSaveResult<'_> {
    fn error(error: SourceSaveError, diagnostic: XmlDiagnostic) -> Self {
        Self {
            error,
            parse_diagnostic: diagnostic,
            tokens: Vec::new(),
            file_result: AtomicFileReplaceResult {
                error: AtomicFileReplaceError::Ok,
                file_synced: false,
                renamed: false,
                directory_synced: false,
            },
            _source: PhantomData,
        }
    }
}

struct ReplaceAdapter<'a>(&'a mut dyn SourceSaveHooks);
impl AtomicFileReplaceHooks for ReplaceAdapter<'_> {
    fn before_write(&mut self, file: &File, remaining: &[u8]) -> io::Result<usize> {
        self.0.before_write(file, remaining)
    }
    fn before_sync_file(&mut self, file: &File) -> io::Result<()> {
        self.0.before_sync_file(file)
    }
    fn before_close(&mut self, file: &File) -> io::Result<()> {
        self.0.before_close(file)
    }
    fn before_rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.0.before_rename(from, to)
    }
    fn before_sync_directory(&mut self, directory: &Path) -> io::Result<()> {
        self.0.before_sync_directory(directory)
    }
}

struct CreateAdapter<'a>(&'a mut dyn SourceSaveHooks);
impl AtomicFileCreateNewHooks for CreateAdapter<'_> {
    fn before_write(&mut self, file: &File, remaining: &[u8]) -> io::Result<usize> {
        self.0.before_write(file, remaining)
    }
}

/// Validate and tokenize borrowed source before touching the destination, then
/// atomically replace an existing regular file or publish a missing file.
/// `token_capacity` bounds the owned token vector and source/path remain borrowed.
pub fn source_save_noop<'source>(
    source: &'source [u8],
    path: &[u8],
    token_capacity: usize,
    mut hooks: Option<&mut dyn SourceSaveHooks>,
) -> SourceSaveResult<'source> {
    if path.is_empty() {
        return SourceSaveResult::error(SourceSaveError::Invalid, XmlDiagnostic::default());
    }
    if path.len() > MAX_ATOMIC_FILE_PATH_BYTES {
        return SourceSaveResult::error(SourceSaveError::PathLimit, XmlDiagnostic::default());
    }
    if source.len() > MAX_INPUT_BYTES {
        return SourceSaveResult::error(SourceSaveError::InputLimit, XmlDiagnostic::default());
    }

    let (tokens, parsed) = tokenize_with_capacity(source, XmlLimits::default(), token_capacity);
    if parsed.diagnostic.category != XmlError::None {
        return SourceSaveResult::error(SourceSaveError::Tokenizer, parsed.diagnostic);
    }

    let replaced = match hooks.as_deref_mut() {
        Some(hook) => atomic_file_replace(path, source, Some(&mut ReplaceAdapter(hook))),
        None => atomic_file_replace(path, source, None),
    };
    let file_result = if replaced.error == AtomicFileReplaceError::NotFound {
        let created = match hooks.as_deref_mut() {
            Some(hook) => atomic_file_create_new(path, source, Some(&mut CreateAdapter(hook))),
            None => atomic_file_create_new(path, source, None),
        };
        map_create_result(created)
    } else {
        replaced
    };
    let error = match file_result.error {
        AtomicFileReplaceError::Ok => SourceSaveError::Ok,
        AtomicFileReplaceError::Durability => SourceSaveError::Durability,
        _ => SourceSaveError::Destination,
    };
    SourceSaveResult {
        error,
        parse_diagnostic: parsed.diagnostic,
        tokens,
        file_result,
        _source: PhantomData,
    }
}

fn map_create_result(created: AtomicFileCreateNewResult) -> AtomicFileReplaceResult {
    AtomicFileReplaceResult {
        error: match created.error {
            AtomicFileCreateNewError::Ok => AtomicFileReplaceError::Ok,
            AtomicFileCreateNewError::Durability => AtomicFileReplaceError::Durability,
            _ => AtomicFileReplaceError::Io,
        },
        file_synced: created.file_synced,
        renamed: created.published,
        directory_synced: created.directory_synced,
    }
}

pub const fn source_save_error_name(error: SourceSaveError) -> &'static str {
    error.name()
}
