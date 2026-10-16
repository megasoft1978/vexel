//! Deterministic Rust port of tests/svg/shape/paint_clip_generator.py.
//! The fixed SplitMix64 stream replaces Python's injected `random.Random` while
//! preserving each choice/range and the exact document template behavior.
pub struct SeededRng(u64);

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
    fn pick<'a>(&mut self, values: &'a [&'a str]) -> &'a str {
        values[(self.next() as usize) % values.len()]
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }
    fn uniform(&mut self, low: f64, high: f64) -> f64 {
        low + self.unit() * (high - low)
    }
    fn range(&mut self, low: u32, high: u32) -> u32 {
        low + (self.next() % u64::from(high - low)) as u32
    }
}

pub fn random_paint_clip_document(rng: &mut SeededRng, index: usize) -> String {
    let fraction = [0.25, 0.5, 0.75][index % 3];
    let gradient_kind = if index % 2 == 0 {
        "linearGradient"
    } else {
        "radialGradient"
    };
    let gradient_units = if (index / 2) % 2 == 0 {
        "objectBoundingBox"
    } else {
        "userSpaceOnUse"
    };
    let outer_units = if (index / 4) % 2 == 0 {
        "objectBoundingBox"
    } else {
        "userSpaceOnUse"
    };
    let inner_units = if outer_units == "objectBoundingBox" {
        "userSpaceOnUse"
    } else {
        "objectBoundingBox"
    };
    let coords = if gradient_kind == "linearGradient" {
        if gradient_units == "objectBoundingBox" {
            " x1=\"0\" y1=\"0\" x2=\"0.22\" y2=\"0\""
        } else {
            " x1=\"0\" y1=\"0\" x2=\"8\" y2=\"0\""
        }
    } else if gradient_units == "objectBoundingBox" {
        " cx=\"0.5\" cy=\"0.5\" r=\"0.25\""
    } else {
        " cx=\"20\" cy=\"16\" r=\"8\""
    };
    let opacity_a = rng.pick(&["0.25", "0.45", "0.7"]);
    let opacity_b = rng.pick(&["0.35", "0.6", "0.85"]);
    let gradient = format!(
        "<{gradient_kind} id=\"paint\" gradientUnits=\"{gradient_units}\"{coords}><stop offset=\"0\" stop-color=\"#e02020\" stop-opacity=\"{opacity_a}\"/><stop offset=\"0.55\" stop-color=\"#20a060\" stop-opacity=\"0.8\"/><stop offset=\"1\" stop-color=\"#2040e0\" stop-opacity=\"{opacity_b}\"/></{gradient_kind}>"
    );
    let clip_rotation = rng.uniform(-18., 18.);
    let clip_scale_x = rng.uniform(0.9, 1.1);
    let clip_scale_y = rng.uniform(0.9, 1.1);
    let clip_transform = format!(
        " transform=\"rotate({clip_rotation:.3} 20 16) scale({clip_scale_x:.3} {clip_scale_y:.3})\""
    );
    let outer_clip = if outer_units == "objectBoundingBox" {
        format!(
            "<clipPath id=\"outer\" clipPathUnits=\"objectBoundingBox\"><rect x=\"0.05\" y=\"0.1\" width=\"0.72\" height=\"0.8\"{clip_transform}/><circle cx=\"0.78\" cy=\"0.52\" r=\"0.2\"/></clipPath>"
        )
    } else {
        let edge = f64::from(rng.range(14, 25)) + fraction;
        format!(
            "<clipPath id=\"outer\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"2.5\" y=\"1.5\" width=\"{edge:.2}\" height=\"28.5\"{clip_transform}/><circle cx=\"31\" cy=\"16\" r=\"7\"/></clipPath>"
        )
    };
    let inner_clip = if inner_units == "objectBoundingBox" {
        format!(
            "<clipPath id=\"inner\" clipPathUnits=\"objectBoundingBox\"><rect x=\"0.22\" y=\"0.08\" width=\"0.72\" height=\"0.84\"{clip_transform}/></clipPath>"
        )
    } else {
        let edge = f64::from(rng.range(25, 36)) + fraction;
        format!(
            "<clipPath id=\"inner\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"1.5\" y=\"1.5\" width=\"{edge:.2}\" height=\"28.5\"{clip_transform}/></clipPath>"
        )
    };
    let fractional = format!(
        "<clipPath id=\"fractional\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"0\" y=\"0\" width=\"{:.2}\" height=\"32\"/></clipPath>",
        22. + fraction
    );
    let x = rng.uniform(1., 7.);
    let y = rng.uniform(1., 5.);
    let width = rng.uniform(25., 31.);
    let height = rng.uniform(20., 25.);
    let tx = rng.uniform(-1.5, 1.5);
    let ty = rng.uniform(-1., 1.);
    let angle = rng.uniform(-18., 18.);
    let sx = rng.uniform(0.85, 1.15);
    let sy = rng.uniform(0.85, 1.15);
    format!(
        "<svg width=\"40\" height=\"32\" viewBox=\"0 0 40 32\"><defs>{gradient}{outer_clip}{inner_clip}{fractional}</defs><rect x=\"4\" y=\"4\" width=\"28\" height=\"20\" fill=\"#503080\"/><g clip-path=\"url(#fractional)\"><g transform=\"translate({tx:.3} {ty:.3}) rotate({angle:.3}) scale({sx:.3} {sy:.3}) clip-path=\"url(#outer)\"><g clip-path=\"url(#inner)\"><rect x=\"{x:.3}\" y=\"{y:.3}\" width=\"{width:.3}\" height=\"{height:.3}\" fill=\"url(#paint)\"/></g></g></g></svg>"
    )
}
