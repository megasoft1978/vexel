//! Typed lower plans enqueue child traversal; lower planners never call the engine.
use crate::{
    decode::Decoder,
    tasks::{Context, Finish, Task},
};
use vexel_scene::*;
#[derive(Debug)]
pub(crate) struct DefinitionWork<'src> {
    pub descriptor: DefinitionDescriptor,
    pub post: PostOperation,
    pub child: SceneId,
    pub target: Finish<'src>,
    pub limit: usize,
    pub definition: ElementId,
    pub stroke: bool,
    pub offset: usize,
}
#[derive(Debug)]
pub(crate) struct PendingPattern {
    pub descriptor: PatternDescriptor,
    pub child: SceneId,
    pub parent: SceneId,
    pub owner: ShapeId,
    pub offered: bool,
    pub stroke: bool,
}
impl<'src> Decoder<'src, '_> {
    pub(crate) fn queue_definition(
        &mut self,
        kind: DefinitionKind,
        value: vexel_xml::XmlSpan,
        node: &Finish<'src>,
        limit: usize,
        operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let definition = self.local_definition(value, control)?;
        let view = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .scene(node.context.scene)?;
        let owner = view.shape_id(node.first_shape).ok_or(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: value.offset,
        })?;
        let bounds = self.local_bounds(
            node.context.scene,
            node.first_shape,
            limit,
            node.context.transform,
            value.offset,
            control,
        )?;
        let mut context = DefinitionContext {
            shared: &mut self.shared,
            builder: self.builder.as_mut().ok_or_else(|| crate::invalid(0))?,
            scene: node.context.scene,
            active: &mut self.active,
            ledger: &mut self.ledger,
        };
        context.validate()?;
        let plan = operations.plan_definition(
            &mut context,
            kind,
            definition,
            owner,
            node.context.transform,
            bounds,
            &node.style,
            control,
        )?;
        self.enqueue_plan(kind, definition, value.offset, plan, node, limit, control)
    }
    fn enqueue_plan(
        &mut self,
        kind: DefinitionKind,
        definition: ElementId,
        offset: usize,
        plan: DefinitionPlan<'src>,
        node: &Finish<'src>,
        limit: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let valid = matches!(
            (&plan.descriptor, kind),
            (DefinitionDescriptor::Clip(_), DefinitionKind::Clip)
                | (DefinitionDescriptor::Mask(_), DefinitionKind::Mask)
                | (DefinitionDescriptor::Pattern(_), DefinitionKind::Pattern)
                | (DefinitionDescriptor::Marker(_), DefinitionKind::Marker)
                | (DefinitionDescriptor::Filter { .. }, DefinitionKind::Filter)
        );
        if !valid {
            return Err(crate::invalid(offset));
        }
        if kind == DefinitionKind::Marker {
            if self.active.markers.len() >= 8 || self.active.markers.contains(&definition) {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset,
                });
            }
            self.active.markers.try_reserve(1).map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset,
            })?;
            self.active.markers.push(definition);
            self.push(Task::MarkerDone { definition })?;
            let DefinitionDescriptor::Marker(desc) = plan.descriptor else {
                return Err(crate::invalid(offset));
            };
            if desc.definition != definition
                || !vexel_geometry::affine_valid(desc.transform)
                || !desc.viewport.iter().all(|n| n.is_finite() && *n > 0.0)
            {
                return Err(crate::invalid(offset));
            }
            for expansion in plan.expansions.into_iter().rev() {
                let (mut context, first, last) =
                    self.expansion_context(expansion, node.context.scene, node.context, control)?;
                context.style = context.style.without_inherited_markers();
                context.host = node.context.host.or(Some(ElementId(node.element)));
                self.push(Task::Walk {
                    cursor: first,
                    limit: last,
                    context,
                })?;
            }
            return Ok(());
        }
        let mut viewport = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .scene(self.root)?
            .viewport;
        if let DefinitionDescriptor::Pattern(desc) = &plan.descriptor {
            if desc.definition != definition
                || desc.width == 0
                || desc.height == 0
                || desc.width > 4096
                || desc.height > 4096
                || !desc.tile_width.is_finite()
                || !desc.tile_height.is_finite()
                || desc.tile_width <= 0.0
                || desc.tile_height <= 0.0
            {
                return Err(crate::invalid(offset));
            }
            viewport.width = desc.width;
            viewport.height = desc.height;
            viewport.view_x = 0.0;
            viewport.view_y = 0.0;
            viewport.view_width = desc.tile_width;
            viewport.view_height = desc.tile_height;
            viewport.aspect = vexel_svg_values::Aspect {
                none: true,
                set: true,
                ..Default::default()
            };
        }
        let child = self.builder_mut()?.new_scene(viewport)?;
        let draft = self.builder_mut()?.begin_draft(child)?;
        self.child_drafts.try_reserve(1).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset,
        })?;
        self.child_drafts.push((child, draft));
        match kind {
            DefinitionKind::Mask => {
                if self.active.masks.len() >= 8 || self.active.masks.contains(&definition) {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset,
                    });
                }
                self.active.masks.try_reserve(1).map_err(|_| Diagnostic {
                    kind: ErrorKind::OutOfMemory,
                    offset,
                })?;
                self.active.masks.push(definition)
            }
            DefinitionKind::Pattern => {
                if self.active.patterns.len() >= 8 || self.active.patterns.contains(&definition) {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset,
                    });
                }
                self.active
                    .patterns
                    .try_reserve(1)
                    .map_err(|_| Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset,
                    })?;
                self.active.patterns.push(definition)
            }
            _ => {}
        }
        let target = Finish {
            context: node.context,
            element: node.element,
            first_shape: node.first_shape,
            group: node.group,
            style: node.style,
            viewport_clip: None,
            reference: None,
            text_root: false,
            clip_done: true,
            mask_done: true,
        };
        self.push(Task::DefinitionDone(DefinitionWork {
            descriptor: plan.descriptor,
            post: plan.post,
            child,
            target,
            limit,
            definition,
            stroke: node
                .style
                .value(vexel_svg_css::Property::Stroke)
                .is_some_and(|v| v.span.offset == offset),
            offset,
        }))?;
        for expansion in plan.expansions.into_iter().rev() {
            let post = expansion.post;
            let (context, first, last) =
                self.expansion_context(expansion, child, node.context, control)?;
            if post != PostOperation::None {
                self.push(Task::ExpansionPost {
                    scene: child,
                    post,
                    first_shape: 0,
                })?;
            }
            self.push(Task::Walk {
                cursor: first,
                limit: last,
                context,
            })?;
        }
        Ok(())
    }
    fn expansion_context(
        &self,
        expansion: Expansion<'src>,
        scene: SceneId,
        parent: Context<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<(Context<'src>, usize, usize)> {
        let end = expansion.range.end().ok_or_else(|| crate::invalid(0))?;
        if end > self.shared.tokens().len()
            || !vexel_geometry::affine_valid(expansion.transform)
            || !expansion.viewport.iter().all(|v| v.is_finite() && *v > 0.0)
        {
            return Err(crate::invalid(0));
        }
        for p in vexel_svg_css::Property::ALL {
            if let Some(v) = expansion.style.value(p) {
                crate::source::charge(control, WorkKind::PaintVisit, 1, v.span.offset)?;
                if v.span.slice(self.shared.source()) != Some(v.bytes) {
                    return Err(crate::invalid(v.span.offset));
                }
            }
        }
        Ok((
            Context {
                scene,
                style: expansion.style,
                transform: expansion.transform,
                viewport: expansion.viewport,
                depth: if expansion.flavor == DecodeFlavor::Marker {
                    parent.depth + 1
                } else {
                    0
                },
                node: if scene == parent.scene {
                    parent.node
                } else {
                    None
                },
                group: if scene == parent.scene {
                    parent.group
                } else {
                    None
                },
                host: None,
                text: None,
                preserve: false,
                flavor: expansion.flavor,
            },
            expansion.range.first,
            end,
        ))
    }
    pub(crate) fn definition_done(
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
                    .builder()
                    .ok_or_else(|| crate::invalid(0))?
                    .scene(child)?;
                let base = clip.geometry.edges.len();
                clip.geometry
                    .edges
                    .try_reserve_exact(view.edges.len())
                    .map_err(|_| Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset: work.offset,
                    })?;
                clip.geometry.edges.extend_from_slice(view.edges);
                clip.geometry
                    .shapes
                    .try_reserve_exact(view.shapes.len())
                    .map_err(|_| Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset: work.offset,
                    })?;
                for shape in view.shapes {
                    crate::source::charge(control, WorkKind::ClipChild, 1, work.offset)?;
                    clip.geometry.shapes.push(ClipChild {
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
                match work.post {
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
                }
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
                let mask = builder.append_mask(
                    work.target.context.scene,
                    Mask {
                        content: child,
                        luminance: desc.luminance,
                        region_object_bounding_box: desc.region_object_bounding_box,
                        custom_region: desc.custom_region,
                        region: desc.region,
                        region_transform: desc.region_transform,
                        source_offset: desc.source_offset,
                    },
                    control,
                )?;
                let mut value = *builder
                    .scene(work.target.context.scene)?
                    .group(group)
                    .ok_or_else(|| crate::invalid(work.offset))?;
                value.mask = Some(mask);
                let token = crate::decode::draft_token(
                    work.target.context.scene,
                    self.root,
                    &self.draft,
                    &self.child_drafts,
                )?;
                builder.finalize_group(token, group, value, control)
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
                    stroke: work.stroke,
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
