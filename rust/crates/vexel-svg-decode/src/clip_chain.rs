//! Conjoin existing clip parents without changing either original shared chain.
use crate::decode::Decoder;
use vexel_scene::*;
impl Decoder<'_, '_> {
    pub(crate) fn append_clip_chain(
        &mut self,
        scene: SceneId,
        base: &Clip,
        parent: Option<ClipId>,
        control: &mut dyn WorkControl,
    ) -> Result<ClipId> {
        if parent.is_none() || parent == base.parent {
            return self.copy_clip(scene, base, parent.or(base.parent), control);
        }
        let mut chain = Vec::new();
        let mut lease = None;
        let result = (|| {
            let mut cursor = base.parent;
            let mut seen = [false; 65536];
            while let Some(id) = cursor {
                crate::source::charge(control, WorkKind::ClipChild, 1, 0)?;
                if id.index() >= seen.len() || seen[id.index()] {
                    return Err(crate::invalid(0));
                }
                seen[id.index()] = true;
                crate::storage::reserve(
                    &mut chain,
                    &mut lease,
                    &mut self.ledger,
                    1,
                    65536,
                    control,
                )?;
                chain.push(id);
                cursor = self
                    .builder
                    .as_ref()
                    .ok_or_else(|| crate::invalid(0))?
                    .traversal_scene(scene)?
                    .clip(id)
                    .ok_or_else(|| crate::invalid(0))?
                    .parent;
            }
            let mut tail = parent;
            for id in chain.iter().rev() {
                // Copy just the descriptor's fixed values; geometry remains borrowed until
                // both admitted replacement buffers are complete.
                let view = self
                    .builder
                    .as_ref()
                    .ok_or_else(|| crate::invalid(0))?
                    .traversal_scene(scene)?;
                let old = view.clip(*id).ok_or_else(|| crate::invalid(0))?;
                let (edges, a) = crate::storage::copy(
                    &mut self.ledger,
                    &old.geometry.edges,
                    AllocationTag::ClipCopy,
                    control,
                )?;
                let (shapes, b) = crate::storage::copy(
                    &mut self.ledger,
                    &old.geometry.shapes,
                    AllocationTag::ClipCopy,
                    control,
                )?;
                let next = Clip {
                    geometry: ClipGeometry { edges, shapes },
                    parent: tail,
                    owner_node: old.owner_node,
                    owner_before_suffix: old.owner_before_suffix,
                    object_bounding_box: old.object_bounding_box,
                    bounds: old.bounds,
                    user_inverse: old.user_inverse,
                };
                let appended = self
                    .builder
                    .as_mut()
                    .ok_or_else(|| crate::invalid(0))?
                    .append_clip(scene, next, control);
                self.ledger.release(a)?;
                self.ledger.release(b)?;
                tail = Some(appended?);
            }
            self.copy_clip(scene, base, tail, control)
        })();
        drop(chain);
        if let Some(lease) = lease {
            self.ledger.release(lease)?;
        }
        result
    }
    fn copy_clip(
        &mut self,
        scene: SceneId,
        base: &Clip,
        parent: Option<ClipId>,
        control: &mut dyn WorkControl,
    ) -> Result<ClipId> {
        let (edges, a) = crate::storage::copy(
            &mut self.ledger,
            &base.geometry.edges,
            AllocationTag::ClipCopy,
            control,
        )?;
        let (shapes, b) = crate::storage::copy(
            &mut self.ledger,
            &base.geometry.shapes,
            AllocationTag::ClipCopy,
            control,
        )?;
        let next = Clip {
            geometry: ClipGeometry { edges, shapes },
            parent,
            owner_node: base.owner_node,
            owner_before_suffix: base.owner_before_suffix,
            object_bounding_box: base.object_bounding_box,
            bounds: base.bounds,
            user_inverse: base.user_inverse,
        };
        let result = self
            .builder
            .as_mut()
            .ok_or_else(|| crate::invalid(0))?
            .append_clip(scene, next, control);
        self.ledger.release(a)?;
        self.ledger.release(b)?;
        result
    }
}
