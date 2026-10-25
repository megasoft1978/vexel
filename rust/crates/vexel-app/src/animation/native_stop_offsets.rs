//! Keeps animated gradient stop offsets monotonic.
//!
//! The renderer rejects a gradient whose stop offsets decrease, so animated
//! offsets are clamped to 0..=1 and raised to the previous stop's effective
//! offset; later stops that would fall behind are raised too.

use super::native_properties::{NativePropertyError, SourceIndex};

/// Element index, attribute name and text of one resolved write.
pub(super) type Resolved = (usize, &'static [u8], Vec<u8>);

fn is_stop(source: &[u8], index: &SourceIndex, at: usize) -> bool {
    index.elements[at].tag.slice(source) == Some(b"stop".as_slice())
}

fn static_offset(source: &[u8], index: &SourceIndex, at: usize) -> f64 {
    let element = &index.elements[at];
    let text = element
        .attributes
        .iter()
        .find(|attribute| attribute.name.slice(source) == Some(b"offset".as_slice()))
        .and_then(|attribute| attribute.value.slice(source))
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::trim);
    match text {
        Some(text) => match text.strip_suffix('%') {
            Some(percent) => percent.trim().parse::<f64>().map(|v| v / 100.0),
            None => text.parse::<f64>(),
        }
        .unwrap_or(0.0),
        None => 0.0,
    }
}

pub(super) fn keep_monotonic(
    source: &[u8],
    index: &SourceIndex,
    resolved: &mut Vec<Resolved>,
) -> Result<(), NativePropertyError> {
    let mut done: Vec<usize> = Vec::new();
    let animated: Vec<usize> = resolved
        .iter()
        .filter(|(_, name, _)| *name == b"offset")
        .map(|(at, _, _)| *at)
        .collect();
    for at in animated {
        if !is_stop(source, index, at) {
            continue;
        }
        let mut first = at;
        while first > 0 && is_stop(source, index, first - 1) {
            first -= 1;
        }
        if done.contains(&first) {
            continue;
        }
        done.push(first);
        let mut previous = 0.0f64;
        let mut cursor = first;
        while cursor < index.elements.len() && is_stop(source, index, cursor) {
            let slot = resolved
                .iter()
                .position(|(element, name, _)| *element == cursor && *name == b"offset");
            let raw = match slot {
                Some(slot) => std::str::from_utf8(&resolved[slot].2)
                    .ok()
                    .and_then(|text| text.parse::<f64>().ok())
                    .ok_or(NativePropertyError::InvalidValue)?,
                None => static_offset(source, index, cursor),
            };
            let effective = raw.max(previous);
            previous = effective;
            let text = effective.to_string().into_bytes();
            match slot {
                Some(slot) => resolved[slot].2 = text,
                None if effective != raw => resolved.push((cursor, b"offset", text)),
                None => {}
            }
            cursor += 1;
        }
    }
    Ok(())
}
