//! Gradient, clip, transform and text tools routed onto the live document:
//! `gradient_add`, `gradient_set_stop`, `element_set_paint`, `clip_add`,
//! `element_set_clip`, `element_set_transform` and `text_add`.

#[path = "ext_paint_args.rs"]
mod args;

use super::PreviewArgumentError;
use crate::project_session::SessionEditError;
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonDocument;
use vexel_v1_document::{
    GradientStop, GradientUnits, Spread, TextAnchor, TextSpec, TransformSpec, V1DocumentError,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geometry {
    Linear {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Radial {
        cx: f64,
        cy: f64,
        r: f64,
        fx: f64,
        fy: f64,
    },
}

/// One validated request of this tool family.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    GradientAdd {
        geometry: Geometry,
        units: GradientUnits,
        spread: Spread,
        stops: Vec<GradientStop>,
    },
    GradientSetStop {
        gradient_id: String,
        index: usize,
        offset: Option<f64>,
        color: Option<String>,
        opacity: Option<f64>,
    },
    SetPaint {
        element_id: String,
        stroke: bool,
        gradient_id: String,
    },
    ClipAdd {
        ellipse: bool,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    },
    SetClip {
        element_id: String,
        clip_id: Option<String>,
    },
    SetTransform {
        element_id: String,
        spec: TransformSpec,
    },
    TextAdd(TextSpec),
}

const GRADIENT_ADD: &str = "gradient_add";
const GRADIENT_SET_STOP: &str = "gradient_set_stop";
const SET_PAINT: &str = "element_set_paint";
const CLIP_ADD: &str = "clip_add";
const SET_CLIP: &str = "element_set_clip";
const SET_TRANSFORM: &str = "element_set_transform";
const TEXT_ADD: &str = "text_add";

/// `None` when `tool` is not in this family.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    Some(match tool {
        GRADIENT_ADD => gradient_add(doc, args),
        GRADIENT_SET_STOP => gradient_set_stop(doc, args),
        SET_PAINT => set_paint(doc, args),
        CLIP_ADD => clip_add(doc, args),
        SET_CLIP => set_clip(doc, args),
        SET_TRANSFORM => set_transform(doc, args),
        TEXT_ADD => text_add(doc, args),
        _ => return None,
    })
}

fn gradient_add(doc: &JsonDocument<'_>, a: usize) -> Result<Op, PreviewArgumentError> {
    let t = GRADIENT_ADD;
    let kind = args::string(doc, a, t, "kind", 16, "`linear` or `radial`")?;
    let kind = args::choice(t, "kind", &kind, &["linear", "radial"])?;
    let units =
        match args::opt_string(doc, a, t, "units", 32, "`userSpace` or `objectBoundingBox`")? {
            None => GradientUnits::ObjectBoundingBox,
            Some(v) => match args::choice(t, "units", &v, &["userSpace", "objectBoundingBox"])? {
                "userSpace" => GradientUnits::UserSpace,
                _ => GradientUnits::ObjectBoundingBox,
            },
        };
    let spread = match args::opt_string(doc, a, t, "spread", 16, "`pad`, `reflect` or `repeat`")? {
        None => Spread::Pad,
        Some(v) => match args::choice(t, "spread", &v, &["pad", "reflect", "repeat"])? {
            "reflect" => Spread::Reflect,
            "repeat" => Spread::Repeat,
            _ => Spread::Pad,
        },
    };
    let n = |key, default: f64| -> Result<f64, PreviewArgumentError> {
        Ok(args::opt_number(doc, a, t, key)?.unwrap_or(default))
    };
    let geometry = if kind == "linear" {
        Geometry::Linear {
            x1: n("x1", 0.0)?,
            y1: n("y1", 0.0)?,
            x2: n("x2", 1.0)?,
            y2: n("y2", 0.0)?,
        }
    } else {
        let (cx, cy) = (n("cx", 0.5)?, n("cy", 0.5)?);
        Geometry::Radial {
            cx,
            cy,
            r: n("r", 0.5)?,
            fx: n("fx", cx)?,
            fy: n("fy", cy)?,
        }
    };
    let text = args::string(
        doc,
        a,
        t,
        "stops",
        512,
        "`offset:#rrggbb:opacity,...` with 2-16 stops, e.g. `0:#ff0000:1,1:#0000ff:1`",
    )?;
    Ok(Op::GradientAdd {
        geometry,
        units,
        spread,
        stops: args::stops(t, &text)?,
    })
}

