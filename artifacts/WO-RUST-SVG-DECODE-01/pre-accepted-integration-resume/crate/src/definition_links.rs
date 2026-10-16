use crate::{decode::Decoder, source::charge, tasks::Finish};
use vexel_scene::*;
impl<'src> Decoder<'src, '_> {
    pub(crate) fn apply_viewport_clip(
        &mut self,
        node: &Finish<'src>,
        points: [Point; 4],
        owner: Affine,
        limit: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if node.first_shape == limit {
            return Ok(());
        }
        let inverse = crate::affine_inverse(owner).ok_or(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: self.shared.tokens()[node.element].name.offset,
        })?;
        let mut edges = Vec::new();
        edges.try_reserve_exact(4).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
        let mut p = [Point { x: 0.0, y: 0.0 }; 4];
        for (i, point) in points.into_iter().enumerate() {
            p[i] =
                vexel_geometry::affine_map_point(inverse, point).ok_or_else(|| crate::invalid(0))?
        }
        for i in 0..4 {
            edges.push(Edge {
                a: p[i],
                b: p[(i + 1) % 4],
            });
        }
        let mut shapes = Vec::new();
        shapes.try_reserve_exact(1).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
        shapes.push(ClipChild {
            edges: TableRange { first: 0, count: 4 },
            evenodd: false,
            filled: true,
        });
        let clip = Clip {
            geometry: ClipGeometry { edges, shapes },
            parent: None,
            owner_node: node.context.node,
            owner_before_suffix: true,
            object_bounding_box: false,
            bounds: [0.0, 0.0, 1.0, 1.0],
            user_inverse: inverse,
        };
        self.attach_clip(node.context.scene, node.first_shape, limit, clip, control)
    }
    pub(crate) fn attach_clip(
        &mut self,
        scene: SceneId,
        first: usize,
        limit: usize,
        clip: Clip,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        for index in first..limit {
            charge(control, WorkKind::ClipChild, 1, 0)?;
            let view = self
                .builder()
                .ok_or_else(|| crate::invalid(0))?
                .scene(scene)?;
            let id = view.shape_id(index).ok_or_else(|| crate::invalid(0))?;
            let mut shape = *view.shape(id).ok_or_else(|| crate::invalid(0))?;
            let mut edges = Vec::new();
            edges
                .try_reserve_exact(clip.geometry.edges.len())
                .map_err(|_| Diagnostic {
                    kind: ErrorKind::OutOfMemory,
                    offset: 0,
                })?;
            edges.extend_from_slice(&clip.geometry.edges);
            let mut shapes = Vec::new();
            shapes
                .try_reserve_exact(clip.geometry.shapes.len())
                .map_err(|_| Diagnostic {
                    kind: ErrorKind::OutOfMemory,
                    offset: 0,
                })?;
            shapes.extend_from_slice(&clip.geometry.shapes);
            let next = Clip {
                geometry: ClipGeometry { edges, shapes },
                parent: shape.clip,
                owner_node: clip.owner_node,
                owner_before_suffix: clip.owner_before_suffix,
                object_bounding_box: clip.object_bounding_box,
                bounds: clip.bounds,
                user_inverse: clip.user_inverse,
            };
            let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
            shape.clip = Some(builder.append_clip(scene, next, control)?);
            builder.finalize_shape(
                crate::decode::draft_token(scene, self.root, &self.draft, &self.child_drafts)?,
                id,
                shape,
                control,
            )?;
        }
        Ok(())
    }
    pub(crate) fn local_definition(
        &mut self,
        value: vexel_xml::XmlSpan,
        control: &mut dyn WorkControl,
    ) -> Result<ElementId> {
        let bytes = value
            .slice(self.shared.source())
            .ok_or_else(|| crate::invalid(value.offset))?;
        if !bytes.starts_with(b"url(#") || bytes.last() != Some(&b')') || bytes.len() < 7 {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: value.offset,
            });
        }
        let id = vexel_xml::XmlSpan {
            offset: value.offset + 5,
            length: bytes.len() - 6,
        };
        self.shared.find_reference(id, control)?.ok_or(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: value.offset,
        })
    }
    pub(crate) fn local_bounds(
        &self,
        scene: SceneId,
        first: usize,
        limit: usize,
        t: Affine,
        offset: usize,
        control: &mut dyn WorkControl,
    ) -> Result<[f64; 4]> {
        let inverse = crate::affine_inverse(t).ok_or(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset,
        })?;
        let view = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .scene(scene)?;
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for shape in view
            .shapes
            .get(first..limit)
            .ok_or_else(|| crate::invalid(offset))?
        {
            for edge in shape
                .edges
                .slice(view.edges)
                .ok_or_else(|| crate::invalid(offset))?
            {
                charge(control, WorkKind::Edge, 1, offset)?;
                for p in [edge.a, edge.b] {
                    let p = vexel_geometry::affine_map_point(inverse, p).ok_or(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset,
                    })?;
                    b[0] = b[0].min(p.x);
                    b[1] = b[1].min(p.y);
                    b[2] = b[2].max(p.x);
                    b[3] = b[3].max(p.y);
                }
            }
        }
        if !b.iter().all(|v| v.is_finite()) {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset,
            });
        }
        Ok([b[0], b[1], b[2] - b[0], b[3] - b[1]])
    }
}
