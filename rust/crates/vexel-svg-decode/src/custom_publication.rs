//! Decoder-owned custom geometry continuation. Unlike the synchronous helper,
//! this route retains the request while typed definition plans suspend.
use crate::{
    Geometry, ResolvedStyle,
    decode::{Decoder, draft_token},
    tasks::{Context, Finish, Task},
};
use vexel_scene::*;
use vexel_xml::XmlSpan;

/// Guarded publication primitive used only by the decoder task. It receives the
/// actual live draft and request capabilities and never retains either borrow.
fn append_custom_guarded(
    shared: &mut crate::decode::Shared<'_, '_>,
    cache: &mut PaintCache,
    ledger: &mut ChargeLedger,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    request: RequestToken,
    scene: SceneId,
    map: SourceMap,
    geometry: Geometry,
    paint: &CssPaint<'_>,
    viewport: [f64; 2],
    operations: &mut crate::operations::Operations<'_>,
    control: &mut dyn WorkControl,
) -> Result<(ResolvedStyle, [bool; 2], usize)> {
    let first_shape = builder.traversal_scene(scene)?.shapes.len();
    let root = builder
        .traversal_scene(scene)?
        .viewport
        .ok_or_else(|| crate::invalid(map.element.start))?;
    let style = crate::parse_style(
        shared.source(),
        paint,
        viewport[0].hypot(viewport[1]) / 2.0f64.sqrt(),
        map.transform,
        root,
        control,
    )?;
    if !style.visible || crate::style_display_none(paint, map.element.start)? {
        return Ok((style, [false; 2], first_shape));
    }
    let mut deferred_pattern_channels = [false; 2];
    for (index, value) in [style.fill_server, style.stroke_server]
        .into_iter()
        .enumerate()
    {
        if let Some(value) = value {
            let bytes = value
                .slice(shared.source())
                .ok_or_else(|| crate::invalid(value.offset))?;
            if !bytes.starts_with(b"url(#") || bytes.last() != Some(&b')') || bytes.len() < 7 {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: value.offset,
                });
            }
            let reference = XmlSpan {
                offset: value.offset + 5,
                length: bytes.len() - 6,
            };
            let definition = shared
                .find_reference(reference, control)?
                .ok_or(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: value.offset,
                })?;
            deferred_pattern_channels[index] = shared
                .tokens()
                .get(definition.0)
                .and_then(|token| token.local_name.slice(shared.source()))
                == Some(b"pattern".as_slice());
        }
    }
    let admission = builder.admit_shape(draft, request, map.element.start, control)?;
    let mut resolve = ResolveContext {
        shared,
        cache,
        ledger,
    };
    crate::publication::append_admitted_custom_geometry_internal(
        &mut resolve,
        builder,
        draft,
        &admission,
        scene,
        map,
        geometry,
        &style,
        viewport,
        operations,
        control,
        deferred_pattern_channels,
    )?;
    Ok((style, deferred_pattern_channels, first_shape))
}

impl<'src, 'ctx> Decoder<'src, 'ctx> {
    /// Queue owned custom geometry into the normal resumable publication engine.
    /// The supplied host identifies the canonical source element whose computed
    /// style and definition ownership are represented by `paint` and `map`.
    pub fn enqueue_custom_geometry(
        &mut self,
        scene: SceneId,
        host: ElementId,
        map: SourceMap,
        geometry: Geometry,
        paint: CssPaint<'src>,
        viewport: [f64; 2],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if self.state != crate::DecoderState::Running
            || self.custom_request.is_some()
            || !viewport.into_iter().all(|v| v.is_finite() && v > 0.0)
            || !vexel_geometry::affine_valid(map.transform)
        {
            return Err(crate::invalid(map.element.start));
        }
        let source = self.shared.source();
        if map.element.slice(source).is_none()
            || self.shared.tokens().get(host.0).is_none()
            || !std::sync::Arc::ptr_eq(
                self.shared.source_owner(),
                self.builder
                    .as_ref()
                    .ok_or_else(|| crate::invalid(0))?
                    .source_owner(),
            )
        {
            return Err(crate::invalid(map.element.start));
        }
        let owns_request = !self
            .shape_request_top()
            .is_some_and(|(active_scene, _, _)| active_scene == scene);
        let request = self.ensure_shape_request(scene, control)?;
        let bytes = match crate::geometry_storage(&geometry) {
            Ok(bytes) => bytes,
            Err(error) => return Err(self.abort(error)),
        };
        let lease = match self.ledger.admit(AllocationTag::None, bytes) {
            Ok(lease) => lease,
            Err(error) => return Err(self.abort(error)),
        };
        if let Err(error) = crate::storage::reserve(
            &mut self.tasks,
            &mut self.task_lease,
            &mut self.ledger,
            2,
            1_048_576,
            control,
        ) {
            self.ledger.release(lease)?;
            return Err(self.abort(error));
        }
        self.custom_request = Some(request);
        self.pending_custom_lease = Some(lease);
        self.tasks.push(Task::CustomDone {
            request,
            owns_request,
        });
        self.tasks.push(Task::CustomGeometry {
            scene,
            request,
            host,
            map,
            geometry,
            paint,
            viewport,
        });
        Ok(())
    }

