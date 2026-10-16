#![forbid(unsafe_code)]
//! V1 source-owning artwork document session.

use std::{fmt, fmt::Write as FmtWrite, io::Write as IoWrite, sync::Arc};
use vexel_document_edit::{EditError, EditLimits, Editor, TemplateAttribute, TemplateKind};

const MAX_DIMENSION: u32 = 4096;
const MAX_STABLE_ID_BYTES: usize = 128;
const MAX_ANIMATION_FRAGMENT_BYTES: usize = 8 * 1024;

/// Errors retain the editor's actionable category where an edit API failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum V1DocumentError {
    InvalidDimensions,
    InvalidGeometry,
    InvalidColor,
    Allocation,
    FormattingLimit,
    InvalidStableId,
    MissingStableId,
    InvalidAnimationFragment,
    Edit(EditError),
}

impl From<EditError> for V1DocumentError {
    fn from(value: EditError) -> Self {
        Self::Edit(value)
    }
}

/// A bounded SVG session. The editor's source is the sole shape representation.
#[derive(Debug)]
pub struct V1Document {
    width: u32,
    height: u32,
    editor: Editor,
}

impl V1Document {
    /// Creates an empty, valid SVG document with dimensions from 1 through 4096.
    pub fn new(width: u32, height: u32) -> Result<Self, V1DocumentError> {
        if !(1..=MAX_DIMENSION).contains(&width) || !(1..=MAX_DIMENSION).contains(&height) {
            return Err(V1DocumentError::InvalidDimensions);
        }
        let mut source = Vec::new();
        source
            .try_reserve(160)
            .map_err(|_| V1DocumentError::Allocation)?;
        write!(
            source,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"/>"
        )
        .map_err(|_| V1DocumentError::Allocation)?;
        let source = Arc::new(source);
        let result = Editor::create(&source, EditLimits::default());
        if result.error != EditError::Ok {
            return Err(V1DocumentError::Edit(result.error));
        }
        Ok(Self {
            width,
            height,
            editor: result
                .editor
                .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?,
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }
    pub const fn height(&self) -> u32 {
        self.height
    }
    pub fn is_dirty(&self) -> bool {
        self.editor.is_dirty()
    }
    pub fn mark_saved(&mut self) {
        self.editor.mark_saved();
    }
    pub fn history_count(&self) -> usize {
        self.editor.history_count()
    }
    pub fn history_cursor(&self) -> usize {
        self.editor.history_cursor()
    }

    /// Returns a fallibly allocated exact snapshot of the editable SVG bytes.
    pub fn source(&self) -> Result<Vec<u8>, V1DocumentError> {
        let length = self.editor.byte_length();
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| V1DocumentError::Allocation)?;
        bytes.resize(length, 0);
        self.editor.materialize(&mut bytes)?;
        Ok(bytes)
    }

    /// Inserts a rectangle from two document-space drag points.
    /// Coordinates are emitted using Rust's shortest round-trip decimal form.
    pub fn add_rectangle(
        &mut self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        fill: &str,
    ) -> Result<(), V1DocumentError> {
        self.add_rectangle_impl(start_x, start_y, end_x, end_y, fill, None)
    }

    /// Inserts a rectangle with a caller-selected stable source ID.
    pub fn add_rectangle_with_id(
        &mut self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        fill: &str,
        element_id: &str,
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(element_id)?;
        self.add_rectangle_impl(start_x, start_y, end_x, end_y, fill, Some(element_id))
    }

    fn add_rectangle_impl(
        &mut self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        fill: &str,
        element_id: Option<&str>,
    ) -> Result<(), V1DocumentError> {
        if ![start_x, start_y, end_x, end_y]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(V1DocumentError::InvalidGeometry);
        }
        if start_x < 0.0
            || start_y < 0.0
            || end_x < 0.0
            || end_y < 0.0
            || start_x > f64::from(self.width)
            || end_x > f64::from(self.width)
            || start_y > f64::from(self.height)
            || end_y > f64::from(self.height)
            || start_x == end_x
            || start_y == end_y
        {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let fill = fill.as_bytes();
        if fill.len() != 7 || fill[0] != b'#' || !fill[1..].iter().all(u8::is_ascii_hexdigit) {
            return Err(V1DocumentError::InvalidColor);
        }
        let x = start_x.min(end_x);
        let y = start_y.min(end_y);
        let width = (start_x.max(end_x)) - x;
        let height = (start_y.max(end_y)) - y;
        let values = [
            numeric_text(x)?,
            numeric_text(y)?,
            numeric_text(width)?,
            numeric_text(height)?,
        ];
        let id = element_id.unwrap_or("").as_bytes();
        let attrs = [
            TemplateAttribute {
                name: b"x",
                value: &values[0],
            },
            TemplateAttribute {
                name: b"y",
                value: &values[1],
            },
            TemplateAttribute {
                name: b"width",
                value: &values[2],
            },
            TemplateAttribute {
                name: b"height",
                value: &values[3],
            },
            TemplateAttribute {
                name: b"fill",
                value: fill,
            },
            TemplateAttribute {
                name: b"id",
                value: id,
            },
        ];
        let root = self
            .editor
            .root()
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
        self.editor.insert_template(
            root,
            TemplateKind::Rect,
            &attrs[..5 + usize::from(element_id.is_some())],
        )?;
        Ok(())
    }

    /// Inserts an ellipse from two document-space drag points.
    /// Coordinates use the same bounded shortest scientific representation as rectangles.
    pub fn add_ellipse(
        &mut self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        fill: &str,
    ) -> Result<(), V1DocumentError> {
        self.add_ellipse_impl(start_x, start_y, end_x, end_y, fill, None)
    }

    /// Inserts an ellipse with a caller-selected stable source ID.
    pub fn add_ellipse_with_id(
        &mut self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        fill: &str,
        element_id: &str,
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(element_id)?;
        self.add_ellipse_impl(start_x, start_y, end_x, end_y, fill, Some(element_id))
    }

    fn add_ellipse_impl(
        &mut self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        fill: &str,
        element_id: Option<&str>,
    ) -> Result<(), V1DocumentError> {
        if ![start_x, start_y, end_x, end_y]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(V1DocumentError::InvalidGeometry);
        }
        if start_x < 0.0
            || start_y < 0.0
            || end_x < 0.0
            || end_y < 0.0
            || start_x > f64::from(self.width)
            || end_x > f64::from(self.width)
            || start_y > f64::from(self.height)
            || end_y > f64::from(self.height)
            || start_x == end_x
            || start_y == end_y
        {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let fill = fill.as_bytes();
        if fill.len() != 7 || fill[0] != b'#' || !fill[1..].iter().all(u8::is_ascii_hexdigit) {
            return Err(V1DocumentError::InvalidColor);
        }
        let left = start_x.min(end_x);
        let right = start_x.max(end_x);
        let top = start_y.min(end_y);
        let bottom = start_y.max(end_y);
        let rx = (right - left) / 2.0;
        let ry = (bottom - top) / 2.0;
        if rx <= 0.0 || ry <= 0.0 {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let values = [
            numeric_text((left + right) / 2.0)?,
            numeric_text((top + bottom) / 2.0)?,
            numeric_text(rx)?,
            numeric_text(ry)?,
        ];
        let id = element_id.unwrap_or("").as_bytes();
        let attrs = [
            TemplateAttribute {
                name: b"cx",
                value: &values[0],
            },
            TemplateAttribute {
                name: b"cy",
                value: &values[1],
            },
            TemplateAttribute {
                name: b"rx",
                value: &values[2],
            },
            TemplateAttribute {
                name: b"ry",
                value: &values[3],
            },
            TemplateAttribute {
                name: b"fill",
                value: fill,
            },
            TemplateAttribute {
                name: b"id",
                value: id,
            },
        ];
        let root = self
            .editor
            .root()
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
        self.editor.insert_template(
            root,
            TemplateKind::Ellipse,
            &attrs[..5 + usize::from(element_id.is_some())],
        )?;
        Ok(())
    }

    /// Inserts one bounded `<animate>` XML element beneath an existing rectangle
    /// or ellipse identified by its stable source ID. The edit is committed by
    /// the source-owning editor as one undoable operation.
    pub fn add_animation_fragment(
        &mut self,
        element_id: &str,
        fragment: &[u8],
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(element_id)?;
        let stable_id = element_id.as_bytes();
        if fragment.is_empty()
            || fragment.len() > MAX_ANIMATION_FRAGMENT_BYTES
            || !is_animation_element(fragment)
        {
            return Err(V1DocumentError::InvalidAnimationFragment);
        }
        let target = self
            .editor
            .try_find_id(stable_id)?
            .ok_or(V1DocumentError::MissingStableId)?;
        if !matches!(self.editor.tag_name(target), Some(b"rect" | b"ellipse")) {
            return Err(V1DocumentError::MissingStableId);
        }
        self.editor.insert_xml_fragment(target, fragment)?;
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), V1DocumentError> {
        self.editor.undo().map_err(Into::into)
    }
    pub fn redo(&mut self) -> Result<(), V1DocumentError> {
        self.editor.redo().map_err(Into::into)
    }
}

fn is_animation_element(fragment: &[u8]) -> bool {
    let Some(rest) = fragment.strip_prefix(b"<animate") else {
        return false;
    };
    matches!(
        rest.first(),
        Some(b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>')
    )
}

fn validate_stable_id(element_id: &str) -> Result<(), V1DocumentError> {
    let bytes = element_id.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_STABLE_ID_BYTES
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(byte))
    {
        return Err(V1DocumentError::InvalidStableId);
    }
    Ok(())
}

fn numeric_text(value: f64) -> Result<Vec<u8>, V1DocumentError> {
    // Shortest scientific formatting needs at most 23 bytes: 17 significant
    // digits, a decimal point, `e`, an exponent sign, and three exponent digits.
    // A fixed stack buffer means even the 5e-324 case never grows a String.
    let mut buffer = FixedText {
        bytes: [0; 32],
        length: 0,
    };
    write!(&mut buffer, "{value:e}").map_err(|_| V1DocumentError::FormattingLimit)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(buffer.length)
        .map_err(|_| V1DocumentError::Allocation)?;
    output.extend_from_slice(&buffer.bytes[..buffer.length]);
    Ok(output)
}

struct FixedText {
    bytes: [u8; 32],
    length: usize,
}

impl fmt::Write for FixedText {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self.length.checked_add(value.len()).ok_or(fmt::Error)?;
        let target = self.bytes.get_mut(self.length..end).ok_or(fmt::Error)?;
        target.copy_from_slice(value.as_bytes());
        self.length = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
