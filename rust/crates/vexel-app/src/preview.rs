use std::sync::mpsc::SyncSender;
use vexel_json::{JsonDocument, JsonType};
use vexel_protocol::{ProtocolBackend, ToolResponse};
use vexel_v1_document::{V1Document, V1DocumentError};

mod animation;
mod args;
pub(crate) use animation::{
    animation_list_json, document_duration, validate_animation_fragment, validate_shape,
};
#[cfg(test)]
pub(crate) use animation::{rasterize, rasterize_at};

pub const DOCUMENT_ID: &str = "preview-document";
pub const SCENE_LIMIT: usize = 128;
pub const CANVAS_SIZE: u32 = 512;
const HISTORY_LIMIT: usize = 128;
const ANIMATION_LIMIT: usize = 16;
const ANIMATION_XML_LIMIT: usize = 4096;
const SOURCE_LIMIT: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rgb: u32,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        rgb: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    List,
    Info,
    Add(Shape),
    AnimationAdd {
        target_id: String,
        xml: Vec<u8>,
    },
    AnimationList,
    Control(Control),
    Export {
        path: String,
        fps: u32,
        duration: f64,
    },
    Undo,
    Redo,
    Save,
    SaveAs {
        path: String,
    },
    Open {
        path: String,
    },
    AttributeSet {
        element_id: String,
        change: AttributeChange,
    },
    Delete {
        element_id: String,
    },
    AddLine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: String,
        width: f64,
    },
    AddPath {
        d: String,
        fill: String,
        stroke: String,
        width: f64,
    },
    ExportSvg,
    RenderPng {
        path: String,
        width: Option<u32>,
        height: Option<u32>,
    },
}

