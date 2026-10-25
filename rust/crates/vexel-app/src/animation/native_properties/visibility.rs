//! A hidden ancestor wins over a child's `visibility` track: SVG lets a child
//! with `visibility="visible"` reveal itself under a hidden parent, so a
//! sampled `visible` under a hidden ancestor is written as `inherit`.

use super::Element;
use std::collections::HashMap;

fn is_hidden(value: &[u8]) -> bool {
    value == b"hidden" || value == b"collapse"
}

/// Static `visibility` attribute of `element`, if any.
fn static_value<'a>(source: &'a [u8], element: &Element) -> Option<&'a [u8]> {
    element
        .attributes
        .iter()
        .find(|a| a.name.slice(source) == Some(b"visibility".as_slice()))
        .and_then(|a| a.value.slice(source))
}

/// Whether `at` is hidden after the sampled writes: the nearest ancestor with
/// an explicit value decides, and a sampled `visible` defers to its own parent.
fn hidden(
    source: &[u8],
    elements: &[Element],
    written: &HashMap<usize, &[u8]>,
    mut at: Option<usize>,
) -> bool {
    while let Some(index) = at {
        let own = written
            .get(&index)
            .copied()
            .or_else(|| static_value(source, &elements[index]));
        match own {
            Some(value) if is_hidden(value) => return true,
            Some(b"visible") if !written.contains_key(&index) => return false,
            _ => at = elements[index].parent,
        }
    }
    false
}

/// Rewrites sampled `visibility: visible` under a hidden ancestor to `inherit`.
pub(super) fn keep_hidden_parents(
    source: &[u8],
    elements: &[Element],
    resolved: &mut [(usize, &'static [u8], Vec<u8>)],
) {
    let written: HashMap<usize, &[u8]> = resolved
        .iter()
        .filter(|(_, name, _)| *name == b"visibility")
        .map(|(at, _, value)| (*at, value.as_slice()))
        .collect();
    if written.is_empty() {
        return;
    }
    let revealing: Vec<usize> = written
        .iter()
        .filter(|(at, value)| {
            **value == b"visible" && hidden(source, elements, &written, elements[**at].parent)
        })
        .map(|(at, _)| *at)
        .collect();
    for (at, name, value) in resolved.iter_mut() {
        if *name == b"visibility" && revealing.contains(at) {
            *value = b"inherit".to_vec();
        }
    }
}
