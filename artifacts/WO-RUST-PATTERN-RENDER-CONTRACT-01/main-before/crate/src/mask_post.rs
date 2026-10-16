//! Actual source/draft/group/range target for deferred mask attachment.
use crate::{
    types::{invalid, stopped},
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskPostTarget {
    scene: SceneId,
    draft: u64,
    generation: u64,
    request: Option<RequestToken>,
    group: GroupId,
    shapes: TableRange,
    mask: MaskId,
}
impl MaskPostTarget {
    pub fn group(self) -> GroupId {
        self.group
    }
    pub fn shapes(self) -> TableRange {
        self.shapes
    }
    pub fn mask(self) -> MaskId {
        self.mask
    }
    pub fn validate(
        self,
        ctx: &DefinitionContext<'_, '_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        ctx.validate()?;
        let active = ctx.builder.active_draft().ok_or_else(|| invalid(0))?;
        if self.scene != ctx.scene
            || self.scene != active.scene
            || self.draft != active.nonce
            || self.generation != ctx.builder.generation
            || self.request != ctx.builder.request_identity()
        {
            return Err(invalid(0));
        }
        let view = ctx.builder.traversal_scene(self.scene)?;
        let group = view.group(self.group).ok_or_else(|| invalid(0))?;
        if group.shapes != self.shapes || view.mask(self.mask).is_none() {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)
    }
}
impl SceneBuilder {
    pub fn mask_post_target(
        &self,
        draft: &DraftToken,
        source: &DecodeSource<'_>,
        owner: DefinitionOwner,
        shapes: TableRange,
        mask: MaskId,
        control: &mut dyn WorkControl,
    ) -> Result<MaskPostTarget> {
        let i = self.draft_index(draft)?;
        if !std::sync::Arc::ptr_eq(self.source_owner(), source.source_owner()) {
            return Err(invalid(0));
        }
        if !owner.matches_draft(draft, self.generation) {
            return Err(invalid(0));
        }
        let DefinitionTarget::Group(group) = owner.target() else {
            return Err(invalid(0));
        };
        self.draft_slot(draft, 9, group.index(), self.scenes[i].groups.len())?;
        let view = self.traversal_scene(draft.mark.scene)?;
        let actual = view.group(group).ok_or_else(|| invalid(0))?;
        if actual.shapes != shapes || view.mask(mask).is_none() {
            return Err(invalid(0));
        }
        shapes.require(view.shapes.len(), 0)?;
        // Canonical lexical subtree, including its end tag. No guessed current group.
        let token = source
            .tokens()
            .get(owner.element().0)
            .ok_or_else(|| invalid(0))?;
        let mut end = token.source.end().ok_or_else(crate::memory::arithmetic)?;
        if token.kind == vexel_xml::TokenKind::StartElement {
            let mut found = false;
            for child in &source.tokens()[owner.element().0 + 1..] {
                control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
                if child.kind == vexel_xml::TokenKind::EndElement && child.depth == token.depth {
                    end = child.source.end().ok_or_else(crate::memory::arithmetic)?;
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(invalid(token.source.offset));
            }
        }
        for shape in shapes.slice(view.shapes).ok_or_else(|| invalid(0))? {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            if shape.source.element.start < token.source.offset || shape.source.element.end > end {
                return Err(invalid(shape.source.element.start));
            }
        }
        control.checkpoint().map_err(stopped)?;
        Ok(MaskPostTarget {
            scene: draft.mark.scene,
            draft: draft.nonce,
            generation: self.generation,
            request: self.request_identity(),
            group,
            shapes,
            mask,
        })
    }
    /// Production mask expansion exit accepts only the explicit target variant.
    /// Legacy AttachMask remains usable by fixture consumers with an external owner.
    pub fn apply_mask_post(
        &mut self,
        draft: &DraftToken,
        post: PostOperation,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let i = self.draft_index(draft)?;
        let PostOperation::AttachTargetedMask(target) = post else {
            return Err(invalid(0));
        };
        if target.scene != draft.mark.scene
            || target.draft != draft.nonce
            || target.generation != self.generation
            || target.request != self.request_identity()
        {
            return Err(invalid(0));
        }
        self.draft_slot(draft, 9, target.group.index(), self.scenes[i].groups.len())?;
        let view = self.traversal_scene(target.scene)?;
        let mut value = *view.group(target.group).ok_or_else(|| invalid(0))?;
        if value.shapes != target.shapes || view.mask(target.mask).is_none() {
            return Err(invalid(0));
        }
        value.mask = Some(target.mask);
        self.finalize_group(draft, target.group, value, control)
    }
}

pub(crate) fn validate_production_post(
    ctx: &DefinitionContext<'_, '_>,
    kind: DefinitionKind,
    flavor: Option<DecodeFlavor>,
    post: PostOperation,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let view = ctx.builder.traversal_scene(ctx.scene)?;
    match post {
        PostOperation::AttachTargetedMask(target)
            if kind == DefinitionKind::Mask && flavor.is_none_or(|f| f == DecodeFlavor::Mask) =>
        {
            target.validate(ctx, control)
        }
        PostOperation::AttachTargetedMask(_) | PostOperation::AttachMask(_) => Err(invalid(0)),
        PostOperation::AttachClip(id)
            if kind == DefinitionKind::Clip && view.clip(id).is_some() =>
        {
            Ok(())
        }
        PostOperation::FinishPattern if kind == DefinitionKind::Pattern => Ok(()),
        PostOperation::FinishMarker if kind == DefinitionKind::Marker => Ok(()),
        PostOperation::FinishGroup(id) if view.group(id).is_some() => Ok(()),
        PostOperation::None => Ok(()),
        _ => Err(invalid(0)),
    }
}