/// One validated `attribute_set` change (names limited to the walkthrough set).
#[derive(Clone, Debug, PartialEq)]
pub enum AttributeChange {
    Fill(String),
    Stroke(String),
    StrokeWidth(f64),
    /// `transform="translate(dx dy)"`: a document-space move by this delta.
    Translate(f64, f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    Play,
    Pause,
    Seek(f64),
}

#[derive(Clone, Debug, PartialEq)]
struct AnimationFragment {
    target_id: String,
    xml: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    shapes: Vec<Shape>,
    element_ids: Vec<String>,
    animations: Vec<AnimationFragment>,
}

#[derive(Debug)]
pub struct PreviewState {
    document: V1Document,
    shapes: Vec<Shape>,
    element_ids: Vec<String>,
    next_element_id: u64,
    animations: Vec<AnimationFragment>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    dirty: bool,
    playing: bool,
    time_seconds: f64,
    duration_seconds: f64,
}

impl PreviewState {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self {
            document: V1Document::new(512, 512).map_err(document_error)?,
            shapes: Vec::new(),
            element_ids: Vec::new(),
            next_element_id: 1,
            animations: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            playing: false,
            time_seconds: 0.0,
            duration_seconds: 0.0,
        })
    }
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }
    pub fn element_ids(&self) -> &[String] {
        &self.element_ids
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn source(&self) -> Result<Vec<u8>, &'static str> {
        self.document.source().map_err(document_error)
    }
    pub fn playback(&self) -> (bool, f64, f64) {
        (self.playing, self.time_seconds, self.duration_seconds)
    }
    pub fn set_frame_time(&mut self, seconds: f64) {
        self.time_seconds = if self.duration_seconds > 0.0 {
            seconds.rem_euclid(self.duration_seconds)
        } else {
            0.0
        };
    }

    /// Build an entire replacement before changing the main-thread-owned state.
    pub fn stage(&self, operation: &Operation) -> Result<(Self, String), &'static str> {
        if let Operation::Control(control) = operation {
            let mut next = PreviewState {
                document: animation::rebuild_document(
                    &self.shapes,
                    &self.element_ids,
                    &self.animations,
                )?,
                shapes: clone_shapes(&self.shapes)?,
                element_ids: clone_ids(&self.element_ids)?,
                next_element_id: self.next_element_id,
                animations: clone_animations(&self.animations)?,
                undo: clone_history(&self.undo)?,
                redo: clone_history(&self.redo)?,
                dirty: self.dirty,
                playing: self.playing,
                time_seconds: self.time_seconds,
                duration_seconds: self.duration_seconds,
            };
            match *control {
                Control::Play => {
                    if next.duration_seconds <= 0.0 {
                        return Err("no preview animation to play");
                    }
                    next.playing = true;
                }
                Control::Pause => next.playing = false,
                Control::Seek(seconds) => {
                    if !seconds.is_finite() || !(0.0..=10.0).contains(&seconds) {
                        return Err("invalid preview animation time");
                    }
                    next.set_frame_time(seconds);
                }
            }
            return Ok((next, "Animation playback updated".to_owned()));
        }
        let (shapes, element_ids, animations, undo, redo, message) = match operation {
            Operation::Add(shape) => {
                validate_shape(*shape)?;
                if self.shapes.len() >= SCENE_LIMIT {
                    return Err("preview scene limit reached");
                }
                let mut shapes = clone_shapes(&self.shapes)?;
                shapes
                    .try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                shapes.push(*shape);
                let mut element_ids = clone_ids(&self.element_ids)?;
                element_ids
                    .try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                element_ids.push(object_id(self.next_element_id));
                let mut undo = clone_history(&self.undo)?;
                if undo.len() == HISTORY_LIMIT {
                    undo.remove(0);
                }
                undo.try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                undo.push(self.snapshot()?);
                (
                    shapes,
                    element_ids,
                    clone_animations(&self.animations)?,
                    undo,
                    Vec::new(),
                    "Shape added",
                )
            }
            Operation::AnimationAdd { target_id, xml } => {
                let mut animations = clone_animations(&self.animations)?;
                if animations.len() >= ANIMATION_LIMIT {
                    return Err("preview animation limit reached");
                }
                if !self
                    .shapes
                    .iter()
                    .enumerate()
                    .any(|(i, _)| self.element_ids.get(i) == Some(target_id))
                {
                    return Err("unknown preview animation target");
                }
                validate_animation_fragment(xml)?;
                animations
                    .try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                animations.push(AnimationFragment {
                    target_id: target_id.clone(),
                    xml: xml.clone(),
                });
                let mut undo = clone_history(&self.undo)?;
                if undo.len() == HISTORY_LIMIT {
                    undo.remove(0);
                }
                undo.try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                undo.push(self.snapshot()?);
                (
                    clone_shapes(&self.shapes)?,
                    clone_ids(&self.element_ids)?,
                    animations,
                    undo,
                    Vec::new(),
                    "Animation added",
                )
            }
            Operation::Undo => {
                let Some(previous) = self.undo.last() else {
                    return Err("nothing to undo");
                };
                let shapes = clone_shapes(&previous.shapes)?;
                let element_ids = clone_ids(&previous.element_ids)?;
                let animations = clone_animations(&previous.animations)?;
                let mut undo = clone_history(&self.undo)?;
                undo.pop();
                let mut redo = clone_history(&self.redo)?;
                redo.try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                redo.push(self.snapshot()?);
                (shapes, element_ids, animations, undo, redo, "Undo complete")
            }
            Operation::Redo => {
                let Some(next) = self.redo.last() else {
                    return Err("nothing to redo");
                };
                let shapes = clone_shapes(&next.shapes)?;
                let element_ids = clone_ids(&next.element_ids)?;
                let animations = clone_animations(&next.animations)?;
                let mut redo = clone_history(&self.redo)?;
                redo.pop();
                let mut undo = clone_history(&self.undo)?;
                if undo.len() == HISTORY_LIMIT {
                    undo.remove(0);
                }
                undo.try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                undo.push(self.snapshot()?);
                (shapes, element_ids, animations, undo, redo, "Redo complete")
            }
            _ => return Err("read-only operation cannot be staged"),
        };
        let document = animation::rebuild_document(&shapes, &element_ids, &animations)?;
        let dirty = !shapes.is_empty();
        let duration_seconds = document_duration(&document)?;
        let next_element_id = if matches!(operation, Operation::Add(_)) {
            self.next_element_id
                .checked_add(1)
                .ok_or("preview stable ID limit reached")?
        } else {
            self.next_element_id
        };
        Ok((
            Self {
                document,
                shapes,
                element_ids,
                next_element_id,
                animations,
                undo,
                redo,
                dirty,
                playing: self.playing && duration_seconds > 0.0,
                time_seconds: if duration_seconds > 0.0 {
                    self.time_seconds.rem_euclid(duration_seconds)
                } else {
                    0.0
                },
                duration_seconds,
            },
            message.to_owned(),
        ))
    }

    pub fn read_result(&self, operation: &Operation) -> Result<String, &'static str> {
        match operation {
            Operation::List => Ok(format!(
                "{{\"documents\":[{{\"documentId\":\"{DOCUMENT_ID}\",\"name\":\"Live preview\",\"width\":512,\"height\":512}}]}}"
            )),
            Operation::Info => Ok(format!(
                "{{\"documentId\":\"{DOCUMENT_ID}\",\"width\":512,\"height\":512,\"elementCount\":{},\"sourceBytes\":{},\"dirty\":{}}}",
                self.shapes.len(),
                self.document.source().map_err(document_error)?.len(),
                self.dirty
            )),
            Operation::AnimationList => animation_list_json(&self.document, &self.element_ids),
            Operation::Control(control) => {
                let (playing, time) = match *control {
                    Control::Play => (self.duration_seconds > 0.0, self.time_seconds),
                    Control::Pause => (false, self.time_seconds),
                    Control::Seek(seconds)
                        if seconds.is_finite() && (0.0..=10.0).contains(&seconds) =>
                    {
                        (self.playing, seconds)
                    }
                    Control::Seek(_) => return Err("invalid preview animation time"),
                };
                Ok(format!(
                    "{{\"ok\":true,\"playing\":{playing},\"timeSeconds\":{time:.3}}}"
                ))
            }
            _ => Err("operation changes the preview"),
        }
    }

    fn snapshot(&self) -> Result<Snapshot, &'static str> {
        Ok(Snapshot {
            shapes: clone_shapes(&self.shapes)?,
            element_ids: clone_ids(&self.element_ids)?,
            animations: clone_animations(&self.animations)?,
        })
    }
}

