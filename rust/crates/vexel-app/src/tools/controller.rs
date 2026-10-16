//! Gesture state machine turning pointer/key events into document edits.

use super::{
    map::CanvasMap,
    pen::{PenPath, Vertex},
};
use crate::model::Tool;
use vexel_v1_document::{V1Document, V1DocumentError};

/// Drags shorter than this many document units on both axes are clicks.
pub const MIN_DRAG: f64 = 1.0;
const DEFAULT_FILL: &str = "#4f8cff";
const DEFAULT_STROKE: &str = "#1f2937";
const DEFAULT_WIDTH: f64 = 2.0;
const ID_PROBES: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerInput {
    Down {
        x: f64,
        y: f64,
    },
    Move {
        x: f64,
        y: f64,
    },
    Up {
        x: f64,
        y: f64,
    },
    /// The "double-click equivalent": finishes an open pen path without adding a vertex
    /// (the click pair that precedes it already added the vertex).
    DoubleClick {
        x: f64,
        y: f64,
    },
    /// The system cancelled the pointer; abandons the current gesture only.
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    /// Cancels the gesture and any open pen path; otherwise clears the selection.
    Escape,
    /// Deletes the selection.
    Delete,
    /// Finishes an open pen path.
    Enter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Nothing changed.
    Ignored,
    /// Selection, preview or pen state changed; the document did not.
    Updated,
    /// Exactly one history entry was added to the document.
    Committed,
    /// A gesture was abandoned without editing the document.
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolError {
    Document(V1DocumentError),
    /// No free `kind-N` id was found within the probe limit.
    IdSpaceExhausted,
}

impl From<V1DocumentError> for ToolError {
    fn from(value: V1DocumentError) -> Self {
        Self::Document(value)
    }
}

type Point = (f64, f64);

#[derive(Debug)]
enum Gesture {
    None,
    Drag {
        start: Point,
        current: Point,
    },
    Move {
        id: String,
        start: Point,
        current: Point,
    },
    Pen {
        pos: Point,
        handle: Option<Point>,
    },
}

#[derive(Debug)]
pub struct ToolController {
    tool: Tool,
    gesture: Gesture,
    pen: PenPath,
    selection: Option<String>,
    fill: String,
    stroke: String,
    width: f64,
    counters: [u64; 4],
}

impl Default for ToolController {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolController {
    pub fn new() -> Self {
        Self {
            tool: Tool::Select,
            gesture: Gesture::None,
            pen: PenPath::new(),
            selection: None,
            fill: DEFAULT_FILL.to_owned(),
            stroke: DEFAULT_STROKE.to_owned(),
            width: DEFAULT_WIDTH,
            counters: [0; 4],
        }
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    /// Switching tools abandons any gesture and open pen path.
    pub fn set_tool(&mut self, tool: Tool) {
        if tool != self.tool {
            self.abandon();
            self.tool = tool;
        }
    }

    pub fn selection(&self) -> Option<&str> {
        self.selection.as_deref()
    }

    /// Selects any element that has a stable id (including kinds the tools cannot move).
    pub fn select_id(&mut self, doc: &V1Document, id: &str) -> Result<(), ToolError> {
        doc.element_kind(id)?;
        self.selection = Some(id.to_owned());
        Ok(())
    }

    pub fn fill(&self) -> &str {
        &self.fill
    }

    pub fn stroke(&self) -> (&str, f64) {
        (&self.stroke, self.width)
    }

    /// Open pen path, for preview painting. Previews are never document edits.
    pub fn pen_vertices(&self) -> &[Vertex] {
        self.pen.vertices()
    }

    pub fn pen_capacity(&self) -> usize {
        self.pen.capacity()
    }

    /// Sets the fill for new shapes and recolours the selection (one history entry).
    /// An invalid colour or an unrecolourable selection changes nothing.
    pub fn set_fill(&mut self, doc: &mut V1Document, colour: &str) -> Result<(), ToolError> {
        check_color(colour)?;
        self.validate_selection(doc);
        if let Some(id) = &self.selection {
            doc.set_fill(id, colour)?;
        }
        colour.clone_into(&mut self.fill);
        Ok(())
    }

    /// Sets the stroke for new shapes and restrokes the selection (one history entry).
    pub fn set_stroke(
        &mut self,
        doc: &mut V1Document,
        colour: &str,
        width: f64,
    ) -> Result<(), ToolError> {
        check_color(colour)?;
        if !(width.is_finite() && (0.0..=256.0).contains(&width)) {
            return Err(V1DocumentError::InvalidStrokeWidth.into());
        }
        self.validate_selection(doc);
        if let Some(id) = &self.selection {
            doc.set_stroke(id, colour, width)?;
        }
        colour.clone_into(&mut self.stroke);
        self.width = width;
        Ok(())
    }

    pub fn pointer(
        &mut self,
        event: PointerInput,
        doc: &mut V1Document,
        map: &dyn CanvasMap,
    ) -> Result<Outcome, ToolError> {
        match event {
            PointerInput::Down { x, y } => {
                let Some(p) = map.to_doc(x, y) else {
                    return Ok(Outcome::Ignored);
                };
                self.gesture = Gesture::None;
                self.down(p, doc)
            }
            PointerInput::Move { x, y } => {
                let Some(p) = map.to_doc(x, y) else {
                    return Ok(Outcome::Ignored);
                };
                Ok(self.move_to(p))
            }
            PointerInput::Up { x, y } => {
                let p = map.to_doc(x, y);
                self.up(p, doc)
            }
            PointerInput::DoubleClick { .. } => {
                if self.tool == Tool::Pen {
                    self.finish_pen(doc, false)
                } else {
                    Ok(Outcome::Ignored)
                }
            }
            PointerInput::Cancel => {
                let had = !matches!(self.gesture, Gesture::None);
                self.gesture = Gesture::None;
                Ok(if had {
                    Outcome::Cancelled
                } else {
                    Outcome::Ignored
                })
            }
        }
    }

    pub fn key(&mut self, key: Key, doc: &mut V1Document) -> Result<Outcome, ToolError> {
        match key {
            Key::Escape => {
                let busy =
                    !matches!(self.gesture, Gesture::None) || !self.pen.vertices().is_empty();
                if busy {
                    self.abandon();
                    Ok(Outcome::Cancelled)
                } else if self.selection.take().is_some() {
                    Ok(Outcome::Updated)
                } else {
                    Ok(Outcome::Ignored)
                }
            }
            Key::Delete => {
                self.validate_selection(doc);
                let Some(id) = self.selection.take() else {
                    return Ok(Outcome::Ignored);
                };
                self.gesture = Gesture::None;
                doc.delete(&id)?;
                Ok(Outcome::Committed)
            }
            Key::Enter => self.finish_pen(doc, false),
        }
    }

    /// Finishes the open pen path as a closed shape (`Z`, filled with the current
    /// fill). Needs at least three vertices; otherwise nothing happens.
    pub fn close_path(&mut self, doc: &mut V1Document) -> Result<Outcome, ToolError> {
        if self.tool != Tool::Pen || self.pen.vertices().len() < 3 {
            return Ok(Outcome::Ignored);
        }
        self.finish_pen(doc, true)
    }

    /// Moves the selection by a document delta as one history entry.
    pub fn move_selection(
        &mut self,
        doc: &mut V1Document,
        dx: f64,
        dy: f64,
    ) -> Result<Outcome, ToolError> {
        let Some(id) = self.selection.clone() else {
            return Ok(Outcome::Ignored);
        };
        doc.translate(&id, dx, dy)?;
        Ok(Outcome::Committed)
    }

    fn abandon(&mut self) {
        self.gesture = Gesture::None;
        self.pen.clear();
    }

    fn validate_selection(&mut self, doc: &V1Document) {
        if let Some(id) = &self.selection {
            if doc.element_kind(id).is_err() {
                self.selection = None;
            }
        }
    }

    fn down(&mut self, p: Point, doc: &mut V1Document) -> Result<Outcome, ToolError> {
        match self.tool {
            Tool::Select => {
                self.validate_selection(doc);
                match doc.hit_test(p.0, p.1) {
                    Some(id) => {
                        self.selection = Some(id.clone());
                        self.gesture = Gesture::Move {
                            id,
                            start: p,
                            current: p,
                        };
                        Ok(Outcome::Updated)
                    }
                    None => Ok(if self.selection.take().is_some() {
                        Outcome::Updated
                    } else {
                        Outcome::Ignored
                    }),
                }
            }
            Tool::Rectangle | Tool::Ellipse | Tool::Line => {
                self.gesture = Gesture::Drag {
                    start: p,
                    current: p,
                };
                Ok(Outcome::Updated)
            }
            Tool::Pen => {
                self.gesture = Gesture::Pen {
                    pos: p,
                    handle: None,
                };
                Ok(Outcome::Updated)
            }
        }
    }

    fn move_to(&mut self, p: Point) -> Outcome {
        match &mut self.gesture {
            Gesture::None => Outcome::Ignored,
            Gesture::Drag { current, .. } | Gesture::Move { current, .. } => {
                *current = p;
                Outcome::Updated
            }
            Gesture::Pen { pos, handle } => {
                *handle = (far(*pos, p)).then_some(p);
                Outcome::Updated
            }
        }
    }

    fn up(&mut self, p: Option<Point>, doc: &mut V1Document) -> Result<Outcome, ToolError> {
        match std::mem::replace(&mut self.gesture, Gesture::None) {
            Gesture::None => Ok(Outcome::Ignored),
            Gesture::Drag { start, current } => self.finish_drag(start, p.unwrap_or(current), doc),
            Gesture::Move { id, start, current } => {
                let end = p.unwrap_or(current);
                let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                if dx.abs() < MIN_DRAG && dy.abs() < MIN_DRAG {
                    return Ok(Outcome::Updated);
                }
                doc.translate(&id, dx, dy)?;
                Ok(Outcome::Committed)
            }
            Gesture::Pen { pos, handle } => {
                let handle = p.map_or(handle, |end| far(pos, end).then_some(end));
                Ok(if self.pen.push(pos, handle) {
                    Outcome::Updated
                } else {
                    Outcome::Ignored
                })
            }
        }
    }

    fn finish_drag(
        &mut self,
        start: Point,
        end: Point,
        doc: &mut V1Document,
    ) -> Result<Outcome, ToolError> {
        let (dx, dy) = ((end.0 - start.0).abs(), (end.1 - start.1).abs());
        match self.tool {
            Tool::Rectangle | Tool::Ellipse if dx < MIN_DRAG || dy < MIN_DRAG => {
                return Ok(Outcome::Cancelled);
            }
            Tool::Line if dx.hypot(dy) < MIN_DRAG => return Ok(Outcome::Cancelled),
            _ => {}
        }
        match self.tool {
            Tool::Rectangle => {
                let id = self.next_id(doc, "rect", 0)?;
                doc.add_rectangle_with_id(start.0, start.1, end.0, end.1, &self.fill, &id)?;
            }
            Tool::Ellipse => {
                let id = self.next_id(doc, "ellipse", 1)?;
                doc.add_ellipse_with_id(start.0, start.1, end.0, end.1, &self.fill, &id)?;
            }
            Tool::Line => {
                let id = self.next_id(doc, "line", 2)?;
                doc.add_line_with_id(
                    start.0,
                    start.1,
                    end.0,
                    end.1,
                    &self.stroke,
                    self.width,
                    &id,
                )?;
            }
            Tool::Select | Tool::Pen => return Ok(Outcome::Ignored),
        }
        Ok(Outcome::Committed)
    }

    fn finish_pen(&mut self, doc: &mut V1Document, close: bool) -> Result<Outcome, ToolError> {
        if self.tool != Tool::Pen || self.pen.vertices().is_empty() {
            return Ok(Outcome::Ignored);
        }
        self.gesture = Gesture::None;
        if self.pen.vertices().len() < 2 {
            self.pen.clear();
            return Ok(Outcome::Cancelled);
        }
        let d = self.pen.path_data(close);
        self.pen.clear();
        let d = d.ok_or(V1DocumentError::InvalidPathData)?;
        let id = self.next_id(doc, "path", 3)?;
        let fill = if close { self.fill.as_str() } else { "none" };
        doc.add_path_with_id(&d, fill, &self.stroke, self.width, &id)?;
        Ok(Outcome::Committed)
    }

    /// Returns the next unused `prefix-N` id; probing skips ids already in the source.
    fn next_id(
        &mut self,
        doc: &V1Document,
        prefix: &str,
        slot: usize,
    ) -> Result<String, ToolError> {
        for _ in 0..ID_PROBES {
            self.counters[slot] += 1;
            let id = format!("{prefix}-{}", self.counters[slot]);
            match doc.element_kind(&id) {
                Err(V1DocumentError::MissingStableId) => return Ok(id),
                Ok(_) => {}
                Err(error) => return Err(error.into()),
            }
        }
        Err(ToolError::IdSpaceExhausted)
    }
}

fn far(a: Point, b: Point) -> bool {
    (b.0 - a.0).hypot(b.1 - a.1) >= MIN_DRAG
}

fn check_color(value: &str) -> Result<(), V1DocumentError> {
    let bytes = value.as_bytes();
    if bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidColor)
    }
}
