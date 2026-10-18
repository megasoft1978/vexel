//! Builds `render_groups` trees from the C scene dumps in `group_data`.
//! Storage is leaked: these are short-lived test processes.
#![allow(dead_code)]

use super::group_data::{Case, ClipData, DocData, ShapeData};
use vexel_geometry::Affine;
use vexel_render::{
    ClipShape, FlatEdge, Linecap, Linejoin, RenderClip, RenderGroup, RenderMask, RenderNode,
    RenderShape, StrokeStyle,
};

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

/// Forward element transform from C `shape_stroke_inverse`.
pub fn forward(m: [f64; 6]) -> Affine {
    let det = m[0] * m[3] - m[1] * m[2];
    let r = 1.0 / det;
    let (a, b, c, d) = (m[3] * r, -m[1] * r, -m[2] * r, m[0] * r);
    Affine {
        a,
        b,
        c,
        d,
        e: -(a * m[4] + c * m[5]),
        f: -(b * m[4] + d * m[5]),
    }
}

pub fn affine(m: [f64; 6]) -> Affine {
    Affine {
        a: m[0],
        b: m[1],
        c: m[2],
        d: m[3],
        e: m[4],
        f: m[5],
    }
}

fn flat(edges: &[[f64; 4]]) -> Vec<FlatEdge> {
    edges
        .iter()
        .map(|e| FlatEdge {
            x0: e[0],
            y0: e[1],
            x1: e[2],
            y1: e[3],
        })
        .collect()
}

fn clips(data: &'static [ClipData]) -> Vec<&'static RenderClip<'static>> {
    // Parents always follow their children in the dump order; build back to front.
    let mut built: Vec<Option<&'static RenderClip<'static>>> = vec![None; data.len()];
    for _ in 0..data.len() {
        for (i, c) in data.iter().enumerate() {
            let parent = c.parent.map(|p| built[p]);
            if built[i].is_some() || matches!(parent, Some(None)) {
                continue;
            }
            let shapes: Vec<ClipShape> = c
                .shapes
                .iter()
                .map(|s| ClipShape {
                    edges: s.0..s.1,
                    evenodd: s.2,
                    filled: s.3,
                })
                .collect();
            built[i] = Some(leak(RenderClip {
                edges: leak(flat(c.edges)),
                shapes: leak(shapes),
                parent: parent.flatten(),
                object_bounding_box: c.object_bbox,
                bounds: c.bounds,
                user_inverse: affine(c.user_inverse),
            }));
        }
    }
    built.into_iter().map(|c| c.expect("clip chain")).collect()
}

fn shape(data: &'static ShapeData) -> RenderShape<'static> {
    let ranges: Vec<_> = data.subpaths.iter().map(|s| s.0..s.1).collect();
    let closed: Vec<bool> = data.subpaths.iter().map(|s| s.2).collect();
    RenderShape {
        fill_gradient: None,
        stroke_gradient: None,
        edges: leak(flat(data.edges)),
        subpaths: leak(ranges),
        subpath_closed: leak(closed),
        stroke_closed: data.stroke_closed,
        evenodd: data.evenodd,
        filled: data.filled,
        fill_rgba: data.fill,
        stroke: Some(StrokeStyle {
            rgba: data.stroke,
            width: data.stroke_width,
            miter_limit: data.miter_limit,
            dash_offset: 0.0,
            cap: [Linecap::Butt, Linecap::Square, Linecap::Round][data.cap as usize],
            join: [Linejoin::Miter, Linejoin::Bevel, Linejoin::Round][data.join as usize],
            crisp_edges: false,
        }),
        transform: forward(data.transform_inverse),
    }
}

/// Mirrors C `render_group_range`: flat shapes between the groups whose parent is `parent`.
fn range(
    doc: &'static DocData,
    table: &[&'static RenderClip<'static>],
    first: usize,
    end: usize,
    parent: Option<usize>,
) -> Vec<RenderNode<'static>> {
    let flat_shape = |i: usize| RenderNode::Shape {
        shape: shape(&doc.shapes[i]),
        clip: doc.shapes[i].clip.map(|c| table[c]),
    };
    let mut nodes = Vec::new();
    let mut cursor = first;
    for (index, group) in doc.groups.iter().enumerate() {
        if group.parent != parent {
            continue;
        }
        nodes.extend((cursor..group.first).map(flat_shape));
        cursor = group.end;
        let mask = group.mask.map(|m| {
            let m = &doc.masks[m];
            let content = range(&m.content, table, 0, m.content.shapes.len(), None);
            &*leak(RenderMask {
                luminance: m.luminance,
                region: m.region,
                region_transform: affine(m.region_transform),
                content: leak(content),
            })
        });
        let children = range(doc, table, group.first, group.end, Some(index));
        nodes.push(RenderNode::Group(RenderGroup {
            opacity: group.opacity,
            clip: None,
            mask,
            children: leak(children),
        }));
    }
    nodes.extend((cursor..end).map(flat_shape));
    nodes
}

pub fn tree(case: &'static Case) -> &'static [RenderNode<'static>] {
    let table = clips(case.clips);
    leak(range(&case.doc, &table, 0, case.doc.shapes.len(), None))
}
