//! Text policy for the scalar renderer.
//!
//! Solid-fill `<text>` is published by the engine as ordinary filled shapes whose
//! glyph outlines are already cubic-flattened (as the C `text_geometry`), so
//! `render_shapes` fills them nonzero with the shape's fill colour and alpha and
//! needs no text-specific path. Stroke on text and `textPath` remain deferred.

use crate::{RenderError, UnsupportedKind};

/// Rejects the text features the renderer still defers, as a typed error.
pub fn require_fill_only_text(stroked: bool, on_path: bool) -> Result<(), RenderError> {
    if stroked || on_path {
        Err(RenderError::Unsupported(UnsupportedKind::Text))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_stroke_and_text_path_stay_unsupported() {
        assert_eq!(require_fill_only_text(false, false), Ok(()));
        for (stroked, on_path) in [(true, false), (false, true), (true, true)] {
            assert_eq!(
                require_fill_only_text(stroked, on_path),
                Err(RenderError::Unsupported(UnsupportedKind::Text))
            );
        }
    }
}
