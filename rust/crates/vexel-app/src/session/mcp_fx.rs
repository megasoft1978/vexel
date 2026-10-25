//! `effect_apply`: 80s-cartoon effect presets as generated fragment markup
//! plus animated tracks, applied (or removed by `idPrefix`) in ONE undoable
//! edit. See `mcp_fx_presets` and `mcp_fx_filters` for the presets.

use super::mcp_face::{MAX_KEYS, facts};
use super::mcp_fx_filters as filters;
use super::mcp_fx_presets::{self as overlays, Ctx, Plan};
pub use super::mcp_fx_spec::{FxSpec, Preset};
use super::mcp_fx_util::FxTrack;
use super::mcp_rig::transformed;
use super::settings;
use crate::project_session::{ProjectTimelineCandidate, ProjectTimelineError, SessionEditError};
use crate::session::{AppDocument, McpOutcome, check_room};
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use vexel_timeline::{BulkKey, BulkTrack, EditLimits};

type Fail = &'static str;

/// Tags a target may have: drawable shapes, groups, text, images and uses.
const TARGET_TAGS: &[&str] = &[
    "g", "rect", "circle", "ellipse", "path", "polygon", "polyline", "line", "text", "image", "use",
];
/// Root children that are not scene content (never wrapped for a scene effect).
const NON_CONTENT: &[&str] = &[
    "defs", "desc", "title", "metadata", "style", "clipPath", "mask", "symbol",
];

fn valid_prefix(p: &str) -> bool {
    // `vexel-` ids are reserved (rig, mouth groups); never generate or remove them.
    (1..=48).contains(&p.len())
        && !p.starts_with("vexel-")
        && p.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn renders(app: &AppDocument) -> bool {
    app.source_bytes().is_ok_and(|source| {
        crate::animation::frame::decode(&source)
            .and_then(|scene| scene.render_sized(0.0, (8, 8), &AtomicBool::new(false)))
            .is_ok()
    })
}

/// Track-id suffixes (after `{prefix}-`) the presets generate. The single source
/// for both add-side collision checks and remove, so they cannot drift.
const FIXED_SUFFIXES: &[&str] = &[
    "opacity", "radius", "ro-x", "ro-y", "bo-x", "bo-y", "smear", "window", "flash", "edge",
    "dot-r", "jitter", "roll",
];

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// True when `id` is exactly a track id some preset generates under `prefix`.
fn generated_track(prefix: &str, id: &[u8]) -> bool {
    let Some(rest) = std::str::from_utf8(id)
        .ok()
        .and_then(|id| id.strip_prefix(prefix))
        .and_then(|r| r.strip_prefix('-'))
    else {
        return false;
    };
    if FIXED_SUFFIXES.contains(&rest) {
        return true;
    }
    // `c{k}-move` (copies) and `{tag}{k}-vis` (cycled frames).
    if let Some(k) = rest.strip_suffix("-move").and_then(|r| r.strip_prefix('c')) {
        return digits(k);
    }
    rest.strip_suffix("-vis").is_some_and(|r| {
        let n = r.trim_start_matches(|c: char| c.is_ascii_alphabetic());
        n.len() < r.len() && digits(n)
    })
}

fn track_ids<'a>(tracks: &'a [(Vec<u8>, Vec<u8>)], prefix: &str) -> Vec<&'a Vec<u8>> {
    tracks
        .iter()
        .filter(|(id, _)| generated_track(prefix, id))
        .map(|(id, _)| id)
        .collect()
}

/// True when some generated track targets the prefix root or one of its
/// children, which proves this call family created the `{prefix}` elements.
fn owns_elements(tracks: &[(Vec<u8>, Vec<u8>)], prefix: &str) -> bool {
    let head = format!("{prefix}-");
    tracks.iter().any(|(id, target)| {
        generated_track(prefix, id)
            && (target.as_slice() == prefix.as_bytes() || target.starts_with(head.as_bytes()))
    })
}

fn is_group(app: &AppDocument, id: &str) -> bool {
    app.document()
        .element_get(id)
        .is_ok_and(|info| info.tag == "g")
}

pub(crate) fn effect_apply(app: &mut AppDocument, spec: FxSpec) -> Result<McpOutcome, Fail> {
    let timeline = facts(app)?;
    let prefix = pick_prefix(app, &spec, &timeline.tracks)?;
    // Inside a batch the group is already open and the batch collapses the steps.
    let group = app.begin_group().ok();
    let result = if spec.remove {
        remove(app, &prefix)
    } else {
        add(app, &spec, &prefix, timeline.duration)
    };
    match (group, result) {
        (Some(group), Ok(reply)) => {
            app.end_group(group)
                .map_err(|_| "project history unavailable")?;
            Ok(McpOutcome::Reply(reply))
        }
        (Some(group), Err(message)) => {
            app.rollback_group(group)
                .map_err(|_| "project history unavailable")?;
            Err(message)
        }
        (None, result) => result.map(McpOutcome::Reply),
    }
}

