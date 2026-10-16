#![allow(dead_code)]

use std::sync::mpsc::SyncSender;
use vexel_json::{JsonDocument, JsonType};
use vexel_protocol::{ProtocolBackend, ToolResponse};
use vexel_v1_document::{V1Document, V1DocumentError};

pub const DOCUMENT_ID: &str = "preview-document";
pub const SCENE_LIMIT: usize = 128;
pub const CANVAS_SIZE: u32 = 512;
const HISTORY_LIMIT: usize = 128;

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
    Undo,
    Redo,
}

#[derive(Debug)]
pub struct PreviewState {
    document: V1Document,
    shapes: Vec<Shape>,
    undo: Vec<Vec<Shape>>,
    redo: Vec<Vec<Shape>>,
    dirty: bool,
}

impl PreviewState {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self {
            document: V1Document::new(512, 512).map_err(document_error)?,
            shapes: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
        })
    }
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn source(&self) -> Result<Vec<u8>, &'static str> {
        self.document.source().map_err(document_error)
    }

    /// Build an entire replacement before changing the main-thread-owned state.
    pub fn stage(&self, operation: &Operation) -> Result<(Self, String), &'static str> {
        let (shapes, undo, redo, message) = match operation {
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
                let mut undo = clone_history(&self.undo)?;
                if undo.len() == HISTORY_LIMIT {
                    undo.remove(0);
                }
                undo.try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                undo.push(clone_shapes(&self.shapes)?);
                (shapes, undo, Vec::new(), "Shape added")
            }
            Operation::Undo => {
                let Some(previous) = self.undo.last() else {
                    return Err("nothing to undo");
                };
                let shapes = clone_shapes(previous)?;
                let mut undo = clone_history(&self.undo)?;
                undo.pop();
                let mut redo = clone_history(&self.redo)?;
                redo.try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                redo.push(clone_shapes(&self.shapes)?);
                (shapes, undo, redo, "Undo complete")
            }
            Operation::Redo => {
                let Some(next) = self.redo.last() else {
                    return Err("nothing to redo");
                };
                let shapes = clone_shapes(next)?;
                let mut redo = clone_history(&self.redo)?;
                redo.pop();
                let mut undo = clone_history(&self.undo)?;
                if undo.len() == HISTORY_LIMIT {
                    undo.remove(0);
                }
                undo.try_reserve(1)
                    .map_err(|_| "preview allocation failed")?;
                undo.push(clone_shapes(&self.shapes)?);
                (shapes, undo, redo, "Redo complete")
            }
            _ => return Err("read-only operation cannot be staged"),
        };
        let document = rebuild_document(&shapes)?;
        let dirty = !shapes.is_empty();
        Ok((
            Self {
                document,
                shapes,
                undo,
                redo,
                dirty,
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
            _ => Err("operation changes the preview"),
        }
    }
}

