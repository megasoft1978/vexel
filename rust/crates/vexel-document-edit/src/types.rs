use crate::Editor;
use vexel_xml::XmlDiagnostic;
pub const DEFAULT_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_HISTORY_COMMANDS: usize = 1_000;
pub const DEFAULT_HISTORY_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_HISTORY_COMMANDS: usize = 300_000;
pub const MAX_HISTORY_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_PIECES: usize = 1_000_000;
pub const MAX_FRAGMENT_BYTES: usize = 64 * 1024;
pub const MAX_GROUP_NODES: usize = 128;
pub const HISTORY_LABEL_CAPACITY: usize = 64;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditError {
    Ok,
    InvalidArgument,
    StaleHandle,
    NotFound,
    DuplicateAttribute,
    DuplicateId,
    InvalidName,
    MalformedXml,
    RootDelete,
    InvalidOperation,
    Limit,
    HistoryLimit,
    OutOfMemory,
    SaveFailed,
}
impl EditError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::InvalidArgument => "invalid_argument",
            Self::StaleHandle => "stale_handle",
            Self::NotFound => "not_found",
            Self::DuplicateAttribute => "duplicate_attribute",
            Self::DuplicateId => "duplicate_id",
            Self::InvalidName => "invalid_name",
            Self::MalformedXml => "malformed_xml",
            Self::RootDelete => "root_delete",
            Self::InvalidOperation => "invalid_operation",
            Self::Limit => "limit",
            Self::HistoryLimit => "history_limit",
            Self::OutOfMemory => "out_of_memory",
            Self::SaveFailed => "save_failed",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditLimits {
    pub history_commands: usize,
    pub history_bytes: usize,
}
impl Default for EditLimits {
    fn default() -> Self {
        Self {
            history_commands: DEFAULT_HISTORY_COMMANDS,
            history_bytes: DEFAULT_HISTORY_BYTES,
        }
    }
}
#[derive(Debug)]
pub struct EditResult {
    pub error: EditError,
    pub diagnostic: XmlDiagnostic,
    pub editor: Option<Editor>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaveResult {
    pub error: EditError,
    pub source_error: vexel_document_io::SourceSaveError,
    pub diagnostic: XmlDiagnostic,
    /// Exact discriminant of the accepted AtomicFileReplaceError (0..=7).
    pub native_error_code: u8,
    pub renamed: bool,
    pub file_synced: bool,
    pub directory_synced: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationStats {
    pub blob_bytes: usize,
    pub piece_bytes: usize,
    pub command_bytes: usize,
    pub cache_bytes: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryAction {
    EditAttribute,
    AddAttribute,
    RemoveAttribute,
    EditText,
    Delete,
    Duplicate,
    Move,
    Group,
    Ungroup,
    AddRectangle,
    AddCircle,
    AddEllipse,
    AddLine,
    AddPath,
    AddGroup,
    AddText,
    AddShape,
    EditObjects,
    EditPath,
    Resize,
    Rotate,
}
impl HistoryAction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::EditAttribute => "Edit attribute",
            Self::AddAttribute => "Add attribute",
            Self::RemoveAttribute => "Remove attribute",
            Self::EditText => "Edit text",
            Self::Delete => "Delete",
            Self::Duplicate => "Duplicate",
            Self::Move => "Move",
            Self::Group => "Group",
            Self::Ungroup => "Ungroup",
            Self::AddRectangle => "Add rectangle",
            Self::AddCircle => "Add circle",
            Self::AddEllipse => "Add ellipse",
            Self::AddLine => "Add line",
            Self::AddPath => "Add path",
            Self::AddGroup => "Add group",
            Self::AddText => "Add text",
            Self::AddShape => "Add shape",
            Self::EditObjects => "Edit objects",
            Self::EditPath => "Edit path",
            Self::Resize => "Resize",
            Self::Rotate => "Rotate",
        }
    }
}
