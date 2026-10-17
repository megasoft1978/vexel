//! Typed compose failures and disclosures. Every rejected or skipped item is
//! reported with a source offset; nothing is dropped silently.

use std::fmt;
use vexel_render::{RenderError, UnsupportedKind};
use vexel_scene::{Diagnostic, ErrorKind};

/// One deferred item that was not drawn, with where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disclosure {
    /// Short name such as `dashed stroke` or `gradient paint`.
    pub feature: &'static str,
    /// Byte offset of the element in the source, when known.
    pub offset: Option<usize>,
    /// Element name at that offset (for example `rect`), when readable.
    pub element: Option<String>,
}

impl fmt::Display for Disclosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} not rendered", self.feature)?;
        if let Some(element) = &self.element {
            write!(f, " (<{element}>")?;
            if let Some(offset) = self.offset {
                write!(f, " at byte {offset}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    /// Requested frame size is zero or beyond the raster axis/area bounds.
    InvalidSize,
    /// Input is not valid UTF-8; `valid_up_to` is the first bad byte.
    InvalidUtf8 { valid_up_to: usize },
    /// XML, number, path or viewport content could not be decoded.
    Malformed { detail: &'static str, offset: usize },
    /// Content outside the ADR-0029 subset that cannot be partially drawn.
    Unsupported {
        feature: &'static str,
        offset: Option<usize>,
        element: Option<String>,
    },
    /// Valid document with nothing drawable (no viewport).
    Hidden,
    /// Source size or a structural decode budget was reached.
    LimitDecode,
    /// A raster work or size budget was reached while rendering.
    LimitRender,
    /// Superseded by a newer request; the frame must be discarded.
    Cancelled,
}

impl fmt::Display for ComposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => write!(f, "invalid frame size"),
            Self::InvalidUtf8 { valid_up_to } => {
                write!(f, "not valid UTF-8 (first bad byte {valid_up_to})")
            }
            Self::Malformed { detail, offset } => {
                write!(f, "malformed SVG: {detail} at byte {offset}")
            }
            Self::Unsupported {
                feature,
                offset,
                element,
            } => {
                write!(f, "unsupported in V1: {feature}")?;
                if let Some(element) = element {
                    write!(f, " (<{element}>")?;
                    if let Some(offset) = offset {
                        write!(f, " at byte {offset}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Self::Hidden => write!(f, "document has nothing drawable"),
            Self::LimitDecode => write!(f, "document exceeds a decode safety limit"),
            Self::LimitRender => write!(f, "document exceeds a render safety limit"),
            Self::Cancelled => write!(f, "superseded by a newer request"),
        }
    }
}

impl std::error::Error for ComposeError {}

impl From<RenderError> for ComposeError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::Cancelled => Self::Cancelled,
            RenderError::LimitExceeded => Self::LimitRender,
            RenderError::Unsupported(kind) => Self::Unsupported {
                feature: feature_name(kind),
                offset: None,
                element: None,
            },
            RenderError::InvalidGeometry | RenderError::TileNotEmpty => Self::Malformed {
                detail: "render geometry",
                offset: 0,
            },
        }
    }
}

pub(crate) fn feature_name(kind: UnsupportedKind) -> &'static str {
    match kind {
        UnsupportedKind::Dash => "dashed stroke",
        UnsupportedKind::GradientPaint => "gradient paint",
        UnsupportedKind::PatternPaint => "pattern paint",
        UnsupportedKind::ImagePaint => "image",
        UnsupportedKind::Clip => "clip path",
        UnsupportedKind::Mask => "mask",
        UnsupportedKind::Filter => "filter",
        UnsupportedKind::OpacityGroup => "opacity group",
        UnsupportedKind::Text => "text",
        UnsupportedKind::AnimationOverlay => "animation",
        UnsupportedKind::Smil => "SMIL animation",
        UnsupportedKind::CrispEdges => "crisp edges",
        UnsupportedKind::PaintOrder => "paint-order",
    }
}

