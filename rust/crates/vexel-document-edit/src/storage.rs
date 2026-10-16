use crate::{EditError, MAX_PIECES, alloc};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// A blob charges its actual Vec capacity exactly once. The shared accounting
/// owner survives the editor; snapshot-held blobs release their own charge.
#[derive(Debug)]
pub(crate) struct Blob {
    pub bytes: Vec<u8>,
    account: Arc<AtomicUsize>,
}
impl Blob {
    pub fn create(
        bytes: &[u8],
        account: &Arc<AtomicUsize>,
        limit: usize,
    ) -> Result<Option<Arc<Self>>, EditError> {
        if bytes.is_empty() {
            return Ok(None);
        }
        let used = account.load(Ordering::Acquire);
        if bytes.len() > limit || used > limit - bytes.len() {
            return Err(EditError::HistoryLimit);
        }
        let data = alloc::copy(bytes)?;
        let charge = data.capacity();
        if charge > limit || used > limit - charge {
            return Err(EditError::HistoryLimit);
        }
        account.fetch_add(charge, Ordering::AcqRel);
        // std Arc control-block allocation is the sole infallible allocation exception.
        Ok(Some(Arc::new(Self {
            bytes: data,
            account: Arc::clone(account),
        })))
    }
}
impl Drop for Blob {
    fn drop(&mut self) {
        self.account
            .fetch_sub(self.bytes.capacity(), Ordering::AcqRel);
    }
}
#[derive(Clone, Debug)]
pub(crate) enum Buffer {
    Original(Arc<Vec<u8>>),
    Blob(Arc<Blob>),
}
impl Buffer {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Original(v) => v,
            Self::Blob(v) => &v.bytes,
        }
    }
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Original(a), Self::Original(b)) => Arc::ptr_eq(a, b),
            (Self::Blob(a), Self::Blob(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Piece {
    pub owner: Buffer,
    pub start: usize,
    pub len: usize,
}
impl Piece {
    pub fn bytes(&self) -> Result<&[u8], EditError> {
        self.owner
            .bytes()
            .get(self.start..self.start.checked_add(self.len).ok_or(EditError::Limit)?)
            .ok_or(EditError::InvalidOperation)
    }
}
fn append_piece(out: &mut Vec<Piece>, piece: Piece) -> Result<(), EditError> {
    if piece.len == 0 {
        return Ok(());
    }
    if let Some(last) = out.last_mut() {
        if last.owner.same(&piece.owner) && last.start.checked_add(last.len) == Some(piece.start) {
            last.len = last.len.checked_add(piece.len).ok_or(EditError::Limit)?;
            return Ok(());
        }
    }
    if out.len() >= out.capacity() {
        return Err(EditError::OutOfMemory);
    }
    out.push(piece);
    Ok(())
}
fn append_range(
    pieces: &[Piece],
    out: &mut Vec<Piece>,
    start: usize,
    end: usize,
) -> Result<(), EditError> {
    let mut cursor = 0usize;
    for p in pieces {
        if cursor >= end {
            break;
        }
        let next = cursor.checked_add(p.len).ok_or(EditError::Limit)?;
        if next > start && cursor < end {
            let within = start.saturating_sub(cursor);
            let take_end = end.saturating_sub(cursor).min(p.len);
            append_piece(
                out,
                Piece {
                    owner: p.owner.clone(),
                    start: p.start.checked_add(within).ok_or(EditError::Limit)?,
                    len: take_end - within,
                },
            )?;
        }
        cursor = next;
    }
    if start == end || cursor >= end {
        Ok(())
    } else {
        Err(EditError::InvalidOperation)
    }
}
pub(crate) fn replace_copy(
    pieces: &[Piece],
    length: usize,
    offset: usize,
    remove: usize,
    insert: Option<&Arc<Blob>>,
) -> Result<(Vec<Piece>, usize), EditError> {
    if offset > length || remove > length - offset {
        return Err(EditError::InvalidArgument);
    }
    let inserted = insert.map_or(0, |b| b.bytes.len());
    let new_length = (length - remove)
        .checked_add(inserted)
        .ok_or(EditError::Limit)?;
    let capacity = pieces.len().checked_add(3).ok_or(EditError::OutOfMemory)?;
    if capacity > MAX_PIECES {
        return Err(EditError::OutOfMemory);
    }
    let mut out = alloc::vector(capacity)?;
    if out.capacity() > MAX_PIECES {
        return Err(EditError::OutOfMemory);
    }
    append_range(pieces, &mut out, 0, offset)?;
    if let Some(blob) = insert {
        append_piece(
            &mut out,
            Piece {
                owner: Buffer::Blob(Arc::clone(blob)),
                start: 0,
                len: inserted,
            },
        )?;
    }
    append_range(pieces, &mut out, offset + remove, length)?;
    Ok((out, new_length))
}
pub(crate) fn read_range(
    pieces: &[Piece],
    length: usize,
    offset: usize,
    output: &mut [u8],
) -> Result<(), EditError> {
    if offset > length || output.len() > length - offset {
        return Err(EditError::InvalidArgument);
    }
    let end = offset.checked_add(output.len()).ok_or(EditError::Limit)?;
    let mut cursor = 0usize;
    let mut used = 0usize;
    for p in pieces {
        if used == output.len() {
            break;
        }
        let next = cursor.checked_add(p.len).ok_or(EditError::Limit)?;
        if next > offset && cursor < end {
            let start = offset.saturating_sub(cursor);
            let last = end.saturating_sub(cursor).min(p.len);
            let bytes = p
                .bytes()?
                .get(start..last)
                .ok_or(EditError::InvalidOperation)?;
            output[used..used + bytes.len()].copy_from_slice(bytes);
            used += bytes.len();
        }
        cursor = next;
    }
    if used == output.len() {
        Ok(())
    } else {
        Err(EditError::InvalidOperation)
    }
}
