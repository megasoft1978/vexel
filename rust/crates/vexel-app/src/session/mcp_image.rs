//! `image_trace`, `image_segment` and `image_inpaint`: native raster tracing
//! (ADR-0036 B5). Images are PNG files read through the grant policy, opened
//! without following a final symlink, size-checked before decoding and
//! pixel-checked before any large allocation. Outputs are new files only.

use super::mcp::{McpHost, McpOutcome, SOURCE_LIMIT, source, stable_ids};
use super::mcp_audio::json_string;
use crate::preview::SCENE_LIMIT;
use crate::project_session::SessionEditError;
use crate::session::AppDocument;
use crate::trace::{
    self, MAX_PIXELS, Rgba, TraceError, TraceOptions, inpaint::inpaint, segment as seg,
};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use vexel_mcp_access::{AccessError, AccessPolicy};
use vexel_png::Compression;

/// Largest PNG file read.
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// Longest reply body returned inline.
const MAX_REPLY: usize = 1024 * 1024;
const MAX_MASK_PATH_D: usize = 256 * 1024;
const O_NOFOLLOW: i32 = 0x100;
/// Source bytes one added `<path>` costs besides its data.
const ELEMENT_OVERHEAD: usize = 160;

#[derive(Clone, Debug, PartialEq)]
pub struct TraceSpec {
    pub path: String,
    pub k: usize,
    pub min_area: Option<u32>,
    pub tolerance: f64,
    pub seed: u64,
    pub region: Option<[usize; 4]>,
    pub group: Option<String>,
    pub prefix: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegmentSpec {
    pub path: String,
    pub seeds: Vec<(usize, usize)>,
    pub include: Vec<Vec<(f64, f64)>>,
    pub exclude: Vec<Vec<(f64, f64)>>,
    pub tolerance: f64,
    pub fill_holes: bool,
    pub out_mask: Option<String>,
    pub trace: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InpaintSpec {
    pub path: String,
    pub mask_path: Option<String>,
    pub polygon: Option<Vec<(f64, f64)>>,
    pub out_path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImageOp {
    Trace(Box<TraceSpec>),
    Segment(Box<SegmentSpec>),
    Inpaint(Box<InpaintSpec>),
}

struct Decoded {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

fn access_error(e: AccessError) -> &'static str {
    match e {
        AccessError::NotGranted => "path not granted",
        AccessError::InvalidPath => "invalid preview tool arguments",
        _ => "path not found",
    }
}

fn trace_error(e: TraceError) -> &'static str {
    e.message()
}

/// Reads and decodes a PNG inside a grant.
fn read_png(policy: &AccessPolicy, path: &str) -> Result<Decoded, &'static str> {
    let canonical = policy.resolve_save_path(path).map_err(|e| match e {
        AccessError::InvalidPath => "path not found",
        other => access_error(other),
    })?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(&canonical)
        .map_err(|_| "image not found")?;
    let meta = file.metadata().map_err(|_| "image not found")?;
    if !meta.is_file() {
        return Err("image not found");
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err("image file larger than 64 MiB");
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "image not found")?;
    if bytes.starts_with(&[0xff, 0xd8]) {
        return Err("image JPEG is not supported; convert it to PNG first");
    }
    let pixels = crate::compose::decode_png(&bytes).map_err(
        |_| "image is not a PNG this build can decode (not interlaced, 16 megapixels at most)",
    )?;
    drop(bytes);
    let (width, height) = (pixels.width as usize, pixels.height as usize);
    let tight = width.checked_mul(4).ok_or("image too large")?;
    if width
        .checked_mul(height)
        .is_none_or(|n| n == 0 || n > MAX_PIXELS)
    {
        return Err(TraceError::BadImage.message());
    }
    let rgba = if pixels.row_stride == tight && pixels.rgba.len() == tight * height {
        pixels.rgba
    } else {
        let mut v = Vec::with_capacity(tight * height);
        for row in 0..height {
            let start = row * pixels.row_stride;
            v.extend_from_slice(
                pixels
                    .rgba
                    .get(start..start + tight)
                    .ok_or("image is not a PNG this build can decode")?,
            );
        }
        v
    };
    Ok(Decoded {
        width,
        height,
        rgba,
    })
}

fn leaf_is_png(path: &str) -> bool {
    let leaf = path.rsplit('/').next().unwrap_or("");
    leaf.len() > 4 && leaf.as_bytes()[leaf.len() - 4..].eq_ignore_ascii_case(b".png")
}

/// Writes a new PNG file; an existing file is never replaced.
fn write_new(policy: &AccessPolicy, out: &str, png: &[u8]) -> Result<String, &'static str> {
    if !leaf_is_png(out) {
        return Err("image output must be a .png path");
    }
    let target = policy.resolve_save_path(out).map_err(access_error)?;
    if std::fs::symlink_metadata(&target).is_ok() {
        return Err("image output file already exists; choose a new file name");
    }
    super::mcp_png::write_atomic(&target, png)?;
    Ok(target.to_string_lossy().into_owned())
}

