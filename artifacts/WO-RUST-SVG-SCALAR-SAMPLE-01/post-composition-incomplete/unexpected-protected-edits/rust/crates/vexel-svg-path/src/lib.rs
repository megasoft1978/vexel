#![forbid(unsafe_code)]
//! Bounded SVG path parsing with owned geometry and source-byte provenance.

mod arc;
mod control;
mod curve;
mod flatten;
mod parser;
mod reader;

use vexel_base::Span;
use vexel_geometry::{Affine, Edge};

pub use flatten::{append_edge, flatten_cubic, flatten_cubic_controlled, transformed_tolerance};
pub use parser::{PathParseSession, parse_path_controlled};

pub const MAX_PATH_COMMANDS: usize = 4 * 1024 * 1024;
pub const MAX_PATH_EDGES: usize = 4 * 1024 * 1024;
pub const MAX_PATH_NUMBERS: usize = MAX_PATH_COMMANDS * 7;
pub const MAX_COORDINATE: f64 = 1_000_000.0;

/// Caller-selected bounds; values above the frozen reference caps are clamped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    pub max_commands: usize,
    pub max_edges: usize,
    pub max_numbers: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            max_commands: MAX_PATH_COMMANDS,
            max_edges: MAX_PATH_EDGES,
            max_numbers: MAX_PATH_NUMBERS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    InvalidSpan,
    InvalidTransform,
    Path,
    Unsupported,
    Number,
    Limit,
    OutOfMemory,
}

/// Failure location is an absolute byte offset in the containing SVG source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathNumber {
    pub source: Span,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathSegment {
    pub command: u8,
    pub source: Span,
    /// None means this group repeated the preceding authored command.
    pub command_offset: Option<usize>,
    pub first_number: usize,
    pub number_count: usize,
    pub first_edge: usize,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathSubpath {
    pub first_edge: usize,
    pub edge_count: usize,
    pub closed: bool,
}

/// Completely owned flattened path; all provenance ranges refer to caller source.
#[derive(Debug, Clone, PartialEq)]
pub struct PathDocument {
    pub edges: Vec<Edge>,
    pub subpaths: Vec<PathSubpath>,
    pub segments: Vec<PathSegment>,
    pub numbers: Vec<PathNumber>,
}

impl PathDocument {
    fn new() -> Self {
        Self {
            edges: Vec::new(),
            subpaths: Vec::new(),
            segments: Vec::new(),
            numbers: Vec::new(),
        }
    }
}

pub(crate) fn oom(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: DiagnosticKind::OutOfMemory,
        offset,
    }
}
pub(crate) fn failure(kind: DiagnosticKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}

/// Parses one path-data span without limiting or copying the containing source.
pub fn parse_path(
    source: &[u8],
    span: Span,
    transform: Affine,
    options: ParseOptions,
) -> Result<PathDocument, Diagnostic> {
    parser::parse(source, span, transform, options)
}