fn gradient_set_stop(doc: &JsonDocument<'_>, a: usize) -> Result<Op, PreviewArgumentError> {
    let t = GRADIENT_SET_STOP;
    let color = args::opt_string(doc, a, t, "color", 7, "a colour like `#336699`")?;
    let offset = args::opt_number(doc, a, t, "offset")?;
    let opacity = args::opt_number(doc, a, t, "opacity")?;
    if color.is_none() && offset.is_none() && opacity.is_none() {
        return Err(PreviewArgumentError::Actionable(format!(
            "{t}: pass at least one of offset (0-1), color (`#rrggbb`) or opacity (0-1)"
        )));
    }
    Ok(Op::GradientSetStop {
        gradient_id: args::id(doc, a, t, "gradientId")?,
        index: args::index(doc, a, t, "index")?,
        offset,
        color,
        opacity,
    })
}

fn set_paint(doc: &JsonDocument<'_>, a: usize) -> Result<Op, PreviewArgumentError> {
    let t = SET_PAINT;
    let property = args::string(doc, a, t, "property", 16, "`fill` or `stroke`")?;
    let property = args::choice(t, "property", &property, &["fill", "stroke"])?;
    Ok(Op::SetPaint {
        element_id: args::id(doc, a, t, "elementId")?,
        stroke: property == "stroke",
        gradient_id: args::id(doc, a, t, "gradientId")?,
    })
}

fn clip_add(doc: &JsonDocument<'_>, a: usize) -> Result<Op, PreviewArgumentError> {
    let t = CLIP_ADD;
    let shape = args::string(doc, a, t, "shape", 16, "`rect` or `ellipse`")?;
    let shape = args::choice(t, "shape", &shape, &["rect", "ellipse"])?;
    Ok(Op::ClipAdd {
        ellipse: shape == "ellipse",
        a: args::number(doc, a, t, "a")?,
        b: args::number(doc, a, t, "b")?,
        c: args::number(doc, a, t, "c")?,
        d: args::number(doc, a, t, "d")?,
    })
}

fn set_clip(doc: &JsonDocument<'_>, a: usize) -> Result<Op, PreviewArgumentError> {
    let t = SET_CLIP;
    let clip_id = match doc.object_get(a, b"clipId") {
        None => None,
        Some(_) => Some(args::id(doc, a, t, "clipId")?),
    };
    Ok(Op::SetClip {
        element_id: args::id(doc, a, t, "elementId")?,
        clip_id,
    })
}

fn set_transform(doc: &JsonDocument<'_>, a: usize) -> Result<Op, PreviewArgumentError> {
    let t = SET_TRANSFORM;
    let n = |key, default: f64| -> Result<f64, PreviewArgumentError> {
        Ok(args::opt_number(doc, a, t, key)?.unwrap_or(default))
    };
    let spec = TransformSpec {
        translate: (n("translateX", 0.0)?, n("translateY", 0.0)?),
        rotate_deg: n("rotate", 0.0)?,
        pivot: (n("pivotX", 0.0)?, n("pivotY", 0.0)?),
        scale: (n("scaleX", 1.0)?, n("scaleY", 1.0)?),
        skew_x_deg: n("skewX", 0.0)?,
    };
    Ok(Op::SetTransform {
        element_id: args::id(doc, a, t, "elementId")?,
        spec,
    })
}

