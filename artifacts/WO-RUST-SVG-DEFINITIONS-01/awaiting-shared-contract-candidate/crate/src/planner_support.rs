//! Bounded style and descriptor helpers used by definition planning.
use crate::common::{parse_length, parse_number};
use crate::{attr, span_bytes};
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::Affine;
use vexel_scene::*;
use vexel_svg_css::{CssPaint, Property};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub(super) fn definition_style<'src>(
    ctx: &mut DefinitionContext<'src, '_>,
    element: usize,
    control: &mut dyn WorkControl,
) -> Result<CssPaint<'src>> {
    let target = *ctx.shared.tokens().get(element).ok_or_else(|| bad(0))?;
    let mut ancestors = Vec::new();
    ancestors
        .try_reserve(64)
        .map_err(|_| oom(target.name.offset))?;
    for (i, token) in ctx.shared.tokens().iter().enumerate().take(element) {
        control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
        match token.kind {
            TokenKind::StartElement if token.depth < target.depth => {
                while ancestors
                    .last()
                    .is_some_and(|(depth, _): &(usize, usize)| *depth >= token.depth)
                {
                    ancestors.pop();
                }
                if ancestors.len() >= 256 {
                    return Err(crate::limit(LimitKind::Depth, token.name.offset));
                }
                ancestors
                    .try_reserve(1)
                    .map_err(|_| oom(token.name.offset))?;
                ancestors.push((token.depth, i));
            }
            TokenKind::EmptyElement | TokenKind::EndElement => {
                while ancestors
                    .last()
                    .is_some_and(|(depth, _): &(usize, usize)| *depth >= token.depth)
                {
                    ancestors.pop();
                }
            }
            _ => {}
        }
    }
    let mut parent = None;
    for (_, i) in ancestors {
        parent = Some(
            ctx.shared
                .compute_style(ElementId(i), parent.as_ref(), control)?,
        );
    }
    ctx.shared
        .compute_style(ElementId(element), parent.as_ref(), control)
}
pub(super) fn add_expansions<'src>(
    ctx: &mut DefinitionContext<'src, '_>,
    root: usize,
    end: usize,
    flavor: DecodeFlavor,
    base_transform: Affine,
    parent: &CssPaint<'src>,
    out: &mut Vec<Expansion<'src>>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let root_depth = ctx.shared.tokens().get(root).ok_or_else(|| bad(0))?.depth;
    let mut i = root + 1;
    while i < end {
        let t = *ctx.shared.tokens().get(i).ok_or_else(|| bad(0))?;
        control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            || t.depth != root_depth + 1
        {
            i += 1;
            continue;
        }
        let child_end = crate::matching_end_controlled(ctx.shared.tokens(), i, control)?;
        let style = ctx
            .shared
            .compute_style(ElementId(i), Some(parent), control)?;
        if matches!(flavor, DecodeFlavor::Clip { .. }) {
            check_clip_style(&style, t.name.offset)?;
        }
        out.try_reserve(1).map_err(|_| oom(t.name.offset))?;
        out.push(Expansion {
            range: TableRange {
                first: i,
                count: child_end - i + 1,
            },
            transform: base_transform,
            viewport: [
                ctx.builder.scene(ctx.scene)?.viewport.width as f64,
                ctx.builder.scene(ctx.scene)?.viewport.height as f64,
            ],
            style,
            flavor,
            post: PostOperation::None,
        });
        i = child_end
            .checked_add(1)
            .ok_or_else(|| number_at(t.name.offset))?;
    }
    Ok(())
}

