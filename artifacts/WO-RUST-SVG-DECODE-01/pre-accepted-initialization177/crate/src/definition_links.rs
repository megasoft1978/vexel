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
        let mut p = [Point { x: 0.0, y: 0.0 }; 4];
        for (i, point) in points.into_iter().enumerate() {
            p[i] =
                vexel_geometry::affine_map_point(inverse, point).ok_or_else(|| crate::invalid(0))?
        }
        let raw = std::array::from_fn::<_, 4, _>(|i| Edge {
            a: p[i],
            b: p[(i + 1) % 4],
        });
        let (edges, edge_lease) =
            crate::storage::copy(&mut self.ledger, &raw, AllocationTag::ClipCopy, control)?;
        let (shapes, shape_lease) = crate::storage::copy(
            &mut self.ledger,
            &[ClipChild {
                edges: TableRange { first: 0, count: 4 },
                evenodd: false,
                filled: true,
            }],
            AllocationTag::ClipCopy,
            control,
        )?;
        let clip = Clip {
            geometry: ClipGeometry { edges, shapes },
            parent: None,
            owner_node: node.context.node,
            owner_before_suffix: true,
            object_bounding_box: false,
            bounds: [0.0, 0.0, 1.0, 1.0],
            user_inverse: inverse,
        };
        let result = self.attach_clip(node.context.scene, node.first_shape, limit, clip, control);
        self.ledger.release(edge_lease)?;
        self.ledger.release(shape_lease)?;
        result
    }
    pub(crate) fn attach_clip(
        &mut self,
        scene: SceneId,
        first: usize,
        limit: usize,
        clip: Clip,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let mut memo: Vec<(Option<ClipId>, ClipId)> = Vec::new();
        let mut memo_lease = None;
        let result = (|| {
            for index in first..limit {
                charge(control, WorkKind::ClipChild, 1, 0)?;
                let view = self
                    .builder
                    .as_ref()
                    .ok_or_else(|| crate::invalid(0))?
                    .scene(scene)?;
                let id = view.shape_id(index).ok_or_else(|| crate::invalid(0))?;
                let mut shape = *view.shape(id).ok_or_else(|| crate::invalid(0))?;
                let key = shape.clip.map_or(usize::MAX, |p| p.index());
                let (mut lo, mut hi) = (0, memo.len());
                while lo < hi {
                    charge(control, WorkKind::ReferenceProbe, 1, 0)?;
                    let mid = lo + (hi - lo) / 2;
                    if memo[mid].0.map_or(usize::MAX, |p| p.index()) < key {
                        lo = mid + 1
                    } else {
                        hi = mid
                    }
                }
                let existing = if let Some((parent, id)) = memo
                    .get(lo)
                    .filter(|v| v.0.map_or(usize::MAX, |p| p.index()) == key)
                {
                    if *parent != shape.clip {
                        return Err(crate::invalid(0));
                    }
                    Some(*id)
                } else {
                    None
                };
                let clip_id = if let Some(id) = existing {
                    id
                } else {
                    let id = self.append_clip_chain(scene, &clip, shape.clip, control)?;
                    crate::storage::reserve(
                        &mut memo,
                        &mut memo_lease,
                        &mut self.ledger,
                        1,
                        65536,
                        control,
                    )?;
                    charge(control, WorkKind::SourceRead, (memo.len() - lo) as u64, 0)?;
                    memo.insert(lo, (shape.clip, id));
                    id
                };
                shape.clip = Some(clip_id);
                self.builder
                    .as_mut()
                    .ok_or_else(|| crate::invalid(0))?
                    .finalize_shape(
                        crate::decode::draft_token(
                            scene,
                            self.root,
                            &self.draft,
                            &self.child_drafts,
                        )?,
                        id,
                        shape,
                        control,
                    )?;
            }
            Ok(())
        })();
        drop(memo);
        if let Some(lease) = memo_lease {
            self.ledger.release(lease)?;
        }
        result
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
