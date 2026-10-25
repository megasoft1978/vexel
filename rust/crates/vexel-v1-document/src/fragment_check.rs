//! Fragment allow-list and `<use>` reference-cycle detection.

use crate::{V1Document, V1DocumentError};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use vexel_document_edit::{
    DocumentHandle, EditError, EditLimits, Editor, document_view_first_child,
    document_view_next_sibling,
};

/// Elements `element_add_fragment` accepts. Scripts, `foreignObject`, `style`
/// and SMIL (`animate*`, `set`) are not on the list.
const ALLOWED: &[&str] = &[
    "g",
    "defs",
    "symbol",
    "use",
    "mask",
    "clipPath",
    "filter",
    "linearGradient",
    "radialGradient",
    "stop",
    "pattern",
    "marker",
    "text",
    "tspan",
    "image",
    "title",
    "desc",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "path",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feConvolveMatrix",
    "feDiffuseLighting",
    "feDisplacementMap",
    "feDistantLight",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feImage",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight",
    "feTile",
    "feTurbulence",
];

/// Most nested elements a fragment may hold.
const MAX_FRAGMENT_DEPTH: usize = 64;

fn local_reference(value: &str) -> bool {
    value.starts_with('#')
}

/// Every `url(` in `value` must open with `#`.
fn urls_are_local(value: &str) -> bool {
    let mut rest = value;
    while let Some(at) = rest.to_ascii_lowercase().find("url(") {
        let after = rest[at + 4..].trim_start_matches([' ', '\t', '"', '\'']);
        if !after.starts_with('#') {
            return false;
        }
        rest = &rest[at + 4..];
    }
    true
}

/// Rejects disallowed elements, `on*` attributes, non-local hrefs (except
/// `data:image/` on `<image>`) and non-local `url(...)` in a standalone fragment.
pub(crate) fn validate_fragment(source: &[u8]) -> Result<(), V1DocumentError> {
    let parsed = Editor::create(&Arc::new(source.to_vec()), EditLimits::default());
    if parsed.error != EditError::Ok {
        return Err(V1DocumentError::Edit(parsed.error));
    }
    let editor = parsed
        .editor
        .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
    let model = editor.try_model()?;
    let root = editor
        .root()
        .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
    let mut stack = vec![(root, 0usize)];
    while let Some((node, depth)) = stack.pop() {
        if depth >= MAX_FRAGMENT_DEPTH {
            return Err(V1DocumentError::Edit(EditError::Limit));
        }
        let tag = editor
            .tag_name(node)
            .and_then(|t| std::str::from_utf8(t).ok());
        let Some(tag) = tag else {
            return Err(V1DocumentError::DisallowedElement);
        };
        if !ALLOWED.contains(&tag) {
            return Err(V1DocumentError::DisallowedElement);
        }
        for index in 0..editor.attribute_count(node) {
            let Some(attribute) = editor.attribute_at(node, index) else {
                continue;
            };
            let name = editor
                .span_bytes(attribute.name)
                .and_then(|n| std::str::from_utf8(n).ok())
                .ok_or(V1DocumentError::DisallowedAttribute)?;
            let value = editor
                .span_bytes(attribute.value)
                .and_then(|v| std::str::from_utf8(v).ok())
                .ok_or(V1DocumentError::DisallowedAttribute)?;
            if name.to_ascii_lowercase().starts_with("on") {
                return Err(V1DocumentError::DisallowedAttribute);
            }
            if matches!(name, "href" | "xlink:href") {
                let data_image = tag == "image" && value.starts_with("data:image/");
                if !local_reference(value) && !data_image {
                    return Err(V1DocumentError::ExternalReference);
                }
            }
            if !urls_are_local(value) {
                return Err(V1DocumentError::ExternalReference);
            }
        }
        let mut child = document_view_first_child(model, node);
        while let Some(item) = child {
            stack.push((item, depth + 1));
            child = document_view_next_sibling(model, item);
        }
    }
    Ok(())
}

impl V1Document {
    /// Fails with [`V1DocumentError::UseCycle`] when any `<use>` can reach
    /// itself: its target (or anything under the target) is a `<use>` chain
    /// leading back to it, or the target contains the `<use>`.
    pub(crate) fn check_use_cycles(&self) -> Result<(), V1DocumentError> {
        let model = self.editor.try_model()?;
        let root = self
            .editor
            .root()
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
        // (use node, referenced node) for every resolvable local href.
        let mut uses: Vec<(DocumentHandle, DocumentHandle)> = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if self.editor.tag_name(node) == Some(b"use") {
                let href = self
                    .raw_attr(node, b"href")
                    .or_else(|| self.raw_attr(node, b"xlink:href"))
                    .and_then(|raw| raw.strip_prefix(b"#"));
                if let Some(target) = href.and_then(|id| self.editor.try_find_id(id).ok().flatten())
                {
                    uses.push((node, target));
                }
            }
            let mut child = document_view_first_child(model, node);
            while let Some(item) = child {
                stack.push(item);
                child = document_view_next_sibling(model, item);
            }
        }
        if uses.is_empty() {
            return Ok(());
        }
        let index: HashMap<DocumentHandle, usize> =
            uses.iter().enumerate().map(|(i, (n, _))| (*n, i)).collect();
        // Edges: use -> every use inside its target's subtree (target included).
        let mut by_target: HashMap<DocumentHandle, Vec<usize>> = HashMap::new();
        let mut edges: Vec<Vec<usize>> = Vec::with_capacity(uses.len());
        for (_, target) in &uses {
            if !by_target.contains_key(target) {
                let mut found = Vec::new();
                let mut stack = vec![*target];
                while let Some(node) = stack.pop() {
                    if let Some(i) = index.get(&node) {
                        found.push(*i);
                    }
                    let mut child = document_view_first_child(model, node);
                    while let Some(item) = child {
                        stack.push(item);
                        child = document_view_next_sibling(model, item);
                    }
                }
                by_target.insert(*target, found);
            }
            edges.push(by_target[target].clone());
        }
        // Iterative three-colour DFS.
        let mut done: HashSet<usize> = HashSet::new();
        let mut on_path: HashSet<usize> = HashSet::new();
        for start in 0..uses.len() {
            if done.contains(&start) {
                continue;
            }
            let mut path: Vec<(usize, usize)> = vec![(start, 0)];
            on_path.insert(start);
            while let Some((node, next)) = path.last().copied() {
                if let Some(&to) = edges[node].get(next) {
                    path.last_mut().map(|top| top.1 += 1);
                    if on_path.contains(&to) {
                        return Err(V1DocumentError::UseCycle);
                    }
                    if !done.contains(&to) {
                        on_path.insert(to);
                        path.push((to, 0));
                    }
                } else {
                    on_path.remove(&node);
                    done.insert(node);
                    path.pop();
                }
            }
        }
        Ok(())
    }
}
