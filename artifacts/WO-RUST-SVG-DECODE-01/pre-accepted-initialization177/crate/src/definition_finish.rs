//! Sealed child publication and typed parent posts.
use crate::{
    decode::Decoder,
    definition_work::{DefinitionWork, PendingPattern},
};
use vexel_scene::*;
impl<'src> Decoder<'src, '_> {
    pub(crate) fn definition_done(
        &mut self,
        work: DefinitionWork<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let mut work = work;
        let lease = work.lease.take();
        let result = self.definition_done_inner(work, control);
        if let Some(lease) = lease {
            self.ledger.release(lease)?;
        }
        result
    }
    fn definition_done_inner(
        &mut self,
        work: DefinitionWork<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let (child, token) = self
            .child_drafts
            .pop()
            .ok_or_else(|| crate::invalid(work.offset))?;
        if child != work.child {
            return Err(crate::invalid(work.offset));
        }
        if let Err(e) = self.builder_mut()?.commit_draft(&token, control) {
            let _ = self.builder_mut()?.rollback_draft(token);
            return Err(e);
        }
        self.builder_mut()?.seal(child, control)?;
        match work.descriptor {
            DefinitionDescriptor::Clip(mut clip) => {
                let view = self
                    .builder
                    .as_ref()
                    .ok_or_else(|| crate::invalid(0))?
                    .scene(child)?;
                let original = std::mem::replace(
                    &mut clip.geometry,
                    ClipGeometry {
                        edges: Vec::new(),
                        shapes: Vec::new(),
                    },
                );
                let base = original.edges.len();
                let edge_count = base
                    .checked_add(view.edges.len())
                    .ok_or_else(|| crate::invalid(work.offset))?;
                let shape_count = original
                    .shapes
                    .len()
                    .checked_add(view.shapes.len())
                    .ok_or_else(|| crate::invalid(work.offset))?;
                let (mut edges, edge_lease) = crate::storage::allocate(
                    &mut self.ledger,
                    edge_count,
                    AllocationTag::ClipCopy,
                    control,
                )?;
                let (mut shapes, shape_lease) = crate::storage::allocate(
                    &mut self.ledger,
                    shape_count,
                    AllocationTag::ClipCopy,
                    control,
                )?;
                for edge in original.edges.iter().chain(view.edges) {
                    crate::source::charge(control, WorkKind::ClipChild, 1, work.offset)?;
                    edges.push(*edge);
                }
                for shape in &original.shapes {
                    crate::source::charge(control, WorkKind::ClipChild, 1, work.offset)?;
                    shapes.push(*shape);
                }
                for shape in view.shapes {
                    crate::source::charge(control, WorkKind::ClipChild, 1, work.offset)?;
                    shapes.push(ClipChild {
                        edges: TableRange {
                            first: base
                                .checked_add(shape.edges.first)
                                .ok_or_else(|| crate::invalid(work.offset))?,
                            count: shape.edges.count,
                        },
                        evenodd: shape.evenodd,
                        filled: shape.filled,
                    });
                }
                clip.geometry = ClipGeometry { edges, shapes };
                let result = match work.post {
                    PostOperation::AttachClip(id) => {
                        let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
                        let token = crate::decode::draft_token(
                            work.target.context.scene,
                            self.root,
                            &self.draft,
                            &self.child_drafts,
                        )?;
                        builder.finalize_clip(token, id, clip, control)?;
                        self.link_clip_id(
                            work.target.context.scene,
                            work.target.first_shape,
                            work.limit,
                            id,
                            control,
                        )
                    }
                    PostOperation::None => self.attach_clip(
                        work.target.context.scene,
                        work.target.first_shape,
                        work.limit,
                        clip,
                        control,
                    ),
                    _ => Err(crate::invalid(work.offset)),
                };
                drop(original);
                self.ledger.release(edge_lease)?;
                self.ledger.release(shape_lease)?;
                result
            }
            DefinitionDescriptor::Mask(desc) => {
                if self.active.masks.pop() != Some(work.definition) {
                    return Err(crate::invalid(work.offset));
                }
                let group = work
                    .target
                    .group
                    .ok_or_else(|| crate::invalid(work.offset))?;
                let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
                let token = crate::decode::draft_token(
                    work.target.context.scene,
                    self.root,
                    &self.draft,
                    &self.child_drafts,
                )?;
                let value = Mask {
                    content: child,
                    luminance: desc.luminance,
                    region_object_bounding_box: desc.region_object_bounding_box,
                    custom_region: desc.custom_region,
                    region: desc.region,
                    region_transform: desc.region_transform,
                    source_offset: desc.source_offset,
                };
                let mask = if let PostOperation::AttachTargetedMask(target) = work.post {
                    builder.finalize_mask(token, target.mask(), value, control)?;
                    target.mask()
                } else {
                    builder.append_mask(work.target.context.scene, value, control)?
                };
                let shapes = builder
                    .scene(work.target.context.scene)?
                    .group(group)
                    .ok_or_else(|| crate::invalid(work.offset))?
                    .shapes;
                let owner = builder.definition_owner(
                    token,
                    &self.shared,
                    work.target
                        .context
                        .host
                        .unwrap_or(ElementId(work.target.element)),
                    DefinitionTarget::Group(group),
                    control,
                )?;
                let target =
                    builder.mask_post_target(token, &self.shared, owner, shapes, mask, control)?;
                builder.apply_mask_post(token, PostOperation::AttachTargetedMask(target), control)
            }

            DefinitionDescriptor::Pattern(desc) => {
                if self.active.patterns.pop() != Some(work.definition) {
                    return Err(crate::invalid(work.offset));
                }
                let owner = self
                    .builder()
                    .ok_or_else(|| crate::invalid(0))?
                    .scene(work.target.context.scene)?
                    .shape_id(work.target.first_shape)
                    .ok_or_else(|| crate::invalid(work.offset))?;
                if self.pending_pattern.is_some() {
                    return Err(crate::invalid(work.offset));
                }
                self.pending_pattern = Some(PendingPattern {
                    descriptor: desc,
                    child,
                    parent: work.target.context.scene,
                    owner,
                    offered: false,
                    channel: work.channel,
                    mapping: work.mapping,
                });
                Ok(())
            }
            DefinitionDescriptor::Filter { filter, primitives } => {
                let group = work
                    .target
                    .group
                    .ok_or_else(|| crate::invalid(work.offset))?;
                let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
                let id = builder.append_filter(
                    work.target.context.scene,
                    filter,
                    &primitives,
                    control,
                )?;
                let mut value = *builder
                    .scene(work.target.context.scene)?
                    .group(group)
                    .ok_or_else(|| crate::invalid(work.offset))?;
                value.filter = Some(id);
                builder.finalize_group(
                    crate::decode::draft_token(
                        work.target.context.scene,
                        self.root,
                        &self.draft,
                        &self.child_drafts,
                    )?,
                    group,
                    value,
                    control,
                )
            }
            DefinitionDescriptor::Marker(_) => Err(crate::invalid(work.offset)),
        }
    }
    pub(crate) fn expansion_post(
        &mut self,
        scene: SceneId,
        post: PostOperation,
        first: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        match post {
            PostOperation::None | PostOperation::FinishMarker | PostOperation::FinishPattern => {
                Ok(())
            }
            PostOperation::FinishGroup(id) => {
                let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
                let view = builder.scene(scene)?;
                let mut group = *view.group(id).ok_or_else(|| crate::invalid(0))?;
                group.shapes.count = view
                    .shapes
                    .len()
                    .checked_sub(group.shapes.first)
                    .ok_or_else(|| crate::invalid(0))?;
                builder.finalize_group(
                    crate::decode::draft_token(scene, self.root, &self.draft, &self.child_drafts)?,
                    id,
                    group,
                    control,
                )
            }
            PostOperation::AttachClip(id) => {
                let limit = self
                    .builder()
                    .ok_or_else(|| crate::invalid(0))?
                    .scene(scene)?
                    .shapes
                    .len();
                self.link_clip_id(scene, first, limit, id, control)
            }
            PostOperation::AttachTargetedMask(_) => self
                .builder
                .as_mut()
                .ok_or_else(|| crate::invalid(0))?
                .apply_mask_post(
                    crate::decode::draft_token(scene, self.root, &self.draft, &self.child_drafts)?,
                    post,
                    control,
                ),
            PostOperation::AttachMask(id) => {
                let builder = self.builder_mut()?;
                if builder.scene(scene)?.mask(id).is_none() {
                    return Err(crate::invalid(0));
                }
                Err(self.contract_blocked(crate::ContractBlocker::MaskPostTarget, 0))
            }
        }
    }
    fn link_clip_id(
        &mut self,
        scene: SceneId,
        first: usize,
        limit: usize,
        clip: ClipId,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        for i in first..limit {
            let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
            let view = builder.scene(scene)?;
            if view.clip(clip).is_none() {
                return Err(crate::invalid(0));
            }
            let id = view.shape_id(i).ok_or_else(|| crate::invalid(0))?;
            let mut shape = *view.shape(id).ok_or_else(|| crate::invalid(0))?;
            shape.clip = Some(clip);
            builder.finalize_shape(
                crate::decode::draft_token(scene, self.root, &self.draft, &self.child_drafts)?,
                id,
                shape,
                control,
            )?;
        }
        Ok(())
    }
}
