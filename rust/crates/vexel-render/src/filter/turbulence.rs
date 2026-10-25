//! `feTurbulence`: the SVG 1.1 reference Perlin noise (section 15.22 sample
//! code) with its exact integer lattice set-up and `f64` arithmetic.
//!
//! The generated RGBA is straight (non-premultiplied) like every layer in the
//! filter chain, which is what Skia and resvg produce after their own
//! premultiply step. Pixel `(x, y)` samples the noise at the user-space
//! position of its top-left corner (resvg/Skia convention, no half-pixel
//! shift). Output is a pure function of the parameters, the view mapping and
//! the pixel position: rows are independent, so the work is split over scoped
//! threads without changing a single byte.

use crate::ViewMap;

const B_SIZE: usize = 0x100;
const BM: i32 = 0xff;
const PERLIN_N: i32 = 0x1000;
const RAND_M: i64 = 2_147_483_647;
const RAND_A: i64 = 16_807;
const RAND_Q: i64 = 127_773;
const RAND_R: i64 = 2_836;
/// Most worker threads for one primitive.
const MAX_THREADS: usize = 8;
/// Fewer rows than this per worker are not worth a thread.
const MIN_ROWS_PER_THREAD: usize = 32;

/// Parameters of one `feTurbulence` primitive.
#[derive(Debug, Clone, Copy)]
pub(super) struct Params {
    pub base: [f64; 2],
    pub octaves: u32,
    pub seed: i64,
    /// `type="turbulence"` (else `fractalNoise`).
    pub turbulence: bool,
    pub stitch: bool,
    /// Primitive subregion `[x, y, width, height]` in user space (stitching).
    pub tile: [f64; 4],
}

fn setup_seed(mut seed: i64) -> i64 {
    if seed <= 0 {
        seed = -(seed % (RAND_M - 1)) + 1;
    }
    if seed > RAND_M - 1 {
        seed = RAND_M - 1;
    }
    seed
}

fn random(seed: i64) -> i64 {
    let result = RAND_A * (seed % RAND_Q) - RAND_R * (seed / RAND_Q);
    if result <= 0 { result + RAND_M } else { result }
}

/// The reference `init`: lattice permutation and unit gradients per channel.
struct Lattice {
    select: [usize; B_SIZE + B_SIZE + 2],
    gradient: [[[f64; 2]; B_SIZE + B_SIZE + 2]; 4],
}

impl Lattice {
    fn new(seed: i64) -> Box<Self> {
        let mut lattice = Box::new(Self {
            select: [0; B_SIZE + B_SIZE + 2],
            gradient: [[[0.0; 2]; B_SIZE + B_SIZE + 2]; 4],
        });
        let mut seed = setup_seed(seed);
        for k in 0..4 {
            for i in 0..B_SIZE {
                lattice.select[i] = i;
                for j in 0..2 {
                    seed = random(seed);
                    lattice.gradient[k][i][j] =
                        (((seed % (B_SIZE as i64 + B_SIZE as i64)) - B_SIZE as i64) as f64)
                            / B_SIZE as f64;
                }
                let g = lattice.gradient[k][i];
                let s = (g[0] * g[0] + g[1] * g[1]).sqrt();
                lattice.gradient[k][i] = [g[0] / s, g[1] / s];
            }
        }
        for i in (1..B_SIZE).rev() {
            let k = lattice.select[i];
            seed = random(seed);
            let j = (seed % B_SIZE as i64) as usize;
            lattice.select[i] = lattice.select[j];
            lattice.select[j] = k;
        }
        for i in 0..B_SIZE + 2 {
            lattice.select[B_SIZE + i] = lattice.select[i];
            for k in 0..4 {
                lattice.gradient[k][B_SIZE + i] = lattice.gradient[k][i];
            }
        }
        lattice
    }
}

/// Lattice cell of one coordinate at one octave (the x or y half of `noise2`).
#[derive(Clone, Copy, Default)]
struct Axis {
    b0: usize,
    b1: usize,
    r0: f64,
    r1: f64,
    s: f64,
}

/// Stitch state of one axis at one octave: wrap threshold and tile size.
#[derive(Clone, Copy)]
struct Wrap {
    size: i32,
    wrap: i32,
}

fn axis(v: f64, wrap: Option<Wrap>) -> Axis {
    let t = v + f64::from(PERLIN_N);
    let ti = t as i32;
    let mut b0 = ti;
    let mut b1 = ti + 1;
    let r0 = t - f64::from(ti);
    let r1 = r0 - 1.0;
    if let Some(w) = wrap {
        if b0 >= w.wrap {
            b0 -= w.size;
        }
        if b1 >= w.wrap {
            b1 -= w.size;
        }
    }
    Axis {
        b0: (b0 & BM) as usize,
        b1: (b1 & BM) as usize,
        r0,
        r1,
        s: r0 * r0 * (3.0 - 2.0 * r0),
    }
}

