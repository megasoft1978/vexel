//! Safe little-endian serialisation of the layout structs (no `unsafe`, no
//! `bytemuck`), field order exactly as `layout.rs` / `shaders/layout.h`.

use crate::encode::Encoded;
use crate::layout::{
    GpuClip, GpuEdge, GpuGradient, GpuSceneHeader, GpuShape, GpuStop, GpuTile, GpuTileEntry,
};

fn u(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn i(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn f(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn header_bytes(h: &GpuSceneHeader) -> Vec<u8> {
    let mut o = Vec::with_capacity(80);
    for v in [
        h.magic,
        h.version,
        h.width,
        h.height,
        h.tiles_x,
        h.tiles_y,
        h.shape_count,
        h.edge_count,
        h.tile_entry_count,
        h.gradient_count,
        h.stop_count,
        h.clip_count,
        h.flags,
        h.background_rgba,
        h.reserved0,
        h.reserved1,
        h.content_x0,
        h.content_y0,
        h.content_x1,
        h.content_y1,
    ] {
        u(&mut o, v);
    }
    o
}

pub fn shape_bytes(shapes: &[GpuShape]) -> Vec<u8> {
    let mut o = Vec::with_capacity(shapes.len() * 64);
    for s in shapes {
        s.bbox.iter().for_each(|v| f(&mut o, *v));
        u(&mut o, s.z);
        u(&mut o, s.paint_kind);
        u(&mut o, s.color_rgba);
        f(&mut o, s.opacity);
        u(&mut o, s.fill_rule);
        f(&mut o, s.stroke_width);
        for v in [
            s.edge_start,
            s.edge_count,
            s.gradient_index,
            s.clip_start,
            s.clip_count,
            s.group_depth,
        ] {
            u(&mut o, v);
        }
    }
    o
}

pub fn edge_bytes(edges: &[GpuEdge]) -> Vec<u8> {
    let mut o = Vec::with_capacity(edges.len() * 16);
    for e in edges {
        for v in [e.x0, e.y0, e.x1, e.y1] {
            i(&mut o, v);
        }
    }
    o
}

pub fn tile_bytes(tiles: &[GpuTile]) -> Vec<u8> {
    let mut o = Vec::with_capacity(tiles.len() * 8);
    for t in tiles {
        u(&mut o, t.list_start);
        u(&mut o, t.list_count);
    }
    o
}

pub fn list_bytes(list: &[GpuTileEntry]) -> Vec<u8> {
    let mut o = Vec::with_capacity(list.len() * 12);
    for e in list {
        for v in [e.shape, e.edge_start, e.edge_count] {
            u(&mut o, v);
        }
    }
    o
}

pub fn gradient_bytes(gradients: &[GpuGradient]) -> Vec<u8> {
    let mut o = Vec::with_capacity(gradients.len() * 48);
    for g in gradients {
        g.inverse.iter().for_each(|v| f(&mut o, *v));
        u(&mut o, g.spread);
        u(&mut o, g.stop_start);
        u(&mut o, g.stop_count);
        f(&mut o, g.focal_ratio);
        u(&mut o, g.reserved0);
        u(&mut o, g.reserved1);
    }
    o
}

pub fn stop_bytes(stops: &[GpuStop]) -> Vec<u8> {
    let mut o = Vec::with_capacity(stops.len() * 20);
    for s in stops {
        f(&mut o, s.offset);
        s.rgba.iter().for_each(|v| f(&mut o, *v));
    }
    o
}

pub fn clip_bytes(clips: &[GpuClip]) -> Vec<u8> {
    let mut o = Vec::with_capacity(clips.len() * 16);
    for c in clips {
        for v in [c.edge_start, c.edge_count, c.fill_rule, c.reserved0] {
            u(&mut o, v);
        }
    }
    o
}

impl Encoded {
    /// Byte buffers in upload order: header, shapes, edges, tiles, tile list,
    /// gradients, stops, clips.
    pub fn buffers(&self) -> [Vec<u8>; 8] {
        [
            header_bytes(&self.header),
            shape_bytes(&self.shapes),
            edge_bytes(&self.edges),
            tile_bytes(&self.tiles),
            list_bytes(&self.tile_list),
            gradient_bytes(&self.gradients),
            stop_bytes(&self.stops),
            clip_bytes(&self.clips),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::encode::encode;
    use crate::input::{GpuPaint, GpuScene, GpuSegment};

    #[test]
    fn sizes_match_layout() {
        let mut scene = GpuScene::new(32, 32);
        let s = |a, b, c, d| GpuSegment {
            x0: a,
            y0: b,
            x1: c,
            y1: d,
        };
        scene.push_fill(
            vec![
                s(1.0, 1.0, 9.0, 1.0),
                s(9.0, 1.0, 9.0, 9.0),
                s(9.0, 9.0, 1.0, 1.0),
            ],
            false,
            GpuPaint::Solid([1, 2, 3, 255]),
        );
        let e = encode(&scene).unwrap();
        let b = e.buffers();
        assert_eq!(b[0].len(), 80);
        assert_eq!(b[1].len(), 64);
        assert_eq!(b[2].len(), 16 * e.edges.len());
        assert_eq!(b[3].len(), 8 * e.tiles.len());
        assert_eq!(b[4].len(), 12 * e.tile_list.len());
        assert_eq!(&b[0][..4], &0x5658_4C31u32.to_le_bytes());
        // z at offset 16, edge_start at 40 (layout key offsets).
        assert_eq!(&b[1][16..20], &0u32.to_le_bytes());
        assert_eq!(&b[1][20..24], &0u32.to_le_bytes());
        assert_eq!(
            &b[1][24..28],
            &u32::from_le_bytes([1, 2, 3, 255]).to_le_bytes()
        );
    }
}
