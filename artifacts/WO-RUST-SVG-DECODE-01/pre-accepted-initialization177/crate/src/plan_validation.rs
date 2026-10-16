//! Validate lower-owned canonical ranges/flavors before any child is published.
use crate::decode::Decoder;
use vexel_scene::*;
use vexel_xml::TokenKind;
impl Decoder<'_, '_> {
    pub(crate) fn validate_post(
        &self,
        kind: DefinitionKind,
        post: PostOperation,
        scene: SceneId,
    ) -> Result<()> {
        let view = self
            .builder
            .as_ref()
            .ok_or_else(|| crate::invalid(0))?
            .scene(scene)?;
        match post {
            PostOperation::None => Ok(()),
            PostOperation::FinishPattern if kind == DefinitionKind::Pattern => Ok(()),
            PostOperation::FinishMarker if kind == DefinitionKind::Marker => Ok(()),
            PostOperation::AttachClip(id)
                if kind == DefinitionKind::Clip && view.clip(id).is_some() =>
            {
                Ok(())
            }
            PostOperation::FinishGroup(id) if view.group(id).is_some() => Ok(()),
            PostOperation::AttachTargetedMask(_) if kind == DefinitionKind::Mask => Ok(()),
            _ => Err(crate::invalid(0)),
        }
    }
    pub(crate) fn validate_expansion(
        &self,
        kind: DefinitionKind,
        expansion: &Expansion<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let flavor = matches!(
            (kind, expansion.flavor),
            (DefinitionKind::Clip, DecodeFlavor::Clip { .. })
                | (DefinitionKind::Mask, DecodeFlavor::Mask)
                | (DefinitionKind::Pattern, DecodeFlavor::Pattern)
                | (DefinitionKind::Marker, DecodeFlavor::Marker)
        );
        if !flavor
            || !vexel_geometry::affine_valid(expansion.transform)
            || !expansion
                .viewport
                .into_iter()
                .all(|v| v.is_finite() && v > 0.0)
        {
            return Err(crate::invalid(0));
        }
        let end = expansion.range.end().ok_or_else(|| crate::invalid(0))?;
        let tokens = self
            .shared
            .tokens()
            .get(expansion.range.first..end)
            .ok_or_else(|| crate::invalid(0))?;
        let mut open = [false; 257];
        let mut attribute_owner = None;
        for token in tokens {
            crate::source::charge(control, WorkKind::TokenVisit, 1, token.source.offset)?;
            if token.depth > 256 {
                return Err(crate::invalid(token.source.offset));
            }
            match token.kind {
                TokenKind::StartElement => {
                    if open[token.depth] {
                        return Err(crate::invalid(token.source.offset));
                    }
                    open[token.depth] = true;
                    attribute_owner = Some(token.depth);
                }
                TokenKind::EmptyElement => attribute_owner = Some(token.depth),
                TokenKind::Attribute => {
                    if attribute_owner != Some(token.depth) {
                        return Err(crate::invalid(token.source.offset));
                    }
                }
                TokenKind::EndElement => {
                    if !open[token.depth] {
                        return Err(crate::invalid(token.source.offset));
                    }
                    open[token.depth] = false;
                    attribute_owner = None;
                }
                _ => attribute_owner = None,
            }
        }
        if open.into_iter().any(|v| v) {
            return Err(crate::invalid(0));
        }
        for property in vexel_svg_css::Property::ALL {
            if let Some(value) = expansion.style.value(property) {
                crate::source::charge(control, WorkKind::PaintVisit, 1, value.span.offset)?;
                if !value.span.slice(self.shared.source()).is_some_and(|b| {
                    b.as_ptr() == value.bytes.as_ptr() && b.len() == value.bytes.len()
                }) {
                    return Err(crate::invalid(value.span.offset));
                }
            }
        }
        Ok(())
    }
}