/// Per-octave tables for one axis: coordinate `i` of the row/column.
struct AxisTable {
    cells: Vec<Axis>,
    octaves: usize,
    len: usize,
}

impl AxisTable {
    fn at(&self, octave: usize, i: usize) -> Axis {
        self.cells[octave * self.len + i]
    }
}

/// Base frequency after the reference stitch adjustment.
fn stitch_frequency(freq: f64, size: f64) -> f64 {
    if freq == 0.0 {
        return freq;
    }
    let lo = (size * freq).floor() / size;
    let hi = (size * freq).ceil() / size;
    if freq / lo < hi / freq { lo } else { hi }
}

fn build_axis(
    coords: impl Iterator<Item = f64>,
    len: usize,
    freq: f64,
    octaves: usize,
    stitch: Option<(f64, f64)>,
) -> AxisTable {
    let mut cells = vec![Axis::default(); octaves * len];
    let coords: Vec<f64> = coords.collect();
    for octave in 0..octaves {
        let scale = f64::from(1u32 << octave);
        let wrap = stitch.map(|(origin, size)| {
            let mut w = (size * freq + 0.5) as i32;
            let mut wrap = (origin * freq + f64::from(PERLIN_N) + f64::from(w)) as i32;
            for _ in 0..octave {
                w *= 2;
                wrap = 2 * wrap - PERLIN_N;
            }
            Wrap { size: w, wrap }
        });
        for (i, c) in coords.iter().enumerate() {
            cells[octave * len + i] = axis(c * freq * scale, wrap);
        }
    }
    AxisTable { cells, octaves, len }
}

/// Fills `out` (`width * height` RGBA8 pixels) with the noise.
pub(super) fn render(params: &Params, view: &ViewMap, out: &mut [u8]) {
    let (width, height) = (view.width as usize, view.height as usize);
    let lattice = Lattice::new(params.seed);
    let octaves = params.octaves as usize;
    let [mut fx, mut fy] = params.base;
    let (mut stitch_x, mut stitch_y) = (None, None);
    if params.stitch {
        let [tx, ty, tw, th] = params.tile;
        fx = stitch_frequency(fx, tw);
        fy = stitch_frequency(fy, th);
        stitch_x = Some((tx, tw));
        stitch_y = Some((ty, th));
    }
    let user_x = (0..width).map(|x| view.view.x + (x as f64 - view.offset_x) / view.scale_x);
    let user_y = (0..height).map(|y| view.view.y + (y as f64 - view.offset_y) / view.scale_y);
    let xs = build_axis(user_x, width, fx, octaves, stitch_x);
    let ys = build_axis(user_y, height, fy, octaves, stitch_y);
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_THREADS)
        .min((height / MIN_ROWS_PER_THREAD).max(1));
    let rows_per = height.div_ceil(threads).max(1);
    let (lattice, xs, ys) = (&*lattice, &xs, &ys);
    std::thread::scope(|scope| {
        for (band, chunk) in out.chunks_mut(rows_per * width * 4).enumerate() {
            scope.spawn(move || {
                for (row, line) in chunk.chunks_exact_mut(width * 4).enumerate() {
                    fill_row(params, lattice, xs, ys, band * rows_per + row, line);
                }
            });
        }
    });
}