fn encode(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, &'static str> {
    vexel_png::encode_rgba8(width as u32, height as u32, rgba, Compression::Fixed)
        .map_err(|_| "image result too large to encode")
}

fn reply(body: String) -> Result<McpOutcome, &'static str> {
    if body.len() > MAX_REPLY {
        return Err(
            "image result too large to return inline; pass groupId, outMask or raise minArea",
        );
    }
    Ok(McpOutcome::Reply(body))
}

impl McpHost {
    pub(super) fn image(
        &mut self,
        app: &mut AppDocument,
        op: ImageOp,
    ) -> Result<McpOutcome, &'static str> {
        match op {
            ImageOp::Trace(spec) => self.image_trace(app, *spec),
            ImageOp::Segment(spec) => image_segment(&self.policy(), *spec),
            ImageOp::Inpaint(spec) => image_inpaint(&self.policy(), *spec),
        }
    }

    fn image_trace(
        &mut self,
        app: &mut AppDocument,
        spec: TraceSpec,
    ) -> Result<McpOutcome, &'static str> {
        let img = read_png(&self.policy(), &spec.path)?;
        let view = Rgba::new(img.width, img.height, &img.rgba).map_err(trace_error)?;
        // Default minArea grows with the image so big art stays under the limits.
        let min_area = spec
            .min_area
            .unwrap_or_else(|| ((img.width * img.height / 12_000) as u32).clamp(30, 400));
        let opts = TraceOptions {
            tolerance: spec.tolerance,
            min_area,
            hole_area: min_area / 2,
            seed: spec.seed,
            ..TraceOptions::default()
        };
        let layers = trace::trace_image(&view, spec.k, &opts, spec.region).map_err(trace_error)?;
        drop(img.rgba);
        let bytes: usize = layers.iter().map(|l| l.d.len()).sum();
        let head = format!(
            "\"width\":{},\"height\":{},\"colors\":{},\"layers\":{},\"bytes\":{bytes}",
            img.width,
            img.height,
            layers.len(),
            layers.len()
        );
        let Some(group) = spec.group else {
            let paths: Vec<String> = layers
                .iter()
                .map(|l| format!("{{\"fill\":\"{}\",\"d\":\"{}\"}}", l.hex(), l.d))
                .collect();
            return reply(format!(
                "{{\"ok\":true,{head},\"paths\":[{}]}}",
                paths.join(",")
            ));
        };
        let src = source(app)?;
        let ids = stable_ids(&src);
        if group != "root" && !ids.contains(&group) {
            return Err(
                "image trace groupId is not in the document; use root or an existing group id",
            );
        }
        if ids.len() + layers.len() > SCENE_LIMIT
            || src.len() + bytes + layers.len() * ELEMENT_OVERHEAD > SOURCE_LIMIT
        {
            return Err(
                "image trace result exceeds the 4096-element or 4 MiB document limit; raise minArea or lower k",
            );
        }
        let new_ids: Vec<String> = (0..layers.len())
            .map(|i| format!("{}-{i}", spec.prefix))
            .collect();
        if new_ids.iter().any(|id| ids.contains(id)) {
            return Err("image trace idPrefix collides with existing ids; choose another idPrefix");
        }
        let parent = (group != "root").then_some(group);
        app.edit(|doc| {
            doc.set_insert_parent(parent.as_deref())?;
            let added = layers
                .iter()
                .zip(&new_ids)
                .try_for_each(|(l, id)| doc.add_path_with_id(&l.d, &l.hex(), "none", 0.0, id));
            doc.set_insert_parent(None)?;
            added
        })
        .map_err(|e| match e {
            SessionEditError::Operation(_) => "image trace edit rejected by the document",
            SessionEditError::History(_) => "project history unavailable",
        })?;
        let list: Vec<String> = new_ids.iter().map(|i| json_string(i)).collect();
        reply(format!(
            "{{\"ok\":true,\"operation\":\"image_trace\",{head},\"dirty\":{},\"elementIds\":[{}]}}",
            app.is_dirty(),
            list.join(",")
        ))
    }
}

