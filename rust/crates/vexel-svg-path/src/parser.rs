//! One grammar engine, driven either by bounded caller steps or by the legacy
//! no-stop adapter. Source and policy lifetimes are independent.
mod commands;
#[cfg(test)]
mod tests;

use crate::control::{legacy_error, reserve, NoStop};
use crate::flatten::{self, GeometryCursor};
use crate::reader::{is_separator, CommaCursor, NumberCursor, Reader};
use crate::{
    failure, Diagnostic, DiagnosticKind, ParseOptions, PathDocument, PathNumber, PathSegment,
    PathSubpath, MAX_PATH_COMMANDS, MAX_PATH_EDGES, MAX_PATH_NUMBERS,
};
use vexel_base::{ControlledError, ControlledStep, Span, WorkControl, WorkKind};
use vexel_geometry::{affine_valid, Affine, Point};

const ZERO: Point = Point { x: 0.0, y: 0.0 };
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Validate,
    Prepare,
    Gap,
    Command,
    OperandGap,
    OperandBoundary,
    Number,
    Flag,
    RecordReserve,
    RecordNumber,
    RecordSegment,
    Apply,
    Geometry,
    MoveSubpath,
    FinishMove,
    CloseSubpath,
    FinishClose,
    CompleteGroup,
    FinalSubpath,
    Done,
    Failed,
}
#[derive(Debug, Clone, Copy)]
enum AfterGeometry {
    Group,
    Move,
    Close,
    Final,
}

#[derive(Debug)]
struct Group {
    command: u8,
    upper: u8,
    relative: bool,
    start: usize,
    group_start: usize,
    end: usize,
    command_offset: Option<usize>,
    count: usize,
    operand: usize,
    numbers: [PathNumber; 7],
    first_edge: usize,
    first_number: usize,
    metadata_index: usize,
    segment_index: usize,
    point: Point,
}
impl Group {
    fn new() -> Self {
        Self {
            command: 0,
            upper: 0,
            relative: false,
            start: 0,
            group_start: 0,
            end: 0,
            command_offset: None,
            count: 0,
            operand: 0,
            numbers: [PathNumber {
                source: Span { start: 0, end: 0 },
                value: 0.0,
            }; 7],
            first_edge: 0,
            first_number: 0,
            metadata_index: 0,
            segment_index: 0,
            point: ZERO,
        }
    }
}

/// A borrowed source cursor with owned pending grammar, metadata and geometry.
/// Construction validates only the span and affine transform in constant time;
/// even the initial comma pass is performed inside `step`.
#[derive(Debug)]
pub struct PathParseSession<'src> {
    reader: Reader<'src>,
    comma: CommaCursor,
    number: NumberCursor,
    phase: Phase,
    terminal_error: Option<ControlledError<Diagnostic>>,
    transform: Affine,
    /// Emit edges in path-local space: flatness still follows `transform`.
    local: bool,
    flatness: f64,
    /// Flatness range that reproduces this parse (see [`Self::tolerance_window`]).
    window: (f64, f64),
    max_commands: usize,
    max_edges: usize,
    max_numbers: usize,
    command_count: usize,
    document: PathDocument,
    group: Group,
    current: Point,
    first: Point,
    cubic_control: Point,
    quad_control: Point,
    command: u8,
    open: bool,
    path_first: bool,
    cubic_reflect: bool,
    quad_reflect: bool,
    previous_number: bool,
    previous_arc_flag: bool,
        had_separator: bool,
    current_subpath_edge: usize,
    last_group_offset: usize,
    geometry: Option<GeometryCursor>,
    geometry_offset: usize,
    after_geometry: AfterGeometry,
}
/// Mapping used for local-space output: leaves in-range points unchanged.
const LOCAL: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

impl<'src> PathParseSession<'src> {
    /// Flatness range `lo <= tol < hi` over which re-parsing the same path
    /// gives the identical document (apart from the output transform). Valid
    /// once the parse has finished.
    pub fn tolerance_window(&self) -> (f64, f64) {
        self.window
    }
    /// Emit edges in path-local space. Flattening decisions still follow the
    /// session transform, so mapping the edges afterwards with that transform
    /// gives the same document as a direct parse.
    pub fn local_output(mut self) -> Self {
        self.local = true;
        self
    }

