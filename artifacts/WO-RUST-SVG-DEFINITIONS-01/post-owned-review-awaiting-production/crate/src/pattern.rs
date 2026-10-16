//! Owned pattern tile mapping retained for the later paint adapter.
use crate::common::{parse_length, parse_number};
use crate::planner_support::unit_flag;
use crate::{attr, span_bytes};
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::{Affine, affine_compose, affine_identity};
use vexel_scene::{
    Diagnostic, ElementId, ErrorKind, LimitKind, PatternDescriptor, Result, Viewport,
};
use vexel_xml::XmlToken;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternMapping {
    pub definition: ElementId,
    pub tile_origin: [f64; 2],
    pub tile_width: f64,
    pub tile_height: f64,
    /// Maps document-user coordinates into the repeated tile's base space.
    pub user_to_base: Affine,
    pub pattern_scale: f64,
}

/// Exact legacy pattern-cache key; all matrix and scalar comparisons are exact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternCacheKey {
    pub descriptor: PatternDescriptor,
}

pub fn same_pattern_cache_key(left: PatternCacheKey, right: PatternCacheKey) -> bool {
    left.descriptor.definition == right.descriptor.definition
        && left.descriptor.width == right.descriptor.width
        && left.descriptor.height == right.descriptor.height
        && left.descriptor.tile_width == right.descriptor.tile_width
        && left.descriptor.tile_height == right.descriptor.tile_height
        && left.descriptor.content_transform == right.descriptor.content_transform
}

pub fn parse_pattern_mapping(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    bounds: [f64; 4],
    viewport: Viewport,
    control: &mut dyn WorkControl,
) -> Result<PatternMapping> {
    let token = tokens.get(element).ok_or_else(|| crate::invalid(0))?;
    let object = unit_flag(source, tokens, element, b"patternUnits", true)?;
    let content_object = unit_flag(source, tokens, element, b"patternContentUnits", false)?;
    let bounds_width = bounds[2] - bounds[0];
    let bounds_height = bounds[3] - bounds[1];
    if (object || content_object) && (bounds_width <= 0. || bounds_height <= 0.) {
        return Err(unsupported(token.name.offset));
    }
    let refs = [viewport.view_width, viewport.view_height];
    let mut values = [0.; 4];
    for (i, name) in [b"x".as_slice(), b"y", b"width", b"height"]
        .iter()
        .enumerate()
    {
        if let Some(span) = attr(source, tokens, element, name) {
            values[i] = if object {
                bbox_number(source, span)?
            } else {
                parse_length(source, span, refs[i % 2])?
            };
        }
    }
    if values[2] <= 0. || values[3] <= 0. {
        return Err(number(token.name.offset));
    }
    let tile_origin = if object {
        [
            bounds[0] + values[0] * bounds_width,
            bounds[1] + values[1] * bounds_height,
        ]
    } else {
        [values[0], values[1]]
    };
    let tile_width = if object {
        values[2] * bounds_width
    } else {
        values[2]
    };
    let tile_height = if object {
        values[3] * bounds_height
    } else {
        values[3]
    };
    if !tile_origin.iter().all(|n| n.is_finite())
        || !tile_width.is_finite()
        || !tile_height.is_finite()
        || tile_width <= 0.
        || tile_height <= 0.
        || tile_width > 65536.
        || tile_height > 65536.
    {
        return Err(crate::limit(LimitKind::Patterns, token.name.offset));
    }
    let pattern_transform = if let Some(span) = attr(source, tokens, element, b"patternTransform") {
        let end = span
            .offset
            .checked_add(span.length)
            .ok_or_else(|| crate::invalid(span.offset))?;
        let (matrix, operations) = vexel_svg_values::parse_transform(
            source,
            vexel_base::Span {
                start: span.offset,
                end,
            },
        )
        .map_err(|error| unsupported(error.offset))?;
        control
            .charge(
                WorkKind::Transform,
                u64::try_from(operations)
                    .map_err(|_| crate::limit(LimitKind::Arithmetic, span.offset))?,
            )
            .map_err(crate::planner_support::stopped)?;
        matrix
    } else {
        affine_identity()
    };
    let pattern_inverse = crate::clip::inverse_affine(pattern_transform).ok_or_else(|| {
        unsupported(
            attr(source, tokens, element, b"patternTransform")
                .map_or(token.name.offset, |s| s.offset),
        )
    })?;
    let translate = Affine {
        e: -tile_origin[0],
        f: -tile_origin[1],
        ..affine_identity()
    };
    let user_to_base =
        affine_compose(translate, pattern_inverse).ok_or_else(|| number(token.name.offset))?;
    Ok(PatternMapping {
        definition: ElementId(element),
        tile_origin,
        tile_width,
        tile_height,
        user_to_base,
        pattern_scale: pattern_transform
            .a
            .hypot(pattern_transform.b)
            .max(pattern_transform.c.hypot(pattern_transform.d)),
    })
}

