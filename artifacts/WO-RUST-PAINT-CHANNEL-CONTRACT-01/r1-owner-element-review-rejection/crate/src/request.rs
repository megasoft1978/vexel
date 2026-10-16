//! One bounded request scope inside the current draft. Marks own no prefix data.
//! Discarding a token does not commit: explicitly abort with the outer draft.
use crate::{
    types::{invalid, stopped},
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestToken {
    scene: SceneId,
    draft: u64,
    nonce: u64,
    generation: u64,
}
#[derive(Debug)]
pub(crate) struct ActiveRequest {
    token: RequestToken,
    pub(crate) mark: BuildMark,
    scene_count: usize,
    ignored: [Ignored; 4],
    text_commands: usize,
}
impl SceneBuilder {
    pub(crate) fn request_protects_draft(&self, draft: &DraftToken) -> bool {
        self.request
            .as_ref()
            .is_some_and(|r| r.token.draft == draft.nonce)
    }
    pub(crate) fn request_protects_scene(&self, id: SceneId) -> bool {
        self.request
            .as_ref()
            .is_some_and(|r| id.index() < r.scene_count)
    }
    pub(crate) fn request_identity(&self) -> Option<RequestToken> {
        self.request.as_ref().map(|r| r.token)
    }
    pub fn begin_request(
        &mut self,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<RequestToken> {
        self.draft_index(draft)?;
        if self.request.is_some() {
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
        self.request = Some(ActiveRequest {
            token,
            mark,
            scene_count: self.scenes.len(),
            ignored: self.ignored,
            text_commands: self.text_commands,
        });
        Ok(token)
    }
    fn request_index(&self, draft: &DraftToken, token: RequestToken) -> Result<usize> {
        let i = self.draft_index(draft)?;
        let active = self.request.as_ref().ok_or_else(|| invalid(0))?;
        if active.token != token
            || token.draft != draft.nonce
            || token.scene != draft.mark.scene
            || token.generation != self.generation
        {
            return Err(invalid(0));
        }
        Ok(i)
    }
    /// Failure leaves the request active for unconditional cleanup. Prefix handles
    /// retain their insertion serials; no whole-scene epoch is an ID authority.
    pub fn commit_request(
        &mut self,
        draft: &DraftToken,
        token: RequestToken,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.request_index(draft, token)?;
        control.checkpoint().map_err(stopped)?;
        crate::validate::scene(self, token.scene, control)?;
        self.request = None;
        Ok(())
    }
    pub fn rollback_request(&mut self, draft: &DraftToken, token: RequestToken) -> Result<()> {
        let i = self.request_index(draft, token)?;
        let active = self.request.as_ref().ok_or_else(|| invalid(0))?;
        let mut released = 0;
        for scene in &self.scenes[active.scene_count..] {
            released = crate::memory::add(released, scene.storage_bytes()?)?;
        }
        // Prevalidate cleanup arithmetic before taking the checkpoint.
        self.scenes[i]
            .epoch
            .checked_add(1)
            .ok_or_else(crate::memory::arithmetic)?;
        let active = self.request.take().ok_or_else(|| invalid(0))?;
        let mut mark = active.mark;
        mark.epoch = self.scenes[i].epoch;
        self.rollback_internal(mark)?;
        self.scenes.truncate(active.scene_count);
        self.stats.live_bytes -= released;
        self.ignored = active.ignored;
        self.text_commands = active.text_commands;
        Ok(())
    }
    /// Recover an abandoned token. Only the same still-active outer draft can
    /// abort. No control polling, allocation, implicit commit or Drop callback.
    pub fn abort_request(&mut self, draft: &DraftToken) -> Result<()> {
        self.draft_index(draft)?;
        let token = self.request.as_ref().ok_or_else(|| invalid(0))?.token;
        self.rollback_request(draft, token)
    }
}

impl SceneBuilder {
    /// Actual document uses, including cached layouts. Cache admission is separate.
    /// Admit after provider/layout and glyph validation, before outline publication.
    /// The caller must end this request with commit or unconditional rollback.
    pub fn admit_text_commands(
        &mut self,
        draft: &DraftToken,
        commands: usize,
        source_offset: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.draft_index(draft)?;
        if self.request.is_none() || source_offset > self.source.len() {
            return Err(invalid(source_offset));
        }
        // Original subtraction priority: huge input is TextCommands, not arithmetic.
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
}
pub const DOCUMENT_TEXT_COMMAND_LIMIT: usize = 4 * 1024 * 1024;
