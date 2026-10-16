//! Scoped decoder draft authorization. Tokens contain values, not builder
//! borrows, so an engine can own its builder and pending draft without selfrefs.
use crate::types::{invalid, stopped};
use crate::*;
#[derive(Debug, Clone, Copy)]
pub(crate) struct ActiveDraft {
    pub scene: SceneId,
    pub nonce: u64,
    pub scene_count: usize,
}
#[derive(Debug)]
pub struct DraftToken {
    pub(crate) mark: BuildMark,
    pub(crate) nonce: u64,
    pub(crate) scene_count: usize,
    ignored: [Ignored; 4],
    text_commands: usize,
}
impl SceneBuilder {
    pub fn begin_draft(&mut self, scene: SceneId) -> Result<DraftToken> {
        let i = self.index(scene)?;
        if matches!(self.initialization, DecodeInitialization::Unresolved | DecodeInitialization::Hidden | DecodeInitialization::HiddenFinalized) { return Err(invalid(0)); }
        if self.scenes[i].sealed || self.draft_depth >= 64 || self.request_protects_scene(scene) {
            return Err(invalid(0));
        }
        if let Some(parent) = self.active_draft() {
            if scene.index() < parent.scene_count {
                return Err(invalid(0));
            }
        }
        let nonce = self.serial()?;
        let mark = self.checkpoint_unrestricted(scene)?;
        let scene_count = self.scenes.len();
        self.drafts[self.draft_depth] = Some(ActiveDraft {
            scene,
            nonce,
            scene_count,
        });
        self.draft_depth += 1;
        Ok(DraftToken {
            mark,
            nonce,
            scene_count,
            ignored: self.ignored,
            text_commands: self.text_commands,
        })
    }
    pub(crate) fn active_draft(&self) -> Option<ActiveDraft> {
        self.draft_depth.checked_sub(1).and_then(|i| self.drafts[i])
    }
    pub(crate) fn draft_index(&self, token: &DraftToken) -> Result<usize> {
        let active = self.active_draft().ok_or_else(|| invalid(0))?;
        if active.scene != token.mark.scene
            || active.nonce != token.nonce
            || active.scene_count != token.scene_count
        {
            return Err(invalid(0));
        }
        let i = self.index(active.scene)?;
        if self.scenes[i].sealed {
            return Err(invalid(0));
        }
        if self.scenes[i]
            .lengths()
            .iter()
            .zip(token.mark.lengths)
            .any(|(now, then)| *now < then)
        {
            return Err(invalid(0));
        }
        Ok(i)
    }
    pub(crate) fn draft_slot(
        &self,
        token: &DraftToken,
        table: usize,
        slot: usize,
        len: usize,
    ) -> Result<usize> {
        let i = self.draft_index(token)?;
        let first = self
            .request
            .as_ref()
            .map_or(token.mark.lengths[table], |r| {
                if r.mark.scene == token.mark.scene {
                    r.mark.lengths[table].max(token.mark.lengths[table])
                } else {
                    token.mark.lengths[table]
                }
            });
        if slot < first || slot >= len {
            return Err(invalid(0));
        }
        Ok(i)
    }
    /// Validation failure/cancellation leaves the token active and rollbackable.
    pub fn commit_draft(
        &mut self,
        token: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.draft_index(token)?;
        if self.request_protects_draft(token) {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)?;
        crate::validate::scene(self, token.mark.scene, control)?;
        self.draft_depth -= 1;
        self.drafts[self.draft_depth] = None;
        Ok(())
    }
    /// Rollback includes all descendants created after this graph checkpoint.
    /// Cleanup is unconditional even when the caller's work control has stopped.
    pub fn rollback_draft(&mut self, mut token: DraftToken) -> Result<()> {
        if self.request_protects_draft(&token) {
            return Err(invalid(0));
        }
        let i = self.draft_index(&token)?;
        let mut released = 0;
        for scene in &self.scenes[token.scene_count..] {
            released = crate::memory::add(released, scene.storage_bytes()?)?;
        }
        token.mark.epoch = self.scenes[i].epoch;
        self.rollback_internal(token.mark)?;
        self.scenes.truncate(token.scene_count);
        self.stats.live_bytes -= released;
        self.ignored = token.ignored;
        self.text_commands = token.text_commands;
        self.draft_depth -= 1;
        self.drafts[self.draft_depth] = None;
        Ok(())
    }
}
