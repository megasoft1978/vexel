use crate::{ElementKind, V1Document, V1DocumentError, numeric_text};
use vexel_document_edit::HistoryAction;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformSpec {
    pub translate: (f64, f64),
    pub rotate_deg: f64,
    pub pivot: (f64, f64),
    pub scale: (f64, f64),
    pub skew_x_deg: f64,
}

impl Default for TransformSpec {
    fn default() -> Self {
        Self {
            translate: (0.0, 0.0),
            rotate_deg: 0.0,
            pivot: (0.0, 0.0),
            scale: (1.0, 1.0),
            skew_x_deg: 0.0,
        }
    }
}

impl TransformSpec {
    pub fn to_matrix(&self) -> [f64; 6] {
        let (r, k) = (
            self.rotate_deg.to_radians(),
            self.skew_x_deg.to_radians().tan(),
        );
        let (cos, sin) = (r.cos(), r.sin());
        let (a, b, c, d) = (
            cos * self.scale.0,
            sin * self.scale.0,
            (cos * k - sin) * self.scale.1,
            (sin * k + cos) * self.scale.1,
        );
        let (px, py) = self.pivot;
        [
            a,
            b,
            c,
            d,
            self.translate.0 + px - a * px - c * py,
            self.translate.1 + py - b * px - d * py,
        ]
    }
}

impl V1Document {
    pub fn set_transform(&mut self, id: &str, spec: &TransformSpec) -> Result<(), V1DocumentError> {
        let values = [
            spec.translate.0,
            spec.translate.1,
            spec.rotate_deg,
            spec.pivot.0,
            spec.pivot.1,
            spec.scale.0,
            spec.scale.1,
            spec.skew_x_deg,
        ];
        if !values.into_iter().all(f64::is_finite) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let node = self
            .editor
            .try_find_id(id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)?;
        if self.element_kind(id)? == ElementKind::Other {
            return Err(V1DocumentError::Unsupported);
        }
        let matrix = spec.to_matrix();
        if matrix.iter().any(|v| !v.is_finite()) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        if matrix == [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
            if raw_attr(&self.editor, node, b"transform").is_some() {
                self.editor.remove_attribute(node, b"transform")?;
            }
        } else {
            let mut value = String::from("matrix(");
            for (i, item) in matrix.iter().enumerate() {
                if i != 0 {
                    value.push(' ');
                }
                value.push_str(
                    std::str::from_utf8(&numeric_text(*item)?)
                        .map_err(|_| V1DocumentError::FormattingLimit)?,
                );
            }
            value.push(')');
            self.editor
                .set_attribute(node, b"transform", value.as_bytes())?;
        }
        self.editor
            .history_relabel_latest(HistoryAction::EditAttribute);
        Ok(())
    }

    pub fn get_transform_matrix(&self, id: &str) -> Result<[f64; 6], V1DocumentError> {
        let node = self
            .editor
            .try_find_id(id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)?;
        let Some(raw) = raw_attr(&self.editor, node, b"transform") else {
            return Ok([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        };
        let text = std::str::from_utf8(raw)
            .map_err(|_| V1DocumentError::UnsupportedTransform)?
            .trim();
        if text.starts_with("matrix(") {
            return parse_list_fn(text, "matrix", 6)
                .and_then(|v| {
                    if v.len() == 6 {
                        Some(v.try_into().ok()?)
                    } else {
                        None
                    }
                })
                .ok_or(V1DocumentError::UnsupportedTransform);
        }
        if text.starts_with("translate(") {
            let v =
                parse_list_fn(text, "translate", 2).ok_or(V1DocumentError::UnsupportedTransform)?;
            return Ok([1., 0., 0., 1., v[0], *v.get(1).unwrap_or(&0.)]);
        }
        if text.starts_with("scale(") {
            let v = parse_list_fn(text, "scale", 2).ok_or(V1DocumentError::UnsupportedTransform)?;
            let y = *v.get(1).unwrap_or(&v[0]);
            return Ok([v[0], 0., 0., y, 0., 0.]);
        }
        if text.starts_with("rotate(") {
            let v =
                parse_list_fn(text, "rotate", 3).ok_or(V1DocumentError::UnsupportedTransform)?;
            let (c, s) = (v[0].to_radians().cos(), v[0].to_radians().sin());
            let (x, y) = (*v.get(1).unwrap_or(&0.), *v.get(2).unwrap_or(&0.));
            return Ok([c, s, -s, c, x - c * x + s * y, y - s * x - c * y]);
        }
        Err(V1DocumentError::UnsupportedTransform)
    }
}

fn raw_attr<'a>(
    editor: &'a vexel_document_edit::Editor,
    node: vexel_document_edit::DocumentHandle,
    name: &[u8],
) -> Option<&'a [u8]> {
    (0..editor.attribute_count(node)).find_map(|i| {
        let a = editor.attribute_at(node, i)?;
        (editor.span_bytes(a.name)? == name)
            .then(|| editor.span_bytes(a.value))
            .flatten()
    })
}

fn parse_list_fn(text: &str, name: &str, max: usize) -> Option<Vec<f64>> {
    let inner = text
        .strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')?;
    if inner.contains(['(', ')']) {
        return None;
    }
    let vals: Option<Vec<f64>> = inner
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f64>().ok().filter(|n| n.is_finite()))
        .collect();
    vals.filter(|v| !v.is_empty() && v.len() <= max)
}
