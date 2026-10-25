//! Target presets of `effect_apply`: glow, chromatic_split, motion_blur_dir
//! (filtered `<use>` copies after the target) and smear (squash-stretch keys).

use super::mcp_fx_presets::{Ctx, Plan, root_window};
use super::mcp_fx_util::{Key, cos_unpaired, num, pulse, scalar, sin_unpaired, track, trs};

fn target<'a>(c: &'a Ctx) -> Result<&'a str, &'static str> {
    c.target
        .as_deref()
        .ok_or("effect_apply: this preset needs a target group")
}

pub(crate) fn glow(c: &Ctx) -> Result<Plan, &'static str> {
    let t = target(c)?;
    let blend = c.blend("plus-lighter")?;
    let inten = c.n("intensity", 0.8);
    let low = inten * (1.0 - c.n("pulse", 0.4));
    let radius = c.n("radius", 8.0);
    let period = c.n("period", 12.0) as u32;
    let m = format!(
        "<g id=\"{p}\"><defs><filter id=\"{p}-f\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur id=\"{p}-blur\" stdDeviation=\"{}\"/></filter></defs><use id=\"{p}-copy\" href=\"#{t}\" filter=\"url(#{p}-f)\" style=\"mix-blend-mode:{blend}\" opacity=\"{}\"/></g>",
        num(radius),
        num(inten),
        p = c.p,
    );
    let wave = pulse(c.s, c.e, low, inten, period)?;
    let keys = |f: &dyn Fn(f64) -> f64| -> Vec<Key> {
        wave.iter()
            .map(|(t, v)| (*t, scalar(f(*v)), Some("ease_in_out")))
            .collect()
    };
    let mut tracks = vec![
        track(
            c.id("opacity"),
            &c.id("copy"),
            "opacity",
            "scalar",
            "linear",
            scalar(inten),
            keys(&|v| v),
        )?,
        track(
            c.id("radius"),
            &c.id("blur"),
            "stdDeviation",
            "scalar",
            "linear",
            scalar(radius),
            keys(&|v| {
                if inten > 0.0 {
                    radius * (0.5 + 0.5 * v / inten)
                } else {
                    radius
                }
            }),
        )?,
    ];
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, &c.parent)
}

pub(crate) fn chromatic_split(c: &Ctx) -> Result<Plan, &'static str> {
    let t = target(c)?;
    let amount = c.n("amount", 6.0);
    let low = amount * (1.0 - c.n("pulse", 0.6));
    let a = c.n("angle", 0.0) * std::f64::consts::PI / 180.0;
    let (dx, dy) = (cos_unpaired(a), sin_unpaired(a));
    let filter = |id: &str, values: &str, off: &str, sign: f64| {
        format!(
            "<filter id=\"{p}-{id}\" x=\"-30%\" y=\"-30%\" width=\"160%\" height=\"160%\"><feColorMatrix type=\"matrix\" values=\"{values}\"/><feOffset id=\"{p}-{off}\" dx=\"{}\" dy=\"{}\"/></filter>",
            num(sign * amount * dx),
            num(sign * amount * dy),
            p = c.p
        )
    };
    let m = format!(
        "<g id=\"{p}\"><defs>{}{}</defs><use id=\"{p}-r\" href=\"#{t}\" filter=\"url(#{p}-fr)\" style=\"mix-blend-mode:screen\"/><use id=\"{p}-b\" href=\"#{t}\" filter=\"url(#{p}-fb)\" style=\"mix-blend-mode:screen\"/></g>",
        filter("fr", "1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0", "ro", 1.0),
        filter("fb", "0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 1 0", "bo", -1.0),
        p = c.p,
    );
    let wave = pulse(c.s, c.e, low, amount, c.n("period", 8.0) as u32)?;
    let mut tracks = Vec::new();
    for (off, sign) in [("ro", 1.0), ("bo", -1.0)] {
        for (prop, comp, name) in [("dx", dx, "x"), ("dy", dy, "y")] {
            let keys: Vec<Key> = wave
                .iter()
                .map(|(t, v)| (*t, scalar(sign * v * comp), Some("ease_in_out")))
                .collect();
            tracks.push(track(
                c.id(&format!("{off}-{name}")),
                &c.id(off),
                prop,
                "scalar",
                "linear",
                scalar(sign * amount * comp),
                keys,
            )?);
        }
    }
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, &c.parent)
}

