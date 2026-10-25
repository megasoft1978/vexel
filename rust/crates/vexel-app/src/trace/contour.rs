//! Exact pixel-boundary contour following. Every boundary edge is directed
//! with the foreground on its right (screen coordinates, y down), so outer
//! contours come out clockwise (positive shoelace area) and holes
//! anticlockwise (negative); one pass finds both with no hierarchy search.

/// A closed lattice polyline with collinear points removed.
pub struct Loop {
    pub points: Vec<(i32, i32)>,
    /// Signed shoelace area in pixels; positive = outer boundary.
    pub area: i64,
}

struct Grid<'a> {
    m: &'a [bool],
    w: i32,
    h: i32,
}

impl Grid<'_> {
    fn fg(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.m[(y * self.w + x) as usize]
    }
    /// Is there a boundary edge leaving lattice vertex (x, y) in direction
    /// `d` (0 east, 1 south, 2 west, 3 north)?
    fn edge(&self, x: i32, y: i32, d: u8) -> bool {
        match d {
            0 => self.fg(x, y) && !self.fg(x, y - 1),
            1 => self.fg(x - 1, y) && !self.fg(x, y),
            2 => self.fg(x - 1, y - 1) && !self.fg(x - 1, y),
            _ => self.fg(x, y - 1) && !self.fg(x - 1, y - 1),
        }
    }
}

const STEP: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// Follows every boundary of `mask` (`w` x `h`, row-major). Stops early with
/// `None` once more than `max_loops` loops exist or the boundary walk needs
/// more than `steps_left` further steps (decremented as steps are taken).
pub fn loops(
    mask: &[bool],
    w: usize,
    h: usize,
    max_loops: usize,
    steps_left: &mut usize,
) -> Option<Vec<Loop>> {
    let g = Grid {
        m: mask,
        w: w as i32,
        h: h as i32,
    };
    let (vw, vh) = (w + 1, h + 1);
    let mut seen = vec![0u8; vw * vh];
    let mut out = Vec::new();
    // Rows with no foreground cannot start a loop.
    let mut row_any = vec![false; h];
    for (y, any) in row_any.iter_mut().enumerate() {
        *any = mask[y * w..(y + 1) * w].iter().any(|&b| b);
    }
    for vy in 0..vh {
        let near = (vy > 0 && row_any[vy - 1]) || (vy < h && row_any[vy]);
        if !near {
            continue;
        }
        for vx in 0..vw {
            for d in 0..4u8 {
                let bit = 1 << d;
                if seen[vy * vw + vx] & bit != 0 || !g.edge(vx as i32, vy as i32, d) {
                    continue;
                }
                if out.len() >= max_loops {
                    return None;
                }
                out.push(follow(
                    &g,
                    &mut seen,
                    vw,
                    (vx as i32, vy as i32),
                    d,
                    steps_left,
                )?);
            }
        }
    }
    Some(out)
}

fn follow(
    g: &Grid<'_>,
    seen: &mut [u8],
    vw: usize,
    start: (i32, i32),
    d0: u8,
    steps_left: &mut usize,
) -> Option<Loop> {
    let mut pts = Vec::new();
    let (mut x, mut y, mut d) = (start.0, start.1, d0);
    loop {
        *steps_left = steps_left.checked_sub(1)?;
        let cell = &mut seen[y as usize * vw + x as usize];
        if *cell & (1 << d) != 0 {
            // Only a corrupt walk revisits an edge; never loop forever.
            break;
        }
        *cell |= 1 << d;
        pts.push((x, y));
        x += STEP[d as usize].0;
        y += STEP[d as usize].1;
        // Prefer the right turn, then straight, then left: diagonal
        // neighbours stay separate and the walk never crosses itself.
        let next = [(d + 1) % 4, d, (d + 3) % 4]
            .into_iter()
            .find(|&nd| g.edge(x, y, nd));
        match next {
            Some(nd) => d = nd,
            None => break,
        }
        if (x, y) == start && d == d0 {
            break;
        }
    }
    let points = compress(&pts);
    let area = shoelace(&points);
    Some(Loop { points, area })
}

/// Drops points whose neighbours are collinear with them (cyclic).
fn compress(p: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let n = p.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
        let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
        if cross != 0 {
            out.push(b);
        }
    }
    out
}

fn shoelace(p: &[(i32, i32)]) -> i64 {
    let n = p.len();
    let mut s = 0i64;
    for i in 0..n {
        let (a, b) = (p[i], p[(i + 1) % n]);
        s += i64::from(a.0) * i64::from(b.1) - i64::from(b.0) * i64::from(a.1);
    }
    s / 2
}

#[cfg(test)]
mod tests {
    use super::loops;

    #[test]
    fn boundary_step_budget_is_enforced() {
        // 1-pixel staircase band: every boundary edge is walked once.
        let n = 600;
        let mut m = vec![false; n * n];
        for y in 0..n {
            for x in 0..n {
                m[y * n + x] = x == y || x + 1 == y;
            }
        }
        let mut ample = 10_000_000;
        let ok = loops(&m, n, n, 100, &mut ample).expect("fits");
        let used = 10_000_000 - ample;
        assert!(!ok.is_empty() && used > 1000);
        let mut tight = used - 1;
        assert!(loops(&m, n, n, 100, &mut tight).is_none());
    }
}
