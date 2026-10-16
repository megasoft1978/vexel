//! Bounded LIFO scene request transactions, including nested pattern children.
use crate::decode::{Decoder, draft_token};
use crate::invalid;
use vexel_base::WorkControl;
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, RequestToken, Result, SceneId};

impl Decoder<'_, '_> {
    pub(crate) fn ensure_shape_request(
        &mut self,
        scene: SceneId,
        control: &mut dyn WorkControl,
    ) -> Result<RequestToken> {
        let top = self.shape_request_top();
        if let Some((active_scene, request, _)) = top {
            if active_scene == scene {
                return Ok(request);
            }
        }
        if self.shape_request_depth == self.shape_requests.len() {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Depth),
                offset: 0,
            });
        }
        let draft = draft_token(scene, self.root, &self.draft, &self.child_drafts)?;
        let builder = self.builder.as_mut().ok_or_else(|| invalid(0))?;
        let request = if let Some((parent_scene, parent_request, _)) = top {
            let parent = draft_token(parent_scene, self.root, &self.draft, &self.child_drafts)?;
            builder.begin_nested_request(parent, parent_request, draft, control)?
        } else {
            builder.begin_request(draft, control)?
        };
        self.shape_requests[self.shape_request_depth] = Some((scene, request, 0));
        self.shape_request_depth += 1;
        Ok(request)
    }

    pub(crate) fn shape_request_top(&self) -> Option<(SceneId, RequestToken, usize)> {
        self.shape_request_depth
            .checked_sub(1)
            .and_then(|index| self.shape_requests[index])
    }

    pub(crate) fn clear_shape_request_top(&mut self) {
        if self.shape_request_depth != 0 {
            self.shape_request_depth -= 1;
            self.shape_requests[self.shape_request_depth] = None;
        }
    }

    pub(crate) fn flush_shape_request(&mut self, control: &mut dyn WorkControl) -> Result<()> {
        let Some((scene, request, _)) = self.shape_request_top() else {
            return Ok(());
        };
        let draft = draft_token(scene, self.root, &self.draft, &self.child_drafts)?;
        let builder = self.builder.as_mut().ok_or_else(|| invalid(0))?;
        if let Err(error) = builder.commit_request(draft, request, control) {
            builder.rollback_request(draft, request)?;
            self.clear_shape_request_top();
            return Err(error);
        }
        self.clear_shape_request_top();
        Ok(())
    }
}
