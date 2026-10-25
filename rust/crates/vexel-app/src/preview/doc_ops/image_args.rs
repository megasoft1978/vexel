//! Arguments of `image_trace`, `image_segment` and `image_inpaint`.

use super::super::ext_structure::document;
use super::super::{JsonType, PreviewArgumentError};
use super::audio_args::{flag, float, invalid, required_text, text, uint};
use crate::session::mcp_image::{ImageOp, InpaintSpec, SegmentSpec, TraceSpec};
use vexel_json::JsonDocument;

type Point = (f64, f64);

fn items(doc: &JsonDocument<'_>, token: usize, max: usize) -> Option<Vec<usize>> {
    let t = doc.token(token).filter(|t| t.kind == JsonType::Array)?;
    if t.child_count > max {
        return None;
    }
    let mut out = Vec::new();
    let mut at = t.first_child;
    while let Some(i) = at {
        out.push(i);
        at = doc.token(i).and_then(|t| t.next_sibling);
    }
    Some(out)
}

fn num(doc: &JsonDocument<'_>, token: usize) -> Option<f64> {
    doc.token(token)
        .filter(|t| t.kind == JsonType::Number)
        .and_then(|_| doc.raw_token(token))
        .filter(|r| r.len() <= 32)
        .and_then(|r| std::str::from_utf8(r).ok())
        .and_then(|r| r.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn point(doc: &JsonDocument<'_>, token: usize) -> Option<Point> {
    match items(doc, token, 2)?.as_slice() {
        [x, y] => Some((num(doc, *x)?, num(doc, *y)?)),
        _ => None,
    }
}

fn polygon(doc: &JsonDocument<'_>, token: usize) -> Option<Vec<Point>> {
    let pts = items(doc, token, 512)?
        .into_iter()
        .map(|p| point(doc, p))
        .collect::<Option<Vec<_>>>()?;
    (pts.len() >= 3).then_some(pts)
}

fn polygons(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
) -> Result<Vec<Vec<Point>>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(Vec::new());
    };
    let bad = || invalid(tool, key, "up to 16 polygons of 3-512 [x,y] points");
    items(doc, token, 16)
        .ok_or_else(bad)?
        .into_iter()
        .map(|p| polygon(doc, p))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(bad)
}

fn pixel(p: Point) -> Option<(usize, usize)> {
    let ok = |v: f64| v >= 0.0 && v.fract() == 0.0 && v < 1e7;
    (ok(p.0) && ok(p.1)).then_some((p.0 as usize, p.1 as usize))
}

fn id_prefix(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}

fn trace(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<ImageOp, PreviewArgumentError> {
    let int = |key: &'static str, lo: u32, hi: u32, default: u32| {
        let v = uint(doc, args, tool, key)?.unwrap_or(default);
        if (lo..=hi).contains(&v) {
            Ok(v)
        } else {
            Err(invalid(tool, key, &format!("an integer from {lo} to {hi}")))
        }
    };
    let tolerance = float(doc, args, tool, "tolerance")?.unwrap_or(1.5);
    if !(0.25..=8.0).contains(&tolerance) {
        return Err(invalid(tool, "tolerance", "a number from 0.25 to 8"));
    }
    let region = match doc.object_get(args, b"region") {
        None => None,
        Some(t) => {
            let v = items(doc, t, 4)
                .and_then(|v| {
                    v.into_iter()
                        .map(|i| num(doc, i))
                        .collect::<Option<Vec<_>>>()
                })
                .and_then(|v| {
                    v.into_iter()
                        .map(|n| pixel((n, 0.0)).map(|p| p.0))
                        .collect::<Option<Vec<_>>>()
                })
                .filter(|v| v.len() == 4);
            Some(match v.as_deref() {
                Some(&[a, b, c, d]) => [a, b, c, d],
                _ => return Err(invalid(tool, "region", "[x0,y0,x1,y1] pixel integers")),
            })
        }
    };
    let prefix = text(doc, args, tool, "idPrefix", 40)?.unwrap_or_else(|| "trace".into());
    if !id_prefix(&prefix) {
        return Err(invalid(tool, "idPrefix", "letters, digits and - _ . :"));
    }
    Ok(ImageOp::Trace(Box::new(TraceSpec {
        path: required_text(doc, args, tool, "path", 1023)?,
        k: int("k", 1, 64, 16)? as usize,
        min_area: uint(doc, args, tool, "minArea")?
            .map(|_| int("minArea", 1, 1_000_000, 30))
            .transpose()?,
        tolerance,
        seed: u64::from(int("seed", 0, u32::MAX, 1)?),
        region,
        group: text(doc, args, tool, "groupId", 120)?,
        prefix,
    })))
}

fn segment(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<ImageOp, PreviewArgumentError> {
    let bad = || invalid(tool, "seeds", "1-64 [x,y] pixel integer pairs");
    let seeds = doc
        .object_get(args, b"seeds")
        .and_then(|t| items(doc, t, 64))
        .filter(|s| !s.is_empty())
        .ok_or_else(bad)?
        .into_iter()
        .map(|s| point(doc, s).and_then(pixel))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(bad)?;
    let tolerance = float(doc, args, tool, "tolerance")?.unwrap_or(36.0);
    if !(0.0..=441.0).contains(&tolerance) {
        return Err(invalid(tool, "tolerance", "a number from 0 to 441"));
    }
    Ok(ImageOp::Segment(Box::new(SegmentSpec {
        path: required_text(doc, args, tool, "path", 1023)?,
        seeds,
        include: polygons(doc, args, tool, "include")?,
        exclude: polygons(doc, args, tool, "exclude")?,
        tolerance,
        fill_holes: flag(doc, args, tool, "fillHoles")?.unwrap_or(true),
        out_mask: text(doc, args, tool, "outMask", 1023)?,
        trace: flag(doc, args, tool, "trace")?.unwrap_or(false),
    })))
}

fn inpaint(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<ImageOp, PreviewArgumentError> {
    let mask_path = text(doc, args, tool, "maskPath", 1023)?;
    let poly = match doc.object_get(args, b"polygon") {
        None => None,
        Some(t) => {
            Some(polygon(doc, t).ok_or_else(|| invalid(tool, "polygon", "3-512 [x,y] points"))?)
        }
    };
    if mask_path.is_some() == poly.is_some() {
        return Err(invalid(
            tool,
            "maskPath",
            "exactly one of maskPath or polygon",
        ));
    }
    Ok(ImageOp::Inpaint(Box::new(InpaintSpec {
        path: required_text(doc, args, tool, "path", 1023)?,
        mask_path,
        polygon: poly,
        out_path: required_text(doc, args, tool, "outPath", 1023)?,
    })))
}

/// `None` when `tool` is not an image tool.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<ImageOp, PreviewArgumentError>> {
    let parser = match tool {
        "image_trace" => trace,
        "image_segment" => segment,
        "image_inpaint" => inpaint,
        _ => return None,
    };
    Some(document(doc, args, tool).and_then(|()| parser(doc, args, tool)))
}
