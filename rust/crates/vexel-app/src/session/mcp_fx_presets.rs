//! Overlay presets of `effect_apply`: speed_lines, film_grain, halftone,
//! vhs_scanlines, vignette, flash. Each returns fragment markup plus the
//! tracks that animate it; the caller applies both in one undoable edit.

use super::mcp_fx_spec::FxSpec;
use super::mcp_fx_util::{
    FxTrack, Key, color_ok, cos_unpaired, flag, mix, num, pulse, scalar, sin_unpaired, track, trs,
    unit,
};

/// What one preset needs to know about the call and the scene.
pub(crate) struct Ctx<'a> {
    pub p: &'a str,
    pub spec: &'a FxSpec,
    /// First and last active tick (`end` < duration - 1 or the last tick).
    pub s: u32,
    pub e: u32,
    pub dur: u32,
    /// x, y, width, height of the area an overlay covers.
    pub region: (f64, f64, f64, f64),
    /// Element the effect acts on (filter presets, smear) or restricts to.
    pub target: Option<String>,
    pub parent: String,
    pub pivot: [f64; 2],
}

pub(crate) struct Plan {
    pub markup: String,
    pub tracks: Vec<FxTrack>,
    pub parent: String,
}

impl Ctx<'_> {
    pub fn n(&self, name: &str, default: f64) -> f64 {
        self.spec
            .nums
            .iter()
            .find(|(k, _)| *k == name)
            .map_or(default, |(_, v)| *v)
    }
    pub fn id(&self, suffix: &str) -> String {
        format!("{}-{suffix}", self.p)
    }
    pub fn color(&self, default: &'static str) -> Result<String, &'static str> {
        match &self.spec.color {
            Some(c) if color_ok(c) => Ok(c.clone()),
            Some(_) => Err("effect_apply: color must be #rgb or #rrggbb"),
            None => Ok(default.to_owned()),
        }
    }
    pub fn blend(&self, default: &'static str) -> Result<&'static str, &'static str> {
        match self.spec.blend.as_deref() {
            None => Ok(default),
            Some("normal") => Ok("normal"),
            Some("screen") => Ok("screen"),
            Some("plus-lighter") => Ok("plus-lighter"),
            Some(_) => Err("effect_apply: blend must be normal, screen or plus-lighter"),
        }
    }
    pub fn plan(
        &self,
        markup: String,
        tracks: Vec<FxTrack>,
        parent: &str,
    ) -> Result<Plan, &'static str> {
        let elements = markup.matches('<').count() - markup.matches("</").count();
        if elements > super::mcp_fx_util::MAX_ELEMENTS {
            return Err(
                "effect_apply: the preset would generate more than 256 elements; lower count or frames",
            );
        }
        Ok(Plan {
            markup,
            tracks,
            parent: parent.to_owned(),
        })
    }
}

/// Root group start, with a clip to the region when a target limits it.
fn open(c: &Ctx, attrs: &str) -> String {
    let (x, y, w, h) = c.region;
    if c.target.is_some() {
        format!(
            "<g id=\"{p}\"{attrs} clip-path=\"url(#{p}-clip)\"><clipPath id=\"{p}-clip\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
            num(x),
            num(y),
            num(w),
            num(h),
            p = c.p
        )
    } else {
        format!("<g id=\"{}\"{attrs}>", c.p)
    }
}

/// Visibility of the root group: shown from `s` to `e` only.
pub(crate) fn root_window(c: &Ctx) -> Result<Option<FxTrack>, &'static str> {
    if c.s == 0 && c.e + 1 >= c.dur {
        return Ok(None);
    }
    let mut keys: Vec<Key> = vec![(c.s, flag(true), None)];
    if c.e + 1 < c.dur {
        keys.push((c.e + 1, flag(false), None));
    }
    Ok(Some(track(
        c.id("window"),
        c.p,
        "visibility",
        "boolean",
        "step",
        flag(c.s == 0),
        keys,
    )?))
}

