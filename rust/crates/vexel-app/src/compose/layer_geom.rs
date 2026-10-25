//! Small affine and pixel-rectangle helpers of the preview layer cache.

use super::layers::Geo;
use super::shapes::OwnedShape;
use vexel_geometry::Affine;

pub(super) const IDENTITY: Affine = Affine {
    a: 1.0,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    e: 0.0,
    f: 0.0,
};

/// `left` after `right` (points pass through `right` first), unchecked.
pub(super) fn mul(l: Affine, r: Affine) -> Affine {
    Affine {
        a: l.a * r.a + l.c * r.b,
        b: l.b * r.a + l.d * r.b,
        c: l.a * r.c + l.c * r.d,
        d: l.b * r.c + l.d * r.d,
        e: l.a * r.e + l.c * r.f + l.e,
        f: l.b * r.e + l.d * r.f + l.f,
    }
}

pub(super) fn det(m: Affine) -> f64 {
    m.a * m.d - m.b * m.c
}

/// Inverse, `None` for a (nearly) singular or non-finite matrix.
pub(super) fn inverse(m: Affine) -> Option<Affine> {
    let det = det(m);
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let (a, b, c, d) = (m.d / det, -m.b / det, -m.c / det, m.a / det);
    let inv = Affine {
        a,
        b,
        c,
        d,
        e: -(a * m.e + c * m.f),
        f: -(b * m.e + d * m.f),
    };
    [inv.a, inv.b, inv.c, inv.d, inv.e, inv.f]
        .iter()
        .all(|v| v.is_finite())
        .then_some(inv)
}

pub(super) fn apply(m: Affine, x: f64, y: f64) -> (f64, f64) {
    (m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f)
}

/// Whether two matrices act the same on a scene: linear parts agree to a
/// relative 1e-6 and translations to 1e-4 user units.
pub(super) fn near(p: Affine, q: Affine) -> bool {
    let lin = |a: f64, b: f64| (a - b).abs() <= 1e-6 * (1.0 + a.abs().max(b.abs()));
    lin(p.a, q.a)
        && lin(p.b, q.b)
        && lin(p.c, q.c)
        && lin(p.d, q.d)
        && (p.e - q.e).abs() <= 1e-4
        && (p.f - q.f).abs() <= 1e-4
}

/// User space to device pixels of the playback view.
pub(super) fn user_to_device(geo: &Geo) -> Affine {
    Affine {
        a: geo.scale_x,
        b: 0.0,
        c: 0.0,
        d: geo.scale_y,
        e: geo.offset_x - geo.view.x * geo.scale_x,
        f: geo.offset_y - geo.view.y * geo.scale_y,
    }
}

/// A user-space motion `d` expressed in device pixels.
pub(super) fn device_motion(geo: &Geo, d: Affine) -> Option<Affine> {
    let s = user_to_device(geo);
    Some(mul(s, mul(d, inverse(s)?)))
}

/// Singular values `(large, small)` of the linear part.
pub(super) fn singular_values(m: Affine) -> (f64, f64) {
    let (p, q, r) = (
        m.a * m.a + m.b * m.b,
        m.a * m.c + m.b * m.d,
        m.c * m.c + m.d * m.d,
    );
    let mean = (p + r) / 2.0;
    let radius = (((p - r) / 2.0).powi(2) + q * q).sqrt();
    (
        (mean + radius).max(0.0).sqrt(),
        (mean - radius).max(0.0).sqrt(),
    )
}