    /// Borrow the checked span without inspecting its path bytes or allocating
    /// document storage. Options retain the accepted frozen-cap clamps.
    pub fn new(
        source: &'src [u8],
        span: Span,
        transform: Affine,
        options: ParseOptions,
    ) -> Result<Self, ControlledError<Diagnostic>> {
        let bytes = span.slice(source).ok_or_else(|| {
            ControlledError::Original(failure(DiagnosticKind::InvalidSpan, span.start))
        })?;
        if !affine_valid(transform) {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::InvalidTransform,
                span.start,
            )));
        }
        Ok(Self {
            reader: Reader {
                bytes,
                base: span.start,
                index: 0,
            },
            comma: CommaCursor::new(),
            number: NumberCursor::new(0),
            phase: Phase::Validate,
            terminal_error: None,
            transform,
            local: false,
            flatness: 0.25,
            window: (0.0, f64::INFINITY),
            max_commands: options.max_commands.min(MAX_PATH_COMMANDS),
            max_edges: options.max_edges.min(MAX_PATH_EDGES),
            max_numbers: options.max_numbers.min(MAX_PATH_NUMBERS),
            command_count: 0,
            document: PathDocument::new(),
            group: Group::new(),
            current: ZERO,
            first: ZERO,
            cubic_control: ZERO,
            quad_control: ZERO,
            command: 0,
            open: false,
            path_first: true,
            cubic_reflect: false,
            quad_reflect: false,
            previous_number: false,
            previous_arc_flag: false,
                        had_separator: false,
            current_subpath_edge: 0,
            last_group_offset: span.start,
            geometry: None,
            geometry_offset: span.start,
            after_geometry: AfterGeometry::Group,
        })
    }

    /// Perform at most `quantum` real cursor/grammar/geometry operations. Each
    /// charge is one unit (therefore <=256), and Pending reports only accepted
    /// charges from this call. Zero quantum checks control without advancing.
    /// Terminal results are sticky and do not consult or charge control again.
    /// SourceRead counts byte visits and EOF/boundary lookups; TokenVisit counts
    /// grammar checks and individual metadata writes; Transform counts bounded
    /// command/tolerance setup; Expansion counts curve nodes or arc samples;
    /// Edge counts transformed append attempts. No unit contains a long loop.
    pub fn step(
        &mut self,
        quantum: u64,
        control: &mut dyn WorkControl,
    ) -> ControlledStep<(), Diagnostic> {
        if let Some(error) = self.terminal_error {
            return ControlledStep::Failed(error);
        }
        if self.phase == Phase::Done {
            return ControlledStep::Complete(());
        }
        if let Err(stop) = control.checkpoint() {
            return self.fail(stop.into());
        }
        let mut charged_units = 0u64;
        while charged_units < quantum {
            if let Err(stop) = control.checkpoint() {
                return self.fail(stop.into());
            }
            if let Err(stop) = control.charge(self.kind(), 1) {
                return self.fail(stop.into());
            }
            let Some(next) = charged_units.checked_add(1) else {
                return self.fail(ControlledError::Overflow);
            };
            charged_units = next;
            if let Err(error) = self.advance() {
                return self.fail(error);
            }
            if self.phase == Phase::Done {
                return ControlledStep::Complete(());
            }
        }
        ControlledStep::Pending { charged_units }
    }

    /// Return an owned document only after completion. Early finish reports the
    /// original path diagnostic at the current parser cursor; it never drives
    /// more work or exposes partial geometry as a successful document.
    pub fn finish(self) -> Result<PathDocument, ControlledError<Diagnostic>> {
        if let Some(error) = self.terminal_error {
            return Err(error);
        }
        if self.phase != Phase::Done {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Path,
                self.reader.absolute(),
            )));
        }
        Ok(self.document)
    }
    fn fail(&mut self, error: ControlledError<Diagnostic>) -> ControlledStep<(), Diagnostic> {
        let error = *self.terminal_error.get_or_insert(error);
        self.phase = Phase::Failed;
        ControlledStep::Failed(error)
    }
    fn kind(&self) -> WorkKind {
        match self.phase {
            Phase::Validate => self.comma.kind(),
            Phase::Gap | Phase::OperandGap | Phase::Flag => WorkKind::SourceRead,
            Phase::Number => self.number.kind(),
            Phase::Prepare | Phase::Apply => WorkKind::Transform,
            Phase::Geometry => self
                .geometry
                .as_ref()
                .map_or(WorkKind::Edge, GeometryCursor::kind),
            _ => WorkKind::TokenVisit,
        }
    }
    fn advance(&mut self) -> Result<(), ControlledError<Diagnostic>> {
        match self.phase {
            Phase::Validate => {
                if self.comma.advance(self.reader.bytes, self.reader.base)? {
                    self.phase = Phase::Prepare;
                }
            }
            Phase::Prepare => {
                self.flatness = flatten::tolerance(self.transform);
                self.next_gap();
            }
            Phase::Gap => {
                if self.reader.peek().is_some_and(is_separator) {
                    self.reader.take()?;
                    self.had_separator = true;
                } else if self.reader.peek().is_some() {
                    self.phase = Phase::Command;
                } else if self.open {
                    self.start_geometry(
                        GeometryCursor::Line(self.current, self.first),
                        self.last_group_offset,
                        AfterGeometry::Final,
                    );
                } else {
                    self.phase = Phase::Done;
                }
            }
            Phase::Command => self.begin_group()?,
            Phase::OperandGap => {
                if self.reader.peek().is_some_and(is_separator) {
                    self.reader.take()?;
                    self.had_separator = true;
                } else {
                    self.phase = Phase::OperandBoundary;
                }
            }
            Phase::OperandBoundary => {
                let argument = self.group.operand;
                let compact_flag = self.group.upper == b'A'
                    && (argument == 4 || (argument == 5 && self.previous_arc_flag));
                if self.previous_number
                    && !self.had_separator
                    && !matches!(self.reader.peek(), Some(b'+' | b'-' | b'.'))
                    && !compact_flag
                {
                    return Err(ControlledError::Original(failure(
                        DiagnosticKind::Path,
                        self.reader.absolute(),
                    )));
                }
                if self.group.upper == b'A' && (argument == 3 || argument == 4) {
                    self.phase = Phase::Flag;
                } else {
                    self.number = NumberCursor::new(self.reader.index);
                    self.phase = Phase::Number;
                }
            }
            Phase::Number => {
                let parsed =
                    self.number
                        .advance(&mut self.reader)
                        .map_err(|error| match error {
                            ControlledError::Original(_) => ControlledError::Original(failure(
                                DiagnosticKind::Path,
                                self.group_offset(),
                            )),
                            other => other,
                        })?;
                if let Some(number) = parsed {
                    self.operand(number)?;
                }
            }
            Phase::Flag => {
                let start = self.reader.absolute();
                match self.reader.peek() {
                    Some(byte @ (b'0' | b'1')) => {
                        self.reader.take()?;
                        self.operand(PathNumber {
                            source: Span {
                                start,
                                end: self.reader.absolute(),
                            },
                            value: f64::from(byte - b'0'),
                        })?;
                    }
                    _ => {
                        return Err(ControlledError::Original(failure(
                            DiagnosticKind::Path,
                            self.group_offset(),
                        )));
                    }
                }
            }
            Phase::RecordReserve => self.reserve_segment()?,
            Phase::RecordNumber => {
                self.document
                    .numbers
                    .push(self.group.numbers[self.group.metadata_index]);
                self.group.metadata_index = self
                    .group
                    .metadata_index
                    .checked_add(1)
                    .ok_or(ControlledError::Overflow)?;
                if self.group.metadata_index == self.group.count {
                    self.phase = Phase::RecordSegment;
                }
            }
            Phase::RecordSegment => {
                let offset = self.reader.base + self.group.start;
                self.group.segment_index = self.document.segments.len();
                self.document.segments.push(PathSegment {
                    command: self.group.command,
                    source: Span {
                        start: offset,
                        end: self.reader.base + self.group.end,
                    },
                    command_offset: self.group.command_offset,
                    first_number: self.group.first_number,
                    number_count: self.group.count,
                    first_edge: self.group.first_edge,
                    edge_count: 0,
                });
                self.phase = if self.group.upper == b'Z' {
                    Phase::FinishClose
                } else {
                    Phase::Apply
                };
            }
            Phase::Apply => self.apply_group()?,
            Phase::Geometry => {
                let cursor = self.geometry.as_mut().ok_or(ControlledError::Overflow)?;
                if cursor.advance(
                    &mut self.document.edges,
                    if self.local { LOCAL } else { self.transform },
                    self.flatness,
                    self.max_edges,
                    self.geometry_offset,
                )? {
                    // Arcs (None) pin the exact tolerance.
                    let (lo, hi) = self.geometry.as_ref().and_then(GeometryCursor::window).unwrap_or((self.flatness, self.flatness.next_up()));
                    self.window = (self.window.0.max(lo), self.window.1.min(hi));
                    self.geometry = None;
                    self.phase = match self.after_geometry {
                        AfterGeometry::Group => Phase::CompleteGroup,
                        AfterGeometry::Move => Phase::MoveSubpath,
                        AfterGeometry::Close => Phase::CloseSubpath,
                        AfterGeometry::Final => Phase::FinalSubpath,
                    };
                }
            }
            Phase::MoveSubpath => {
                self.record_subpath(false, self.group_offset())?;
                self.phase = Phase::FinishMove;
            }
            Phase::FinishMove => {
                self.current = self.group.point;
                self.first = self.group.point;
                self.current_subpath_edge = self.document.edges.len();
                self.open = true;
                self.command = if self.group.relative { b'l' } else { b'L' };
                self.cubic_reflect = false;
                self.quad_reflect = false;
                self.next_gap();
            }
            Phase::CloseSubpath => {
                self.record_subpath(true, self.last_group_offset)?;
                self.phase = Phase::RecordReserve;
            }
            Phase::FinishClose => {
                self.complete_segment()?;
                self.current = self.first;
                self.open = false;
                self.command = 0;
                self.cubic_reflect = false;
                self.quad_reflect = false;
                self.next_gap();
            }
            Phase::CompleteGroup => {
                self.check_points(&[self.group.point])?;
                self.complete_segment()?;
                self.current = self.group.point;
                self.open = true;
                self.next_gap();
            }
            Phase::FinalSubpath => {
                self.record_subpath(false, self.last_group_offset)?;
                self.phase = Phase::Done;
            }
            Phase::Done | Phase::Failed => (),
        }
        Ok(())
    }
    fn begin_group(&mut self) -> Result<(), ControlledError<Diagnostic>> {
        self.group = Group::new();
        self.group.start = self.reader.index;
        self.group.first_edge = self.document.edges.len();
        let byte = self.reader.peek().ok_or(ControlledError::Overflow)?;
        if byte.is_ascii_alphabetic() {
            self.group.command_offset = Some(self.reader.absolute());
            self.command = byte;
            self.reader.take()?;
            self.previous_number = false;
        } else if self.command == 0 || matches!(self.command, b'Z' | b'z') {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Path,
                self.reader.absolute(),
            )));
        } else {
            if self.previous_number && !self.had_separator && !matches!(byte, b'+' | b'-' | b'.') {
                return Err(ControlledError::Original(failure(
                    DiagnosticKind::Path,
                    self.reader.absolute(),
                )));
            }
            self.previous_number = false;
        }
        self.command_count = self
            .command_count
            .checked_add(1)
            .ok_or(ControlledError::Overflow)?;
        if self.command_count > self.max_commands {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Limit,
                self.reader.absolute(),
            )));
        }
        self.group.command = self.command;
        self.group.relative = self.command.is_ascii_lowercase();
        self.group.upper = self.command.to_ascii_uppercase();
        if self.path_first && self.group.upper != b'M' {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Path,
                self.reader.absolute().saturating_sub(1),
            )));
        }
        self.path_first = false;
        if self.group.upper == b'Z' {
            if self.open {
                self.start_geometry(
                    GeometryCursor::Line(self.current, self.first),
                    self.last_group_offset,
                    AfterGeometry::Close,
                );
            } else {
                self.phase = Phase::RecordReserve;
            }
            return Ok(());
        }
        self.group.count = match self.group.upper {
            b'H' | b'V' => 1,
            b'C' => 6,
            b'S' | b'Q' => 4,
            b'T' | b'M' | b'L' => 2,
            b'A' => 7,
            _ => {
                return Err(ControlledError::Original(failure(
                    DiagnosticKind::Unsupported,
                    self.reader.absolute().saturating_sub(1),
                )));
            }
        };
        self.group.group_start = self.reader.index;
        self.last_group_offset = self.group_offset();
        self.had_separator = false;
        self.phase = Phase::OperandGap;
        Ok(())
    }
    fn operand(&mut self, number: PathNumber) -> Result<(), ControlledError<Diagnostic>> {
        self.group.numbers[self.group.operand] = number;
        self.previous_number = true;
        self.previous_arc_flag = self.group.upper == b'A' && matches!(self.group.operand, 3 | 4);
        self.group.operand = self
            .group
            .operand
            .checked_add(1)
            .ok_or(ControlledError::Overflow)?;
        self.had_separator = false;
        self.phase = if self.group.operand == self.group.count {
            Phase::RecordReserve
        } else {
            Phase::OperandGap
        };
        Ok(())
    }
    fn reserve_segment(&mut self) -> Result<(), ControlledError<Diagnostic>> {
        let offset = self.reader.base + self.group.start;
        let required = self
            .document
            .numbers
            .len()
            .checked_add(self.group.count)
            .ok_or(ControlledError::Overflow)?;
        if self.document.segments.len() >= self.max_commands || required > self.max_numbers {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Limit,
                offset,
            )));
        }
        reserve(&mut self.document.segments, 1, self.max_commands, offset)?;
        reserve(
            &mut self.document.numbers,
            self.group.count,
            self.max_numbers,
            offset,
        )?;
        self.group.first_number = self.document.numbers.len();
        self.group.end = self.reader.index;
        self.phase = if self.group.count == 0 {
            Phase::RecordSegment
        } else {
            Phase::RecordNumber
        };
        Ok(())
    }
    fn record_subpath(
        &mut self,
        closed: bool,
        offset: usize,
    ) -> Result<(), ControlledError<Diagnostic>> {
        let edge_count = self
            .document
            .edges
            .len()
            .checked_sub(self.current_subpath_edge)
            .ok_or(ControlledError::Overflow)?;
        if edge_count == 0 {
            return Ok(());
        }
        reserve(&mut self.document.subpaths, 1, MAX_PATH_EDGES, offset)?;
        self.document.subpaths.push(PathSubpath {
            first_edge: self.current_subpath_edge,
            edge_count,
            closed,
        });
        Ok(())
    }
    fn complete_segment(&mut self) -> Result<(), ControlledError<Diagnostic>> {
        self.document.segments[self.group.segment_index].edge_count = self
            .document
            .edges
            .len()
            .checked_sub(self.group.first_edge)
            .ok_or(ControlledError::Overflow)?;
        Ok(())
    }
    fn check_points(&self, points: &[Point]) -> Result<(), ControlledError<Diagnostic>> {
        if points.iter().all(|point| flatten::bounded(*point)) {
            Ok(())
        } else {
            Err(ControlledError::Original(failure(
                DiagnosticKind::Number,
                self.group_offset(),
            )))
        }
    }
    fn group_offset(&self) -> usize {
        self.reader.base + self.group.group_start
    }
    fn next_gap(&mut self) {
        self.had_separator = false;
        self.phase = Phase::Gap;
    }
    fn start_geometry(&mut self, cursor: GeometryCursor, offset: usize, after: AfterGeometry) {
        self.geometry = Some(cursor);
        self.geometry_offset = offset;
        self.after_geometry = after;
        self.phase = Phase::Geometry;
    }
}

