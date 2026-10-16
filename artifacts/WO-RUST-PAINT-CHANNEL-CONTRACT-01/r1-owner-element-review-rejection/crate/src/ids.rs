use crate::{Diagnostic, ErrorKind, Result};
/// Checked half-open table range, with no raw pointers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TableRange {
    pub first: usize,
    pub count: usize,
}
impl TableRange {
    pub const EMPTY: Self = Self { first: 0, count: 0 };
    pub fn end(self) -> Option<usize> {
        self.first.checked_add(self.count)
    }
    pub fn slice<T>(self, values: &[T]) -> Option<&[T]> {
        values.get(self.first..self.end()?)
    }
    pub(crate) fn require(self, length: usize, offset: usize) -> Result<()> {
        if self.end().is_some_and(|end| end <= length) {
            Ok(())
        } else {
            Err(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset,
            })
        }
    }
}
/// Source token identity. Decoders validate it against their retained token stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneId {
    pub(crate) owner: u64,
    pub(crate) slot: u32,
    pub(crate) serial: u64,
}
impl SceneId {
    pub fn index(self) -> usize {
        self.slot as usize
    }
}
macro_rules! handles { ($($name:ident),*) => { $(
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct $name { pub(crate) owner: u64, pub(crate) scene: u32, pub(crate) slot: u32, pub(crate) serial: u64 }
    impl $name { pub fn index(self) -> usize { self.slot as usize } }
)* }; }
handles!(
    ShapeId,
    TransformId,
    ClipId,
    PatternId,
    MaskId,
    FilterId,
    GroupId
);