fn text_add(doc: &JsonDocument<'_>, a: usize) -> Result<Op, PreviewArgumentError> {
    let t = TEXT_ADD;
    let anchor = match args::opt_string(doc, a, t, "anchor", 16, "`start`, `middle` or `end`")? {
        None => TextAnchor::Start,
        Some(v) => match args::choice(t, "anchor", &v, &["start", "middle", "end"])? {
            "middle" => TextAnchor::Middle,
            "end" => TextAnchor::End,
            _ => TextAnchor::Start,
        },
    };
    Ok(Op::TextAdd(TextSpec {
        x: args::number(doc, a, t, "x")?,
        y: args::number(doc, a, t, "y")?,
        content: args::string(
            doc,
            a,
            t,
            "content",
            1024,
            "1-256 characters without control characters",
        )?,
        font_size: args::opt_number(doc, a, t, "fontSize")?.unwrap_or(16.0),
        anchor,
        fill: args::opt_string(doc, a, t, "fill", 7, "a colour like `#336699`")?
            .unwrap_or_else(|| "#000000".to_owned()),
    }))
}

/// Static error text per tool and failure class (missing target, invalid
/// value, unsupported target).
fn failure(tool: &'static str, error: &V1DocumentError) -> &'static str {
    let (missing, invalid, unsupported) = match tool {
        GRADIENT_ADD => (
            "gradient_add: document has no usable defs",
            "gradient_add: a coordinate, radius or stop is out of range; coordinates are finite numbers up to 1000000, r > 0, stops are 2-16 `offset:#rrggbb:opacity` with offsets ascending in 0-1",
            "gradient_add: unsupported document structure",
        ),
        GRADIENT_SET_STOP => (
            "gradient_set_stop: gradientId or index not found; use a gradientId returned by gradient_add and an index below its stop count",
            "gradient_set_stop: offset (0-1, ordered against neighbouring stops), color (`#rrggbb`) or opacity (0-1) is invalid",
            "gradient_set_stop: target is not a gradient",
        ),
        SET_PAINT => (
            "element_set_paint: elementId or gradientId not found; use ids returned by element_add and gradient_add",
            "element_set_paint: invalid ids",
            "element_set_paint: elementId must be a rect, ellipse, circle, line or path",
        ),
        CLIP_ADD => (
            "clip_add: document has no usable defs",
            "clip_add: a and b must be finite numbers up to 1000000 and c and d must be greater than 0 (rect x,y,w,h; ellipse cx,cy,rx,ry)",
            "clip_add: unsupported document structure",
        ),
        SET_CLIP => (
            "element_set_clip: elementId or clipId not found; use a clipId returned by clip_add",
            "element_set_clip: invalid ids",
            "element_set_clip: elementId must be a shape or group",
        ),
        SET_TRANSFORM => (
            "element_set_transform: elementId not found",
            "element_set_transform: every value must be a finite number, scale values must keep the matrix finite",
            "element_set_transform: element cannot be transformed",
        ),
        _ => (
            "text_add: document has no root",
            "text_add: x and y must be finite numbers up to 1000000, content 1-256 characters without control characters, fontSize 1-1024, fill `#rrggbb`",
            "text_add: unsupported document structure",
        ),
    };
    match error {
        V1DocumentError::MissingStableId | V1DocumentError::InvalidStableId => missing,
        V1DocumentError::InvalidGeometry
        | V1DocumentError::InvalidColor
        | V1DocumentError::FormattingLimit => invalid,
        V1DocumentError::Unsupported | V1DocumentError::UnsupportedTransform => unsupported,
        _ => match tool {
            GRADIENT_ADD => "gradient_add: edit failed",
            GRADIENT_SET_STOP => "gradient_set_stop: edit failed",
            SET_PAINT => "element_set_paint: edit failed",
            CLIP_ADD => "clip_add: edit failed",
            SET_CLIP => "element_set_clip: edit failed",
            SET_TRANSFORM => "element_set_transform: edit failed",
            _ => "text_add: edit failed",
        },
    }
}

fn edit<R>(
    app: &mut AppDocument,
    tool: &'static str,
    f: impl FnOnce(&mut vexel_v1_document::V1Document) -> Result<R, V1DocumentError>,
) -> Result<R, &'static str> {
    app.edit(f).map_err(|error| match error {
        SessionEditError::Operation(error) => failure(tool, &error),
        SessionEditError::History(_) => "project history unavailable",
    })
}

