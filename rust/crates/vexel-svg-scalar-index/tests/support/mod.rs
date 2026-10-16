#![allow(dead_code)]
//! Explicit fixture providers; these have no production registration.
use std::cell::Cell;
use std::sync::Arc;
use vexel_scene::*;
use vexel_svg_scalar_index::*;

pub struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
pub struct StopOn {
    pub kind: WorkKind,
    pub left: usize,
    pub stop: Stop,
}
impl WorkControl for StopOn {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == self.kind {
            if self.left == 0 {
                return Err(self.stop);
            }
            self.left -= 1;
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
pub fn map() -> ViewMapping {
    ViewMapping {
        scale_x: 1.0,
        scale_y: 1.0,
        offset_x: 0.0,
        offset_y: 0.0,
    }
}
pub fn job(graph: &SceneGraph, budget: usize) -> RasterJob {
    RasterJob::new(ChargeLedger::new(graph.source_owner(), budget, 256).unwrap()).unwrap()
}
pub fn source(paths: &[String], width: u32, height: u32) -> SourceOwner {
    let mut text =
        format!("<svg width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">");
    for (i, path) in paths.iter().enumerate() {
        text.push_str(&format!("<path id=\"p{i}\" d=\"{path}\">"));
        if i == 0 {
            text.push_str("<animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/>");
        }
        text.push_str("</path>");
    }
    text.push_str("</svg>");
    Arc::new(text.into_bytes())
}
pub fn graph(
    paths: &[String],
    width: u32,
    height: u32,
    evenodd: bool,
    clip_inverse: Option<Affine>,
    uniform: bool,
) -> SceneGraph {
    let source = source(paths, width, height);
    let lexical = LexicalDocument::parse(source.clone(), 256 * 1024 * 1024, &mut Run).unwrap();
    assert!(!lexical.tokens().is_empty());
    let animation = vexel_svg_animation::parse_owned(&source, lexical.tokens()).unwrap();
    if !paths.is_empty() {
        assert_eq!(animation.tracks().len(), 1);
    }
    let viewport = Viewport {
        width,
        height,
        view_x: 0.0,
        view_y: 0.0,
        view_width: width as f64,
        view_height: height as f64,
        aspect: Default::default(),
    };
    let mut b =
        SceneBuilder::new(source.clone(), animation, 1, viewport, Limits::default()).unwrap();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let clip = clip_inverse.map(|inverse| {
        let edges = vec![
            Edge {
                a: Point { x: 2., y: 2. },
                b: Point { x: 5., y: 2. },
            },
            Edge {
                a: Point { x: 5., y: 2. },
                b: Point { x: 5., y: 5. },
            },
            Edge {
                a: Point { x: 5., y: 5. },
                b: Point { x: 2., y: 5. },
            },
            Edge {
                a: Point { x: 2., y: 5. },
                b: Point { x: 2., y: 2. },
            },
        ];
        // The unfilled child contributes to counts but does not visit edges.
        let geometry = ClipGeometry {
            edges,
            shapes: vec![
                ClipChild {
                    edges: TableRange { first: 0, count: 4 },
                    evenodd: false,
                    filled: true,
                },
                ClipChild {
                    edges: TableRange { first: 0, count: 4 },
                    evenodd: false,
                    filled: false,
                },
            ],
        };
        b.append_clip(
            root,
            Clip {
                geometry,
                parent: None,
                owner_node: None,
                owner_before_suffix: false,
                object_bounding_box: false,
                bounds: [0., 0., width as f64, height as f64],
                user_inverse: inverse,
            },
            &mut Run,
        )
        .unwrap()
    });
    let mut at = 0usize;
    for path in paths {
        let needle = format!("d=\"{path}\"");
        let start = source[at..]
            .windows(needle.len())
            .position(|x| x == needle.as_bytes())
            .unwrap()
            + at
            + 3;
        let span = Span {
            start,
            end: start + path.len(),
        };
        at = span.end;
        let parsed = vexel_svg_path::parse_path(
            &source,
            span,
            vexel_geometry::affine_identity(),
            Default::default(),
        )
        .unwrap();
        let colors = (0..parsed.edges.len())
            .map(|i| {
                if uniform {
                    [51, 102, 153, 255]
                } else {
                    [i as u8, 2, 3, 255]
                }
            })
            .collect();
        let mut shape = Shape::new(SourceMap {
            element: span,
            transform: vexel_geometry::affine_identity(),
            transform_node: None,
        });
        shape.evenodd = evenodd;
        shape.clip = clip;
        b.append_shape(
            root,
            ShapeInput {
                shape,
                path: parsed,
                edge_colors: colors,
            },
            &mut Run,
        )
        .unwrap();
    }
    b.commit_draft(&draft, &mut Run).unwrap();
    b.freeze().unwrap()
}
#[derive(Default)]
pub struct Resources {
    pub clip_visits: Cell<usize>,
    pub paint_visits: Cell<usize>,
    pub stroke_visits: Cell<usize>,
}
fn stopped(s: Stop) -> Diagnostic {
    Diagnostic {
        kind: match s {
            Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
fn invalid() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    }
}
impl ControlledKernelResources for Resources {
    fn paint_sample(
        &self,
        paint: &Paint,
        stops: &[PaintStop],
        x: f64,
        _y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]> {
        let mut color = [0; 4];
        for stop in stops {
            control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
            self.paint_visits.set(self.paint_visits.get() + 1);
            if stop.offset <= x {
                color = stop.rgba;
            }
        }
        if paint.kind == PaintKind::Solid && stops.is_empty() {
            return Err(invalid());
        }
        Ok(color)
    }
    fn clip_contains(
        &self,
        view: SceneView<'_>,
        mut id: ClipId,
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        loop {
            control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
            let clip = view.clip(id).ok_or_else(invalid)?;
            let a = clip.user_inverse;
            let p = Point {
                x: a.a * x + a.c * y + a.e,
                y: a.b * x + a.d * y + a.f,
            };
            let mut inside = false;
            for child in &clip.geometry.shapes {
                control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
                if !child.filled {
                    continue;
                }
                let mut wind = 0i64;
                let mut parity = false;
                for edge in child
                    .edges
                    .slice(&clip.geometry.edges)
                    .ok_or_else(invalid)?
                {
                    control.charge(WorkKind::ClipEdge, 1).map_err(stopped)?;
                    let up = edge.a.y <= p.y && edge.b.y > p.y;
                    let down = edge.b.y <= p.y && edge.a.y > p.y;
                    if (up || down)
                        && edge.a.x
                            + (p.y - edge.a.y) * (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y)
                            > p.x
                    {
                        wind += if up { 1 } else { -1 };
                        parity = !parity;
                    }
                }
                inside |= if child.evenodd { parity } else { wind != 0 };
            }
            if !inside {
                return Ok(false);
            }
            if let Some(parent) = clip.parent {
                id = parent;
            } else {
                return Ok(true);
            }
        }
    }
    fn clip_boundary_edges(
        &self,
        view: SceneView<'_>,
        mut id: ClipId,
        visitor: &mut dyn FnMut(Edge) -> bool,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        loop {
            control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
            let clip = view.clip(id).ok_or_else(invalid)?;
            let inv = clip.user_inverse;
            let det = inv.a * inv.d - inv.b * inv.c;
            if !det.is_finite() || det == 0.0 {
                return Ok(false);
            }
            let forward = Affine {
                a: inv.d / det,
                b: -inv.b / det,
                c: -inv.c / det,
                d: inv.a / det,
                e: (inv.c * inv.f - inv.d * inv.e) / det,
                f: (inv.b * inv.e - inv.a * inv.f) / det,
            };
            let point = |p: Point| Point {
                x: forward.a * p.x + forward.c * p.y + forward.e,
                y: forward.b * p.x + forward.d * p.y + forward.f,
            };
            for child in &clip.geometry.shapes {
                control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
                if !child.filled {
                    continue;
                }
                for &edge in child
                    .edges
                    .slice(&clip.geometry.edges)
                    .ok_or_else(invalid)?
                {
                    control.charge(WorkKind::ClipEdge, 1).map_err(stopped)?;
                    self.clip_visits.set(self.clip_visits.get() + 1);
                    if !visitor(Edge {
                        a: point(edge.a),
                        b: point(edge.b),
                    }) {
                        return Err(invalid());
                    }
                }
            }
            if let Some(parent) = clip.parent {
                id = parent;
            } else {
                return Ok(true);
            }
        }
    }
    fn clip_work_counts(
        &self,
        view: SceneView<'_>,
        mut id: ClipId,
        control: &mut dyn WorkControl,
    ) -> Result<ClipWorkCounts> {
        let mut counts = ClipWorkCounts {
            chain_elements: 0,
            child_shapes: 0,
            edges: 0,
        };
        loop {
            control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
            let clip = view.clip(id).ok_or_else(invalid)?;
            counts.chain_elements += 1;
            for child in &clip.geometry.shapes {
                control.charge(WorkKind::ClipChild, 1).map_err(stopped)?;
                counts.child_shapes += 1;
                counts.edges += child.edges.count;
            }
            if let Some(parent) = clip.parent {
                id = parent;
            } else {
                return Ok(counts);
            }
        }
    }
    fn stroke_sample_work(
        &self,
        view: SceneView<'_>,
        id: ShapeId,
        _cache: JobDashView<'_>,
        _x: f64,
        _y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)> {
        let shape = view.shape(id).ok_or_else(invalid)?;
        if shape.stroke.rgba[3] == 0 || shape.stroke.width <= 0.0 {
            return Ok((0, false, false));
        }
        let edges = shape.edges.slice(view.edges).ok_or_else(invalid)?;
        for _ in edges {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            self.stroke_visits.set(self.stroke_visits.get() + 1);
        }
        Ok((edges.len(), false, true))
    }
}
pub fn setup_cost(index: &ScalarIndex, job: &mut RasterJob, n: usize) {
    job.reserve(index.scratch.stroke_cost, n, &mut Run).unwrap();
    for _ in 0..n {
        job.extend(index.scratch.stroke_cost, &[0], &mut Run)
            .unwrap();
    }
}
