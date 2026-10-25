//! Generic fragment insertion and `<use>` instances.

use crate::draw::{check_paint, parse_number};
use crate::{ElementKind, V1Document, V1DocumentError, validate_stable_id};
use vexel_document_edit::{DocumentHandle, encode_attribute};

/// Longest fragment the generic insert accepts.
pub const MAX_FRAGMENT_BYTES: usize = 1024 * 1024;

/// Ids declared inside a standalone XML fragment, in source order.
pub fn fragment_ids(xml: &[u8]) -> Result<Vec<String>, V1DocumentError> {
    crate::structure_defs::fragment_ids(xml)
}

/// Optional members of a new `<use>`; `fill`, `stroke` and `opacity` are CSS
/// presentation attributes the referenced content inherits.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UseSpec {
    pub x: f64,
    pub y: f64,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub opacity: Option<f64>,
    /// Affine matrix `[a b c d e f]` written as `transform="matrix(...)"`.
    pub transform: Option<[f64; 6]>,
}

impl V1Document {
    pub(crate) fn paint_target(&self, id: &str) -> Result<DocumentHandle, V1DocumentError> {
        let node = self.find_node(id)?;
        if matches!(self.kind_of(node), ElementKind::Other | ElementKind::Text) {
            return Err(V1DocumentError::Unsupported);
        }
        Ok(node)
    }

    fn fragment_parent(&self, parent: &str) -> Result<DocumentHandle, V1DocumentError> {
        if parent == "root" {
            return self.editor.root().ok_or(V1DocumentError::Edit(
                vexel_document_edit::EditError::InvalidOperation,
            ));
        }
        self.find_node(parent)
    }

    /// Appends one XML element fragment (any element, including `<defs>`,
    /// `<symbol>`, `<mask>`, `<filter>`) as the last child of `parent` (`"root"`
    /// or any element id). Its ids must be new. One history entry. Returns the
    /// ids the fragment declares.
    pub fn add_fragment(
        &mut self,
        parent: &str,
        xml: &[u8],
    ) -> Result<Vec<String>, V1DocumentError> {
        if xml.is_empty() || xml.len() > MAX_FRAGMENT_BYTES {
            return Err(V1DocumentError::InvalidAnimationFragment);
        }
        let parent_node = self.fragment_parent(parent)?;
        crate::fragment_check::validate_fragment(xml)?;
        let ids = crate::structure_defs::fragment_ids(xml)?;
        for (at, id) in ids.iter().enumerate() {
            validate_stable_id(id)?;
            if ids[..at].contains(id) || self.editor.try_find_id(id.as_bytes())?.is_some() {
                return Err(V1DocumentError::InvalidStableId);
            }
        }
        self.insert_checked(parent, parent_node, xml)?;
        Ok(ids)
    }

    /// Inserts `xml` under `parent`; when it holds a `<use>`, a scratch copy
    /// of the document takes the insert first and a reference cycle refuses
    /// it before the real document changes.
    fn insert_checked(
        &mut self,
        parent: &str,
        parent_node: DocumentHandle,
        xml: &[u8],
    ) -> Result<(), V1DocumentError> {
        if xml.windows(4).any(|w| w == b"<use") {
            let mut scratch = V1Document::from_source(&self.source()?)?;
            let node = scratch.fragment_parent(parent)?;
            scratch.editor.insert_xml_fragment(node, xml)?;
            scratch.check_use_cycles()?;
        }
        self.editor.insert_xml_fragment(parent_node, xml)?;
        Ok(())
    }