/// Drive the same borrowed session in bounded steps under the caller's policy.
/// A failure retains its original diagnostic or typed stop and never returns
/// a successful partial document.
pub fn parse_path_controlled(
    source: &[u8],
    span: Span,
    transform: Affine,
    options: ParseOptions,
    control: &mut dyn WorkControl,
) -> Result<PathDocument, ControlledError<Diagnostic>> {
    let mut session = PathParseSession::new(source, span, transform, options)?;
    loop {
        match session.step(256, control) {
            ControlledStep::Pending { .. } => (),
            ControlledStep::Complete(()) => return session.finish(),
            ControlledStep::Failed(error) => return Err(error),
        }
    }
}

/// `parse_path_controlled` with edges left in path-local space (see
/// [`PathParseSession::local_output`]).
pub fn parse_path_local_controlled(
    source: &[u8],
    span: Span,
    transform: Affine,
    options: ParseOptions,
    control: &mut dyn WorkControl,
) -> Result<(PathDocument, (f64, f64)), ControlledError<Diagnostic>> {
    let mut session = PathParseSession::new(source, span, transform, options)?.local_output();
    loop {
        match session.step(256, control) {
            ControlledStep::Pending { .. } => (),
            ControlledStep::Complete(()) => {
                let window = session.tolerance_window();
                return session.finish().map(|document| (document, window));
            }
            ControlledStep::Failed(error) => return Err(error),
        }
    }
}

pub(crate) fn parse(
    source: &[u8],
    span: Span,
    transform: Affine,
    options: ParseOptions,
) -> Result<PathDocument, Diagnostic> {
    parse_path_controlled(source, span, transform, options, &mut NoStop)
        .map_err(|error| legacy_error(error, span.start))
}