fn document_error(error: V1DocumentError) -> &'static str {
    match error {
        V1DocumentError::InvalidDimensions
        | V1DocumentError::InvalidGeometry
        | V1DocumentError::InvalidColor
        | V1DocumentError::InvalidStableId
        | V1DocumentError::MissingStableId
        | V1DocumentError::InvalidPathData
        | V1DocumentError::InvalidStrokeWidth
        | V1DocumentError::Unsupported
        | V1DocumentError::UnsupportedTransform => "shape validation failed",
        V1DocumentError::InvalidAnimationFragment => "invalid preview animation fragment",
        V1DocumentError::Allocation | V1DocumentError::FormattingLimit => {
            "preview allocation or formatting failed"
        }
        V1DocumentError::Edit(_) => "document edit failed",
    }
}
fn clone_shapes(input: &[Shape]) -> Result<Vec<Shape>, &'static str> {
    let mut v = Vec::new();
    v.try_reserve_exact(input.len())
        .map_err(|_| "preview allocation failed")?;
    v.extend_from_slice(input);
    Ok(v)
}
fn clone_history(input: &[Snapshot]) -> Result<Vec<Snapshot>, &'static str> {
    let mut v = Vec::new();
    v.try_reserve_exact(input.len())
        .map_err(|_| "preview allocation failed")?;
    for s in input {
        v.push(Snapshot {
            shapes: clone_shapes(&s.shapes)?,
            element_ids: clone_ids(&s.element_ids)?,
            animations: clone_animations(&s.animations)?,
        });
    }
    Ok(v)
}
fn clone_ids(input: &[String]) -> Result<Vec<String>, &'static str> {
    let mut v = Vec::new();
    v.try_reserve_exact(input.len())
        .map_err(|_| "preview allocation failed")?;
    v.extend(input.iter().cloned());
    Ok(v)
}
fn clone_animations(input: &[AnimationFragment]) -> Result<Vec<AnimationFragment>, &'static str> {
    let mut v = Vec::new();
    v.try_reserve_exact(input.len())
        .map_err(|_| "preview allocation failed")?;
    for animation in input {
        let mut xml = Vec::new();
        xml.try_reserve_exact(animation.xml.len())
            .map_err(|_| "preview allocation failed")?;
        xml.extend_from_slice(&animation.xml);
        v.push(AnimationFragment {
            target_id: animation.target_id.clone(),
            xml,
        });
    }
    Ok(v)
}
pub(crate) fn object_id(index: u64) -> String {
    format!("vexel-object-{index}")
}
pub struct UiRequest {
    pub operation: Operation,
    pub reply: std::sync::mpsc::SyncSender<Result<String, &'static str>>,
}
pub struct LiveBackend {
    requests: SyncSender<UiRequest>,
}
impl LiveBackend {
    pub fn new(requests: SyncSender<UiRequest>) -> Self {
        Self { requests }
    }
}
impl ProtocolBackend for LiveBackend {
    fn call_tool(
        &mut self,
        doc: &JsonDocument<'_>,
        args: usize,
        name: &str,
    ) -> Option<ToolResponse> {
        let op = match parse_operation(doc, args, name) {
            Ok(v) => v,
            Err(e) => return Some(tool_error(e)),
        };
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        if self
            .requests
            .send(UiRequest {
                operation: op,
                reply: tx,
            })
            .is_err()
        {
            return Some(tool_error("preview window is closed"));
        }
        Some(match rx.recv() {
            Ok(Ok(json)) => ToolResponse {
                structured_json: json.into_bytes(),
                content_json: b"[]".to_vec(),
                is_error: false,
            },
            Ok(Err(e)) => tool_error(e),
            Err(_) => tool_error("preview window is closed"),
        })
    }
}
fn tool_error(message: &str) -> ToolResponse {
    let text = match message {
        "preview window is closed" => message,
        "nothing to undo" => message,
        "nothing to redo" => message,
        "preview scene limit reached" => message,
        "unsupported preview shape" => message,
        "unknown preview document" => message,
        "invalid preview fill color" => message,
        "unsupported preview operation" => message,
        "path not granted" => message,
        "path not found" => message,
        "unsupported file type" => message,
        "invalid project file" => message,
        "unknown element" => message,
        "unsupported attribute" => message,
        "document has no path" => message,
        "export_svg too large" => message,
        "file operation failed" => message,
        m if m.starts_with("export ") => m,
        _ => "invalid preview tool arguments",
    };
    let mut s = b"{\"error\":\"".to_vec();
    s.extend_from_slice(text.as_bytes());
    s.extend_from_slice(b"\"}");
    ToolResponse {
        structured_json: s,
        content_json: format!("[{{\"type\":\"text\",\"text\":\"{text}\"}}]").into_bytes(),
        is_error: true,
    }
}
pub(super) fn parse_operation(
    doc: &JsonDocument<'_>,
    args: usize,
    name: &str,
) -> Result<Operation, &'static str> {
    if name == "document_list" {
        return Ok(Operation::List);
    }
    if name == "document_open" {
        return args::open(doc, args);
    }
    if string_field(doc, args, b"documentId").as_deref() != Some(DOCUMENT_ID) {
        return Err("unknown preview document");
    }
    match name {
        "document_info" => Ok(Operation::Info),
        "save" => Ok(Operation::Save),
        "export_svg" => Ok(Operation::ExportSvg),
        "render_png" => crate::session::mcp_png::parse(doc, args),
        "save_as" => args::save_as(doc, args),
        "attribute_set" => args::attribute_set(doc, args),
        "element_delete" => args::delete(doc, args),
        "animation_list" => Ok(Operation::AnimationList),
        "undo" => Ok(Operation::Undo),
        "redo" => Ok(Operation::Redo),
        "animation_add" => {
            let target_id =
                string_field(doc, args, b"targetId").ok_or("invalid preview tool arguments")?;
            if target_id.len() > 128 || !target_id.starts_with("vexel-object-") {
                return Err("invalid preview tool arguments");
            }
            let token = doc
                .object_get(args, b"xml")
                .ok_or("invalid preview tool arguments")?;
            if doc
                .raw_token(token)
                .is_none_or(|raw| raw.len() > ANIMATION_XML_LIMIT * 6 + 2)
            {
                return Err("invalid preview tool arguments");
            }
            let xml = doc
                .decoded_string(token)
                .ok_or("invalid preview tool arguments")?;
            validate_animation_fragment(&xml)?;
            Ok(Operation::AnimationAdd { target_id, xml })
        }
        "animation_control" => {
            let action = string_field(doc, args, b"action")
                .or_else(|| string_field(doc, args, b"operation"))
                .ok_or("invalid preview tool arguments")?;
            let control = match action.as_str() {
                "play" => Control::Play,
                "pause" => Control::Pause,
                "seek" => {
                    let token = doc
                        .object_get(args, b"timeMs")
                        .ok_or("invalid preview tool arguments")?;
                    if doc
                        .token(token)
                        .is_none_or(|t| !matches!(t.kind, JsonType::Number | JsonType::String))
                    {
                        return Err("invalid preview tool arguments");
                    }
                    let raw = if doc.token(token).is_some_and(|t| t.kind == JsonType::String) {
                        String::from_utf8(
                            doc.decoded_string(token)
                                .ok_or("invalid preview tool arguments")?,
                        )
                        .map_err(|_| "invalid preview tool arguments")?
                    } else {
                        std::str::from_utf8(
                            doc.raw_token(token)
                                .ok_or("invalid preview tool arguments")?,
                        )
                        .map_err(|_| "invalid preview tool arguments")?
                        .to_owned()
                    };
                    if raw.len() > 32 {
                        return Err("invalid preview tool arguments");
                    }
                    let milliseconds = raw
                        .parse::<f64>()
                        .ok()
                        .filter(|n| n.is_finite())
                        .ok_or("invalid preview tool arguments")?;
                    if !(0.0..=10_000.0).contains(&milliseconds) {
                        return Err("invalid preview tool arguments");
                    }
                    Control::Seek(milliseconds / 1000.0)
                }
                _ => return Err("invalid preview tool arguments"),
            };
            Ok(Operation::Control(control))
        }
        "export_animation" => {
            let path = doc
                .object_get(args, b"path")
                .filter(|t| {
                    doc.raw_token(*t)
                        .is_some_and(|raw| raw.len() <= 1023 * 6 + 2)
                })
                .and_then(|t| doc.decoded_string(t))
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .filter(|p| !p.is_empty() && p.len() < 1024)
                .ok_or("invalid preview tool arguments")?;
            if string_field(doc, args, b"format").as_deref() != Some("gif") {
                return Err("invalid preview tool arguments");
            }
            let json_number = |key: &[u8]| {
                doc.object_get(args, key)
                    .filter(|t| {
                        doc.token(*t)
                            .is_some_and(|tok| tok.kind == JsonType::Number)
                    })
                    .and_then(|t| doc.raw_token(t))
                    .filter(|raw| raw.len() <= 32)
                    .and_then(|raw| std::str::from_utf8(raw).ok()?.parse::<f64>().ok())
                    .filter(|n| n.is_finite())
                    .ok_or("invalid preview tool arguments")
            };
            let fps = json_number(b"fps")?;
            let duration = json_number(b"duration")?;
            if fps.fract() != 0.0
                || !(1.0..=30.0).contains(&fps)
                || !(0.01..=10.0).contains(&duration)
            {
                return Err("invalid preview tool arguments");
            }
            Ok(Operation::Export {
                path,
                fps: fps as u32,
                duration,
            })
        }
        "element_add" => {
            if string_field(doc, args, b"parentId")
                .as_deref()
                .is_some_and(|p| p != "root")
            {
                return Err("invalid preview tool arguments");
            }
            let template =
                string_field(doc, args, b"template").ok_or("invalid preview tool arguments")?;
            let attrs = doc
                .object_get(args, b"attributes")
                .ok_or("invalid preview tool arguments")?;
            if doc.token(attrs).is_none_or(|t| t.kind != JsonType::Object) {
                return Err("invalid preview tool arguments");
            }
            let allowed: &[&[u8]] = match template.as_str() {
                "rect" => &[b"x", b"y", b"width", b"height", b"fill"],
                "ellipse" => &[b"cx", b"cy", b"rx", b"ry", b"fill"],
                "line" => args::LINE_KEYS,
                "path" => args::PATH_KEYS,
                _ => return Err("unsupported preview shape"),
            };
            let mut key = doc.token(attrs).and_then(|t| t.first_child);
            while let Some(key_token) = key {
                let name = doc
                    .decoded_string(key_token)
                    .ok_or("invalid preview tool arguments")?;
                if !allowed
                    .iter()
                    .any(|candidate| *candidate == name.as_slice())
                {
                    return Err("invalid preview tool arguments");
                }
                let value = doc
                    .token(key_token)
                    .and_then(|t| t.next_sibling)
                    .ok_or("invalid preview tool arguments")?;
                if doc.token(value).is_none_or(|t| t.kind != JsonType::String) {
                    return Err("invalid preview tool arguments");
                }
                key = doc.token(value).and_then(|t| t.next_sibling);
            }
            match template.as_str() {
                "line" => return args::line(doc, attrs),
                "path" => return args::path(doc, attrs),
                _ => {}
            }
            let num = |k: &[u8]| number_field(doc, attrs, k);
            let fill = parse_color(
                string_field(doc, attrs, b"fill")
                    .ok_or("invalid preview tool arguments")?
                    .as_str(),
            )?;
            let shape = match template.as_str() {
                "rect" => Shape::Rect {
                    x: num(b"x")?,
                    y: num(b"y")?,
                    width: num(b"width")?,
                    height: num(b"height")?,
                    rgb: fill,
                },
                "ellipse" => Shape::Ellipse {
                    cx: num(b"cx")?,
                    cy: num(b"cy")?,
                    rx: num(b"rx")?,
                    ry: num(b"ry")?,
                    rgb: fill,
                },
                _ => return Err("unsupported preview shape"),
            };
            validate_shape(shape)?;
            Ok(Operation::Add(shape))
        }
        _ => Err("unsupported preview operation"),
    }
}
fn string_field(doc: &JsonDocument<'_>, obj: usize, key: &[u8]) -> Option<String> {
    let token = doc.object_get(obj, key)?;
    if doc.raw_token(token)?.len() > 256 {
        return None;
    }
    String::from_utf8(doc.decoded_string(token)?).ok()
}
fn number_field(doc: &JsonDocument<'_>, obj: usize, key: &[u8]) -> Result<f64, &'static str> {
    let text = string_field(doc, obj, key).ok_or("invalid preview tool arguments")?;
    if text.len() > 32 {
        return Err("invalid preview tool arguments");
    }
    text.parse::<f64>()
        .ok()
        .filter(|n: &f64| n.is_finite())
        .ok_or("invalid preview tool arguments")
}
fn parse_color(value: &str) -> Result<u32, &'static str> {
    if value.len() != 7
        || !value.starts_with('#')
        || !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return Err("invalid preview fill color");
    }
    u32::from_str_radix(&value[1..], 16).map_err(|_| "invalid preview fill color")
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