fn exists(app: &AppDocument, id: &str) -> bool {
    app.document().element_get(id).is_ok()
}

fn pick_prefix(
    app: &AppDocument,
    spec: &FxSpec,
    tracks: &[(Vec<u8>, Vec<u8>)],
) -> Result<String, Fail> {
    let taken = |p: &str| {
        exists(app, p) || exists(app, &format!("{p}-scene")) || !track_ids(tracks, p).is_empty()
    };
    match &spec.prefix {
        Some(p) if !valid_prefix(p) => {
            Err("effect_apply: idPrefix must be 1-48 characters of A-Z a-z 0-9 _ -")
        }
        Some(p) if spec.remove => Ok(p.clone()),
        Some(p) if taken(p) => {
            Err("effect_apply: idPrefix is already used; choose another or remove it first")
        }
        Some(p) => Ok(p.clone()),
        None if spec.remove => Err("effect_apply: remove needs the idPrefix of the earlier call"),
        None => (1..10_000)
            .map(|n| format!("fx-{}-{n}", spec.preset.name()))
            .find(|p| !taken(p))
            .ok_or("effect_apply: no free idPrefix"),
    }
}

fn add(app: &mut AppDocument, spec: &FxSpec, prefix: &str, dur: u32) -> Result<String, Fail> {
    let preset = spec.preset;
    let num = |name: &str| spec.nums.iter().find(|(k, _)| *k == name).map(|(_, v)| *v);
    let s = num("start").map_or(0, |v| v as u32);
    if s + 1 >= dur {
        return Err(
            "effect_apply: start must be at least 2 ticks below the timeline duration (lengthen it with native_timeline_set_duration)",
        );
    }
    let last = dur - 1;
    let e = num("ticks")
        .map_or(last, |t| (s + t as u32).min(last))
        .max(s + 1);
    let needs_target = matches!(preset, Preset::Smear);
    let filter_preset = matches!(
        preset,
        Preset::Glow | Preset::ChromaticSplit | Preset::MotionBlurDir | Preset::Smear
    );
    let (w, h) = app.size();
    let mut region = (0.0, 0.0, f64::from(w), f64::from(h));
    let mut pivot = [0.0, 0.0];
    let mut parent = "root".to_owned();
    let mut target = spec.target.clone();
    let mut wrap: Option<Vec<String>> = None;
    if let Some(id) = &target {
        let info = app
            .document()
            .element_get(id)
            .map_err(|_| "effect_apply: target must be the id of an element in the document")?;
        if !TARGET_TAGS.contains(&info.tag.as_str()) {
            return Err("effect_apply: target must be a group or a shape");
        }
        parent = info.parent.clone().unwrap_or_else(|| "root".to_owned());
        if let Some((x0, y0, x1, y1)) = info.bounds {
            region = (x0, y0, x1 - x0, y1 - y0);
            pivot = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
        }
        let needs_bounds = !filter_preset || preset == Preset::Smear;
        if needs_bounds && (info.bounds.is_none() || transformed(app, id)) {
            return Err(
                "effect_apply: the target needs usable bounds and no transform on it or its ancestors for this preset",
            );
        }
    } else if needs_target {
        return Err("effect_apply: smear needs a target group");
    } else if filter_preset {
        let (tree, _) = app
            .document()
            .tree_get(1, 1000)
            .map_err(|_| "effect_apply: cannot read the scene")?;
        let ids: Vec<String> = tree
            .children
            .iter()
            .filter(|n| !NON_CONTENT.contains(&n.tag.as_str()))
            .filter_map(|n| n.id.clone())
            .collect();
        if ids.is_empty() {
            return Err("effect_apply: the scene has no content to apply the effect to");
        }
        target = Some(format!("{prefix}-scene"));
        wrap = Some(ids);
    }
    let ctx = Ctx {
        p: prefix,
        spec,
        s,
        e,
        dur,
        region,
        target,
        parent,
        pivot,
    };
    let plan: Plan = match preset {
        Preset::SpeedLines => overlays::speed_lines(&ctx),
        Preset::FilmGrain => overlays::film_grain(&ctx),
        Preset::Halftone => overlays::halftone(&ctx),
        Preset::VhsScanlines => overlays::vhs_scanlines(&ctx),
        Preset::Vignette => overlays::vignette(&ctx),
        Preset::Flash => overlays::flash(&ctx),
        Preset::Glow => filters::glow(&ctx),
        Preset::ChromaticSplit => filters::chromatic_split(&ctx),
        Preset::MotionBlurDir => filters::motion_blur_dir(&ctx),
        Preset::Smear => filters::smear(&ctx),
    }?;
    check_room(app, plan.markup.matches(" id=\"").count() + 2)?;
    let was = renders(app);
    if let Some(ids) = &wrap {
        let scene = format!("{prefix}-scene");
        app.edit(|doc| {
            let group = doc.group(ids)?;
            doc.rename_element(&group, &scene)
        })
        .map_err(|_| "effect_apply: could not group the scene content")?;
    }
    let mut element_ids: Vec<String> = Vec::new();
    if !plan.markup.is_empty() {
        element_ids = app
            .edit(|doc| doc.add_fragment(&plan.parent, plan.markup.as_bytes()))
            .map_err(|e| match e {
                SessionEditError::Operation(_) => {
                    "effect_apply: the generated markup was refused (see docs/mcp.md); check target and parameters"
                }
                SessionEditError::History(_) => "project history unavailable",
            })?;
        if was && !renders(app) {
            return Err("effect_apply: the renderer cannot draw the generated effect; edit undone");
        }
    }
    let (tracks, keys) = write(app, &plan.tracks, dur)?;
    let ids = element_ids
        .iter()
        .map(|i| format!("\"{i}\""))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\"ok\":true,\"operation\":\"effect_apply\",\"preset\":\"{}\",\"idPrefix\":\"{prefix}\",\"startTick\":{s},\"endTick\":{e},\"elementIds\":[{ids}],\"tracks\":{tracks},\"keys\":{keys}}}",
        preset.name()
    ))
}

