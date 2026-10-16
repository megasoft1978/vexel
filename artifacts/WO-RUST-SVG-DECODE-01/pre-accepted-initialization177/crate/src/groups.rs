//! Owning group entry/exit and deferred definition linking.
use crate::{
    decode::Decoder,
    tasks::{Context, Finish, Task},
};
use vexel_scene::*;
use vexel_svg_css::Property;
impl<'src> Decoder<'src, '_> {
    pub(crate) fn begin_group(
        &mut self,
        style: CssPaint<'src>,
        scene: SceneId,
        parent: Option<GroupId>,
        first: usize,
        element: usize,
        transform: Affine,
        viewport: [f64; 2],
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<Option<GroupId>> {
        let opacity = crate::style::parse_opacity(
            self.shared.source(),
            style.value(Property::Opacity),
            control,
        )?;
        let filter = style
            .value(Property::Filter)
            .filter(|v| !v.bytes.eq_ignore_ascii_case(b"none"));
        let mask = style
            .value(Property::Mask)
            .filter(|v| !v.bytes.eq_ignore_ascii_case(b"none"));
        let blend = if let Some(v) = style.value(Property::MixBlendMode) {
            [
                b"normal".as_slice(),
                b"multiply",
                b"screen",
                b"darken",
                b"lighten",
            ]
            .iter()
            .position(|n| *n == v.bytes)
            .ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: v.span.offset,
            })? as u8
        } else {
            0
        };
        let isolation = if let Some(v) = style.value(Property::Isolation) {
            if v.bytes.eq_ignore_ascii_case(b"isolate") {
                true
            } else if v.bytes.eq_ignore_ascii_case(b"auto") {
                false
            } else {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: v.span.offset,
                });
            }
        } else {
            false
        };
        if opacity == 1.0 && filter.is_none() && mask.is_none() && blend == 0 && !isolation {
            return Ok(None);
        }
        let group = self.builder_mut()?.append_group(
            scene,
            OpacityGroup {
                shapes: TableRange { first, count: 0 },
                parent,
                opacity,
                filter: None,
                mask: None,
                blend_mode: blend,
                isolation,
            },
            control,
        )?;
        if let Some(v) = filter {
            if matches!(operations, crate::operations::Operations::Fixture(_)) {
                return Err(self.contract_blocked(
                    crate::ContractBlocker::DefinitionOwnerBeforeGeometry,
                    v.span.offset,
                ));
            }
            let node = Finish {
                context: Context {
                    scene,
                    style,
                    transform,
                    viewport,
                    depth: 0,
                    node: None,
                    group: Some(group),
                    host: None,
                    text: None,
                    preserve: false,
                    flavor: DecodeFlavor::Normal,
                },
                element,
                first_shape: first,
                group: Some(group),
                style,
                viewport_clip: None,
                reference: None,
                text_root: false,
                clip_done: true,
                mask_done: true,
            };
            self.queue_definition(
                DefinitionKind::Filter,
                v.span,
                &node,
                first,
                None,
                operations,
                control,
            )?;
        }
        Ok(Some(group))
    }
    pub(crate) fn finish_node(
        &mut self,
        mut node: Finish<'src>,
        operations: &mut crate::operations::Operations<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let scene = node.context.scene;
        let limit = self
            .builder()
            .ok_or_else(|| crate::invalid(0))?
            .scene(scene)?
            .shapes
            .len();
        if let Some((points, owner_transform)) = node.viewport_clip.take() {
            self.apply_viewport_clip(&node, points, owner_transform, limit, control)?;
        }
        if !node.clip_done && limit != node.first_shape {
            node.clip_done = true;
            if let Some(v) = node
                .style
                .value(Property::ClipPath)
                .filter(|v| !v.bytes.eq_ignore_ascii_case(b"none"))
            {
                self.push(Task::Finish(node.clone()), control)?;
                self.queue_definition(
                    DefinitionKind::Clip,
                    v.span,
                    &node,
                    limit,
                    None,
                    operations,
                    control,
                )?;
                return Ok(());
            }
        }
        if let Some(group) = node.group {
            let builder = self.builder.as_mut().ok_or_else(|| crate::invalid(0))?;
            let mut value = *builder
                .scene(scene)?
                .group(group)
                .ok_or_else(|| crate::invalid(0))?;
            value.shapes.count = limit
                .checked_sub(node.first_shape)
                .ok_or_else(|| crate::invalid(0))?;
            builder.finalize_group(
                crate::decode::draft_token(scene, self.root, &self.draft, &self.child_drafts)?,
                group,
                value,
                control,
            )?;
            if !node.mask_done && limit != node.first_shape {
                node.mask_done = true;
                if let Some(v) = node
                    .style
                    .value(Property::Mask)
                    .filter(|v| !v.bytes.eq_ignore_ascii_case(b"none"))
                {
                    self.push(Task::Finish(node.clone()), control)?;
                    self.queue_definition(
                        DefinitionKind::Mask,
                        v.span,
                        &node,
                        limit,
                        None,
                        operations,
                        control,
                    )?;
                    return Ok(());
                }
            }
        }

        if let Some(target) = node.reference {
            if self.active.uses.pop() != Some(target) {
                return Err(crate::invalid(0));
            }
        }
        let _ = node.text_root;
        Ok(())
    }
}
