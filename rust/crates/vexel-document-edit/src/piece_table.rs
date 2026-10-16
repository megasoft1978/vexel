use crate::{
    alloc,
    cache::Cache,
    storage::{self, Blob, Buffer, Piece},
    *,
};
use std::{
    cell::Cell,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};
use vexel_document::OwnedDocumentModel;
use vexel_xml::{XmlDiagnostic, XmlError};
#[derive(Clone, Debug)]
pub(crate) struct Command {
    pub offset: usize,
    pub before: Option<Arc<Blob>>,
    pub after: Option<Arc<Blob>>,
    pub before_state: u64,
    pub after_state: u64,
    pub label: &'static str,
    pub target_slot: u32,
    pub coalescible: bool,
}
#[derive(Debug)]
pub struct Editor {
    pub(crate) original: Arc<Vec<u8>>,
    pub(crate) pieces: Vec<Piece>,
    pub(crate) byte_len: usize,
    pub(crate) limits: EditLimits,
    pub(crate) history: Vec<Command>,
    pub(crate) cursor: usize,
    pub(crate) account: Arc<AtomicUsize>,
    pub(crate) state_id: u64,
    pub(crate) saved_state: u64,
    pub(crate) next_state: u64,
    pub(crate) cache: OnceLock<Cache>,
    pub(crate) diagnostic: Cell<XmlDiagnostic>,
    pub(crate) next_auto_id: u64,
}
impl Editor {
    /// Caller ownership is borrowed only for construction and then cloned.
    /// Invalid C null-source arguments cannot be expressed with this API.
    /// ```compile_fail
    /// use vexel_document_edit::{Editor, EditLimits};
    /// Editor::create(None, EditLimits::default());
    /// ```
    pub fn create(source: &Arc<Vec<u8>>, mut limits: EditLimits) -> EditResult {
        let fail = |error, diagnostic| EditResult {
            error,
            diagnostic,
            editor: None,
        };
        if source.len() > DEFAULT_SOURCE_BYTES {
            return fail(
                EditError::Limit,
                XmlDiagnostic {
                    category: XmlError::InputLimit,
                    offset: 0,
                },
            );
        }
        if limits.history_commands == 0 {
            limits.history_commands = DEFAULT_HISTORY_COMMANDS;
        }
        if limits.history_bytes == 0 {
            limits.history_bytes = DEFAULT_HISTORY_BYTES;
        }
        if limits.history_commands > MAX_HISTORY_COMMANDS
            || limits.history_bytes > MAX_HISTORY_BYTES
        {
            return fail(EditError::Limit, XmlDiagnostic::default());
        }
        let mut pieces = match alloc::vector(usize::from(!source.is_empty())) {
            Ok(v) => v,
            Err(e) => return fail(e, XmlDiagnostic::default()),
        };
        if !source.is_empty() {
            pieces.push(Piece {
                owner: Buffer::Original(Arc::clone(source)),
                start: 0,
                len: source.len(),
            });
        }
        let cache = match Cache::build(Arc::clone(source)) {
            Ok(v) => v,
            Err((e, d)) => return fail(e, d),
        };
        let slot = OnceLock::new();
        let _ = slot.set(cache);
        EditResult {
            error: EditError::Ok,
            diagnostic: XmlDiagnostic::default(),
            editor: Some(Self {
                original: Arc::clone(source),
                pieces,
                byte_len: source.len(),
                limits,
                history: Vec::new(),
                cursor: 0,
                account: Arc::new(AtomicUsize::new(0)),
                state_id: 1,
                saved_state: 1,
                next_state: 2,
                cache: slot,
                diagnostic: Cell::new(XmlDiagnostic::default()),
                next_auto_id: 1,
            }),
        }
    }
    pub(crate) fn index(&self) -> Result<&Cache, EditError> {
        if self.cache.get().is_none() {
            let mut bytes = alloc::vector(self.byte_len)?;
            bytes.resize(self.byte_len, 0);
            storage::read_range(&self.pieces, self.byte_len, 0, &mut bytes)?;
            let built = Cache::build(Arc::new(bytes)).map_err(|(e, d)| {
                self.diagnostic.set(d);
                e
            })?;
            let _ = self.cache.set(built);
        }
        self.cache.get().ok_or(EditError::InvalidOperation)
    }
    pub(crate) fn handle_index(&self, handle: DocumentHandle) -> Result<&Cache, EditError> {
        let cache = self.cache.get().ok_or(EditError::StaleHandle)?;
        cache.node(handle)?;
        Ok(cache)
    }
    pub fn try_model(&self) -> Result<&OwnedDocumentModel, EditError> {
        Ok(&self.index()?.model)
    }
    /// Convenience query for already-valid sessions. Use try_model for fallible cache rebuilds.
    pub fn model(&self) -> &OwnedDocumentModel {
        self.try_model()
            .expect("document index allocation failed; use try_model")
    }
    pub fn byte_length(&self) -> usize {
        self.byte_len
    }
    pub fn is_dirty(&self) -> bool {
        self.state_id != self.saved_state
    }
    pub fn mark_saved(&mut self) {
        self.saved_state = self.state_id;
    }
    pub fn mark_unsaved(&mut self) {
        self.saved_state = 0;
    }
    pub fn history_count(&self) -> usize {
        self.history.len()
    }
    pub fn history_cursor(&self) -> usize {
        self.cursor
    }
    pub fn history_bytes(&self) -> usize {
        self.account.load(Ordering::Acquire)
    }
    pub fn parse_diagnostic(&self) -> XmlDiagnostic {
        self.diagnostic.get()
    }
    pub fn original_source(&self) -> &[u8] {
        &self.original
    }
    pub fn allocation_stats(&self) -> AllocationStats {
        let cache_bytes = self.cache.get().map_or(0, |c| {
            c.source.capacity()
                + c.tokens.capacity() * std::mem::size_of::<vexel_xml::XmlToken>()
                + c.nodes.capacity() * std::mem::size_of::<crate::cache::NodeInfo>()
                + c.attributes.capacity() * std::mem::size_of::<crate::cache::AttributeInfo>()
                + c.texts.capacity() * std::mem::size_of::<crate::cache::TextInfo>()
        });
        AllocationStats {
            blob_bytes: self.history_bytes(),
            piece_bytes: self.pieces.capacity() * std::mem::size_of::<Piece>(),
            command_bytes: self.history.capacity() * std::mem::size_of::<Command>(),
            cache_bytes,
        }
    }
    pub fn materialize(&self, output: &mut [u8]) -> Result<usize, EditError> {
        if output.len() < self.byte_len {
            return Err(EditError::InvalidArgument);
        }
        storage::read_range(&self.pieces, self.byte_len, 0, &mut output[..self.byte_len])?;
        Ok(self.byte_len)
    }
    /// C-style measured materialization: reports required length on capacity
    /// exhaustion and leaves the output untouched.
    pub fn materialize_into(&self, output: &mut [u8], out_length: &mut usize) -> bool {
        *out_length = self.byte_len;
        self.materialize(output).is_ok()
    }
    pub fn read_range(&self, offset: usize, output: &mut [u8]) -> Result<(), EditError> {
        storage::read_range(&self.pieces, self.byte_len, offset, output)
    }
    pub(crate) fn copy_range(&self, offset: usize, len: usize) -> Result<Vec<u8>, EditError> {
        let mut bytes = alloc::vector(len)?;
        bytes.resize(len, 0);
        self.read_range(offset, &mut bytes)?;
        Ok(bytes)
    }
    pub(crate) fn candidate(
        &self,
        region: SourceSpan,
        offset: usize,
        remove: usize,
        insert: &[u8],
    ) -> Result<Vec<u8>, EditError> {
        if region.offset > self.byte_len
            || region.length > self.byte_len - region.offset
            || offset < region.offset
            || offset - region.offset > region.length
            || remove > region.length - (offset - region.offset)
        {
            return Err(EditError::InvalidArgument);
        }
        let length = (region.length - remove)
            .checked_add(insert.len())
            .ok_or(EditError::Limit)?;
        let mut out = alloc::vector(length)?;
        out.resize(length, 0);
        let prefix = offset - region.offset;
        self.read_range(region.offset, &mut out[..prefix])?;
        out[prefix..prefix + insert.len()].copy_from_slice(insert);
        self.read_range(offset + remove, &mut out[prefix + insert.len()..])?;
        Ok(out)
    }
    /// C validation helper: validates only the nominated region; fragment mode
    /// wraps it in <probe>, start-tag mode turns an open start into an empty tag.
    pub fn validate_replace(
        &self,
        region: SourceSpan,
        offset: usize,
        remove: usize,
        insert: &[u8],
        fragment: bool,
        start_tag: bool,
    ) -> Result<(), EditError> {
        let mut bytes = self.candidate(region, offset, remove, insert)?;
        if start_tag {
            if bytes.last() != Some(&b'>') {
                return Err(EditError::MalformedXml);
            }
            if bytes.len() < 2 || bytes[bytes.len() - 2] != b'/' {
                let end = bytes.len() - 1;
                let mut expanded =
                    alloc::vector(bytes.len().checked_add(1).ok_or(EditError::Limit)?)?;
                expanded.extend_from_slice(&bytes[..end]);
                expanded.extend_from_slice(b"/>");
                bytes = expanded;
            }
        }
        if fragment {
            let mut wrapped = alloc::vector(bytes.len().checked_add(15).ok_or(EditError::Limit)?)?;
            wrapped.extend_from_slice(b"<probe>");
            wrapped.extend_from_slice(&bytes);
            wrapped.extend_from_slice(b"</probe>");
            bytes = wrapped;
        }
        if bytes.len() > DEFAULT_SOURCE_BYTES {
            return Err(EditError::MalformedXml);
        }
        crate::cache::parse_tokens(&bytes)
            .map(|_| ())
            .map_err(|(e, _)| {
                if e == EditError::OutOfMemory {
                    e
                } else {
                    EditError::MalformedXml
                }
            })
    }
    pub(crate) fn prepared_cache(
        &self,
        offset: usize,
        remove: usize,
        insert: &[u8],
    ) -> Result<Cache, EditError> {
        let candidate = self.candidate(
            SourceSpan {
                offset: 0,
                length: self.byte_len,
            },
            offset,
            remove,
            insert,
        )?;
        Cache::build(Arc::new(candidate)).map_err(|(e, _)| e)
    }
    /// Low-level C commit helper. Caller validates first. All allocation and
    /// capacity checks precede publication; descriptor/blob owners are shared.
    pub fn commit_replace(
        &mut self,
        offset: usize,
        remove: usize,
        insert: &[u8],
        target: u32,
        coalescible: bool,
        action: HistoryAction,
    ) -> Result<(), EditError> {
        self.replace(offset, remove, insert, action.label(), target, coalescible)
    }
    pub(crate) fn replace(
        &mut self,
        offset: usize,
        remove: usize,
        insert: &[u8],
        label: &'static str,
        target: u32,
        coalescible: bool,
    ) -> Result<(), EditError> {
        if label.is_empty()
            || label.len() >= HISTORY_LABEL_CAPACITY
            || offset > self.byte_len
            || remove > self.byte_len - offset
        {
            return Err(EditError::InvalidArgument);
        }
        if (self.byte_len - remove)
            .checked_add(insert.len())
            .is_none_or(|n| n > DEFAULT_SOURCE_BYTES)
            || self.next_state == u64::MAX
        {
            return Err(EditError::Limit);
        }
        let before_bytes = self.copy_range(offset, remove)?;
        if before_bytes == insert {
            return Ok(());
        }
        let coalesce = coalescible
            && self.cursor == self.history.len()
            && self.saved_state != self.state_id
            && self
                .history
                .last()
                .is_some_and(|c| c.coalescible && c.target_slot == target && c.offset == offset);
        let before = if coalesce {
            None
        } else {
            Blob::create(&before_bytes, &self.account, self.limits.history_bytes)?
        };
        let after = Blob::create(insert, &self.account, self.limits.history_bytes)?;
        let (pieces, length) =
            storage::replace_copy(&self.pieces, self.byte_len, offset, remove, after.as_ref())?;
        // Reserve a separate command vector only if growth is required; no
        // fallible operation remains after the first mutation below.
        let mut grown = None;
        if !coalesce
            && self.history.len() < self.limits.history_commands
            && self.history.len() == self.history.capacity()
        {
            let cap = if self.history.capacity() == 0 {
                8
            } else {
                self.history
                    .capacity()
                    .checked_mul(2)
                    .ok_or(EditError::OutOfMemory)?
            }
            .min(self.limits.history_commands);
            let mut v = alloc::vector(cap)?;
            if v.capacity() > self.limits.history_commands {
                return Err(EditError::OutOfMemory);
            }
            v.extend(self.history.iter().cloned());
            grown = Some(v);
        }
        if let Some(v) = grown {
            self.history = v;
        }
        self.history.truncate(self.cursor);
        let state = self.next_state;
        if coalesce {
            let previous = self
                .history
                .last_mut()
                .expect("preflight coalescing command");
            previous.after = after;
            previous.after_state = state;
            previous.label = label;
        } else {
            if self.history.len() == self.limits.history_commands {
                self.history.remove(0);
            }
            self.history.push(Command {
                offset,
                before,
                after,
                before_state: self.state_id,
                after_state: state,
                label,
                target_slot: target,
                coalescible,
            });
            self.cursor = self.history.len();
        }
        self.pieces = pieces;
        self.byte_len = length;
        self.state_id = state;
        self.next_state += 1;
        self.cache = OnceLock::new();
        Ok(())
    }
    pub(crate) fn publish_with_cache(
        &mut self,
        offset: usize,
        remove: usize,
        insert: &[u8],
        label: &'static str,
        target: u32,
        cache: Cache,
    ) -> Result<(), EditError> {
        self.replace(offset, remove, insert, label, target, false)?;
        // Insert/wrap/duplicate always change source; retain the preflight cache.
        self.cache = OnceLock::new();
        let _ = self.cache.set(cache);
        Ok(())
    }
    pub fn undo(&mut self) -> Result<(), EditError> {
        self.history_step(false)
    }
    pub fn redo(&mut self) -> Result<(), EditError> {
        self.history_step(true)
    }
    fn history_step(&mut self, forward: bool) -> Result<(), EditError> {
        let index = if forward {
            self.cursor
        } else {
            self.cursor
                .checked_sub(1)
                .ok_or(EditError::InvalidOperation)?
        };
        let command = self
            .history
            .get(index)
            .cloned()
            .ok_or(EditError::InvalidOperation)?;
        let (remove, insert, state) = if forward {
            (&command.before, &command.after, command.after_state)
        } else {
            (&command.after, &command.before, command.before_state)
        };
        let remove = remove.as_ref().map_or(0, |b| b.bytes.len());
        let bytes = insert.as_ref().map_or(&[][..], |b| b.bytes.as_slice());
        self.validate_replace(
            SourceSpan {
                offset: 0,
                length: self.byte_len,
            },
            command.offset,
            remove,
            bytes,
            false,
            false,
        )?;
        let (pieces, length) = storage::replace_copy(
            &self.pieces,
            self.byte_len,
            command.offset,
            remove,
            insert.as_ref(),
        )?;
        self.pieces = pieces;
        self.byte_len = length;
        self.state_id = state;
        if forward {
            self.cursor += 1;
        } else {
            self.cursor -= 1;
        }
        self.cache = OnceLock::new();
        Ok(())
    }
    pub(crate) fn jump_to(&mut self, target: usize) -> Result<(), EditError> {
        if target > self.history.len() {
            return Err(EditError::InvalidArgument);
        }
        if target == self.cursor {
            return Ok(());
        }
        let mut pieces = alloc::vector(self.pieces.len())?;
        pieces.extend(self.pieces.iter().cloned());
        let mut length = self.byte_len;
        let mut cursor = self.cursor;
        let mut state = self.state_id;
        while cursor != target {
            let forward = cursor < target;
            let command = &self.history[if forward { cursor } else { cursor - 1 }];
            let (remove, insert) = if forward {
                (&command.before, &command.after)
            } else {
                (&command.after, &command.before)
            };
            let removed = remove.as_ref().map_or(0, |b| b.bytes.len());
            let (next, next_length) =
                storage::replace_copy(&pieces, length, command.offset, removed, insert.as_ref())?;
            let mut bytes = alloc::vector(next_length)?;
            bytes.resize(next_length, 0);
            storage::read_range(&next, next_length, 0, &mut bytes)?;
            crate::cache::parse_tokens(&bytes).map_err(|(e, _)| {
                if e == EditError::OutOfMemory {
                    e
                } else {
                    EditError::MalformedXml
                }
            })?;
            pieces = next;
            length = next_length;
            state = if forward {
                command.after_state
            } else {
                command.before_state
            };
            if forward {
                cursor += 1;
            } else {
                cursor -= 1;
            }
        }
        self.pieces = pieces;
        self.byte_len = length;
        self.cursor = cursor;
        self.state_id = state;
        self.cache = OnceLock::new();
        Ok(())
    }
    pub fn save_as(
        &mut self,
        path: &[u8],
        hooks: Option<&mut dyn vexel_document_io::SourceSaveHooks>,
    ) -> SaveResult {
        let empty = SaveResult {
            error: EditError::InvalidArgument,
            source_error: vexel_document_io::SourceSaveError::Invalid,
            diagnostic: XmlDiagnostic::default(),
            native_error_code: 0,
            renamed: false,
            file_synced: false,
            directory_synced: false,
        };
        if path.is_empty() {
            return empty;
        }
        let cache = match self.index() {
            Ok(c) => c,
            Err(_) => return empty,
        };
        let result =
            vexel_document_io::source_save_noop(&cache.source, path, cache.tokens.len(), hooks);
        let summary = SaveResult {
            error: if result.error == vexel_document_io::SourceSaveError::Ok {
                EditError::Ok
            } else {
                EditError::SaveFailed
            },
            source_error: result.error,
            diagnostic: result.parse_diagnostic,
            native_error_code: result.file_result.error as u8,
            renamed: result.file_result.renamed,
            file_synced: result.file_result.file_synced,
            directory_synced: result.file_result.directory_synced,
        };
        if summary.source_error == vexel_document_io::SourceSaveError::Ok || summary.renamed {
            self.mark_saved();
        }
        summary
    }
}