fn document_error(error: V1DocumentError) -> &'static str {
    match error {
        V1DocumentError::InvalidDimensions
        | V1DocumentError::InvalidGeometry
        | V1DocumentError::InvalidColor => "shape validation failed",
        V1DocumentError::Allocation | V1DocumentError::FormattingLimit => {
            "preview allocation or formatting failed"
        }
        V1DocumentError::InvalidStableId
        | V1DocumentError::MissingStableId
        | V1DocumentError::InvalidAnimationFragment => "animation validation failed",
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
fn clone_history(input: &[Vec<Shape>]) -> Result<Vec<Vec<Shape>>, &'static str> {
    let mut v = Vec::new();
    v.try_reserve_exact(input.len())
        .map_err(|_| "preview allocation failed")?;
    for s in input {
        v.push(clone_shapes(s)?);
    }
    Ok(v)
}
fn shape_rgb(shape: Shape) -> u32 {
    match shape {
        Shape::Rect { rgb, .. } | Shape::Ellipse { rgb, .. } => rgb,
    }
}
fn rebuild_document(shapes: &[Shape]) -> Result<V1Document, &'static str> {
    let mut doc = V1Document::new(512, 512).map_err(document_error)?;
    for shape in shapes {
        let mut color = [0u8; 7];
        color[0] = b'#';
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let rgb = shape_rgb(*shape);
        for (index, shift) in [20, 16, 12, 8, 4, 0].into_iter().enumerate() {
            color[index + 1] = HEX[((rgb >> shift) & 0xf) as usize];
        }
        let color = std::str::from_utf8(&color).map_err(|_| "preview color encoding failed")?;
        match *shape {
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => doc.add_rectangle(x, y, x + width, y + height, &color),
            Shape::Ellipse { cx, cy, rx, ry, .. } => {
                doc.add_ellipse(cx - rx, cy - ry, cx + rx, cy + ry, &color)
            }
        }
        .map_err(document_error)?;
    }
    Ok(doc)
}
fn validate_shape(shape: Shape) -> Result<(), &'static str> {
    let (l, t, r, b) = match shape {
        Shape::Rect {
            x,
            y,
            width,
            height,
            ..
        } => (x, y, x + width, y + height),
        Shape::Ellipse { cx, cy, rx, ry, .. } => (cx - rx, cy - ry, cx + rx, cy + ry),
    };
    if shape_rgb(shape) > 0x00ff_ffff
        || ![l, t, r, b].into_iter().all(f64::is_finite)
        || l < 0.0
        || t < 0.0
        || r > 512.0
        || b > 512.0
        || r <= l
        || b <= t
    {
        return Err("shape geometry must be finite, non-degenerate, and inside 512x512");
    }
    Ok(())
}