    /// Inserts `<use id href="#target">` as the last child of `parent`. The
    /// target must exist and must not contain `parent` (no cycles). One
    /// history entry.
    pub fn add_use(
        &mut self,
        parent: &str,
        id: Option<&str>,
        target: &str,
        spec: &UseSpec,
    ) -> Result<String, V1DocumentError> {
        validate_stable_id(target)?;
        let id = match id {
            Some(id) => id.to_owned(),
            None => self.next_stable_id()?,
        };
        let id = id.as_str();
        validate_stable_id(id)?;
        let target_node = self.find_node(target)?;
        let parent_node = self.fragment_parent(parent)?;
        let model = self.editor.try_model()?;
        let mut up = Some(parent_node);
        while let Some(node) = up {
            if node == target_node {
                return Err(V1DocumentError::Unsupported);
            }
            up = model.parent(node);
        }
        if !spec.x.is_finite() || !spec.y.is_finite() {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let mut tag = format!("<use id=\"{}\" href=\"#{}\"", attr(id)?, attr(target)?);
        if let Some(m) = spec.transform {
            if m.iter().any(|v| !v.is_finite()) {
                return Err(V1DocumentError::InvalidGeometry);
            }
            let m = m.map(|v| v.to_string()).join(" ");
            tag.push_str(&format!(" transform=\"matrix({m})\""));
        }
        for (name, value) in [("x", spec.x), ("y", spec.y)] {
            if value != 0.0 {
                tag.push_str(&format!(" {name}=\"{value}\""));
            }
        }
        for (name, value) in [("fill", &spec.fill), ("stroke", &spec.stroke)] {
            if let Some(value) = value {
                check_paint(value)?;
                tag.push_str(&format!(" {name}=\"{}\"", attr(value)?));
            }
        }
        if let Some(opacity) = spec.opacity {
            if !(0.0..=1.0).contains(&opacity) {
                return Err(V1DocumentError::InvalidGeometry);
            }
            tag.push_str(&format!(" opacity=\"{opacity}\""));
        }
        tag.push_str("/>");
        if self.editor.try_find_id(id.as_bytes())?.is_some() {
            return Err(V1DocumentError::InvalidStableId);
        }
        self.insert_checked(parent, parent_node, tag.as_bytes())?;
        Ok(id.to_owned())
    }

    /// Whether a document point hits what a `<use>` shows: the translated
    /// referenced shape (or, for a `<g>`/`<symbol>`, any child shape).
    pub(crate) fn hit_use(&self, node: DocumentHandle, x: f64, y: f64, depth: usize) -> bool {
        let Some(href) = self
            .raw_attr(node, b"href")
            .or_else(|| self.raw_attr(node, b"xlink:href"))
            .and_then(|raw| raw.strip_prefix(b"#"))
            .and_then(|raw| std::str::from_utf8(raw).ok())
        else {
            return false;
        };
        let num = |name: &[u8]| match self.raw_attr(node, name) {
            None => Some(0.0),
            Some(raw) => parse_number(raw),
        };
        let (Some(ux), Some(uy)) = (num(b"x"), num(b"y")) else {
            return false;
        };
        let (px, py) = (x - ux, y - uy);
        let Ok(target) = self.find_node(href) else {
            return false;
        };
        if self.editor.tag_name(target) == Some(b"use") {
            return false;
        }
        if self.editor.tag_name(target) == Some(b"symbol") {
            let Ok(model) = self.editor.try_model() else {
                return false;
            };
            let mut child = vexel_document_edit::document_view_first_child(model, target);
            while let Some(item) = child {
                if self.hit_depth(item, px, py, depth + 1).is_some() {
                    return true;
                }
                child = vexel_document_edit::document_view_next_sibling(model, item);
            }
            return false;
        }
        // The target is referenced by id, so it passes hit_id's id filter; its
        // own transform is applied by hit_id, matching how `<use>` renders it.
        self.hit_depth(target, px, py, depth + 1).is_some()
    }
}

fn attr(value: &str) -> Result<String, V1DocumentError> {
    let bytes = encode_attribute(value.as_bytes(), b'"')?;
    String::from_utf8(bytes).map_err(|_| V1DocumentError::InvalidStableId)
}

#[cfg(test)]
#[path = "fragment_tests.rs"]
mod tests;
