//! Seeded synthetic SVG scenes. Same (profile, n) always yields the same bytes.

use std::fmt::Write;

/// Bumped when generator output changes, so cached scene files are not reused.
pub const SCENE_VERSION: &str = "v1";
pub const CANVAS: u32 = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Fill,
    Lin,
    Rad,
    Stroke,
    Nest,
    Opac,
    Clip,
    Blur,
    Text,
    Mix,
}

pub const PROFILES: [Profile; 10] = [
    Profile::Fill,
    Profile::Lin,
    Profile::Rad,
    Profile::Stroke,
    Profile::Nest,
    Profile::Opac,
    Profile::Clip,
    Profile::Blur,
    Profile::Text,
    Profile::Mix,
];

impl Profile {
    pub fn name(self) -> &'static str {
        match self {
            Profile::Fill => "fill",
            Profile::Lin => "lin",
            Profile::Rad => "rad",
            Profile::Stroke => "stroke",
            Profile::Nest => "nest",
            Profile::Opac => "opac",
            Profile::Clip => "clip",
            Profile::Blur => "blur",
            Profile::Text => "text",
            Profile::Mix => "mix",
        }
    }
    pub fn parse(name: &str) -> Option<Profile> {
        PROFILES.into_iter().find(|p| p.name() == name)
    }
}

/// 64-bit linear congruential generator (Knuth MMIX constants).
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Lcg(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4294967296.0
    }
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n.max(1)
    }
    fn color(&mut self) -> String {
        format!("#{:06x}", self.next_u32() & 0xff_ffff)
    }
}

fn seed_for(profile: Profile, n: usize) -> u64 {
    (profile as u64 + 1) * 1_000_003 + n as u64
}

#[derive(Clone, Copy)]
enum Paint {
    Flat,
    Lin,
    Rad,
    Stroke,
    Clip,
    Blur,
    Text,
}

/// One leaf element with a stable `s{i}` id.
fn leaf(out: &mut String, rng: &mut Lcg, i: usize, paint: Paint, defs: usize) {
    let c = f64::from(CANVAS);
    let (x, y) = (rng.range(0.0, c - 40.0), rng.range(0.0, c - 40.0));
    let size = rng.range(6.0, 48.0);
    let attrs = match paint {
        Paint::Flat => format!(
            "fill=\"{}\" fill-opacity=\"{:.2}\"",
            rng.color(),
            rng.range(0.4, 1.0)
        ),
        Paint::Lin => format!("fill=\"url(#lg{})\"", rng.below(defs as u32)),
        Paint::Rad => format!("fill=\"url(#rg{})\"", rng.below(defs as u32)),
        Paint::Stroke => {
            let join = ["round", "miter", "bevel"][rng.below(3) as usize];
            let cap = ["butt", "round", "square"][rng.below(3) as usize];
            let dash = if rng.below(3) == 0 {
                format!(
                    " stroke-dasharray=\"{:.1} {:.1}\"",
                    rng.range(2.0, 12.0),
                    rng.range(2.0, 9.0)
                )
            } else {
                String::new()
            };
            format!(
                "fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linejoin=\"{join}\" \
                 stroke-linecap=\"{cap}\" stroke-miterlimit=\"4\"{dash}",
                rng.color(),
                rng.range(1.0, 40.0)
            )
        }
        Paint::Clip => format!(
            "fill=\"{}\" clip-path=\"url(#c{})\"",
            rng.color(),
            rng.below(defs as u32)
        ),
        Paint::Blur => {
            let blurred = i % 25 == 0;
            let filter = if blurred {
                format!(" filter=\"url(#f{})\"", rng.below(defs as u32))
            } else {
                String::new()
            };
            format!("fill=\"{}\"{filter}", rng.color())
        }
        Paint::Text => format!("fill=\"{}\"", rng.color()),
    };
    if let Paint::Text = paint {
        let _ = writeln!(
            out,
            "<text id=\"s{i}\" x=\"{x:.1}\" y=\"{:.1}\" font-size=\"{:.1}\" \
             font-family=\"Helvetica\" {attrs}>w{}</text>",
            y + size,
            rng.range(8.0, 28.0),
            rng.below(1000)
        );
        return;
    }
    let kind = rng.below(5);
    let _ = match kind {
        0 => writeln!(
            out,
            "<rect id=\"s{i}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{size:.1}\" height=\"{:.1}\" \
             rx=\"{:.1}\" {attrs}/>",
            size * rng.range(0.5, 1.5),
            rng.range(0.0, 6.0)
        ),
        1 => writeln!(
            out,
            "<circle id=\"s{i}\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{:.1}\" {attrs}/>",
            size / 2.0
        ),
        2 => writeln!(
            out,
            "<ellipse id=\"s{i}\" cx=\"{x:.1}\" cy=\"{y:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" {attrs}/>",
            size / 2.0,
            size / 3.0
        ),
        3 => {
            let mut d = String::new();
            for k in 0..5 {
                let a = f64::from(k) * 1.2566 + rng.range(-0.3, 0.3);
                let r = size * rng.range(0.4, 0.8);
                let cmd = if k == 0 { 'M' } else { 'L' };
                let _ = write!(d, "{cmd}{:.1} {:.1} ", x + r * a.cos(), y + r * a.sin());
            }
            writeln!(out, "<path id=\"s{i}\" d=\"{}Z\" {attrs}/>", d)
        }
        _ => writeln!(
            out,
            "<path id=\"s{i}\" d=\"M{x:.1} {y:.1} C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1} \
             Q{:.1} {:.1} {x:.1} {y:.1} Z\" {attrs}/>",
            x + size,
            y - size / 2.0,
            x + size * 1.5,
            y + size,
            x + size / 2.0,
            y + size * 1.2,
            x - size / 2.0,
            y + size / 2.0
        ),
    };
}