fn bbox_number(source: &[u8], span: vexel_xml::XmlSpan) -> Result<f64> {
    let bytes = span_bytes(source, span).ok_or_else(|| crate::invalid(span.offset))?;
    let percent = bytes.last() == Some(&b'%');
    let number_span = if percent {
        vexel_xml::XmlSpan {
            offset: span.offset,
            length: span
                .length
                .checked_sub(1)
                .ok_or_else(|| crate::invalid(span.offset))?,
        }
    } else {
        span
    };
    let value = parse_number(source, number_span, 0.)?;
    Ok(if percent { value / 100. } else { value })
}

fn unsupported(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Unsupported,
        offset,
    }
}
fn number(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Number,
        offset,
    }
}

pub(crate) fn pattern_descriptor_from_mapping(
    source: &[u8],
    tokens: &[XmlToken],
    mapping: PatternMapping,
    bounds: [f64; 4],
    transform: Affine,
    viewport: Viewport,
) -> Result<PatternDescriptor> {
    let element = mapping.definition.0;
    let token = tokens.get(element).ok_or_else(|| crate::invalid(0))?;
    let mut density_x = viewport.width as f64 / viewport.view_width;
    let mut density_y = viewport.height as f64 / viewport.view_height;
    if !viewport.aspect.none {
        let density = if viewport.aspect.slice {
            density_x.max(density_y)
        } else {
            density_x.min(density_y)
        };
        density_x = density;
        density_y = density;
    }
    let transform_scale = transform
        .a
        .hypot(transform.b)
        .max(transform.c.hypot(transform.d));
    let width = (mapping.tile_width * density_x * transform_scale * mapping.pattern_scale * 4.)
        .ceil()
        .max(1.);
    let height = (mapping.tile_height * density_y * transform_scale * mapping.pattern_scale * 4.)
        .ceil()
        .max(1.);
    if !width.is_finite()
        || !height.is_finite()
        || width > 4096.
        || height > 4096.
        || width * height > 1_048_576.
    {
        return Err(crate::limit(LimitKind::Patterns, token.name.offset));
    }
    let content_object = unit_flag(source, tokens, element, b"patternContentUnits", false)?;
    let mut content = if let Some(span) = attr(source, tokens, element, b"viewBox") {
        let values = crate::common::number_list(source, span, 4).map_err(|_| Diagnostic {
            kind: ErrorKind::Viewport,
            offset: span.offset,
        })?;
        if values.len() != 4 || values[2] <= 0. || values[3] <= 0. {
            return Err(Diagnostic {
                kind: ErrorKind::Viewport,
                offset: span.offset,
            });
        }
        let aspect = if let Some(a) = attr(source, tokens, element, b"preserveAspectRatio") {
            let end = a
                .offset
                .checked_add(a.length)
                .ok_or_else(|| crate::invalid(a.offset))?;
            vexel_svg_values::parse_preserve_aspect(
                source,
                vexel_base::Span {
                    start: a.offset,
                    end,
                },
            )
            .map_err(|e| unsupported(e.offset))?
        } else {
            vexel_svg_values::Aspect {
                align_x: 0.5,
                align_y: 0.5,
                none: false,
                slice: false,
                set: true,
            }
        };
        vexel_svg_values::viewbox_transform(
            values[0],
            values[1],
            values[2],
            values[3],
            0.,
            0.,
            mapping.tile_width,
            mapping.tile_height,
            aspect,
        )
        .map_err(|_| Diagnostic {
            kind: ErrorKind::Viewport,
            offset: span.offset,
        })?
    } else if content_object {
        Affine {
            a: bounds[2] - bounds[0],
            d: bounds[3] - bounds[1],
            e: bounds[0],
            f: bounds[1],
            ..affine_identity()
        }
    } else {
        affine_identity()
    };
    if attr(source, tokens, element, b"viewBox").is_none() {
        content = affine_compose(
            Affine {
                e: -mapping.tile_origin[0],
                f: -mapping.tile_origin[1],
                ..affine_identity()
            },
            content,
        )
        .ok_or_else(|| crate::limit(LimitKind::Arithmetic, token.name.offset))?;
    }
    Ok(PatternDescriptor {
        definition: mapping.definition,
        width: width as u32,
        height: height as u32,
        tile_width: mapping.tile_width,
        tile_height: mapping.tile_height,
        content_transform: content,
        error_offset: token.name.offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_cache_key_uses_exact_render_identity_fields() {
        let descriptor = PatternDescriptor {
            definition: ElementId(3),
            width: 32,
            height: 16,
            tile_width: 2.5,
            tile_height: 1.25,
            content_transform: affine_identity(),
            error_offset: 10,
        };
        let same = PatternDescriptor {
            error_offset: 99,
            ..descriptor
        };
        assert!(same_pattern_cache_key(
            PatternCacheKey { descriptor },
            PatternCacheKey { descriptor: same },
        ));
        let changed = PatternDescriptor {
            tile_width: 2.500_000_000_1,
            ..descriptor
        };
        assert!(!same_pattern_cache_key(
            PatternCacheKey { descriptor },
            PatternCacheKey {
                descriptor: changed
            },
        ));
    }
}