fn reply(operation: &str, extra: &str, app: &AppDocument) -> McpOutcome {
    McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"{operation}\"{extra},\"dirty\":{}}}",
        app.is_dirty()
    ))
}

pub(crate) fn apply(app: &mut AppDocument, op: Op) -> Result<McpOutcome, &'static str> {
    match op {
        Op::GradientAdd {
            geometry,
            units,
            spread,
            stops,
        } => {
            let id = edit(app, GRADIENT_ADD, |doc| match geometry {
                Geometry::Linear { x1, y1, x2, y2 } => {
                    doc.add_linear_gradient(x1, y1, x2, y2, units, spread, &stops)
                }
                Geometry::Radial { cx, cy, r, fx, fy } => {
                    doc.add_radial_gradient(cx, cy, r, fx, fy, units, spread, &stops)
                }
            })?;
            let ids = (0..stops.len())
                .map(|i| format!("\"{id}-stop-{i}\""))
                .collect::<Vec<_>>()
                .join(",");
            Ok(reply(
                GRADIENT_ADD,
                &format!(",\"gradientId\":\"{id}\",\"stopIds\":[{ids}]"),
                app,
            ))
        }
        Op::GradientSetStop {
            gradient_id,
            index,
            offset,
            color,
            opacity,
        } => {
            edit(app, GRADIENT_SET_STOP, |doc| {
                doc.set_gradient_stop(&gradient_id, index, offset, color.as_deref(), opacity)
            })?;
            Ok(reply(
                GRADIENT_SET_STOP,
                &format!(",\"gradientId\":\"{gradient_id}\",\"index\":{index}"),
                app,
            ))
        }
        Op::SetPaint {
            element_id,
            stroke,
            gradient_id,
        } => {
            edit(app, SET_PAINT, |doc| {
                if stroke {
                    doc.set_stroke_ref(&element_id, &gradient_id)
                } else {
                    doc.set_fill_ref(&element_id, &gradient_id)
                }
            })?;
            Ok(reply(
                SET_PAINT,
                &format!(
                    ",\"elementId\":\"{element_id}\",\"property\":\"{}\",\"gradientId\":\"{gradient_id}\"",
                    if stroke { "stroke" } else { "fill" }
                ),
                app,
            ))
        }
        Op::ClipAdd {
            ellipse,
            a,
            b,
            c,
            d,
        } => {
            let id = edit(app, CLIP_ADD, |doc| {
                if ellipse {
                    doc.add_clip_ellipse(a, b, c, d)
                } else {
                    doc.add_clip_rect(a, b, c, d)
                }
            })?;
            Ok(reply(CLIP_ADD, &format!(",\"clipId\":\"{id}\""), app))
        }
        Op::SetClip {
            element_id,
            clip_id,
        } => {
            edit(app, SET_CLIP, |doc| {
                doc.set_clip(&element_id, clip_id.as_deref())
            })?;
            let clip = clip_id.map_or_else(|| "null".to_owned(), |id| format!("\"{id}\""));
            Ok(reply(
                SET_CLIP,
                &format!(",\"elementId\":\"{element_id}\",\"clipId\":{clip}"),
                app,
            ))
        }
        Op::SetTransform { element_id, spec } => {
            edit(app, SET_TRANSFORM, |doc| {
                doc.set_transform(&element_id, &spec)
            })?;
            Ok(reply(
                SET_TRANSFORM,
                &format!(",\"elementId\":\"{element_id}\""),
                app,
            ))
        }
        Op::TextAdd(spec) => {
            let id = edit(app, TEXT_ADD, |doc| doc.add_text(&spec))?;
            Ok(reply(TEXT_ADD, &format!(",\"elementId\":\"{id}\""), app))
        }
    }
}

#[cfg(test)]
#[path = "ext_paint_tests.rs"]
mod tests;
