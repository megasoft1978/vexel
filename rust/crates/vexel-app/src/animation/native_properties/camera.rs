//! Camera: a render-time view transform from the reserved `@camera` tracks.
//!
//! The camera looks at the viewBox centre displaced by `(x, y)`, scaled by `zoom` and
//! turned by `rotate` degrees about that centre. It is applied by wrapping the root
//! element's children in one `<g transform>` in the per-tick snapshot, so preview, PNG
//! and export share it. Without camera samples the snapshot is untouched.
use super::{NativePropertyError, Patch, SourceIndex};
use vexel_timeline::PropertyValue;

/// Camera channel values at one tick (defaults are the identity view).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Camera {
    x: f64,
    y: f64,
    zoom: f64,
    rotate: f64,
}

impl Camera {
    pub(super) fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            rotate: 0.0,
        }
    }

    /// Stores one sampled channel; `false` for a property the camera does not have.
    pub(super) fn set(&mut self, property: &[u8], value: PropertyValue) -> bool {
        let PropertyValue::Scalar(v) = value else {
            return false;
        };
        match property {
            b"x" => self.x = v,
            b"y" => self.y = v,
            b"zoom" => self.zoom = v,
            b"rotate" => self.rotate = v,
            _ => return false,
        }
        true
    }

    /// SVG matrix `[a b c d e f]` for a view centred on `(cx, cy)`.
    pub(super) fn matrix(&self, cx: f64, cy: f64) -> [f64; 6] {
        let zoom = if self.zoom.is_finite() {
            self.zoom.clamp(0.01, 100.0)
        } else {
            1.0
        };
        let (sin, cos) = self.rotate.to_radians().sin_cos();
        let (a, b, c, d) = (zoom * cos, zoom * sin, -zoom * sin, zoom * cos);
        let (tx, ty) = (cx + self.x, cy + self.y);
        [a, b, c, d, cx - (a * tx + c * ty), cy - (b * tx + d * ty)]
    }
}

fn attribute<'a>(source: &'a [u8], element: &super::Element, name: &[u8]) -> Option<&'a [u8]> {
    element
        .attributes
        .iter()
        .find(|a| a.name.slice(source) == Some(name))
        .and_then(|a| a.value.slice(source))
}

fn numbers(text: &[u8]) -> Option<Vec<f64>> {
    let text = std::str::from_utf8(text).ok()?;
    let mut out = Vec::new();
    for part in text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
    {
        let part = part.strip_suffix("px").unwrap_or(part);
        let n: f64 = part.parse().ok()?;
        if !n.is_finite() || out.len() == 4 {
            return None;
        }
        out.push(n);
    }
    Some(out)
}

/// View centre of the root element: from `viewBox`, else `width`/`height`.
fn centre(source: &[u8], root: &super::Element) -> Option<(f64, f64)> {
    if let Some(v) = attribute(source, root, b"viewBox").and_then(numbers) {
        if let [x, y, w, h] = v[..] {
            return Some((x + w / 2.0, y + h / 2.0));
        }
    }
    let w = numbers(attribute(source, root, b"width")?)?;
    let h = numbers(attribute(source, root, b"height")?)?;
    Some((w.first()? / 2.0, h.first()? / 2.0))
}

fn matrix_text(m: [f64; 6]) -> String {
    let n: Vec<String> = m.iter().map(|v| v.to_string()).collect();
    format!("matrix({})", n.join(" "))
}

/// Patches that wrap the root's content in the camera group.
pub(super) fn wrap(
    source: &[u8],
    index: &SourceIndex,
    camera: &Camera,
) -> Result<Vec<Patch>, NativePropertyError> {
    let root = index
        .elements
        .first()
        .ok_or(NativePropertyError::InvalidSvg)?;
    if root.kind != vexel_xml::TokenKind::StartElement {
        return Err(NativePropertyError::InvalidSvg);
    }
    let (cx, cy) = centre(source, root).ok_or(NativePropertyError::InvalidSvg)?;
    let open_end = root.opening.offset + root.opening.length;
    let close = source
        .windows(5)
        .rposition(|w| w == b"</svg")
        .filter(|at| *at >= open_end)
        .ok_or(NativePropertyError::InvalidSvg)?;
    Ok(vec![
        Patch {
            range: open_end..open_end,
            replacement: format!("<g transform=\"{}\">", matrix_text(camera.matrix(cx, cy)))
                .into_bytes(),
        },
        Patch {
            range: close..close,
            replacement: b"</g>".to_vec(),
        },
    ])
}
