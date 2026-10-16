//! Local-reference expansion retains the use host and excludes definition ancestors.
use crate::{
    decode::Decoder,
    source::{attribute, basic_shape, element_name},
    tasks::{Context, Finish, Task},
};
use vexel_scene::*;
impl<'src> Decoder<'src, '_> {
    pub(crate) fn expand_use(
        &mut self,
        element: usize,
        parent: Context<'src>,
        _operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let t = self.shared.tokens()[element];
        let source = self.shared.source();
        let tokens = self.shared.tokens();
        if !use_attributes_supported(source, tokens, element, self.root_index, control)? {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            });
        }
        let style = self
            .shared
            .compute_style(ElementId(element), Some(&parent.style), control)?;
        if crate::style_display_none(&style, t.name.offset)? {
            return Ok(());
        }
        let plain = attribute(source, tokens, element, b"href", None);
        let xlink = attribute(source, tokens, element, b"href", Some(b"xlink"));
        if plain.is_some() == xlink.is_some()
            || xlink.is_some() && !xlink_bound(source, tokens, self.root_index)
        {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            });
        }
        let href = xlink.or(plain).unwrap();
        let raw = href
            .slice(source)
            .ok_or_else(|| crate::invalid(href.offset))?;
        if raw.len() < 2 || raw[0] != b'#' {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            });
        }
        let target = self
            .shared
            .find_reference(
                vexel_xml::XmlSpan {
                    offset: href.offset + 1,
                    length: href.length - 1,
                },
                control,
            )?
            .ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: href.offset,
            })?;
        if self.active.uses.contains(&target) {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: href.offset,
            });
        }
        if self.active.uses.len() >= 16 || self.use_instances >= 65_536 {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::References),
                offset: t.name.offset,
            });
        }
        self.use_instances += 1;
        let target_token = tokens[target.0];
        let symbol = element_name(source, &target_token, b"symbol");
        if !symbol
            && !element_name(source, &target_token, b"g")
            && !basic_shape(source, &target_token)
        {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: target_token.name.offset,
            });
        }
        let end = *self
            .ends
            .get(target.0)
            .filter(|i| **i != usize::MAX)
            .ok_or(Diagnostic {
                kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
                offset: target_token.name.offset,
            })?;
        let span = crate::source::element_span(tokens, target.0, end, source)?;
        let bytes = span.end - span.start;
        if bytes > 64 * 1024 || bytes > 1_048_576usize.saturating_sub(self.reference_bytes) {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::References),
                offset: t.name.offset,
            });
        }
        self.reference_bytes += bytes;
        let local = self.authored_transform(element, control)?;
        let xs = attribute(source, tokens, element, b"x", None);
        let ys = attribute(source, tokens, element, b"y", None);
        let x = xs
            .map(|v| crate::read_length(source, v, parent.viewport[0]))
            .transpose()
            .map_err(|_| Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: xs.or(ys).map_or(t.name.offset, |v| v.offset),
            })?
            .unwrap_or(0.0);
        let y = ys
            .map(|v| crate::read_length(source, v, parent.viewport[1]))
            .transpose()
            .map_err(|_| Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: xs.or(ys).map_or(t.name.offset, |v| v.offset),
            })?
            .unwrap_or(0.0);
        let suffix = if symbol {
            crate::viewport::symbol_attributes_supported(source, tokens, target.0, control)?;
            crate::viewport::symbol_mapping_controlled(
                source,
                tokens,
                target.0,
                element,
                parent.viewport[0],
                parent.viewport[1],
                control,
            )
            .map_err(|_| Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: attribute(source, tokens, element, b"preserveAspectRatio", None)
                    .map_or(t.name.offset, |v| v.offset),
            })?
        } else {
            Affine {
                e: x,
                f: y,
                ..vexel_geometry::affine_identity()
            }
        };
        let instance = vexel_geometry::affine_compose(
            parent.transform,
            vexel_geometry::affine_compose(local, suffix).ok_or(Diagnostic {
                kind: ErrorKind::Number,
                offset: t.name.offset,
            })?,
        )
        .ok_or(Diagnostic {
            kind: ErrorKind::Number,
            offset: t.name.offset,
        })?;
        let node = self.builder_mut()?.append_transform(
            parent.scene,
            TransformFrame {
                source_offset: t.source.offset,
                parent: parent.node,
                attribute: local,
                suffix,
                motion: vexel_geometry::affine_identity(),
            },
            control,
        )?;
        if self.visits >= 4_194_304 {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Expansions),
                offset: target_token.name.offset,
            });
        }
        self.visits += 1;
        let first_shape = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .scene(parent.scene)?
            .shapes
            .len();
        let group = self.begin_group(style, parent.scene, parent.group, first_shape, control)?;
        let context = Context {
            style,
            transform: instance,
            node: Some(node),
            depth: parent.depth + 1,
            host: parent.host.or(Some(ElementId(element))),
            group: group.or(parent.group),
            ..parent
        };
        self.active.uses.try_reserve(1).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: t.name.offset,
        })?;
        self.active.uses.push(target);
        self.push(Task::Finish(Finish {
            context,
            element,
            first_shape,
            group,
            style,
            viewport_clip: None,
            reference: Some(target),
            text_root: false,
            clip_done: false,
            mask_done: false,
        }))?;
        self.push(Task::Render {
            element: target.0,
            context,
            symbol,
        })?;
        Ok(())
    }
}

