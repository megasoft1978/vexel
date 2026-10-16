//! Pen tool state: bounded vertex list and path-data generation.

use std::fmt::Write;

/// Vertex cap; keeps generated `d` under the 8 KiB document limit even for
/// all-cubic paths with 9-byte coordinates.
pub const MAX_PEN_VERTICES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub pos: (f64, f64),
    /// Outgoing cubic handle; the incoming handle is its mirror through `pos`.
    pub handle: Option<(f64, f64)>,
}

/// Rounds to 1/1000 document unit so output is short and deterministic.
pub fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0 + 0.0
}

#[derive(Debug)]
pub struct PenPath {
    vertices: Vec<Vertex>,
}

impl PenPath {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(MAX_PEN_VERTICES),
        }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn capacity(&self) -> usize {
        self.vertices.capacity()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Adds a vertex; returns false when it duplicates the last vertex or the cap is reached.
    pub fn push(&mut self, pos: (f64, f64), handle: Option<(f64, f64)>) -> bool {
        let pos = (round3(pos.0), round3(pos.1));
        let handle = handle.map(|h| (round3(h.0), round3(h.1)));
        if self.vertices.len() >= MAX_PEN_VERTICES
            || self.vertices.last().is_some_and(|last| last.pos == pos)
        {
            return false;
        }
        self.vertices.push(Vertex { pos, handle });
        true
    }

    /// `M x y` then `L` or `C` segments; `close` adds the segment back to the first vertex and ` Z`.
    pub fn path_data(&self, close: bool) -> Option<String> {
        let first = self.vertices.first()?;
        let mut d = String::new();
        write!(d, "M {} {}", first.pos.0, first.pos.1).ok()?;
        for pair in self.vertices.windows(2) {
            segment(&mut d, pair[0], pair[1])?;
        }
        if close {
            segment(&mut d, *self.vertices.last()?, *first)?;
            d.push_str(" Z");
        }
        Some(d)
    }
}

fn segment(d: &mut String, from: Vertex, to: Vertex) -> Option<()> {
    let incoming = to
        .handle
        .map(|h| (round3(2.0 * to.pos.0 - h.0), round3(2.0 * to.pos.1 - h.1)));
    if from.handle.is_none() && incoming.is_none() {
        write!(d, " L {} {}", to.pos.0, to.pos.1).ok()
    } else {
        let out = from.handle.unwrap_or(from.pos);
        let inn = incoming.unwrap_or(to.pos);
        write!(
            d,
            " C {} {} {} {} {} {}",
            out.0, out.1, inn.0, inn.1, to.pos.0, to.pos.1
        )
        .ok()
    }
}