    pub(crate) fn publish_custom_task(
        &mut self,
        scene: SceneId,
        request: RequestToken,
        host: ElementId,
        map: SourceMap,
        geometry: Geometry,
        paint: CssPaint<'src>,
        viewport: [f64; 2],
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let lease = self
            .pending_custom_lease
            .take()
            .ok_or_else(|| crate::invalid(map.element.start))?;
        let result = self.publish_custom_task_inner(
            scene, request, host, map, geometry, paint, viewport, operations, control,
        );
        self.ledger.release(lease)?;
        result
    }

    fn publish_custom_task_inner(
        &mut self,
        scene: SceneId,
        request: RequestToken,
        host: ElementId,
        map: SourceMap,
        geometry: Geometry,
        paint: CssPaint<'src>,
        viewport: [f64; 2],
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if self.custom_request != Some(request)
            || self
                .shape_request_top()
                .is_none_or(|(s, r, _)| s != scene || r != request)
        {
            return Err(crate::invalid(map.element.start));
        }
        let draft = draft_token(scene, self.root, &self.draft, &self.child_drafts)?;
        let result = append_custom_guarded(
            &mut self.shared,
            &mut self.cache,
            &mut self.ledger,
            self.builder.as_mut().ok_or_else(|| crate::invalid(0))?,
            &draft,
            request,
            scene,
            map,
            geometry,
            &paint,
            viewport,
            operations,
            control,
        );
        let (style, deferred_pattern_channels, first_shape) = result?;
        if !style.visible || crate::style_display_none(&paint, map.element.start)? {
            return Ok(());
        }
        let context = Context {
            scene,
            style: paint,
            transform: map.transform,
            viewport,
            depth: 0,
            node: map.transform_node,
            group: None,
            host: Some(host),
            text: None,
            preserve: false,
            flavor: DecodeFlavor::Normal,
        };
        let finish = Finish {
            context,
            element: host.0,
            first_shape,
            group: None,
            style: paint,
            viewport_clip: None,
            reference: None,
            text_root: false,
            clip_done: false,
            mask_done: false,
        };
        self.push(Task::Finish(finish.clone()), control)?;
        let patterns = [
            (style.fill_server, PaintChannel::Fill),
            (style.stroke_server, PaintChannel::Stroke),
        ];
        for (index, (value, channel)) in patterns.into_iter().enumerate().rev() {
            if deferred_pattern_channels[index] {
                self.push(
                    Task::PatternPlan {
                        node: finish.clone(),
                        value: value.ok_or_else(|| crate::invalid(map.element.start))?,
                        channel,
                    },
                    control,
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn finish_custom_task(
        &mut self,
        request: RequestToken,
        owns_request: bool,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if self.custom_request != Some(request) {
            return Err(crate::invalid(0));
        }
        if owns_request {
            let Some((scene, active, _)) = self.shape_request_top() else {
                return Err(crate::invalid(0));
            };
            if active != request {
                return Err(crate::invalid(0));
            }
            let draft = draft_token(scene, self.root, &self.draft, &self.child_drafts)?;
            let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
            if let Err(error) = builder.commit_request(&draft, request, control) {
                builder.rollback_request(&draft, request)?;
                self.clear_shape_request_top();
                self.custom_request = None;
                return Err(error);
            }
            self.clear_shape_request_top();
        }
        self.custom_request = None;
        Ok(())
    }
}
