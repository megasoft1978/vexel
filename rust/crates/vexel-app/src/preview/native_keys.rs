//! Optional bulk `keys` of `native_timeline_add_property_track`: each member is
//! `{tick, value, interp?}`, parsed with the same value forms as
//! `native_timeline_set_property_key`.

use super::native_args::{ease_at, tick, timeline_value};
use super::native_timeline::PropertyKey;
use super::{JsonDocument, JsonType, PreviewArgumentError};

/// Most keys one `native_timeline_add_property_track` call may carry.
pub(crate) const MAX_BULK_KEYS: usize = 256;
/// Largest raw `keys` array, in bytes.
const KEYS_BYTES: usize = 256 * 1024;

const FORM: &str = "an array of at most 256 objects {tick, value, interp?}";

pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<Vec<PropertyKey>, PreviewArgumentError> {
    let Some(array) = doc.object_get(args, b"keys") else {
        return Ok(Vec::new());
    };
    let bad = |at: &str, form: &str| super::native_args::invalid(tool, at, form, None);
    let token = doc.token(array).filter(|t| t.kind == JsonType::Array);
    let raw_fits = doc
        .raw_token(array)
        .is_some_and(|raw| raw.len() <= KEYS_BYTES);
    let Some(token) = token.filter(|t| t.child_count <= MAX_BULK_KEYS && raw_fits) else {
        return Err(bad("keys", FORM));
    };
    let mut keys = Vec::new();
    keys.try_reserve(token.child_count)
        .map_err(|_| PreviewArgumentError::Generic("preview allocation failed"))?;
    let mut item = token.first_child;
    while let Some(at) = item {
        let index = keys.len();
        let wrap = |error: PreviewArgumentError| match error {
            PreviewArgumentError::Actionable(message) => {
                PreviewArgumentError::Actionable(format!("keys[{index}]: {message}"))
            }
            other => other,
        };
        let member = doc.token(at).filter(|t| t.kind == JsonType::Object);
        let Some(member) = member else {
            return Err(bad(
                &format!("keys[{index}]"),
                "an object {tick, value, interp?}",
            ));
        };
        let mut name = member.first_child;
        while let Some(key) = name {
            let allowed = doc
                .decoded_string(key)
                .is_some_and(|k| matches!(k.as_slice(), b"tick" | b"value" | b"interp"));
            if !allowed {
                return Err(bad(
                    &format!("keys[{index}]"),
                    "only the members tick, value and interp",
                ));
            }
            name = doc
                .token(key)
                .and_then(|t| t.next_sibling)
                .and_then(|v| doc.token(v))
                .and_then(|t| t.next_sibling);
        }
        keys.push(PropertyKey {
            tick: tick(doc, at, tool, "tick").map_err(wrap)?,
            value: timeline_value(doc, at, tool).map_err(wrap)?,
            ease: ease_at(doc, at, tool, "interp").map_err(wrap)?,
        });
        item = doc.token(at).and_then(|t| t.next_sibling);
    }
    Ok(keys)
}
