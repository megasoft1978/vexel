//! Text operations on `V1Document`.

use crate::{ElementKind, V1Document, V1DocumentError, validate_stable_id};
use vexel_document_edit::{DocumentHandle, HistoryAction, TemplateAttribute, encode_attribute};

const MAX_COORDINATE: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSpec {
    pub x: f64,
    pub y: f64,
    pub content: String,
    pub font_size: f64,
    pub anchor: TextAnchor,
    pub fill: String,
}

fn check_color(value: &str) -> Result<(), V1DocumentError> {
    let bytes = value.as_bytes();
    if bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidColor)
    }
}

fn check_content(content: &str) -> Result<(), V1DocumentError> {
    let count = content.chars().count();
    if !(1..=256).contains(&count) || content.chars().any(char::is_control) {
        Err(V1DocumentError::InvalidGeometry)
    } else {
        Ok(())
    }
}

fn coordinate(value: f64) -> Result<Vec<u8>, V1DocumentError> {
    if !value.is_finite() || value.abs() > MAX_COORDINATE {
        return Err(V1DocumentError::InvalidGeometry);
    }
    Ok(crate::numeric_text(value)?)
}

impl V1Document {
    pub fn add_text(&mut self, spec: &TextSpec) -> Result<String, V1DocumentError> {
        check_content(&spec.content)?;
        check_color(&spec.fill)?;
        if !spec.font_size.is_finite() || !(1.0..=1024.0).contains(&spec.font_size) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let x = coordinate(spec.x)?;
        let y = coordinate(spec.y)?;
        let size = crate::numeric_text(spec.font_size)?;
        let anchor = match spec.anchor {
            TextAnchor::Start => b"start".as_slice(),
            TextAnchor::Middle => b"middle".as_slice(),
            TextAnchor::End => b"end".as_slice(),
        };
        let attrs = [
            TemplateAttribute {
                name: b"x",
                value: &x,
            },
            TemplateAttribute {
                name: b"y",
                value: &y,
            },
            TemplateAttribute {
                name: b"font-size",
                value: &size,
            },
            TemplateAttribute {
                name: b"text-anchor",
                value: anchor,
            },
            TemplateAttribute {
                name: b"fill",
                value: spec.fill.as_bytes(),
            },
        ];
        let root = self.editor.root().ok_or(V1DocumentError::Unsupported)?;
        let handle = self
            .editor
            .insert_text_element(root, &attrs, spec.content.as_bytes())?;
        for index in 0..self.editor.attribute_count(handle) {
            if let Some(attribute) = self.editor.attribute_at(handle, index) {
                if self.editor.span_bytes(attribute.name) == Some(b"id") {
                    let mut bytes = vec![0; attribute.value.length];
                    self.editor.decoded_span(attribute.value, &mut bytes)?;
                    return String::from_utf8(bytes).map_err(|_| V1DocumentError::InvalidStableId);
                }
            }
        }
        Err(V1DocumentError::MissingStableId)
    }

    pub fn set_text(&mut self, id: &str, content: &str) -> Result<(), V1DocumentError> {
        check_content(content)?;
        let node = self.find_text(id)?;
        self.editor.set_text_content(node, content.as_bytes())?;
        Ok(())
    }

    pub fn set_text_fill(&mut self, id: &str, color: &str) -> Result<(), V1DocumentError> {
        check_color(color)?;
        let node = self.find_text(id)?;
        let attribute = (0..self.editor.attribute_count(node))
            .find_map(|index| {
                let attribute = self.editor.attribute_at(node, index)?;
                (self.editor.span_bytes(attribute.name) == Some(b"fill")).then_some(attribute)
            })
            .ok_or(V1DocumentError::Unsupported)?;
        let replacement = encode_attribute(color.as_bytes(), attribute.quote)?;
        self.editor
            .replace_source_span(attribute.value, &replacement)?;
        self.editor
            .history_relabel_latest(HistoryAction::EditAttribute);
        Ok(())
    }

    fn find_text(&self, id: &str) -> Result<DocumentHandle, V1DocumentError> {
        validate_stable_id(id)?;
        let node = self
            .editor
            .try_find_id(id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)?;
        if self.element_kind(id)? != ElementKind::Text {
            return Err(V1DocumentError::Unsupported);
        }
        Ok(node)
    }
}