fn xlink_bound(source: &[u8], tokens: &[vexel_xml::XmlToken], root: usize) -> bool {
    tokens
        .get(root)
        .is_some_and(|t| t.kind == vexel_xml::TokenKind::StartElement)
        && attribute(source, tokens, root, b"xlink", Some(b"xmlns")).and_then(|v| v.slice(source))
            == Some(b"http://www.w3.org/1999/xlink")
}
pub fn use_attributes_supported(
    source: &[u8],
    tokens: &[vexel_xml::XmlToken],
    element: usize,
    root: usize,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    const ALLOWED: &[&[u8]] = &[
        b"href",
        b"id",
        b"class",
        b"style",
        b"x",
        b"y",
        b"width",
        b"height",
        b"preserveAspectRatio",
        b"fill",
        b"fill-rule",
        b"fill-opacity",
        b"stroke-opacity",
        b"opacity",
        b"visibility",
        b"display",
        b"color",
        b"transform",
        b"clip-path",
        b"filter",
        b"mask",
        b"mix-blend-mode",
        b"isolation",
        b"vector-effect",
        b"shape-rendering",
        b"paint-order",
        b"marker",
        b"marker-start",
        b"marker-mid",
        b"marker-end",
        b"mask-type",
        b"color-interpolation-filters",
        b"stroke",
        b"stroke-width",
        b"stroke-linecap",
        b"stroke-linejoin",
        b"stroke-miterlimit",
        b"stroke-dasharray",
        b"stroke-dashoffset",
        b"font-family",
        b"font-size",
        b"font-weight",
        b"font-style",
        b"text-anchor",
        b"letter-spacing",
        b"word-spacing",
    ];
    for a in tokens
        .iter()
        .skip(element + 1)
        .take_while(|t| t.kind == vexel_xml::TokenKind::Attribute)
    {
        crate::source::charge(control, WorkKind::TokenVisit, 1, a.name.offset)?;
        if a.prefix.length != 0 {
            if a.prefix.slice(source) != Some(b"xlink")
                || a.local_name.slice(source) != Some(b"href")
                || !xlink_bound(source, tokens, root)
            {
                return Ok(false);
            }
        } else if !a
            .local_name
            .slice(source)
            .is_some_and(|name| ALLOWED.contains(&name))
        {
            return Ok(false);
        }
    }
    Ok(tokens.get(element).is_some_and(|t| {
        matches!(
            t.kind,
            vexel_xml::TokenKind::StartElement | vexel_xml::TokenKind::EmptyElement
        )
    }))
}