/// Step visibility tracks that show one of `ids` at a time, `period` ticks each.
pub(crate) fn cycle(c: &Ctx, ids: &[String], period: u32) -> Result<Vec<FxTrack>, &'static str> {
    let mut out = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let mut keys: Vec<Key> = Vec::new();
        let (mut t, mut k) = (c.s, 0usize);
        while t <= c.e {
            keys.push((t, flag(k % ids.len() == i), None));
            if keys.len() > super::mcp_fx_util::MAX_TRACK_KEYS {
                return Err(
                    "effect_apply: too many keys for one track; raise period or shorten ticks",
                );
            }
            t += period;
            k += 1;
        }
        if c.e + 1 < c.dur {
            keys.push((c.e + 1, flag(false), None));
        }
        out.push(track(
            format!("{id}-vis"),
            id,
            "visibility",
            "boolean",
            "step",
            flag(false),
            keys,
        )?);
    }
    Ok(out)
}

pub(crate) fn speed_lines(c: &Ctx) -> Result<Plan, &'static str> {
    const VARIANTS: u32 = 3;
    let (x, y, w, h) = c.region;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let diag = (w * w + h * h).sqrt() / 2.0;
    let count = c.n("count", 24.0) as u32;
    let len = c.n("length", diag * 0.35);
    let seed = c.n("seed", 1.0) as u32;
    let angle = c.n("angle", 0.0);
    let period = c.n("period", 2.0) as u32;
    let color = c.color("#ffffff")?;
    let parallel = match c.spec.mode.as_deref() {
        None | Some("radial") => false,
        Some("parallel") => true,
        Some(_) => return Err("effect_apply: mode must be radial or parallel"),
    };
    let mut m = open(c, &format!(" opacity=\"{}\"", num(c.n("intensity", 0.8))));
    let mut ids = Vec::new();
    for v in 0..VARIANTS {
        let id = c.id(&format!("v{v}"));
        let rot = if parallel {
            format!(
                " transform=\"rotate({} {} {})\"",
                num(angle),
                num(cx),
                num(cy)
            )
        } else {
            String::new()
        };
        m.push_str(&format!("<g id=\"{id}\"{rot}>"));
        for i in 0..count {
            let key = v * 1000 + i;
            if parallel {
                let l = len * (0.5 + unit(seed, key, 2));
                let lx = cx - diag + unit(seed, key, 1) * (2.0 * diag - l).max(0.0);
                let ly = cy - diag + unit(seed, key, 4) * 2.0 * diag;
                let th = 1.5 + unit(seed, key, 3) * 3.0;
                m.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\"/>",
                    num(lx),
                    num(ly),
                    num(l),
                    num(th)
                ));
            } else {
                let a = (f64::from(i) + (unit(seed, key, 1) - 0.5) * 0.8) * std::f64::consts::TAU
                    / f64::from(count);
                let (ca, sa) = (cos_unpaired(a), sin_unpaired(a));
                let r1 = diag * 1.1;
                let r0 = (r1 - len * (0.5 + unit(seed, key, 2))).max(diag * 0.1);
                let wd = 2.0 + unit(seed, key, 3) * (diag * 0.02 + 3.0);
                m.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{}\" fill=\"{color}\"/>",
                    num(cx + r0 * ca),
                    num(cy + r0 * sa),
                    num(cx + r1 * ca - wd * sa),
                    num(cy + r1 * sa + wd * ca),
                    num(cx + r1 * ca + wd * sa),
                    num(cy + r1 * sa - wd * ca)
                ));
            }
        }
        m.push_str("</g>");
        ids.push(id);
    }
    m.push_str("</g>");
    let tracks = cycle(c, &ids, period)?;
    c.plan(m, tracks, "root")
}