pub(crate) fn motion_blur_dir(c: &Ctx) -> Result<Plan, &'static str> {
    let t = target(c)?;
    let copies = c.n("copies", 6.0) as u32;
    let amount = c.n("amount", 24.0);
    let inten = c.n("intensity", 0.6);
    let low = 1.0 - c.n("pulse", 0.5);
    let a = c.n("angle", 0.0) * std::f64::consts::PI / 180.0;
    let (dx, dy) = (cos_unpaired(a), sin_unpaired(a));
    // Softening along the dominant axis only (anisotropic sigma, at most 32).
    let (sx, sy) = (
        (amount * 0.15 * dx.abs()).min(32.0),
        (amount * 0.15 * dy.abs()).min(32.0),
    );
    let mut m = format!(
        "<g id=\"{p}\"><defs><filter id=\"{p}-f\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"{} {}\"/></filter></defs><g id=\"{p}-trail\" filter=\"url(#{p}-f)\">",
        num(sx),
        num(sy),
        p = c.p
    );
    let wave = pulse(c.s, c.e, low, 1.0, c.n("period", 12.0) as u32)?;
    let pose = |k: u32, m: f64| {
        let d = -(f64::from(k) / f64::from(copies)) * amount * m;
        trs([d * dx, d * dy], 0.0, [1.0, 1.0], [0.0, 0.0], 0.0)
    };
    let mut tracks = Vec::new();
    for k in 1..=copies {
        let op = inten * (1.0 - f64::from(k - 1) / f64::from(copies));
        m.push_str(&format!(
            "<use id=\"{p}-c{k}\" href=\"#{t}\" opacity=\"{}\"/>",
            num(op),
            p = c.p
        ));
        let keys: Vec<Key> = wave
            .iter()
            .map(|(tk, v)| (*tk, pose(k, *v), Some("ease_in_out")))
            .collect();
        tracks.push(track(
            c.id(&format!("c{k}-move")),
            &c.id(&format!("c{k}")),
            "transform",
            "trs",
            "linear",
            pose(k, 1.0),
            keys,
        )?);
    }
    m.push_str("</g></g>");
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, &c.parent)
}

pub(crate) fn smear(c: &Ctx) -> Result<Plan, &'static str> {
    let t = target(c)?;
    let stretch = c.n("stretch", 1.8);
    let attack = c.n("attack", ((c.e - c.s) / 3).max(1) as f64) as u32;
    let a = c.n("angle", 0.0) * std::f64::consts::PI / 180.0;
    let (ca, sa) = (cos_unpaired(a), sin_unpaired(a));
    let along = ca * ca;
    let (sx, sy) = (
        stretch.powf(2.0 * along - 1.0),
        stretch.powf(1.0 - 2.0 * along),
    );
    let skew = 20.0 * 2.0 * ca * sa * (1.0 - 1.0 / stretch);
    let peak = (c.s + attack).min(c.e);
    let pose = |on: bool| {
        if on {
            trs([0.0, 0.0], 0.0, [sx, sy], c.pivot, skew)
        } else {
            trs([0.0, 0.0], 0.0, [1.0, 1.0], c.pivot, 0.0)
        }
    };
    let mut keys: Vec<Key> = Vec::new();
    if c.s > 0 {
        keys.push((c.s - 1, pose(false), None));
    }
    keys.push((c.s, pose(false), Some("ease_out")));
    keys.push((peak, pose(true), Some("ease_in_out")));
    keys.push((c.e.max(peak + 1), pose(false), None));
    let mut tracks = vec![track(
        c.id("smear"),
        t,
        "transform",
        "trs",
        "linear",
        pose(false),
        keys,
    )?];
    let trail = c.n("trail", 0.0) as u32;
    if trail == 0 {
        return c.plan(String::new(), tracks, &c.parent);
    }
    let amount = c.n("amount", 12.0);
    let inten = c.n("intensity", 0.5);
    let mut m = format!("<g id=\"{}\">", c.p);
    for k in 1..=trail {
        let op = inten * (1.0 - f64::from(k - 1) / f64::from(trail));
        m.push_str(&format!(
            "<use id=\"{p}-c{k}\" href=\"#{t}\" opacity=\"{}\"/>",
            num(op),
            p = c.p
        ));
        let d = -f64::from(k) * amount;
        let at = |m: f64| trs([d * m * ca, d * m * sa], 0.0, [1.0, 1.0], [0.0, 0.0], 0.0);
        let mut keys: Vec<Key> = Vec::new();
        if c.s > 0 {
            keys.push((c.s - 1, at(0.0), None));
        }
        keys.push((c.s, at(0.0), Some("ease_out")));
        keys.push((peak, at(1.0), Some("ease_in_out")));
        keys.push((c.e.max(peak + 1), at(0.0), None));
        tracks.push(track(
            c.id(&format!("c{k}-move")),
            &c.id(&format!("c{k}")),
            "transform",
            "trs",
            "linear",
            at(0.0),
            keys,
        )?);
    }
    m.push_str("</g>");
    tracks.extend(root_window(c)?);
    c.plan(m, tracks, &c.parent)
}
