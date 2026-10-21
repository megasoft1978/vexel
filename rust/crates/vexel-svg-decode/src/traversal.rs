//! Iterative traversal of canonical tokens and owned instance contexts.
use crate::{
    decode::Decoder,
    source::{attribute, basic_shape, charge},
    tasks::{Context, Finish, Task},
};
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_xml::TokenKind;
impl<'src> Decoder<'src, '_> {
    pub(crate) fn process(
        &mut self,
        task: Task<'src>,
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if self.shape_request_boundary(&task) {
            self.flush_shape_request(control)?;
        }
        charge(control, WorkKind::TokenVisit, 1, 0)?;
        match task {
            Task::Walk {
                cursor,
                limit,
                context,
            } => self.walk(cursor, limit, context, operations, control),
            Task::Render {
                element,
                context,
                symbol,
            } => self.enter(element, context, symbol, operations, control),
            Task::Finish(node) => self.finish_node(node, operations, control),
            Task::DefinitionDone(work) => self.definition_done(work, control),
            Task::PatternPlan {
                node,
                value,
                channel,
            } => self.queue_definition(
                DefinitionKind::Pattern,
                value,
                &node,
                node.first_shape + 1,
                Some(channel),
                operations,
                control,
            ),
            Task::Markers {
                node,
                values,
                mut cursor,
                mut seen,
            } => {
                while cursor < 3 {
                    let slot = cursor;
                    cursor += 1;
                    if let Some(value) = values[slot] {
                        let id = self.local_definition(value, control)?;
                        if seen.contains(&Some(id)) {
                            continue;
                        }
                        seen[slot] = Some(id);
                        self.push(
                            Task::Markers {
                                node: node.clone(),
                                values,
                                cursor,
                                seen,
                            },
                            control,
                        )?;
                        self.queue_definition(
                            DefinitionKind::Marker,
                            value,
                            &node,
                            node.first_shape + 1,
                            None,
                            operations,
                            control,
                        )?;
                        return Ok(());
                    }
                }
                Ok(())
            }
            Task::MarkerDone { definition } => {
                if self.active.markers.pop() != Some(definition) {
                    return Err(crate::invalid(0));
                }
                Ok(())
            }
            Task::ExpansionPost {
                scene,
                post,
                first_shape,
            } => self.expansion_post(scene, post, first_shape, control),
            Task::Text { span, cursor } => self.process_text_task(span, cursor, control),
            Task::CustomGeometry {
                scene,
                request,
                host,
                map,
                geometry,
                paint,
                viewport,
            } => self.publish_custom_task(
                scene, request, host, map, geometry, paint, viewport, operations, control,
            ),
            Task::CustomDone {
                request,
                owns_request,
            } => self.finish_custom_task(request, owns_request, control),
        }
    }
    fn walk(
        &mut self,
        cursor: usize,
        limit: usize,
        context: Context<'src>,
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if cursor >= limit {
            return Ok(());
        }
        let t = *self
            .shared
            .tokens()
            .get(cursor)
            .ok_or_else(|| crate::invalid(0))?;
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement) {
            self.push(
                Task::Walk {
                    cursor: cursor + 1,
                    limit,
                    context,
                },
                control,
            )?;
            match t.kind {
                TokenKind::ProcessingInstruction => {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: t.name.offset,
                    });
                }
                TokenKind::Cdata if context.text.is_none() => {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: t.value.offset,
                    });
                }
                TokenKind::Text if context.text.is_none() => self.push(
                    Task::Text {
                        span: t.value,
                        cursor: 0,
                    },
                    control,
                )?,
                _ => {}
            }
            return Ok(());
        }
        let end = *self
            .ends
            .get(cursor)
            .filter(|e| **e != usize::MAX && **e < limit)
            .ok_or(Diagnostic {
                kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
                offset: t.name.offset,
            })?;
        if self.visits >= 4_194_304 {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Expansions),
                offset: t.name.offset,
            });
        }
        self.visits += 1;
        self.push(
            Task::Walk {
                cursor: end + 1,
                limit,
                context,
            },
            control,
        )?;
        let source = self.shared.source();
        let name = t.local_name.slice(source).unwrap_or_default();
        let definition =
            t.prefix.length == 0 && matches!(name, b"defs" | b"symbol" | b"marker" | b"clipPath");
        let meta = t.prefix.length == 0
            && matches!(name, b"title" | b"desc" | b"metadata" | b"meta")
            || t.prefix.slice(source) == Some(b"sodipodi") && name == b"namedview";
        let animation = t.prefix.length == 0
            && matches!(
                name,
                b"animate" | b"animateTransform" | b"animateMotion" | b"animateColor" | b"set"
            );
        if definition || meta || animation || vexel_svg_css::is_style_element(source, &t) {
            return Ok(());
        }
        self.enter(cursor, context, false, operations, control)
    }
    pub(crate) fn enter(
        &mut self,
        element: usize,
        parent: Context<'src>,
        symbol: bool,
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let t = *self
            .shared
            .tokens()
            .get(element)
            .ok_or_else(|| crate::invalid(0))?;
        let end = *self
            .ends
            .get(element)
            .filter(|e| **e != usize::MAX)
            .ok_or(Diagnostic {
                kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
                offset: t.name.offset,
            })?;
        let root = t.depth == 0 && !self.root_seen && parent.flavor == DecodeFlavor::Normal;
        let source = self.shared.source();
        let name = t.local_name.slice(source).unwrap_or_default();
        if parent.depth > 64 || t.depth > 64 {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Depth),
                offset: t.name.offset,
            });
        }
        if t.prefix.length != 0
            || root && name != b"svg"
            || !root
                && !symbol
                && !matches!(name, b"g" | b"svg" | b"use" | b"a" | b"switch")
                && !basic_shape(source, &t)
        {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            });
        }
        if name == b"tspan" && parent.text.is_none() {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            });
        }
        if name == b"tspan" {
            crate::tspan::reject_unsupported(source, self.shared.tokens(), element)?;
        }
        if root {
            self.root_seen = true;
        }
        if name == b"use" {
            return self.expand_use(element, parent, operations, control);
        }
        if symbol {
            crate::viewport::symbol_attributes_supported(
                source,
                self.shared.tokens(),
                element,
                control,
            )?;
        } else if element != self.root_index || parent.depth != 0 {
            if !crate::ignored::attributes_supported(
                source,
                self.shared.tokens(),
                element,
                root,
                &mut self.visited,
                &mut self.ignored,
                control,
            )? {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: t.name.offset,
                });
            }
        }
        let style = if element == self.root_index && root {
            self.root_style
                .take()
                .ok_or_else(|| crate::invalid(t.name.offset))?
        } else {
            self.shared
                .compute_style(ElementId(element), Some(&parent.style), control)?
        };
        if crate::style_display_none(&style, t.name.offset)? {
            return Ok(());
        }
        let local = if root {
            self.root_local
        } else {
            self.authored_transform(element, control)?
        };
        let current =
            vexel_geometry::affine_compose(parent.transform, local).ok_or(Diagnostic {
                kind: ErrorKind::Number,
                offset: t.name.offset,
            })?;
        let mut context = Context {
            style,
            transform: current,
            depth: parent.depth + 1,
            ..parent
        };
        let mut suffix = vexel_geometry::affine_identity();
        let mut viewport_clip = None;
        if !root && name == b"svg" {
            let view = crate::viewport::nested_viewport_controlled(
                source,
                self.shared.tokens(),
                element,
                parent.viewport[0],
                parent.viewport[1],
                current,
                control,
            )?;
            if view.empty {
                return Ok(());
            }
            context.transform = view.child_transform;
            context.viewport = [view.child_width, view.child_height];
            suffix = view.viewport_transform;
            viewport_clip = Some((view.clip_points, current));
        }
        if symbol {
            // Without a viewBox the symbol keeps the inherited viewport for percentages (approximation).
            if let Some(span) = attribute(source, self.shared.tokens(), element, b"viewBox", None) {
                let v = crate::viewport::read_viewbox_controlled(source, span, control).map_err(
                    |e| {
                        if e.kind == ErrorKind::Number {
                            Diagnostic {
                                kind: ErrorKind::Unsupported,
                                offset: t.name.offset,
                            }
                        } else {
                            e
                        }
                    },
                )?;
                context.viewport = [v[2], v[3]];
            }
            if style
                .value(Property::ClipPath)
                .is_some_and(|v| !v.bytes.eq_ignore_ascii_case(b"none"))
            {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: style.value(Property::ClipPath).unwrap().span.offset,
                });
            }
        }
        let node = self.builder_mut()?.append_transform(
            parent.scene,
            TransformFrame {
                source_offset: t.source.offset,
                parent: parent.node,
                attribute: local,
                suffix,
                motion: vexel_geometry::affine_identity(),
            },
            control,
        )?;
        context.node = Some(node);
        let first_shape = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .traversal_scene(parent.scene)?
            .shapes
            .len();
        let group = self.begin_group(
            style,
            parent.scene,
            parent.group,
            first_shape,
            element,
            context.transform,
            context.viewport,
            operations,
            control,
        )?;
        context.group = group.or(parent.group);
        let host = parent.host.unwrap_or(ElementId(element));
        let host_end = *self.ends.get(host.0).ok_or_else(|| crate::invalid(0))?;
        let source_map = SourceMap {
            element: crate::source::element_span(self.shared.tokens(), host.0, host_end, source)?,
            transform: context.transform,
            transform_node: context.node,
        };
        let mut text_root = false;
        let mut markers = None;
        let mut patterns = [None; 2];
        let mut pattern_count = 0;
        if matches!(name, b"text" | b"tspan") {
            self.flush_shape_request(control)?;
            if name == b"text" {
                if parent.text.is_some() {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: t.name.offset,
                    });
                }
                text_root = true;
                context.text = Some(parent.depth);
                self.text_states[parent.depth] = TextState::default();
            }
            let text_slot = context.text.ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            })?;
            let element_view = self.shared.element(ElementId(element), end + 1, control)?;
            let provider = self.media.as_deref_mut().ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            })?;
            let request = TextDecodeRequest {
                element: element_view,
                source_map,
                end: end + 1,
                style: &style,
                transform: context.transform,
                state: &mut self.text_states[text_slot],
                preserve_space: parent.preserve,
                provider,
                cache: &mut self.text_cache,
                scene: parent.scene,
                ledger: &mut self.ledger,
            };
            let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
            request.validate_source_map(builder, control)?;
            context.preserve = operations.decode_text(
                request,
                builder,
                crate::decode::draft_token(
                    parent.scene,
                    self.root,
                    &self.draft,
                    &self.child_drafts,
                )?,
                control,
            )?;
        } else if name == b"image" {
            self.flush_shape_request(control)?;
            let element_view = self.shared.element(ElementId(element), end + 1, control)?;
            let provider = self.media.as_deref_mut().ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            })?;
            let request = ImageDecodeRequest {
                element: element_view,
                source_map,
                style: &style,
                transform: context.transform,
                provider,
                scene: parent.scene,
                ledger: &mut self.ledger,
            };
            let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
            request.validate_source_map(builder, control)?;
            let id = operations.decode_image(
                request,
                builder,
                crate::decode::draft_token(
                    parent.scene,
                    self.root,
                    &self.draft,
                    &self.child_drafts,
                )?,
                control,
            )?;
            let actual = builder
                .traversal_scene(parent.scene)?
                .shape(id)
                .copied()
                .ok_or_else(|| crate::invalid(t.name.offset))?;
            if actual.source.element != source_map.element
                || actual.source.transform != source_map.transform
                || actual.source.transform_node != source_map.transform_node
            {
                return Err(crate::invalid(t.name.offset));
            }
        } else if basic_shape(source, &t) {
            let root_viewport = self
                .builder()
                .ok_or_else(|| crate::invalid(0))?
                .traversal_scene(self.root)?
                .viewport
                .ok_or_else(|| crate::invalid(0))?;
            let mut resolved = crate::parse_style(
                source,
                &style,
                parent.viewport[0].hypot(parent.viewport[1]) / 2.0f64.sqrt(),
                context.transform,
                root_viewport,
                control,
            )?;
            if resolved.visible {
                markers = Some(resolved.markers);
                for (server, stroke) in [
                    (resolved.fill_server, false),
                    (resolved.stroke_server, true),
                ] {
                    if let Some(value) = server {
                        if value
                            .slice(source)
                            .is_some_and(|v| v.starts_with(b"url(#") && v.last() == Some(&b')'))
                        {
                            let id = self.local_definition(value, control)?;
                            if self.shared.tokens()[id.0].local_name.slice(source)
                                == Some(b"pattern")
                            {
                                patterns[pattern_count] = Some((
                                    value,
                                    if stroke {
                                        PaintChannel::Stroke
                                    } else {
                                        PaintChannel::Fill
                                    },
                                ));
                                pattern_count += 1;
                                if stroke {
                                    resolved.stroke_server = None
                                } else {
                                    resolved.fill_server = None
                                }
                            }
                        }
                    }
                }
                let geometry_source = if name == b"path" {
                    if attribute(source, self.shared.tokens(), element, b"d", None).is_none() {
                        return Err(Diagnostic {
                            kind: ErrorKind::Path,
                            offset: t.name.offset,
                        });
                    }
                    let view = self.shared.element(ElementId(element), end + 1, control)?;
                    Some(self.shared.canonical_path_source(view, control)?)
                } else {
                    None
                };
                let batch = self.batch_root_shapes
                    && parent.scene == self.root
                    && parent.host.is_none()
                    && parent.group.is_none()
                    && parent.depth == 1;
                let enclosing = self.shape_request_top();
                let child_in_parent_request =
                    enclosing.is_some_and(|(scene, _, _)| scene != parent.scene);
                let same_scene_request = enclosing.filter(|(scene, _, _)| *scene == parent.scene);
                let defer_request = batch || pattern_count != 0 || same_scene_request.is_some();
                let owns_request = !child_in_parent_request && same_scene_request.is_none();
                let request = if batch || pattern_count != 0 {
                    Some(self.ensure_shape_request(parent.scene, control)?)
                } else if let Some((_, request, _)) = same_scene_request {
                    Some(request)
                } else if child_in_parent_request {
                    None
                } else {
                    self.flush_shape_request(control)?;
                    let draft = crate::decode::draft_token(
                        parent.scene,
                        self.root,
                        &self.draft,
                        &self.child_drafts,
                    )?;
                    self.builder
                        .as_mut()
                        .ok_or_else(|| crate::invalid(0))?
                        .begin_request(draft, control)?
                        .into()
                };
                let draft = crate::decode::draft_token(
                    parent.scene,
                    self.root,
                    &self.draft,
                    &self.child_drafts,
                )?;
                let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
                let admission = if let Some(request) = request {
                    let admission_result = match geometry_source {
                        Some(proof) => builder.admit_shape_with_source(
                            draft,
                            request,
                            source_map.element.start,
                            proof,
                            control,
                        ),
                        None => {
                            builder.admit_shape(draft, request, source_map.element.start, control)
                        }
                    };
                    match admission_result {
                        Ok(admission) => Some(admission),
                        Err(error) => {
                            if owns_request {
                                builder.rollback_request(draft, request)?;
                                self.clear_shape_request_top();
                            }
                            return Err(error);
                        }
                    }
                } else {
                    None
                };
                let result = (|| {
                    let tokens = self.shared.tokens();
                    let geometry = crate::element_geometry_controlled(
                        source,
                        tokens,
                        element,
                        context.transform,
                        parent.viewport,
                        control,
                    )?;
                    if self
                        .point_count
                        .checked_add(geometry.point_count)
                        .is_none_or(|n| n > 4_194_304)
                    {
                        return Err(Diagnostic {
                            kind: ErrorKind::Limit(LimitKind::PathCommands),
                            offset: t.name.offset,
                        });
                    }
                    self.point_count += geometry.point_count;
                    let mut resolve = ResolveContext {
                        shared: &mut self.shared,
                        cache: &mut self.cache,
                        ledger: &mut self.ledger,
                    };
                    if let Some(admission) = admission.as_ref() {
                        crate::publication::append_admitted_geometry_internal(
                            &mut resolve,
                            builder,
                            draft,
                            admission,
                            parent.scene,
                            source_map,
                            geometry,
                            &resolved,
                            parent.viewport,
                            operations,
                            control,
                        )
                    } else {
                        crate::publication::append_geometry_internal(
                            &mut resolve,
                            builder,
                            parent.scene,
                            source_map,
                            geometry,
                            &resolved,
                            parent.viewport,
                            operations,
                            control,
                        )
                    }
                })();
                match result {
                    Ok(_) => {
                        let flush = if batch {
                            let index = self
                                .shape_request_depth
                                .checked_sub(1)
                                .ok_or_else(|| crate::invalid(0))?;
                            let (_, _, count) = self.shape_requests[index]
                                .as_mut()
                                .ok_or_else(|| crate::invalid(0))?;
                            *count = count.checked_add(1).ok_or(Diagnostic {
                                kind: ErrorKind::Limit(LimitKind::Arithmetic),
                                offset: t.name.offset,
                            })?;
                            *count >= 128
                        } else {
                            owns_request && !defer_request
                        };
                        if flush {
                            let request = request.ok_or_else(|| crate::invalid(0))?;
                            if let Err(error) = builder.commit_request(draft, request, control) {
                                builder.rollback_request(draft, request)?;
                                self.clear_shape_request_top();
                                return Err(error);
                            }
                            self.clear_shape_request_top();
                        }
                    }
                    Err(error) => {
                        if owns_request {
                            if let Some(request) = request {
                                builder.rollback_request(draft, request)?;
                                self.clear_shape_request_top();
                            }
                        }
                        return Err(error);
                    }
                }
            }
        }
        let finish = Finish {
            context,
            element,
            first_shape,
            group,
            style,
            viewport_clip,
            reference: None,
            text_root,
            clip_done: false,
            mask_done: false,
        };
        self.push(Task::Finish(finish.clone()), control)?;
        if end != element {
            let selected = if name == b"switch" {
                crate::viewport::select_switch_child_controlled(
                    source,
                    self.shared.tokens(),
                    element,
                    end,
                    self.language,
                    control,
                )?
            } else {
                Some((element + 1, end))
            };
            if let Some((first, limit)) = selected {
                self.push(
                    Task::Walk {
                        cursor: first,
                        limit,
                        context,
                    },
                    control,
                )?;
            }
        }
        if let Some(values) = markers {
            if values.iter().any(Option::is_some) {
                self.push(
                    Task::Markers {
                        node: finish.clone(),
                        values,
                        cursor: 0,
                        seen: [None; 3],
                    },
                    control,
                )?;
            }
        }
        for (value, channel) in patterns.into_iter().rev().flatten() {
            self.push(
                Task::PatternPlan {
                    node: finish.clone(),
                    value,
                    channel,
                },
                control,
            )?;
        }
        Ok(())
    }
    pub(crate) fn authored_transform(
        &mut self,
        element: usize,
        control: &mut dyn WorkControl,
    ) -> Result<Affine> {
        let t = self.shared.tokens()[element];
        let Some(span) = attribute(
            self.shared.source(),
            self.shared.tokens(),
            element,
            b"transform",
            None,
        ) else {
            return Ok(vexel_geometry::affine_identity());
        };

        let (local, n) =
            crate::transform::parse_transform_controlled(self.shared.source(), span, control)?;
        if n > 1_048_576usize.saturating_sub(self.transform_ops) {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Transforms),
                offset: t.name.offset,
            });
        }
        self.transform_ops += n;
        Ok(local)
    }
}