/// Grain frames: `<defs>` filters plus one group per frame; returns the ids.
fn grain(
    c: &Ctx,
    tag: &str,
    frames: u32,
    seed: u32,
    opacity: f64,
    freq: f64,
    oct: u32,
) -> (String, Vec<String>) {
    let (x, y, w, h) = c.region;
    let mut defs = String::from("<defs>");
    let mut groups = String::new();
    let mut ids = Vec::new();
    for i in 0..frames {
        let fid = c.id(&format!("{tag}{i}"));
        let s = 1 + mix(seed, i, 7) % 9999;
        defs.push_str(&format!(
            "<filter id=\"{fid}-f\" x=\"0\" y=\"0\" width=\"1\" height=\"1\"><feTurbulence type=\"fractalNoise\" baseFrequency=\"{}\" numOctaves=\"{oct}\" seed=\"{s}\"/><feColorMatrix type=\"matrix\" values=\"1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 0 1\"/></filter>",
            num(freq)
        ));
        groups.push_str(&format!(
            "<g id=\"{fid}\" style=\"mix-blend-mode:soft-light\" opacity=\"{}\" filter=\"url(#{fid}-f)\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#fff\"/></g>",
            num(opacity),
            num(x),
            num(y),
            num(w),
            num(h)
        ));
        ids.push(fid);
    }
    defs.push_str("</defs>");
    let m = defs + &groups;
    (m, ids)
}

pub(crate) fn film_grain(c: &Ctx) -> Result<Plan, &'static str> {
    let (defs, ids) = grain(
        c,
        "f",
        c.n("frames", 3.0) as u32,
        c.n("seed", 1.0) as u32,
        c.n("intensity", 0.4),
        c.n("frequency", 0.85),
        c.n("octaves", 2.0) as u32,
    );
    let m = format!("{}{defs}</g>", open(c, ""));
    let mut tracks = cycle(c, &ids, c.n("period", 2.0) as u32)?;
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, "root")
}

pub(crate) fn halftone(c: &Ctx) -> Result<Plan, &'static str> {
    let (x, y, w, h) = c.region;
    let size = c.n("size", 8.0);
    let (d0, d1) = (c.n("dot", 0.25), c.n("dotTo", 0.7));
    let color = c.color("#000000")?;
    let m = format!(
        "{}<defs><pattern id=\"{p}-pat\" x=\"0\" y=\"0\" width=\"{sz}\" height=\"{sz}\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate({})\"><circle id=\"{p}-dot\" cx=\"{half}\" cy=\"{half}\" r=\"{}\" fill=\"{color}\"/></pattern></defs><rect id=\"{p}-rect\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"url(#{p}-pat)\" opacity=\"{}\" style=\"mix-blend-mode:multiply\"/></g>",
        open(c, ""),
        num(c.n("angle", 45.0)),
        num(d0 * size / 2.0),
        num(x),
        num(y),
        num(w),
        num(h),
        num(c.n("intensity", 0.8)),
        p = c.p,
        sz = num(size),
        half = num(size / 2.0),
    );
    let r = |d: f64| scalar(d * size / 2.0);
    let mut tracks = vec![track(
        c.id("dot-r"),
        &c.id("dot"),
        "r",
        "scalar",
        "linear",
        r(d0),
        vec![
            (c.s, r(d0), Some("ease_in_out")),
            (c.e.max(c.s + 1), r(d1), None),
        ],
    )?];
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, "root")
}

pub(crate) fn vhs_scanlines(c: &Ctx) -> Result<Plan, &'static str> {
    let (x, y, w, h) = c.region;
    let pitch = c.n("pitch", 4.0);
    let seed = c.n("seed", 1.0) as u32;
    let jitter = c.n("jitter", 2.0);
    let noise = c.n("noise", 0.25);
    let period = c.n("period", 2.0) as u32;
    let (p, bar) = (c.p, h / 8.0);
    let mut m = format!(
        "{}<defs><pattern id=\"{p}-pat\" x=\"0\" y=\"0\" width=\"8\" height=\"{}\" patternUnits=\"userSpaceOnUse\"><rect width=\"8\" height=\"{}\" fill=\"#000\"/></pattern></defs><g id=\"{p}-lines\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"url(#{p}-pat)\" opacity=\"{}\"/></g><g id=\"{p}-bar\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#fff\" opacity=\"0.12\"/></g>",
        open(c, ""),
        num(pitch),
        num(pitch / 2.0),
        num(x),
        num(y),
        num(w),
        num(h),
        num(c.n("intensity", 0.35)),
        num(x),
        num(y - bar),
        num(w),
        num(bar),
    );
    let mut tracks = Vec::new();
    let at = |tx: f64, ty: f64| trs([tx, ty], 0.0, [1.0, 1.0], [0.0, 0.0], 0.0);
    if jitter > 0.0 {
        let mut keys: Vec<Key> = Vec::new();
        let (mut t, mut k) = (c.s, 0u32);
        while t <= c.e {
            let dx = (unit(seed, k, 1) - 0.5) * 2.0 * jitter;
            let dy = (unit(seed, k, 2) - 0.5) * jitter / 2.0;
            keys.push((t, at(dx, dy), None));
            t += period;
            k += 1;
        }
        tracks.push(track(
            c.id("jitter"),
            &c.id("lines"),
            "transform",
            "trs",
            "step",
            at(0.0, 0.0),
            keys,
        )?);
    }
    let cycle_len = (period * 12).max(24);
    let mut keys: Vec<Key> = Vec::new();
    let mut t = c.s;
    while t <= c.e {
        keys.push((t, at(0.0, 0.0), None));
        keys.push((t + cycle_len - 1, at(0.0, h + bar), None));
        t += cycle_len;
    }
    tracks.push(track(
        c.id("roll"),
        &c.id("bar"),
        "transform",
        "trs",
        "linear",
        at(0.0, 0.0),
        keys,
    )?);
    if noise > 0.0 {
        let (defs, ids) = grain(c, "n", 2, seed, noise * 0.5, 0.9, 1);
        m.push_str(&defs);
        tracks.extend(cycle(c, &ids, period)?);
    }
    m.push_str("</g>");
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, "root")
}

