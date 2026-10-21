use crate::{V1Document, V1DocumentError};
use vexel_document_edit::HistoryAction;

impl V1Document {
    pub fn set_attribute(
        &mut self,
        id: &str,
        name: &str,
        value: &str,
    ) -> Result<(), V1DocumentError> {
        if value.len() > 64 {
            return Err(V1DocumentError::Unsupported);
        }
        validate(name, value)?;
        let node = self
            .editor
            .try_find_id(id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)?;
        if matches!(name, "fill" | "stroke") && value.starts_with("url(#") {
            let target = value
                .strip_prefix("url(#")
                .and_then(|v| v.strip_suffix(')'))
                .ok_or(V1DocumentError::InvalidColor)?;
            if self.editor.try_find_id(target.as_bytes())?.is_none() {
                return Err(V1DocumentError::MissingStableId);
            }
        }
        self.editor
            .set_attribute(node, name.as_bytes(), value.as_bytes())?;
        self.editor
            .history_relabel_latest(HistoryAction::EditAttribute);
        Ok(())
    }

    pub fn remove_attribute(&mut self, id: &str, name: &str) -> Result<(), V1DocumentError> {
        if !allowed(name) {
            return Err(V1DocumentError::Unsupported);
        }
        let node = self
            .editor
            .try_find_id(id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)?;
        self.editor.remove_attribute(node, name.as_bytes())?;
        self.editor
            .history_relabel_latest(HistoryAction::EditAttribute);
        Ok(())
    }
}

fn allowed(name: &str) -> bool {
    matches!(
        name,
        "opacity"
            | "fill-opacity"
            | "stroke-opacity"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "fill-rule"
            | "visibility"
            | "display"
            | "fill"
            | "stroke"
            | "stroke-width"
    )
}

fn validate(name: &str, value: &str) -> Result<(), V1DocumentError> {
    if !allowed(name) {
        return Err(V1DocumentError::Unsupported);
    }
    let bad = || V1DocumentError::Unsupported;
    match name {
        "opacity" | "fill-opacity" | "stroke-opacity" => {
            let n = value.parse::<f64>().map_err(|_| bad())?;
            if !n.is_finite() || !(0.0..=1.0).contains(&n) {
                return Err(bad());
            }
        }
        "stroke-linecap" => {
            if !matches!(value, "butt" | "round" | "square") {
                return Err(bad());
            }
        }
        "stroke-linejoin" => {
            if !matches!(value, "miter" | "round" | "bevel") {
                return Err(bad());
            }
        }
        "stroke-dasharray" => {
            if value != "none" {
                let nums: Vec<_> = value
                    .split(|c: char| c == ',' || c.is_ascii_whitespace())
                    .filter(|s| !s.is_empty())
                    .collect();
                if nums.is_empty()
                    || nums.len() > 8
                    || nums
                        .iter()
                        .any(|s| s.parse::<f64>().map_or(true, |n| !n.is_finite() || n < 0.0))
                {
                    return Err(bad());
                }
            }
        }
        "stroke-dashoffset" => {
            if value.parse::<f64>().map_or(true, |n| !n.is_finite()) {
                return Err(bad());
            }
        }
        "fill-rule" => {
            if !matches!(value, "nonzero" | "evenodd") {
                return Err(bad());
            }
        }
        "visibility" => {
            if !matches!(value, "visible" | "hidden") {
                return Err(bad());
            }
        }
        "display" => {
            if !matches!(value, "inline" | "none") {
                return Err(bad());
            }
        }
        "stroke-width" => {
            let n = value.parse::<f64>().map_err(|_| bad())?;
            if !n.is_finite() || !(0.0..=256.0).contains(&n) {
                return Err(V1DocumentError::InvalidStrokeWidth);
            }
        }
        "fill" | "stroke" => {
            if value == "none" {
                return Ok(());
            }
            if value.len() == 7
                && value.starts_with('#')
                && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Ok(());
            }
            if value
                .strip_prefix("url(#")
                .and_then(|v| v.strip_suffix(')'))
                .is_some()
            {
                return Ok(());
            }
            return Err(V1DocumentError::InvalidColor);
        }
        _ => unreachable!(),
    }
    Ok(())
}