fn write(app: &mut AppDocument, tracks: &[FxTrack], dur: u32) -> Result<(usize, usize), Fail> {
    let total: usize = tracks.iter().map(|t| t.keys.len() + 1).sum();
    if tracks.is_empty() {
        return Ok((0, 0));
    }
    if total > MAX_KEYS {
        return Err("effect_apply: too many keys for one call (at most 16384)");
    }
    if tracks.iter().any(|t| t.keys.iter().any(|k| k.0 >= dur)) {
        return Err(
            "effect_apply: every key tick must be below the timeline duration (lengthen it with native_timeline_set_duration or shorten start/ticks)",
        );
    }
    // Ease names travel as JSON strings, like the other track tools.
    let eases: Vec<Vec<Option<Vec<u8>>>> = tracks
        .iter()
        .map(|t| {
            t.keys
                .iter()
                .map(|k| k.2.map(|name| format!("\"{name}\"").into_bytes()))
                .collect()
        })
        .collect();
    let key_sets: Vec<Vec<BulkKey<'_>>> = tracks
        .iter()
        .zip(&eases)
        .map(|(t, ease)| {
            t.keys
                .iter()
                .zip(ease)
                .map(|((tick, value, _), ease)| BulkKey {
                    tick: *tick,
                    value,
                    ease: ease.as_deref(),
                })
                .collect()
        })
        .collect();
    let bulk: Vec<BulkTrack<'_>> = tracks
        .iter()
        .zip(&key_sets)
        .map(|(t, keys)| BulkTrack {
            id: t.id.as_bytes(),
            target: t.target.as_bytes(),
            property: t.property.as_bytes(),
            value_type: t.value_type.as_bytes(),
            interpolation: t.interpolation.as_bytes(),
            value: &t.value,
            name: t.id.as_bytes(),
            noise: None,
            keys,
        })
        .collect();
    let failure = Cell::new(None);
    app.edit_timeline(settings(), EditLimits::default(), |c| {
        c.add_property_tracks(&bulk, &failure)
    })
    .map_err(|_| "effect_apply: the timeline rejected the tracks (track limit of 256, size limit, or the target already has a track for that property)")?;
    Ok((tracks.len(), total))
}

fn remove(app: &mut AppDocument, prefix: &str) -> Result<String, Fail> {
    let timeline = facts(app)?;
    let stale: Vec<Vec<u8>> = track_ids(&timeline.tracks, prefix)
        .into_iter()
        .cloned()
        .collect();
    let scene = format!("{prefix}-scene");
    let owned = owns_elements(&timeline.tracks, prefix);
    let has_root = owned && is_group(app, prefix);
    let has_scene = owned && is_group(app, &scene);
    if stale.is_empty() && !has_root && !has_scene {
        return Err("effect_apply: nothing was generated under that idPrefix");
    }
    if !stale.is_empty() {
        app.edit_timeline(settings(), EditLimits::default(), |c| {
            let mut current: Option<ProjectTimelineCandidate> = None;
            for id in &stale {
                current = Some(match current {
                    None => c.remove_track(id)?,
                    Some(prior) => prior.remove_track(id)?,
                });
            }
            current.ok_or(ProjectTimelineError::Limit)
        })
        .map_err(|_| "effect_apply: the timeline rejected removing the tracks")?;
    }
    if has_scene {
        app.edit(|doc| doc.ungroup(&scene))
            .map_err(|_| "effect_apply: could not ungroup the scene content")?;
    }
    if has_root {
        app.edit(|doc| doc.delete(prefix))
            .map_err(|_| "effect_apply: could not delete the generated elements")?;
    }
    Ok(format!(
        "{{\"ok\":true,\"operation\":\"effect_apply\",\"removed\":true,\"idPrefix\":\"{prefix}\",\"tracks\":{}}}",
        stale.len()
    ))
}
