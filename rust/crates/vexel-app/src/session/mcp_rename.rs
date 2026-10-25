//! `attribute_set` with `name: "id"`: renames an element, the references to
//! it and the native timeline tracks that target it as one undo step.

use super::AppDocument;
use super::mcp::{source, stable_ids};
use crate::preview::native_timeline::mcp_mouth::MOUTHS_ID;
use crate::preview::native_timeline::mcp_rig::{RIG_ID, check_rename, retarget};
use crate::preview::native_timeline::settings;
use crate::project_session::SessionEditError;
use vexel_timeline::EditLimits;

pub(super) fn rename(app: &mut AppDocument, old: &str, new: &str) -> Result<(), &'static str> {
    app.document()
        .element_kind(old)
        .map_err(|_| "unknown element")?;
    if old == new {
        return Ok(());
    }
    if old == RIG_ID || new == RIG_ID {
        return Err("attribute_set id: the id vexel-rig belongs to the rig and cannot be renamed");
    }
    if old == MOUTHS_ID || new == MOUTHS_ID {
        return Err(
            "attribute_set id: the id vexel-mouths belongs to the mouth sets and cannot be renamed",
        );
    }
    // Check up front so a clash cannot strand retargeted tracks.
    if stable_ids(&source(app)?).iter().any(|id| id == new) {
        return Err("attribute_set id is already used by another element; choose an unused id");
    }
    check_rename(app, old, new)?;
    // Inside a batch the group is already open and the batch collapses the steps.
    let group = app.begin_group().ok();
    let result = rename_steps(app, old, new);
    match (group, result) {
        (Some(group), Ok(())) => app
            .end_group(group)
            .map_err(|_| "project history unavailable"),
        (Some(group), Err(message)) => {
            app.rollback_group(group)
                .map_err(|_| "project history unavailable")?;
            Err(message)
        }
        (None, result) => result,
    }
}

fn rename_steps(app: &mut AppDocument, old: &str, new: &str) -> Result<(), &'static str> {
    app.retarget_timeline(
        settings(),
        EditLimits::default(),
        old.as_bytes(),
        new.as_bytes(),
    )
    .map_err(|_| "native timeline edit rejected")?;
    app.edit(|doc| doc.rename_element(old, new))
        .map_err(|error| match error {
            SessionEditError::Operation(_) => {
                "attribute_set id must be 1-128 characters of A-Z a-z 0-9 _ - . : and unused"
            }
            SessionEditError::History(_) => "project history unavailable",
        })?;
    retarget(app, old, new)
}