fn gradient_defs(out: &mut String, rng: &mut Lcg, count: usize, radial: bool) {
    for k in 0..count {
        let (c0, c1, c2) = (rng.color(), rng.color(), rng.color());
        if radial {
            let _ = write!(
                out,
                "<radialGradient id=\"rg{k}\" cx=\"0.5\" cy=\"0.5\" r=\"0.5\" fx=\"{:.2}\" fy=\"{:.2}\">",
                rng.range(0.3, 0.7),
                rng.range(0.3, 0.7)
            );
        } else {
            let _ = write!(
                out,
                "<linearGradient id=\"lg{k}\" x1=\"0\" y1=\"0\" x2=\"{:.2}\" y2=\"{:.2}\">",
                rng.range(0.4, 1.0),
                rng.range(0.0, 1.0)
            );
        }
        let _ = write!(
            out,
            "<stop offset=\"0\" stop-color=\"{c0}\"/><stop offset=\"0.55\" stop-color=\"{c1}\" \
             stop-opacity=\"0.8\"/><stop offset=\"1\" stop-color=\"{c2}\"/>"
        );
        out.push_str(if radial {
            "</radialGradient>\n"
        } else {
            "</linearGradient>\n"
        });
    }
}

fn clip_defs(out: &mut String, rng: &mut Lcg, count: usize) {
    let c = f64::from(CANVAS);
    for k in 0..count {
        let (x, y, r) = (rng.range(0.0, c), rng.range(0.0, c), rng.range(30.0, 120.0));
        let shape = if k % 2 == 0 {
            format!("<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{r:.1}\"/>")
        } else {
            format!(
                "<path d=\"M{x:.1} {y:.1} l{r:.1} {:.1} l{:.1} {r:.1} z\"/>",
                r / 3.0,
                -r / 2.0
            )
        };
        let _ = writeln!(out, "<clipPath id=\"c{k}\">{shape}</clipPath>");
    }
}

fn filter_defs(out: &mut String, rng: &mut Lcg, count: usize) {
    for k in 0..count {
        let _ = writeln!(
            out,
            "<filter id=\"f{k}\"><feGaussianBlur stdDeviation=\"{:.1}\"/></filter>",
            rng.range(0.5, 2.5)
        );
    }
}