fn image_segment(policy: &AccessPolicy, spec: SegmentSpec) -> Result<McpOutcome, &'static str> {
    let img = read_png(policy, &spec.path)?;
    let view = Rgba::new(img.width, img.height, &img.rgba).map_err(trace_error)?;
    let mask = seg::segment(
        &view,
        &seg::Spec {
            seeds: spec.seeds,
            include: spec.include,
            exclude: spec.exclude,
            tolerance: spec.tolerance,
            fill_holes: spec.fill_holes,
        },
    )
    .map_err(trace_error)?;
    drop(img.rgba);
    let stats = mask.stats();
    let mut body = format!(
        "{{\"ok\":true,\"width\":{},\"height\":{},\"area\":{},\"bbox\":[{},{},{},{}]",
        mask.width,
        mask.height,
        stats.area,
        stats.bbox[0],
        stats.bbox[1],
        stats.bbox[2],
        stats.bbox[3]
    );
    if let Some(out) = &spec.out_mask {
        let rgba: Vec<u8> = mask
            .bits
            .iter()
            .flat_map(|&b| {
                if b {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect();
        let written = write_new(policy, out, &encode(mask.width, mask.height, &rgba)?)?;
        body.push_str(&format!(",\"maskPath\":{}", json_string(&written)));
    }
    if spec.trace {
        let opts = TraceOptions::default();
        let (d, _) =
            trace::trace(&mask.bits, mask.width, mask.height, &opts).map_err(trace_error)?;
        if d.len() > MAX_MASK_PATH_D {
            return Err("image mask outline too large to return; use outMask instead of trace");
        }
        body.push_str(&format!(",\"d\":\"{d}\""));
    }
    body.push('}');
    reply(body)
}

fn image_inpaint(policy: &AccessPolicy, spec: InpaintSpec) -> Result<McpOutcome, &'static str> {
    let img = read_png(policy, &spec.path)?;
    let view = Rgba::new(img.width, img.height, &img.rgba).map_err(trace_error)?;
    let mask: Vec<bool> = if let Some(mask_path) = &spec.mask_path {
        let m = read_png(policy, mask_path)?;
        if (m.width, m.height) != (img.width, img.height) {
            return Err("image inpaint maskPath must have the same size as path");
        }
        m.rgba
            .chunks_exact(4)
            .map(|p| p[3] >= 128 && p[0] >= 128)
            .collect()
    } else {
        let poly = spec
            .polygon
            .as_ref()
            .ok_or(TraceError::BadRegion.message())?;
        seg::rasterize(std::slice::from_ref(poly), img.width, img.height)
    };
    let filled = mask.iter().filter(|&&b| b).count();
    let out = inpaint(&view, &mask).map_err(trace_error)?;
    let png = encode(img.width, img.height, &out)?;
    let written = write_new(policy, &spec.out_path, &png)?;
    reply(format!(
        "{{\"ok\":true,\"outPath\":{},\"width\":{},\"height\":{},\"filled\":{filled},\"bytes\":{}}}",
        json_string(&written),
        img.width,
        img.height,
        png.len()
    ))
}
