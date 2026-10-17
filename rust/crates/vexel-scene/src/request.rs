//! Bounded LIFO request scopes. Marks own no prefix data; cleanup is explicit.
use crate::{
    types::{invalid, stopped},
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestToken {
    pub(crate) scene: SceneId,
    pub(crate) draft: u64,
    pub(crate) nonce: u64,
    pub(crate) generation: u64,
}

#[derive(Debug)]
pub(crate) struct ActiveRequest {
    pub(crate) token: RequestToken,
    pub(crate) mark: BuildMark,
    scene_count: usize,
    ignored: [Ignored; 4],
    text_commands: usize,
}

impl SceneBuilder {
    fn top_request(&self) -> Option<&ActiveRequest> {
        self.request_depth
            .checked_sub(1)
            .and_then(|i| self.requests[i].as_ref())
    }
    pub(crate) fn request_identity(&self) -> Option<RequestToken> {
        self.top_request().map(|r| r.token)
    }
    pub(crate) fn request_allows_draft(&self, draft: &DraftToken, token: RequestToken) -> bool {
        let Some(active) = self.top_request().filter(|r| r.token == token) else {
            return false;
        };
        let drafts = &self.drafts[..self.draft_depth];
        let Some(parent) = drafts
            .iter()
            .position(|e| e.is_some_and(|d| d.nonce == active.token.draft))
        else {
            return false;
        };
        drafts
            .iter()
            .position(|e| e.is_some_and(|d| d.nonce == draft.nonce))
            .is_some_and(|i| i >= parent)
    }
    pub(crate) fn request_protects_draft(&self, draft: &DraftToken) -> bool {
        self.requests[..self.request_depth]
            .iter()
            .flatten()
            .any(|r| r.token.draft == draft.nonce)
    }
    pub(crate) fn request_protects_scene(&self, id: SceneId) -> bool {
        self.requests[..self.request_depth]
            .iter()
            .flatten()
            .any(|r| id.index() < r.scene_count)
    }
    pub fn begin_request(
        &mut self,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<RequestToken> {
        self.draft_index(draft)?;
        if self.request_depth != 0 {
            return Err(invalid(0));
        }
        self.open_request(draft, control)
    }
    /// Open an explicit child scope in the current descendant draft. The parent
    /// request must be the current top scope and the child draft must be live.
    pub fn begin_nested_request(
        &mut self,
        parent: &DraftToken,
        parent_request: RequestToken,
        child: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<RequestToken> {
        self.draft_index(child)?;
        let parent_live = self.drafts[..self.draft_depth].iter().flatten().any(|d| {
            d.nonce == parent.nonce
                && d.scene == parent.mark.scene
                && d.scene_count == parent.scene_count
        });
        let Some(active) = self.top_request() else {
            return Err(invalid(0));
        };
        if active.token != parent_request
            || active.token.draft != parent.nonce
            || child.nonce == parent.nonce
            || child.mark.scene == parent.mark.scene
            || child.mark.scene.index() < active.scene_count
            || !parent_live
            || self.request_depth >= 64
            || self.draft_depth > 64
        {
            return Err(invalid(0));
        }
        self.open_request(child, control)
    }
    fn open_request(
        &mut self,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<RequestToken> {
        if self.request_depth >= 64 {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)?;
        let token = RequestToken {
            scene: draft.mark.scene,
            draft: draft.nonce,
            nonce: self.serial()?,
            generation: self.generation,
        };
        let mark = self.checkpoint_internal(token.scene)?;
        self.requests[self.request_depth] = Some(ActiveRequest {
            token,
            mark,
            scene_count: self.scenes.len(),
            ignored: self.ignored,
            text_commands: self.text_commands,
        });
        self.request_depth += 1;
        Ok(token)
    }
    fn request_index(&self, draft: &DraftToken, token: RequestToken) -> Result<usize> {
        let i = self.draft_index(draft)?;
        let active = self.top_request().ok_or_else(|| invalid(0))?;
        if active.token != token
            || token.draft != draft.nonce
            || token.scene != draft.mark.scene
            || token.generation != self.generation
        {
            return Err(invalid(0));
        }
        Ok(i)
    }
    pub fn commit_request(
        &mut self,
        draft: &DraftToken,
        token: RequestToken,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.request_index(draft, token)?;
        control.checkpoint().map_err(stopped)?;
        let from = self.top_request().map_or([0; 28], |r| r.mark.lengths);
        crate::validate::scene_from(self, token.scene, control, &from)?;
        for scene in &self.scenes {
            for admission in &scene.shape_admissions {
                control
                    .charge(crate::WorkKind::TokenVisit, 1)
                    .map_err(stopped)?;
                if admission.active && admission.request == token {
                    return Err(invalid(admission.source_offset));
                }
            }
        }
        self.request_depth -= 1;
        self.requests[self.request_depth] = None;
        Ok(())
    }
    pub fn rollback_request(&mut self, draft: &DraftToken, token: RequestToken) -> Result<()> {
        let i = self.request_index(draft, token)?;
        let active = self.top_request().ok_or_else(|| invalid(0))?;
        let mut released = 0;
        for scene in &self.scenes[active.scene_count..] {
            released = crate::memory::add(released, scene.storage_bytes()?)?;
        }
        self.scenes[i]
            .epoch
            .checked_add(1)
            .ok_or_else(crate::memory::arithmetic)?;
        let active = self.requests[self.request_depth - 1]
            .take()
            .ok_or_else(|| invalid(0))?;
        let mut mark = active.mark;
        mark.epoch = self.scenes[i].epoch;
        self.rollback_internal(mark)?;
        self.scenes.truncate(active.scene_count);
        self.stats.live_bytes -= released;
        self.ignored = active.ignored;
        self.text_commands = active.text_commands;
        self.request_depth -= 1;
        Ok(())
    }
    pub fn abort_request(&mut self, draft: &DraftToken) -> Result<()> {
        self.draft_index(draft)?;
        let token = self.top_request().ok_or_else(|| invalid(0))?.token;
        self.rollback_request(draft, token)
    }
    pub fn admit_text_commands(
        &mut self,
        draft: &DraftToken,
        commands: usize,
        source_offset: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.draft_index(draft)?;
        if self.request_depth == 0 || source_offset > self.source.len() {
            return Err(invalid(source_offset));
        }
        if commands > DOCUMENT_TEXT_COMMAND_LIMIT - self.text_commands {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::TextCommands),
                offset: source_offset,
            });
        }
        let total = self
            .text_commands
            .checked_add(commands)
            .ok_or_else(crate::memory::arithmetic)?;
        control.checkpoint().map_err(stopped)?;
        self.text_commands = total;
        Ok(())
    }
    pub fn document_text_commands(&self) -> usize {
        self.text_commands
    }
    pub(crate) fn static_request_child(&self, child: SceneId) -> Result<RequestToken> {
        let active = self.top_request().ok_or_else(|| invalid(0))?;
        if child.index() < active.scene_count {
            return Err(invalid(0));
        }
        Ok(active.token)
    }
}
pub const DOCUMENT_TEXT_COMMAND_LIMIT: usize = 4 * 1024 * 1024;
#[cfg(test)]
#[path = "request_tests.rs"]
mod incremental_tests;
