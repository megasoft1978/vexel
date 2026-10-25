//! Exact colour table for gradients whose maps do not mix x and y.
//!
//! When both inverse maps are diagonal, the scalar the sampler turns into the
//! stop parameter (`key`: linear numerator, concentric radial squared
//! distance) is `x_term(x) + y_term(y)` with the very operations
//! [`PreparedPaint::sample`] performs, so a scan evaluates each column and
//! row term once. The key maps to `t` and on to `amount` by correctly rounded
//! operations, which are monotone, and each colour channel is a linear blend
//! of `amount`. Cells of the key axis whose two edges shade to the same
//! colour of one stop segment, each channel's blend at least `1e-9` from a
//! rounding boundary, hold only that colour (floating error is far below the
//! margin). Keys past the edges that reach `t <= first offset` or
//! `t >= last offset` take the pad colours. Every other key is left to
//! [`PreparedPaint::sample`], so lookups never differ from sampling.
use crate::paint::{PreparedPaint, radial_t_of_key};
use vexel_scene::{PaintKind, Spread};

const CELLS: usize = 4096;

#[derive(Debug, Clone)]
pub struct KeyTable<'a> {
    paint: PreparedPaint<'a>,
    linear: bool,
    below: [u8; 4],
    above: [u8; 4],
    edges: Vec<f64>,
    cells: Vec<[u8; 4]>,
    uniform: Vec<bool>,
    /// Last cell of the run of uniform cells of one colour that starts at each cell.
    run_end: Vec<u32>,
    inverse_width: f64,
}

impl<'a> PreparedPaint<'a> {
    /// A table for this paint at every point within `bound` of the origin,
    /// when the maps are diagonal and sampling is a pure function of the key.
    pub fn key_table(&self, bound: f64) -> Option<KeyTable<'a>> {
        let paint = self.paint;
        let diagonal = |m: vexel_geometry::Affine| m.b == 0.0 && m.c == 0.0;
        if paint.spread != Spread::Pad
            || !diagonal(paint.user_inverse)
            || !diagonal(paint.gradient_inverse)
            || !self.always_placed(bound)
        {
            return None;
        }
        let linear = paint.kind == PaintKind::Linear;
        let length = self.constants[2];
        let (first, last) = (
            self.section[0].offset,
            self.section[self.section.len() - 1].offset,
        );
        let t_of = |key: f64| {
            if linear {
                key / length
            } else {
                radial_t_of_key(self.constants[2], key)
            }
        };
        let (k0, k1) = if linear {
            if length == 0.0 {
                return None;
            }
            let mut low = first * length;
            let mut step = low.abs().max(1.0) * 1.0e-9;
            for _ in 0..200 {
                if t_of(low) <= first {
                    break;
                }
                low -= step;
                step *= 2.0;
            }
            let mut high = last * length;
            let mut step = high.abs().max(1.0) * 1.0e-9;
            for _ in 0..200 {
                if t_of(high) >= last {
                    break;
                }
                high += step;
                step *= 2.0;
            }
            (low, high)
        } else {
            let mut high = last * last * -self.constants[2];
            for _ in 0..400 {
                if t_of(high) >= last {
                    break;
                }
                high = high * (1.0 + 1.0e-9) + f64::MIN_POSITIVE;
            }
            (0.0, high)
        };
        if !(t_of(k0) <= first && t_of(k1) >= last && k1 > k0 && (k1 - k0).is_finite()) {
            return None;
        }
        let mut edges: Vec<f64> = (0..=CELLS)
            .map(|i| k0 + (k1 - k0) * (i as f64 / CELLS as f64))
            .collect();
        edges[0] = k0;
        edges[CELLS] = k1;
        let shaded: Vec<Option<(usize, [u8; 4], bool)>> = edges
            .iter()
            .map(|&key| {
                let t = t_of(key);
                t.is_finite().then(|| self.shade(t))
            })
            .collect();
        let mut cells = vec![[0u8; 4]; CELLS];
        let mut uniform = vec![false; CELLS];
        for i in 0..CELLS {
            if edges[i] > edges[i + 1] {
                return None;
            }
            if let (Some((sa, ca, true)), Some((sb, cb, true))) = (shaded[i], shaded[i + 1])
                && sa == sb
                && ca == cb
            {
                cells[i] = ca;
                uniform[i] = true;
            }
        }
        let mut run_end = vec![0u32; CELLS];
        for i in (0..CELLS).rev() {
            run_end[i] = if uniform[i] && i + 1 < CELLS && uniform[i + 1] && cells[i + 1] == cells[i] {
                run_end[i + 1]
            } else {
                i as u32
            };
        }
        Some(KeyTable {
            paint: *self,
            linear,
            below: self.section[0].rgba,
            above: self.section[self.section.len() - 1].rgba,
            inverse_width: CELLS as f64 / (k1 - k0),
            edges,
            cells,
            uniform,
            run_end,
        })
    }

    /// Stop segment of `t` (0 and past the last are the pad ends), its colour
    /// as [`Self::sample`] computes it, and whether every channel's blend is
    /// clear of a rounding boundary.
    fn shade(&self, t: f64) -> (usize, [u8; 4], bool) {
        let section = self.section;
        if t <= section[0].offset {
            return (0, section[0].rgba, true);
        }
        let last = section.len() - 1;
        if t >= section[last].offset {
            return (last + 1, section[last].rgba, true);
        }
        let mut right = 1;
        while right < last && t > section[right].offset {
            right += 1;
        }
        let left = right - 1;
        let range = section[right].offset - section[left].offset;
        let amount = if range <= 0.0 {
            1.0
        } else {
            (t - section[left].offset) / range
        };
        let (mut output, mut safe) = ([0; 4], true);
        for channel in 0..4 {
            let blend = (1.0 - amount) * f64::from(section[left].rgba[channel])
                + amount * f64::from(section[right].rgba[channel]);
            safe &= (blend - blend.floor() - 0.5).abs() > 1.0e-9;
            output[channel] = blend.round().clamp(0.0, 255.0) as u8;
        }
        (right, output, safe)
    }
}