pub fn generate(profile: Profile, n: usize) -> String {
    let mut rng = Lcg::new(seed_for(profile, n));
    let c = CANVAS;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{c}\" height=\"{c}\" \
         viewBox=\"0 0 {c} {c}\">\n<rect id=\"bg\" width=\"{c}\" height=\"{c}\" fill=\"#ffffff\"/>\n"
    );
    let defs = n.clamp(1, 48);
    let mut body = String::new();
    let mut shared = String::new();
    match profile {
        Profile::Fill => (0..n).for_each(|i| leaf(&mut body, &mut rng, i, Paint::Flat, defs)),
        Profile::Lin => {
            gradient_defs(&mut shared, &mut rng, defs, false);
            (0..n).for_each(|i| leaf(&mut body, &mut rng, i, Paint::Lin, defs));
        }
        Profile::Rad => {
            gradient_defs(&mut shared, &mut rng, defs, true);
            (0..n).for_each(|i| leaf(&mut body, &mut rng, i, Paint::Rad, defs));
        }
        Profile::Stroke => (0..n).for_each(|i| leaf(&mut body, &mut rng, i, Paint::Stroke, defs)),
        Profile::Clip => {
            clip_defs(&mut shared, &mut rng, defs);
            (0..n).for_each(|i| leaf(&mut body, &mut rng, i, Paint::Clip, defs));
        }
        Profile::Blur => {
            filter_defs(&mut shared, &mut rng, defs.min(8));
            (0..n).for_each(|i| leaf(&mut body, &mut rng, i, Paint::Blur, defs.min(8)));
        }
        Profile::Text => (0..n).for_each(|i| leaf(&mut body, &mut rng, i, Paint::Text, defs)),
        Profile::Nest => nested(&mut body, &mut rng, n, defs),
        Profile::Opac => opacity_groups(&mut body, &mut rng, n, defs),
        Profile::Mix => {
            let d = defs.min(16);
            gradient_defs(&mut shared, &mut rng, d, false);
            gradient_defs(&mut shared, &mut rng, d, true);
            clip_defs(&mut shared, &mut rng, d);
            filter_defs(&mut shared, &mut rng, d.min(4));
            for i in 0..n {
                let paint = match i % 8 {
                    0 | 1 => Paint::Flat,
                    2 => Paint::Lin,
                    3 => Paint::Rad,
                    4 => Paint::Stroke,
                    5 => Paint::Clip,
                    6 if i % 16 == 6 => Paint::Text,
                    6 => Paint::Flat,
                    _ if i % 40 == 7 => Paint::Blur,
                    _ => Paint::Flat,
                };
                let count = if matches!(paint, Paint::Blur) {
                    d.min(4)
                } else {
                    d
                };
                if i % 24 == 11 {
                    let _ = writeln!(
                        body,
                        "<g id=\"g{i}\" opacity=\"{:.2}\">",
                        rng.range(0.3, 0.9)
                    );
                    leaf(&mut body, &mut rng, i, paint, count);
                    body.push_str("</g>\n");
                } else {
                    leaf(&mut body, &mut rng, i, paint, count);
                }
            }
        }
    }
    if !shared.is_empty() {
        out.push_str("<defs>\n");
        out.push_str(&shared);
        out.push_str("</defs>\n");
    }
    out.push_str(&body);
    out.push_str("</svg>\n");
    out
}

/// Chains of 8 nested groups with rotate/scale, 16 leaves at the innermost level.
fn nested(out: &mut String, rng: &mut Lcg, n: usize, defs: usize) {
    const DEPTH: usize = 8;
    const LEAVES: usize = 16;
    let mut i = 0;
    let mut chain = 0;
    while i < n {
        for d in 0..DEPTH {
            let _ = writeln!(
                out,
                "<g id=\"g{chain}_{d}\" transform=\"rotate({:.1} 256 256) scale({:.3})\">",
                rng.range(-9.0, 9.0),
                rng.range(0.97, 1.03)
            );
        }
        for _ in 0..LEAVES.min(n - i) {
            leaf(out, rng, i, Paint::Flat, defs);
            i += 1;
        }
        out.push_str(&"</g>\n".repeat(DEPTH));
        chain += 1;
    }
}

/// Opacity groups of 4 leaves each; every fifth group nests another one.
fn opacity_groups(out: &mut String, rng: &mut Lcg, n: usize, defs: usize) {
    let mut i = 0;
    let mut group = 0;
    while i < n {
        let nested = group % 5 == 4;
        let _ = writeln!(
            out,
            "<g id=\"g{group}\" opacity=\"{:.2}\">",
            rng.range(0.3, 0.9)
        );
        if nested {
            let _ = writeln!(
                out,
                "<g id=\"h{group}\" opacity=\"{:.2}\">",
                rng.range(0.4, 0.95)
            );
        }
        for _ in 0..4.min(n - i) {
            leaf(out, rng, i, Paint::Flat, defs);
            i += 1;
        }
        out.push_str(if nested { "</g>\n</g>\n" } else { "</g>\n" });
        group += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_sized() {
        for p in PROFILES {
            let a = generate(p, 40);
            assert_eq!(a, generate(p, 40), "{}", p.name());
            assert!(a.contains("id=\"s39\""), "{} lacks last leaf", p.name());
            assert!(!a.contains("id=\"s40\""));
        }
        assert_ne!(generate(Profile::Fill, 40), generate(Profile::Fill, 41));
    }

    #[test]
    fn lcg_is_stable() {
        let mut r = Lcg::new(1);
        let first = [r.next_u32(), r.next_u32()];
        let mut again = Lcg::new(1);
        assert_eq!(first, [again.next_u32(), again.next_u32()]);
        assert!((0.0..1.0).contains(&again.unit()));
    }
}
