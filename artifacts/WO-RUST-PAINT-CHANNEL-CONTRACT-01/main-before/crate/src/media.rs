use crate::{Diagnostic, ErrorKind, Result, TableRange, WorkControl};
/// Owned premultiplied RGBA8, top row first. Padding is included in the allocation.
#[derive(Debug)]
pub struct ImagePixels {
    pub rgba: Vec<u8>,
    pub byte_count: usize,
    pub row_stride: usize,
    pub width: u32,
    pub height: u32,
}
impl ImagePixels {
    pub fn view(&self) -> ImageView<'_> {
        ImageView {
            rgba: &self.rgba,
            byte_count: self.byte_count,
            row_stride: self.row_stride,
            width: self.width,
            height: self.height,
        }
    }
    pub fn validate(&self, offset: usize) -> Result<()> {
        let row = (self.width as usize).checked_mul(4);
        let bytes = self.row_stride.checked_mul(self.height as usize);
        if self.width == 0
            || self.height == 0
            || row.is_none_or(|r| r > self.row_stride)
            || bytes.is_none_or(|b| b != self.byte_count || b > self.rgba.len())
        {
            Err(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset,
            })
        } else {
            Ok(())
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ImageView<'a> {
    pub rgba: &'a [u8],
    pub byte_count: usize,
    pub row_stride: usize,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    Normal,
    Italic,
    Oblique,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextVerb {
    Move,
    Line,
    Cubic,
    Close,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextCommand {
    pub verb: TextVerb,
    pub values: [f64; 6],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextGlyph {
    pub glyph_id: Option<u16>,
    pub x: f64,
    pub y: f64,
    pub advance: f64,
    pub commands: TableRange,
}
#[derive(Debug)]
pub struct TextLayout {
    pub glyphs: Vec<TextGlyph>,
    pub commands: Vec<TextCommand>,
    pub advance_width: f64,
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
}
#[derive(Debug, Clone, Copy)]
pub struct TextRequest<'a> {
    pub text: &'a [u8],
    pub font_families: &'a [&'a [u8]],
    pub size_points: f64,
    pub weight: f64,
    pub style: TextStyle,
    pub letter_spacing_points: f64,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct TextState {
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub text_bytes: usize,
    pub glyph_count: usize,
    pub pending_space: bool,
    pub has_position: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaError {
    Unsupported,
    Invalid,
    InvalidArgument,
    Limit,
    OutOfMemory,
    InvalidUtf8,
    Animated,
    Corrupt,
}
/// Providers borrow inputs during each call and return Rust-owned results.
/// No default or fake provider is supplied; native implementation is a later gate.
pub trait MediaProvider {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        control: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError>;
    fn decode_image(
        &mut self,
        encoded: &[u8],
        control: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError>;
}
impl TextRequest<'_> {
    /// Validate bounded provider input before entering a native API.
    pub fn validate(&self, control: &mut dyn WorkControl) -> Result<()> {
        control.checkpoint().map_err(crate::types::stopped)?;
        if self.text.len() > 64 * 1024 || self.font_families.len() > 32 {
            return Err(crate::types::limit(crate::LimitKind::TextBytes));
        }
        for byte in self.text {
            let _ = byte;
            control
                .charge(crate::WorkKind::Media, 1)
                .map_err(crate::types::stopped)?;
        }
        if std::str::from_utf8(self.text).is_err()
            || !self.size_points.is_finite()
            || self.size_points <= 0.0
            || self.size_points > 4096.
            || !self.weight.is_finite()
            || !(100.0..=900.0).contains(&self.weight)
            || !self.letter_spacing_points.is_finite()
            || self.letter_spacing_points.abs() > 4096.
        {
            return Err(crate::types::invalid(0));
        }
        for family in self.font_families {
            if family.len() > 256 {
                return Err(crate::types::limit(crate::LimitKind::TextBytes));
            }
            for _ in *family {
                control
                    .charge(crate::WorkKind::Media, 1)
                    .map_err(crate::types::stopped)?;
            }
            if family.is_empty() || std::str::from_utf8(family).is_err() || family.contains(&0) {
                return Err(crate::types::invalid(0));
            }
        }
        Ok(())
    }
}
impl TextLayout {
    pub fn validate(&self, control: &mut dyn WorkControl) -> Result<()> {
        control.checkpoint().map_err(crate::types::stopped)?;
        if self.glyphs.len() > 65536 {
            return Err(crate::types::limit(crate::LimitKind::Glyphs));
        }
        if self.commands.len() > 262144 {
            return Err(crate::types::limit(crate::LimitKind::TextCommands));
        }
        if ![self.advance_width, self.ascent, self.descent, self.leading]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(crate::types::invalid(0));
        }
        for glyph in &self.glyphs {
            control
                .charge(crate::WorkKind::Media, 1)
                .map_err(crate::types::stopped)?;
            glyph.commands.require(self.commands.len(), 0)?;
            if ![glyph.x, glyph.y, glyph.advance]
                .into_iter()
                .all(f64::is_finite)
            {
                return Err(crate::types::invalid(0));
            }
        }
        for command in &self.commands {
            control
                .charge(crate::WorkKind::Media, 1)
                .map_err(crate::types::stopped)?;
            if !command.values.into_iter().all(f64::is_finite) {
                return Err(crate::types::invalid(0));
            }
        }
        Ok(())
    }
    pub fn storage_bytes(&self) -> Result<usize> {
        crate::memory::add(
            crate::memory::bytes(&self.glyphs)?,
            crate::memory::bytes(&self.commands)?,
        )
    }
}