fn fill_row(params: &Params, lattice: &Lattice, xs: &AxisTable, ys: &AxisTable, y: usize, line: &mut [u8]) {
    let turbulence = params.turbulence;
    for (x, pixel) in line.chunks_exact_mut(4).enumerate() {
        let mut sum = [0.0f64; 4];
        for octave in 0..xs.octaves {
            let (ax, ay) = (xs.at(octave, x), ys.at(octave, y));
            let i = lattice.select[ax.b0];
            let j = lattice.select[ax.b1];
            let b00 = lattice.select[i + ay.b0];
            let b10 = lattice.select[j + ay.b0];
            let b01 = lattice.select[i + ay.b1];
            let b11 = lattice.select[j + ay.b1];
            let ratio = f64::from(1u32 << octave);
            for (k, total) in sum.iter_mut().enumerate() {
                let g = &lattice.gradient[k];
                let u = ax.r0 * g[b00][0] + ay.r0 * g[b00][1];
                let v = ax.r1 * g[b10][0] + ay.r0 * g[b10][1];
                let a = u + ax.s * (v - u);
                let u = ax.r0 * g[b01][0] + ay.r1 * g[b01][1];
                let v = ax.r1 * g[b11][0] + ay.r1 * g[b11][1];
                let b = u + ax.s * (v - u);
                let n = a + ay.s * (b - a);
                *total += if turbulence { n.abs() } else { n } / ratio;
            }
        }
        for (byte, n) in pixel.iter_mut().zip(sum) {
            let value = if turbulence { n * 255.0 } else { (n * 255.0 + 255.0) / 2.0 };
            *byte = (value.clamp(0.0, 255.0) + 0.5) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewBox;

    fn view(size: u32) -> ViewMap {
        let vb = ViewBox { x: 0.0, y: 0.0, width: f64::from(size), height: f64::from(size) };
        ViewMap::from_viewbox(vb, size, size, None).unwrap()
    }

    fn params(turbulence: bool) -> Params {
        Params {
            base: [0.05, 0.05],
            octaves: 3,
            seed: 7,
            turbulence,
            stitch: false,
            tile: [0.0, 0.0, 64.0, 64.0],
        }
    }

    #[test]
    fn reference_random_sequence() {
        // Park-Miller minimal standard: seed 1 -> 16807 -> 282475249.
        assert_eq!(random(1), 16_807);
        assert_eq!(random(16_807), 282_475_249);
        assert_eq!(setup_seed(0), 1);
        assert_eq!(setup_seed(-5), 6);
        assert_eq!(setup_seed(i64::MAX), RAND_M - 1);
    }

    #[test]
    fn lattice_is_a_permutation_with_unit_gradients() {
        let lattice = Lattice::new(3);
        let mut seen = [false; B_SIZE];
        for i in 0..B_SIZE {
            seen[lattice.select[i]] = true;
            assert_eq!(lattice.select[i], lattice.select[i + B_SIZE]);
            for k in 0..4 {
                let g = lattice.gradient[k][i];
                assert!((g[0].hypot(g[1]) - 1.0).abs() < 1e-12);
            }
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn output_is_deterministic_and_seed_dependent() {
        let map = view(64);
        let (mut a, mut b, mut c) = (vec![0u8; 64 * 64 * 4], vec![0u8; 64 * 64 * 4], vec![0u8; 64 * 64 * 4]);
        render(&params(false), &map, &mut a);
        render(&params(false), &map, &mut b);
        let mut other = params(false);
        other.seed = 8;
        render(&other, &map, &mut c);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn fractal_noise_centres_on_mid_grey_and_turbulence_is_darker() {
        let map = view(64);
        let (mut f, mut t) = (vec![0u8; 64 * 64 * 4], vec![0u8; 64 * 64 * 4]);
        render(&params(false), &map, &mut f);
        render(&params(true), &map, &mut t);
        let mean = |v: &[u8]| v.iter().step_by(4).map(|b| f64::from(*b)).sum::<f64>() / 4096.0;
        assert!((mean(&f) - 127.5).abs() < 12.0, "fractal mean {}", mean(&f));
        assert!(mean(&t) < 100.0, "turbulence mean {}", mean(&t));
    }

    #[test]
    fn zero_frequency_is_flat() {
        let map = view(16);
        let mut p = params(false);
        p.base = [0.0, 0.0];
        let mut out = vec![0u8; 16 * 16 * 4];
        render(&p, &map, &mut out);
        assert!(out.chunks_exact(4).all(|px| px == [128, 128, 128, 128]));
    }

    #[test]
    fn thread_split_does_not_change_output() {
        let map = view(200);
        let mut whole = vec![0u8; 200 * 200 * 4];
        render(&params(false), &map, &mut whole);
        let lattice = Lattice::new(7);
        let xs = build_axis((0..200).map(|x| x as f64), 200, 0.05, 3, None);
        let ys = build_axis((0..200).map(|y| y as f64), 200, 0.05, 3, None);
        let mut single = vec![0u8; 200 * 200 * 4];
        for (y, line) in single.chunks_exact_mut(800).enumerate() {
            fill_row(&params(false), &lattice, &xs, &ys, y, line);
        }
        assert_eq!(whole, single);
    }

    #[test]
    fn stitched_noise_tiles_seamlessly() {
        // With stitching, the lattice wraps at the tile size: x = 0 and
        // x = tile width see the same cell.
        let p = Params { stitch: true, tile: [0.0, 0.0, 32.0, 32.0], ..params(false) };
        let fx = stitch_frequency(p.base[0], 32.0);
        let table = build_axis([0.0, 32.0].into_iter(), 2, fx, 3, Some((0.0, 32.0)));
        for octave in 0..3 {
            let (a, b) = (table.at(octave, 0), table.at(octave, 1));
            assert_eq!((a.b0, a.b1), (b.b0, b.b1), "octave {octave}");
            assert!((a.r0 - b.r0).abs() < 1e-9);
        }
    }
}