pub(crate) fn vignette(c: &Ctx) -> Result<Plan, &'static str> {
    let (x, y, w, h) = c.region;
    let inten = c.n("intensity", 0.7);
    let low = inten * (1.0 - c.n("pulse", 0.3));
    let color = c.color("#000000")?;
    let m = format!(
        "{}<defs><radialGradient id=\"{p}-g\" cx=\"0.5\" cy=\"0.5\" r=\"0.75\"><stop id=\"{p}-s0\" offset=\"{}\" stop-color=\"{color}\" stop-opacity=\"0\"/><stop id=\"{p}-s1\" offset=\"1\" stop-color=\"{color}\" stop-opacity=\"{}\"/></radialGradient></defs><rect id=\"{p}-rect\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"url(#{p}-g)\"/></g>",
        open(c, ""),
        num(c.n("softness", 0.35)),
        num(inten),
        num(x),
        num(y),
        num(w),
        num(h),
        p = c.p,
    );
    let keys: Vec<Key> = pulse(c.s, c.e, low, inten, c.n("period", 24.0) as u32)?
        .into_iter()
        .map(|(t, v)| (t, scalar(v), Some("ease_in_out")))
        .collect();
    let mut tracks = vec![track(
        c.id("edge"),
        &c.id("s1"),
        "stop-opacity",
        "scalar",
        "linear",
        scalar(inten),
        keys,
    )?];
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, "root")
}

pub(crate) fn flash(c: &Ctx) -> Result<Plan, &'static str> {
    let (x, y, w, h) = c.region;
    let inten = c.n("intensity", 1.0);
    let attack = c.n("attack", 1.0) as u32;
    let fall = c.n("falloff", 6.0) as u32;
    let color = c.color("#ffffff")?;
    let blend = c.blend("normal")?;
    let peak = c.s + attack;
    let end = peak + fall;
    if end >= c.dur {
        return Err(
            "effect_apply: flash start + attack + falloff must be below the timeline duration (lengthen it with native_timeline_set_duration)",
        );
    }
    let m = format!(
        "{}<rect id=\"{p}-rect\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\" opacity=\"0\" style=\"mix-blend-mode:{blend}\"/></g>",
        open(c, ""),
        num(x),
        num(y),
        num(w),
        num(h),
        p = c.p,
    );
    let mut keys: Vec<Key> = Vec::new();
    if c.s > 0 {
        keys.push((c.s - 1, scalar(0.0), None));
    }
    keys.push((c.s, scalar(0.0), Some("ease_out")));
    keys.push((peak, scalar(inten), Some("ease_out_cubic")));
    keys.push((end, scalar(0.0), None));
    let tracks = vec![track(
        c.id("flash"),
        &c.id("rect"),
        "opacity",
        "scalar",
        "linear",
        scalar(0.0),
        keys,
    )?];
    c.plan(m, tracks, "root")
}