impl KeyTable<'_> {
    /// The key part that depends on the user-space column `x`.
    pub fn x_term(&self, x: f64) -> f64 {
        let paint = self.paint.paint;
        let mut gx = paint.user_inverse.a * x + paint.user_inverse.e;
        if let Some((w, _)) = self.paint.bbox {
            gx = (gx - paint.bounds[0]) / w;
        }
        gx = paint.gradient_inverse.a * gx + paint.gradient_inverse.e;
        if self.linear {
            (gx - paint.coordinates[0]) * self.paint.constants[0]
        } else {
            let p = gx - paint.coordinates[3];
            p * p
        }
    }

    /// The key part that depends on the user-space row `y`.
    pub fn y_term(&self, y: f64) -> f64 {
        let paint = self.paint.paint;
        let mut gy = paint.user_inverse.d * y + paint.user_inverse.f;
        if let Some((_, h)) = self.paint.bbox {
            gy = (gy - paint.bounds[1]) / h;
        }
        gy = paint.gradient_inverse.d * gy + paint.gradient_inverse.f;
        if self.linear {
            (gy - paint.coordinates[1]) * self.paint.constants[1]
        } else {
            let p = gy - paint.coordinates[4];
            p * p
        }
    }

    /// The colour [`PreparedPaint::sample`] writes for `key`, when the table
    /// can vouch for it.
    #[inline]
    pub fn lookup(&self, key: f64) -> Option<[u8; 4]> {
        let (low, high) = (self.edges[0], self.edges[self.cells.len()]);
        if key <= low {
            return Some(self.below);
        }
        if key >= high {
            return Some(self.above);
        }
        let cell = (((key - low) * self.inverse_width) as usize).min(self.cells.len() - 1);
        (self.edges[cell] <= key && key <= self.edges[cell + 1] && self.uniform[cell])
            .then(|| self.cells[cell])
    }

    /// The one colour the sampler gives for every key in `[low_key, high_key]`,
    /// when the table can vouch for all of them: the range lies past one edge
    /// or inside a run of uniform cells of a single colour. Every key of the
    /// range lies in the closed span of those cells, and the table's promise
    /// for each cell holds on its closed interval.
    #[inline]
    pub fn lookup_range(&self, low_key: f64, high_key: f64) -> Option<[u8; 4]> {
        let last = self.cells.len();
        let (low, high) = (self.edges[0], self.edges[last]);
        if high_key <= low {
            return Some(self.below);
        }
        if low_key >= high {
            return Some(self.above);
        }
        if low_key <= low || high_key >= high {
            return None;
        }
        let cell_of = |key: f64| (((key - low) * self.inverse_width) as usize).min(last - 1);
        let (first, final_cell) = (cell_of(low_key), cell_of(high_key));
        (first <= final_cell
            && self.edges[first] <= low_key
            && high_key <= self.edges[final_cell + 1]
            && self.uniform[first]
            && self.run_end[first] as usize >= final_cell)
            .then(|| self.cells[first])
    }

    /// The paint this table belongs to.
    pub fn paint(&self) -> &PreparedPaint<'_> {
        &self.paint
    }
}
