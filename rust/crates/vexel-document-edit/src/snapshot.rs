use crate::{
    EditError, Editor, alloc,
    storage::{Piece, read_range},
};
#[derive(Debug)]
pub struct EditSnapshot {
    pieces: Vec<Piece>,
    byte_len: usize,
}
impl Editor {
    /// Copies descriptors, shares immutable owners and their accounting owner.
    /// Snapshots are Send+Sync and safely outlive the editor and caller Arc.
    pub fn snapshot(&self) -> Result<EditSnapshot, EditError> {
        let mut total = 0usize;
        for piece in &self.pieces {
            if piece.len == 0 {
                return Err(EditError::InvalidOperation);
            }
            piece.bytes()?;
            total = total.checked_add(piece.len).ok_or(EditError::Limit)?;
        }
        if total != self.byte_len {
            return Err(EditError::InvalidOperation);
        }
        let mut pieces = alloc::vector(self.pieces.len())?;
        if pieces.capacity() > crate::MAX_PIECES {
            return Err(EditError::OutOfMemory);
        }
        pieces.extend(self.pieces.iter().cloned());
        Ok(EditSnapshot {
            pieces,
            byte_len: total,
        })
    }
}
impl EditSnapshot {
    pub fn byte_length(&self) -> usize {
        self.byte_len
    }
    pub fn materialize_into(&self, output: &mut [u8], out_length: &mut usize) -> bool {
        *out_length = 0;
        match self.materialize(output) {
            Ok(n) => {
                *out_length = n;
                true
            }
            Err(_) => false,
        }
    }
    pub fn materialize(&self, output: &mut [u8]) -> Result<usize, EditError> {
        if output.len() < self.byte_len {
            return Err(EditError::InvalidArgument);
        }
        read_range(&self.pieces, self.byte_len, 0, &mut output[..self.byte_len])?;
        Ok(self.byte_len)
    }
}