/// Deterministic fixed-size renderer for the intentionally limited preview scene.
pub fn rasterize(shapes: &[Shape]) -> Result<Vec<u8>, &'static str> {
    if shapes.len() > SCENE_LIMIT {
        return Err("preview scene limit reached");
    }
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(512 * 512 * 4)
        .map_err(|_| "preview allocation failed")?;
    pixels.resize(512 * 512 * 4, 255);
    for s in shapes {
        validate_shape(*s)?;
        let rgb = shape_rgb(*s);
        let color = [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8];
        let (l, t, r, b) = match *s {
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => (x, y, x + width, y + height),
            Shape::Ellipse { cx, cy, rx, ry, .. } => (cx - rx, cy - ry, cx + rx, cy + ry),
        };
        for y in t.floor().max(0.0) as u32..b.ceil().min(512.0) as u32 {
            for x in l.floor().max(0.0) as u32..r.ceil().min(512.0) as u32 {
                let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                let inside = match *s {
                    Shape::Rect { .. } => px >= l && px < r && py >= t && py < b,
                    Shape::Ellipse { cx, cy, rx, ry, .. } => {
                        ((px - cx) / rx).powi(2) + ((py - cy) / ry).powi(2) <= 1.0
                    }
                };
                if inside {
                    let at = (y as usize * 512 + x as usize) * 4;
                    pixels[at..at + 3].copy_from_slice(&color);
                    pixels[at + 3] = 255;
                }
            }
        }
    }
    Ok(pixels)
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
    if string_field(doc, args, b"documentId").as_deref() != Some(DOCUMENT_ID) {
        return Err("unknown preview document");
    }
    match name {
        "document_info" => Ok(Operation::Info),
        "undo" => Ok(Operation::Undo),
        "redo" => Ok(Operation::Redo),
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
mod tests {
    use super::*;
    #[test]
    fn raster_bounds_mapping_and_pixel_change() {
        let p = rasterize(&[Shape::Rect {
            x: 10.,
            y: 20.,
            width: 30.,
            height: 40.,
            rgb: 0x123456,
        }])
        .unwrap();
        assert_eq!(
            &p[(25 * 512 + 20) * 4..(25 * 512 + 20) * 4 + 4],
            &[0x12, 0x34, 0x56, 255]
        );
        assert_eq!(&p[..4], &[255, 255, 255, 255]);
        let e = rasterize(&[Shape::Ellipse {
            cx: 100.,
            cy: 100.,
            rx: 20.,
            ry: 10.,
            rgb: 0xff0000,
        }])
        .unwrap();
        assert_eq!(
            &e[(100 * 512 + 100) * 4..(100 * 512 + 100) * 4 + 3],
            &[255, 0, 0]
        );
    }
    #[test]
    fn invalid_operation_does_not_change_source_or_history() {
        let s = PreviewState::new().unwrap();
        let source = s.source().unwrap();
        assert!(
            s.stage(&Operation::Add(Shape::Rect {
                x: 510.,
                y: 1.,
                width: 5.,
                height: 2.,
                rgb: 0
            }))
            .is_err()
        );
        assert_eq!(s.source().unwrap(), source);
        assert!(!s.is_dirty());
        let edited = s
            .stage(&Operation::Add(Shape::Rect {
                x: 2.,
                y: 2.,
                width: 2.,
                height: 2.,
                rgb: 0,
            }))
            .unwrap()
            .0;
        let (restored, _) = edited.stage(&Operation::Undo).unwrap();
        assert_eq!(restored.source().unwrap(), source);
        assert!(!restored.is_dirty());
    }
    #[test]
    fn undo_redo_restore_exact_source_and_branch_order() {
        let blank = PreviewState::new().unwrap();
        let first = Operation::Add(Shape::Rect {
            x: 1.,
            y: 1.,
            width: 2.,
            height: 2.,
            rgb: 1,
        });
        let (one, _) = blank.stage(&first).unwrap();
        let source = one.source().unwrap();
        let (two, _) = one
            .stage(&Operation::Add(Shape::Ellipse {
                cx: 8.,
                cy: 8.,
                rx: 2.,
                ry: 3.,
                rgb: 2,
            }))
            .unwrap();
        let (back, _) = two.stage(&Operation::Undo).unwrap();
        assert_eq!(back.shapes().len(), 1);
        assert_eq!(back.source().unwrap(), source);
        let (redo, _) = back.stage(&Operation::Redo).unwrap();
        assert_eq!(redo.source().unwrap(), two.source().unwrap());
        let (branched, _) = back.stage(&first).unwrap();
        assert!(branched.stage(&Operation::Redo).is_err());
    }
    #[test]
    fn rejects_invalid_shapes_and_caps_scene() {
        for s in [
            Shape::Rect {
                x: f64::NAN,
                y: 0.,
                width: 1.,
                height: 1.,
                rgb: 0,
            },
            Shape::Rect {
                x: 0.,
                y: 0.,
                width: 0.,
                height: 1.,
                rgb: 0,
            },
            Shape::Ellipse {
                cx: 511.,
                cy: 2.,
                rx: 2.,
                ry: 1.,
                rgb: 0,
            },
        ] {
            assert!(validate_shape(s).is_err())
        }
        let mut state = PreviewState::new().unwrap();
        for _ in 0..SCENE_LIMIT {
            state = state
                .stage(&Operation::Add(Shape::Rect {
                    x: 1.,
                    y: 1.,
                    width: 1.,
                    height: 1.,
                    rgb: 0,
                }))
                .unwrap()
                .0;
        }
        assert!(
            state
                .stage(&Operation::Add(Shape::Rect {
                    x: 2.,
                    y: 2.,
                    width: 1.,
                    height: 1.,
                    rgb: 0
                }))
                .is_err()
        );
    }

    #[test]
    fn request_mapping_rejects_non_finite_color_and_extra_attributes() {
        fn parse(attributes: &str) -> Result<Operation, &'static str> {
            let json = format!(
                "{{\"documentId\":\"{DOCUMENT_ID}\",\"parentId\":\"root\",\"template\":\"rect\",\"attributes\":{attributes}}}"
            );
            let doc = JsonDocument::parse_owned(json.as_bytes()).unwrap();
            parse_operation(&doc, doc.root(), "element_add")
        }
        assert!(
            parse(r##"{"x":"1e999","y":"1","width":"2","height":"3","fill":"#AABBCC"}"##).is_err()
        );
        assert!(parse(r#"{"x":"1","y":"1","width":"2","height":"3","fill":"red"}"#).is_err());
        assert!(
            parse(
                r##"{"x":"1","y":"1","width":"2","height":"3","fill":"#AABBCC","opacity":"0.5"}"##
            )
            .is_err()
        );
    }
}
