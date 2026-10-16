//! CoreText glyph layout for the engine's `TextLayoutProvider` (macOS only).
//!
//! The engine never depends on a platform crate; this adapter is the wiring
//! the engine documents: `vexel-platform-apple::text_layout` outlines are
//! handed over as scene `TextLayout` values.

use vexel_base::WorkControl;
use vexel_platform_apple as p;
use vexel_scene::{
    MediaError, TableRange, TextCommand, TextGlyph, TextLayout, TextRequest, TextStyle, TextVerb,
};
use vexel_svg_engine::TextLayoutProvider;

pub(crate) struct CoreText;

impl TextLayoutProvider for CoreText {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> Result<TextLayout, MediaError> {
        let style = match request.style {
            TextStyle::Normal => p::TextStyle::Normal,
            TextStyle::Italic => p::TextStyle::Italic,
            TextStyle::Oblique => p::TextStyle::Oblique,
        };
        let layout = p::text_layout(&p::TextRequest {
            text: request.text,
            font_families: request.font_families,
            size_points: request.size_points,
            weight: request.weight,
            style,
            letter_spacing_points: request.letter_spacing_points,
        })
        .map_err(|error| match error {
            p::TextLayoutError::Limit => MediaError::Limit,
            p::TextLayoutError::OutOfMemory => MediaError::OutOfMemory,
            p::TextLayoutError::InvalidUtf8 => MediaError::InvalidUtf8,
            p::TextLayoutError::InvalidArgument => MediaError::InvalidArgument,
            _ => MediaError::Unsupported,
        })?;
        Ok(TextLayout {
            glyphs: layout
                .glyphs
                .iter()
                .map(|g| TextGlyph {
                    glyph_id: Some(g.glyph_id),
                    x: g.x,
                    y: g.y,
                    advance: g.advance,
                    commands: TableRange {
                        first: g.command_start,
                        count: g.command_count,
                    },
                })
                .collect(),
            commands: layout
                .commands
                .iter()
                .map(|c| TextCommand {
                    verb: match c.verb {
                        p::TextVerb::Move => TextVerb::Move,
                        p::TextVerb::Line => TextVerb::Line,
                        p::TextVerb::Cubic => TextVerb::Cubic,
                        p::TextVerb::Close => TextVerb::Close,
                    },
                    values: c.values,
                })
                .collect(),
            advance_width: layout.advance_width,
            ascent: layout.ascent,
            descent: layout.descent,
            leading: layout.leading,
        })
    }
}
