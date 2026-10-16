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
    pub channel: PaintChannel,
    pub mapping: Option<PatternMapping>,
    pub offset: usize,
    pub lease: Option<AllocationLease>,
}
#[derive(Debug)]
pub(crate) struct PendingPattern {
    pub descriptor: PatternDescriptor,
    pub child: SceneId,
    pub parent: SceneId,
    pub owner: ShapeId,
    pub offered: bool,
    pub channel: PaintChannel,
    pub mapping: Option<PatternMapping>,
    pub request: Option<RequestToken>,
}
impl<'src> Decoder<'src, '_> {
    pub(crate) fn queue_definition(
        &mut self,
        kind: DefinitionKind,
        value: vexel_xml::XmlSpan,
        node: &Finish<'src>,
        limit: usize,
        channel: Option<PaintChannel>,
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let definition = self.local_definition(value, control)?;
        let view = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .traversal_scene(node.context.scene)?;
        let shape = view.shape_id(node.first_shape);
        let target = if matches!(kind, DefinitionKind::Filter | DefinitionKind::Mask) {
            DefinitionTarget::Group(node.group.ok_or_else(|| crate::invalid(value.offset))?)
        } else {
            DefinitionTarget::Shape(shape.ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: value.offset,
            })?)
        };
        let bounds = if limit == node.first_shape {
            [0.0; 4]
        } else {
            self.local_bounds(
                node.context.scene,
                node.first_shape,
                limit,
                node.context.transform,
                value.offset,
                control,
            )?
        };
        let typed = if matches!(
            operations,
            crate::operations::Operations::Production(_)
                | crate::operations::Operations::ScopedProduction(_)
        ) {
            Some(
                self.builder
                    .as_ref()
                    .ok_or_else(|| crate::invalid(0))?
                    .definition_owner(
                        crate::decode::draft_token(
                            node.context.scene,
                            self.root,
                            &self.draft,
                            &self.child_drafts,
                        )?,
                        &self.shared,
                        node.context.host.unwrap_or(ElementId(node.element)),
                        target,
                        control,
                    )?,
            )
        } else {
            None
        };
        let mut context = DefinitionContext {
            shared: &mut self.shared,
            builder: self.builder.as_mut().ok_or_else(|| crate::invalid(0))?,
            scene: node.context.scene,
            active: &mut self.active,
            ledger: &mut self.ledger,
        };
        context.validate()?;
        let (plan, mapping) = match operations {
            crate::operations::Operations::Fixture(ops) => (
                ops.plan_definition(
                    &mut context,
                    kind,
                    definition,
                    shape.ok_or_else(|| crate::invalid(value.offset))?,
                    node.context.transform,
                    bounds,
                    &node.style,
                    control,
                )?,
                None,
            ),
            crate::operations::Operations::Production(ops) => {
                let owner = typed.ok_or_else(|| crate::invalid(0))?;
                let result = if kind == DefinitionKind::Pattern {
                    plan_channel_aware_pattern(
                        *ops,
                        &mut context,
                        definition,
                        owner,
                        channel.ok_or_else(|| crate::invalid(value.offset))?,
                        node.context.transform,
                        bounds,
                        &node.style,
                        control,
                    )?
                } else {
                    plan_channel_aware_definition(
                        *ops,
                        &mut context,
                        kind,
                        definition,
                        owner,
                        node.context.transform,
                        bounds,
                        &node.style,
                        control,
                    )?
                };
                match result {
                    ProductionDefinitionPlan::Definition(p) => (p, None),
                    ProductionDefinitionPlan::Pattern { plan, mapping } => (plan, Some(mapping)),
                }
            }
            crate::operations::Operations::ScopedProduction(ops) => {
                let owner = typed.ok_or_else(|| crate::invalid(0))?;
                let result = if kind == DefinitionKind::Pattern {
                    plan_production_pattern(
                        *ops,
                        &mut context,
                        definition,
                        owner,
                        channel.ok_or_else(|| crate::invalid(value.offset))?,
                        node.context.transform,
                        bounds,
                        &node.style,
                        control,
                    )?
                } else {
                    plan_production_definition(
                        *ops,
                        &mut context,
                        kind,
                        definition,
                        owner,
                        node.context.transform,
                        bounds,
                        &node.style,
                        control,
                    )?
                };
                match result {
                    ProductionDefinitionPlan::Definition(plan) => (plan, None),
                    ProductionDefinitionPlan::Pattern { plan, mapping } => (plan, Some(mapping)),
                }
            }
        };
        if kind == DefinitionKind::Filter {
            if !plan.expansions.is_empty() || plan.post != PostOperation::None {
                return Err(crate::invalid(value.offset));
            }
            let DefinitionDescriptor::Filter { filter, primitives } = plan.descriptor else {
                return Err(crate::invalid(value.offset));
            };
            let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
            builder.publish_group_filter(
                crate::decode::draft_token(
                    node.context.scene,
                    self.root,
                    &self.draft,
                    &self.child_drafts,
                )?,
                typed.ok_or_else(|| crate::invalid(value.offset))?,
                filter,
                &primitives,
                control,
            )?;
            return Ok(());
        }
        self.enqueue_plan(
            kind,
            definition,
            value.offset,
            plan,
            mapping,
            channel,
            node,
            limit,
            control,
        )
    }
    fn enqueue_plan(
        &mut self,
        kind: DefinitionKind,
        definition: ElementId,
        offset: usize,
        plan: DefinitionPlan<'src>,
        mapping: Option<PatternMapping>,
        channel: Option<PaintChannel>,
        node: &Finish<'src>,
        limit: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let expansions_lease = self
            .ledger
            .admit_vec(AllocationTag::None, &plan.expansions)?;
        let bytes = match &plan.descriptor {
            DefinitionDescriptor::Clip(c) => c
                .geometry
                .edges
                .capacity()
                .checked_mul(std::mem::size_of::<Edge>())
                .and_then(|n| {
                    c.geometry
                        .shapes
                        .capacity()
                        .checked_mul(std::mem::size_of::<ClipChild>())
                        .and_then(|b| n.checked_add(b))
                }),
            DefinitionDescriptor::Filter { primitives, .. } => primitives
                .capacity()
                .checked_mul(std::mem::size_of::<FilterPrimitive>()),
            _ => Some(0),
        }
        .ok_or_else(|| crate::failure(ErrorKind::Limit(LimitKind::Arithmetic), offset))?;
        let lease = self.ledger.admit(AllocationTag::None, bytes)?;
        let result = self.enqueue_admitted(
            kind, definition, offset, plan, mapping, channel, node, limit, lease, control,
        );
        self.ledger.release(expansions_lease)?;
        result
    }
    fn enqueue_admitted(
        &mut self,
        kind: DefinitionKind,
        definition: ElementId,
        offset: usize,
        plan: DefinitionPlan<'src>,
        mapping: Option<PatternMapping>,
        channel: Option<PaintChannel>,
        node: &Finish<'src>,
        limit: usize,
        lease: AllocationLease,
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
        for expansion in &plan.expansions {
            self.validate_expansion(kind, expansion, control)?;
            self.validate_post(kind, expansion.post, node.context.scene)?;
        }
        self.validate_post(kind, plan.post, node.context.scene)?;
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
            self.push(Task::MarkerDone { definition }, control)?;
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
                let post = expansion.post;
                if post != PostOperation::None {
                    self.push(
                        Task::ExpansionPost {
                            scene: node.context.scene,
                            post,
                            first_shape: node.first_shape,
                        },
                        control,
                    )?;
                }
                let (mut context, first, last) =
                    self.expansion_context(expansion, node.context.scene, node.context, control)?;
                context.style = context.style.without_inherited_markers();
                context.host = node.context.host.or(Some(ElementId(node.element)));
                self.push(
                    Task::Walk {
                        cursor: first,
                        limit: last,
                        context,
                    },
                    control,
                )?;
            }
            self.ledger.release(lease)?;
            return Ok(());
        }
        let mut viewport = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .traversal_scene(self.root)?
            .viewport
            .ok_or_else(|| crate::invalid(0))?;
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
        crate::storage::reserve(
            &mut self.child_drafts,
            &mut self.child_draft_lease,
            &mut self.ledger,
            1,
            65,
            control,
        )?;
        let child = self.builder_mut()?.new_scene(viewport)?;
        let draft = self.builder_mut()?.begin_draft(child)?;
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
        for expansion in plan.expansions.iter().rev() {
            if expansion.post != PostOperation::None {
                self.push(
                    Task::ExpansionPost {
                        scene: node.context.scene,
                        post: expansion.post,
                        first_shape: node.first_shape,
                    },
                    control,
                )?;
            }
        }
        self.push(
            Task::DefinitionDone(DefinitionWork {
                descriptor: plan.descriptor,
                post: plan.post,
                child,
                target,
                limit,
                definition,
                channel: channel.unwrap_or(PaintChannel::Fill),
                mapping,
                offset,
                lease: Some(lease),
            }),
            control,
        )?;
        for expansion in plan.expansions.into_iter().rev() {
            let (context, first, last) =
                self.expansion_context(expansion, child, node.context, control)?;
            self.push(
                Task::Walk {
                    cursor: first,
                    limit: last,
                    context,
                },
                control,
            )?;
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
                if !v.span.slice(self.shared.source()).is_some_and(|bytes| {
                    bytes.as_ptr() == v.bytes.as_ptr() && bytes.len() == v.bytes.len()
                }) {
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
}