pub(super) fn validate_clip_styles<'src>(
    ctx: &mut DefinitionContext<'src, '_>,
    root: usize,
    end: usize,
    root_style: &CssPaint<'src>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let root_depth = ctx.shared.tokens().get(root).ok_or_else(|| bad(0))?.depth;
    let mut stack = Vec::new();
    stack.try_reserve(64).map_err(|_| oom(0))?;
    stack.push((root_depth, root_style.clone()));
    for i in root + 1..end {
        let token = *ctx.shared.tokens().get(i).ok_or_else(|| bad(0))?;
        control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
        if token.kind == TokenKind::EndElement {
            while stack.last().is_some_and(|(depth, _)| *depth >= token.depth) {
                stack.pop();
            }
            continue;
        }
        if !matches!(
            token.kind,
            TokenKind::StartElement | TokenKind::EmptyElement
        ) {
            continue;
        }
        while stack.last().is_some_and(|(depth, _)| *depth >= token.depth) {
            stack.pop();
        }
        let parent = stack.last().map(|(_, style)| style);
        let style = ctx.shared.compute_style(ElementId(i), parent, control)?;
        check_clip_style(&style, token.name.offset)?;
        if token.kind == TokenKind::StartElement {
            if stack.len() >= crate::clip::MAX_CLIP_DEPTH {
                return Err(crate::limit(LimitKind::Depth, token.name.offset));
            }
            stack.try_reserve(1).map_err(|_| oom(token.name.offset))?;
            stack.push((token.depth, style));
        }
    }
    Ok(())
}
pub(super) fn check_clip_style(style: &CssPaint<'_>, _offset: usize) -> Result<()> {
    for p in Property::ALL {
        if p != Property::ClipRule && style.value(p).is_some() {
            return Err(unsupported(style.value(p).unwrap().span.offset));
        }
    }
    if let Some(v) = style.value(Property::ClipRule) {
        if !v.bytes.eq_ignore_ascii_case(b"nonzero") && !v.bytes.eq_ignore_ascii_case(b"evenodd") {
            return Err(unsupported(v.span.offset));
        }
    }
    Ok(())
}
pub(super) fn kind_matches(src: &[u8], span: XmlSpan, kind: DefinitionKind) -> bool {
    let v = span_bytes(src, span).unwrap_or_default();
    let expected: &[u8] = match kind {
        DefinitionKind::Clip => b"clipPath",
        DefinitionKind::Mask => b"mask",
        DefinitionKind::Pattern => b"pattern",
        DefinitionKind::Marker => b"marker",
        DefinitionKind::Filter => b"filter",
    };
    v == expected
}
pub(super) fn unit_flag(
    src: &[u8],
    tokens: &[XmlToken],
    element: usize,
    name: &[u8],
    default: bool,
) -> Result<bool> {
    let Some(s) = attr(src, tokens, element, name) else {
        return Ok(default);
    };
    match span_bytes(src, s).unwrap_or_default() {
        b"objectBoundingBox" => Ok(true),
        b"userSpaceOnUse" => Ok(false),
        _ => Err(unsupported(s.offset)),
    }
}
pub(super) fn mask_region(
    src: &[u8],
    span: XmlSpan,
    object_bbox: bool,
    reference: f64,
) -> Result<f64> {
    if object_bbox {
        let b = span_bytes(src, span).ok_or_else(|| bad(span.offset))?;
        let pct = b.last() == Some(&b'%');
        let s = if pct {
            XmlSpan {
                offset: span.offset,
                length: span.length - 1,
            }
        } else {
            span
        };
        let n = parse_number(src, s, 0.)?;
        Ok(if pct { n / 100. } else { n })
    } else {
        let b = span_bytes(src, span).ok_or_else(|| bad(span.offset))?;
        if b.last() == Some(&b'%') {
            return Ok(parse_number(
                src,
                XmlSpan {
                    offset: span.offset,
                    length: span.length - 1,
                },
                0.,
            )? * reference
                / 100.);
        }
        parse_length(src, span, reference)
    }
}
pub(super) fn pattern_descriptor(
    src: &[u8],
    tokens: &[XmlToken],
    element: usize,
    bounds: [f64; 4],
    transform: Affine,
    viewport: Viewport,
    control: &mut dyn WorkControl,
) -> Result<PatternDescriptor> {
    let mapping = crate::parse_pattern_mapping(src, tokens, element, bounds, viewport, control)?;
    crate::pattern::pattern_descriptor_from_mapping(
        src, tokens, mapping, bounds, transform, viewport,
    )
}
pub(super) fn unsupported_attr(
    src: &[u8],
    tokens: &[XmlToken],
    element: usize,
    allowed: &[&[u8]],
) -> Option<usize> {
    tokens
        .get(element + 1..)
        .unwrap_or(&[])
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find_map(|a| {
            (a.prefix.length != 0
                || !allowed.contains(&span_bytes(src, a.local_name).unwrap_or_default()))
            .then_some(a.name.offset)
        })
}
pub(super) fn bad(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: o,
    }
}
pub(super) fn unsupported(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: o,
    }
}
pub(super) fn number_at(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Number,
        offset: o,
    }
}
pub(super) fn clip_wrapper_prefix_len(
    _viewport: Viewport,
    object_bounding_box: bool,
) -> Option<usize> {
    const OBJECT_PREFIX: &[u8] = b"<svg width=\"1\" height=\"1\" viewBox=\"0 0 1 1\">";
    if object_bounding_box {
        Some(OBJECT_PREFIX.len())
    } else {
        Some(crate::clip::CLIP_WRAPPER_WORST_BYTES)
    }
}
pub(super) fn oom(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: o,
    }
}
pub(super) fn stopped(s: vexel_base::Stop) -> Diagnostic {
    Diagnostic {
        kind: match s {
            vexel_base::Stop::Canceled => ErrorKind::Canceled,
            vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
