//! `element_add` extras (explicit id, opacity, transform, gradient paint)
//! applied inside the shape's own edit, so one undo removes everything.

use super::AppDocument;
use super::mcp::{McpHost, SOURCE_LIMIT, source, stable_ids};
use crate::preview::{SCENE_LIMIT, add_extras::AddExtras, object_id};
use vexel_v1_document::{V1Document, V1DocumentError};

impl McpHost {
    /// Checks the scene and source limits and returns the id the new element
    /// gets (the explicit `extras.id` when given, else the next free
    /// `vexel-object-N`), the current element count and the extras. The id
    /// counter advances only when the caller's edit succeeds.
    pub(super) fn reserve_add(
        &mut self,
        app: &AppDocument,
    ) -> Result<(String, usize, Option<AddExtras>), &'static str> {
        let _phase = crate::profile::phase("element_id_scan");
        let extras = self.extras.take();
        let source = source(app)?;
        let ids = stable_ids(&source);
        if ids.len() >= SCENE_LIMIT {
            return Err("preview scene limit reached");
        }
        if source.len() > SOURCE_LIMIT {
            return Err("preview source limit reached");
        }
        let gradient = extras.as_ref().and_then(|e| e.gradient.as_ref());
        if gradient.is_some_and(|(gradient, _)| !ids.contains(gradient)) {
            return Err(
                "element_add gradientId is not in the document; create one with gradient_add",
            );
        }
        if let Some(id) = extras.as_ref().and_then(|e| e.id.clone()) {
            if ids.contains(&id) {
                return Err("element_add id already exists in the document; choose an unused id");
            }
            return Ok((id, ids.len(), extras));
        }
        let mut id = object_id(self.next_id);
        while ids.contains(&id) {
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or("preview stable ID limit reached")?;
            id = object_id(self.next_id);
        }
        Ok((id, ids.len(), extras))
    }

    /// Advances the id counter after an automatically named element.
    pub(super) fn added(&mut self, extras: &Option<AddExtras>) {
        if extras.as_ref().is_none_or(|e| e.id.is_none()) {
            self.next_id += 1;
        }
    }
}

/// Applies the optional members to the element just added as `id`.
pub(super) fn apply(
    doc: &mut V1Document,
    id: &str,
    extras: &Option<AddExtras>,
) -> Result<(), V1DocumentError> {
    let Some(extras) = extras else {
        return Ok(());
    };
    if let Some(opacity) = extras.opacity {
        doc.set_attribute(id, "opacity", &format!("{opacity}"))?;
    }
    if let Some(spec) = &extras.transform {
        doc.set_transform(id, spec)?;
    }
    if let Some((gradient, stroke)) = &extras.gradient {
        if *stroke {
            doc.set_stroke_ref(id, gradient)?;
        } else {
            doc.set_fill_ref(id, gradient)?;
        }
    }
    Ok(())
}

/// Room for `adding` more id-bearing elements within the scene and source
/// limits, for edits that bypass `element_add`.
pub(crate) fn check_room(app: &AppDocument, adding: usize) -> Result<(), &'static str> {
    let source = source(app)?;
    if stable_ids(&source).len() + adding > SCENE_LIMIT {
        return Err("preview scene limit reached");
    }
    if source.len() > SOURCE_LIMIT {
        return Err("preview source limit reached");
    }
    Ok(())
}