/// Name of the element starting at `offset` (at its `<` or just after it).
pub(crate) fn element_at(source: &[u8], offset: usize) -> Option<String> {
    let rest = source.get(offset..)?;
    let rest = rest.strip_prefix(b"<").unwrap_or(rest);
    let end = rest
        .iter()
        .position(|b| !(b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_')))
        .unwrap_or(rest.len());
    (end > 0 && end <= 64).then(|| String::from_utf8_lossy(&rest[..end]).into_owned())
}

/// Name of the start tag enclosing `offset`: the element whose `<` is the last
/// one before it. Attribute values cannot contain `<`, so this is sound.
pub(crate) fn enclosing_element(source: &[u8], offset: usize) -> Option<String> {
    const MAX_TAG_LOOKBACK: usize = 4096;
    let head = source.get(..offset)?;
    let base = head.len().saturating_sub(MAX_TAG_LOOKBACK);
    let start = base + head[base..].iter().rposition(|&b| b == b'<')?;
    element_at(source, start)
}

/// Maps a decoder diagnostic. `Unsupported` keeps its element for disclosure.
pub(crate) fn from_diagnostic(diagnostic: Diagnostic, source: &[u8]) -> ComposeError {
    let offset = diagnostic.offset;
    match diagnostic.kind {
        ErrorKind::Unsupported => {
            // The offset may sit inside an attribute value (e.g. a `fill` in a
            // `style`), so prefer the enclosing start tag; only an offset that
            // is itself at a `<` names the element starting there.
            let element = if source.get(offset) == Some(&b'<') {
                element_at(source, offset)
            } else {
                enclosing_element(source, offset)
            };
            let feature = match element.as_deref() {
                Some("filter") => "filter",
                Some("mask") => "mask",
                Some("text" | "tspan" | "textPath") => "text",
                Some("image") => "image",
                Some("marker") => "marker",
                _ => "unsupported element or attribute",
            };
            ComposeError::Unsupported {
                feature,
                offset: Some(offset),
                element,
            }
        }
        ErrorKind::UnsupportedFeTurbulence
        | ErrorKind::UnsupportedFeDisplacementMap
        | ErrorKind::UnsupportedFeConvolveMatrix
        | ErrorKind::UnsupportedFeImage
        | ErrorKind::UnsupportedFeLighting
        | ErrorKind::UnsupportedFeMorphology => ComposeError::Unsupported {
            feature: "filter",
            offset: Some(offset),
            element: element_at(source, offset),
        },
        ErrorKind::Canceled => ComposeError::Cancelled,
        ErrorKind::Limit(_) | ErrorKind::OutOfMemory => ComposeError::LimitDecode,
        ErrorKind::Xml(_) => ComposeError::Malformed {
            detail: "XML",
            offset,
        },
        ErrorKind::Number => ComposeError::Malformed {
            detail: "number",
            offset,
        },
        ErrorKind::Path => ComposeError::Malformed {
            detail: "path data",
            offset,
        },
        ErrorKind::Viewport => ComposeError::Malformed {
            detail: "viewport",
            offset,
        },
        ErrorKind::InvalidArgument => ComposeError::Malformed {
            detail: "invalid value",
            offset,
        },
    }
}

#[cfg(test)]
mod enclosing_tests {
    use super::{element_at, enclosing_element};

    #[test]
    fn offset_inside_attribute_names_the_start_tag() {
        let source = b"<svg><g style=\"fill:red\"><path/></g></svg>";
        let fill = source.windows(4).position(|w| w == b"fill").unwrap();
        assert_eq!(enclosing_element(source, fill).as_deref(), Some("g"));
    }

    #[test]
    fn offset_at_a_tag_names_that_tag() {
        let source = b"<svg><filter id=\"f\"/></svg>";
        let filter = source.windows(7).position(|w| w == b"<filter").unwrap();
        assert_eq!(element_at(source, filter).as_deref(), Some("filter"));
    }

    #[test]
    fn lookback_is_bounded() {
        let mut source = b"<svg>".to_vec();
        source.extend(std::iter::repeat_n(b' ', 10_000));
        assert_eq!(enclosing_element(&source, source.len()), None);
    }
}