/// Signed pixel rectangle, `x1`/`y1` exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub const EMPTY: Self = Self {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
    };

    pub fn width(self) -> usize {
        (self.x1 - self.x0).max(0) as usize
    }

    pub fn height(self) -> usize {
        (self.y1 - self.y0).max(0) as usize
    }

    pub fn is_empty(self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    pub fn intersect(self, o: Self) -> Self {
        Self {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    pub fn union(self, o: Self) -> Self {
        if self.is_empty() {
            return o;
        }
        if o.is_empty() {
            return self;
        }
        Self {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    pub fn contains(self, o: Self) -> bool {
        o.is_empty() || (self.x0 <= o.x0 && self.y0 <= o.y0 && o.x1 <= self.x1 && o.y1 <= self.y1)
    }

    pub fn grow(self, dx: i32, dy: i32) -> Self {
        Self {
            x0: self.x0 - dx,
            y0: self.y0 - dy,
            x1: self.x1 + dx,
            y1: self.y1 + dy,
        }
    }

    /// Smallest rectangle covering the image of `self` under `m`.
    pub fn map(self, m: Affine) -> Option<Self> {
        let corners = [
            (self.x0, self.y0),
            (self.x1, self.y0),
            (self.x0, self.y1),
            (self.x1, self.y1),
        ];
        let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (x, y) in corners {
            let (px, py) = apply(m, f64::from(x), f64::from(y));
            l = l.min(px);
            r = r.max(px);
            t = t.min(py);
            b = b.max(py);
        }
        from_floats(l, t, r, b)
    }
}

/// Rectangle around the float box, `None` when not finite or absurdly large.
pub(super) fn from_floats(l: f64, t: f64, r: f64, b: f64) -> Option<IRect> {
    const LIMIT: f64 = 1.0e7;
    if ![l, t, r, b]
        .iter()
        .all(|v| v.is_finite() && v.abs() < LIMIT)
    {
        return None;
    }
    Some(IRect {
        x0: l.floor() as i32,
        y0: t.floor() as i32,
        x1: r.ceil() as i32,
        y1: b.ceil() as i32,
    })
}

/// Conservative device pixel rectangle of `shape`, not clamped to the tile
/// (the same reach as `diff::device_rect`).
pub(super) fn shape_bounds(shape: &OwnedShape, geo: &Geo) -> Option<IRect> {
    if shape.edges.is_empty() {
        return None;
    }
    let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for e in &shape.edges {
        for (x, y) in [(e.x0, e.y0), (e.x1, e.y1)] {
            let (dx, dy) = geo.to_device(x, y);
            l = l.min(dx);
            r = r.max(dx);
            t = t.min(dy);
            b = b.max(dy);
        }
    }
    let mut pad = 2.0;
    if let Some(style) = shape.stroke {
        let tr = shape.transform;
        let norm = (tr.a * tr.a + tr.b * tr.b + tr.c * tr.c + tr.d * tr.d).sqrt();
        let reach = style.width.abs() / 2.0 * style.miter_limit.max(2.0) * norm;
        pad += reach * geo.scale_x.abs().max(geo.scale_y.abs());
    }
    from_floats(l - pad, t - pad, r + pad, b + pad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_round_trips() {
        let m = Affine {
            a: 1.1,
            b: 0.2,
            c: -0.3,
            d: 0.9,
            e: 14.0,
            f: -7.5,
        };
        let p = mul(m, inverse(m).unwrap());
        assert!(near(p, IDENTITY));
        assert!(inverse(Affine { a: 0.0, ..m }).is_some());
        assert!(
            inverse(Affine {
                a: 1.0,
                b: 2.0,
                c: 2.0,
                d: 4.0,
                e: 0.0,
                f: 0.0
            })
            .is_none()
        );
    }

    #[test]
    fn singular_values_of_scale_and_rotation() {
        let (hi, lo) = singular_values(Affine {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 0.5,
            e: 0.0,
            f: 0.0,
        });
        assert!((hi - 2.0).abs() < 1e-12 && (lo - 0.5).abs() < 1e-12);
        let (c, s) = (0.3f64.cos(), 0.3f64.sin());
        let (hi, lo) = singular_values(Affine {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 3.0,
            f: 1.0,
        });
        assert!((hi - 1.0).abs() < 1e-12 && (lo - 1.0).abs() < 1e-12);
    }
}
