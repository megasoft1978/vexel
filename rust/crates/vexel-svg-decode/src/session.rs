//! Pattern suspension and final scene ownership.
use crate::{
    decode::{Decoder, DecoderState, draft_token},
    invalid,
};
use vexel_scene::*;
impl Decoder<'_, '_> {
    pub(crate) fn pattern_event(
        &mut self,
        job: Option<&RasterJob>,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'_>> {
        // The compatibility event promises a render-ready animation view.
        // Never manufacture one while the parent parser is still pending.
        if self
            .builder
            .as_ref()
            .ok_or_else(|| invalid(0))?
            .initialization()
            != DecodeInitialization::Finalized
        {
            let error = self.contract_blocked(crate::ContractBlocker::LegacyPatternReadyView, 0);
            return Err(self.abort(error));
        }
        let (ticket, child, descriptor) = self
            .prepare_pattern_event(job, control)
            .map_err(|e| self.abort(e))?;
        // Only the transient view borrow is reconstructed. The real shared
        // request minted/validated this ticket and these exact descriptor fields.
        let child = self
            .builder
            .as_ref()
            .ok_or_else(|| invalid(0))?
            .scene(child)?;
        Ok(DecodeEvent::NeedPattern(PatternRequest {
            ticket,
            child,
            definition: descriptor.definition,
            width: descriptor.width,
            height: descriptor.height,
            tile_width: descriptor.tile_width,
            tile_height: descriptor.tile_height,
            content_transform: descriptor.content_transform,
            error_offset: descriptor.error_offset,
        }))
    }
    pub(crate) fn pattern_event_traversal(
        &mut self,
        job: Option<&RasterJob>,
        control: &mut dyn WorkControl,
    ) -> Result<crate::TraversalDecodeEvent<'_>> {
        let (ticket, child, descriptor) = self
            .prepare_pattern_event(job, control)
            .map_err(|e| self.abort(e))?;
        let child = self
            .builder
            .as_ref()
            .ok_or_else(|| invalid(0))?
            .traversal_scene(child)?;
        Ok(crate::TraversalDecodeEvent::NeedPattern(PatternRequest {
            ticket,
            child,
            definition: descriptor.definition,
            width: descriptor.width,
            height: descriptor.height,
            tile_width: descriptor.tile_width,
            tile_height: descriptor.tile_height,
            content_transform: descriptor.content_transform,
            error_offset: descriptor.error_offset,
        }))
    }
    fn prepare_pattern_event(
        &mut self,
        job: Option<&RasterJob>,
        control: &mut dyn WorkControl,
    ) -> Result<(PatternTicket, SceneId, PatternDescriptor)> {
        let pending = self
            .pending_pattern
            .as_mut()
            .ok_or_else(|| crate::invalid(0))?;
        if pending.offered {
            return Err(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset: pending.descriptor.error_offset,
            });
        }
        let child = pending.child;
        let descriptor = pending.descriptor;
        control
            .checkpoint()
            .map_err(|s| crate::source::stopped(s, descriptor.error_offset))?;
        let builder = self.builder.as_ref().ok_or_else(|| crate::invalid(0))?;
        let request = if let Some(mapping) = pending.mapping {
            self.tickets.request_mapped_traversal(
                builder,
                draft_token(pending.parent, self.root, &self.draft, &self.child_drafts)?,
                &self.shared,
                child,
                descriptor,
                mapping,
                pending.owner,
                pending.channel,
                job.ok_or_else(|| crate::invalid(descriptor.error_offset))?,
                control,
            )?
        } else {
            self.tickets
                .request(builder.traversal_scene(child)?, descriptor, control)?
        };
        pending.offered = true;
        Ok((request.ticket, child, descriptor))
    }
    pub fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        result: Result<vexel_raster::Tile>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let pending = self
            .pending_pattern
            .take()
            .ok_or_else(|| self.abort(crate::invalid(0)))?;
        if !pending.offered {
            return Err(self.abort(crate::invalid(pending.descriptor.error_offset)));
        }
        let tile = result.map_err(|e| self.abort(e))?;
        let (child, pattern) = self
            .tickets
            .complete(ticket, tile, control)
            .map_err(|e| self.abort(e))?;
        if child != pending.child {
            return Err(self.abort(crate::invalid(pending.descriptor.error_offset)));
        }
        let view = self
            .builder
            .as_ref()
            .ok_or_else(|| crate::invalid(0))?
            .traversal_scene(pending.parent)?;
        if view.shape(pending.owner).is_none() {
            return Err(self.abort(crate::invalid(pending.descriptor.error_offset)));
        }
        // The accepted descriptor lacks the instance paint mapping. Preserve the
        // completed tile ownership but never publish a guessed mapping or success.
        drop(pattern);
        let _ = pending.channel;
        let error = self.contract_blocked(
            crate::ContractBlocker::PatternPaintHandoff,
            pending.descriptor.error_offset,
        );
        Err(self.abort(error))
    }
    /// Complete the exact ticket in the same external job used by step_production.
    pub fn resume_pattern_production(
        &mut self,
        ticket: PatternTicket,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let owned = result.as_ref().ok().map(|v| v.pixels);
        let pending_scope = self
            .pending_pattern
            .as_ref()
            .map(|pending| (pending.parent, pending.owner.index(), pending.request));
        let completed = (|| {
            let pending = self.pending_pattern.take().ok_or_else(|| invalid(0))?;
            if !pending.offered || pending.mapping.is_none() {
                return Err(invalid(pending.descriptor.error_offset));
            }
            self.tickets
                .complete_mapped(
                    ticket,
                    self.builder.as_mut().ok_or_else(|| invalid(0))?,
                    draft_token(pending.parent, self.root, &self.draft, &self.child_drafts)?,
                    job,
                    result,
                    control,
                )
                .map(|_| ())
        })();
        if completed.is_err() {
            if let Some(pixels) = owned {
                if job.slice(pixels).is_ok() {
                    job.release(pixels)?;
                }
            }
        }
        completed.map_err(|e| self.abort(e))?;
        let followup_pattern = pending_scope.is_some_and(|(scene, shape, _)| {
            matches!(self.tasks.last(), Some(crate::tasks::Task::PatternPlan { node, .. })
                if node.context.scene == scene && node.first_shape == shape)
        });
        if let Some((_, _, Some(request))) = pending_scope {
            let custom_owner = self.custom_request == Some(request);
            if !custom_owner
                && !followup_pattern
                && self
                    .shape_request_top()
                    .is_some_and(|(_, active, _)| active == request)
            {
                self.flush_shape_request(control).map_err(|e| self.abort(e))
            } else {
                Ok(())
            }
        } else if self.custom_request.is_none() {
            self.flush_shape_request(control).map_err(|e| self.abort(e))
        } else {
            Ok(())
        }
    }

    /// Compatibility drawable finish. Hidden documents use finish_decoded.
    pub fn finish(self, control: &mut dyn WorkControl) -> Result<SceneGraph> {
        match self.finish_decoded(control)? {
            DecodedScene::Drawable(graph) => Ok(graph),
            DecodedScene::Hidden(_) => Err(invalid(0)),
        }
    }
    pub fn finish_decoded(mut self, control: &mut dyn WorkControl) -> Result<DecodedScene> {
        if self.state != DecoderState::Complete
            || !self.tasks.is_empty()
            || self.pending_pattern.is_some()
            || self.tickets.outstanding()
        {
            return Err(self.abort(invalid(0)));
        }
        if let Some(t) = self.draft.take() {
            if let Err(e) = self.builder_mut()?.commit_draft(&t, control) {
                let _ = self.builder_mut()?.rollback_draft(t);
                return Err(e);
            }
        }
        let graph = self
            .builder
            .take()
            .ok_or_else(|| invalid(0))?
            .freeze_decode(control)?;
        for lease in self.retained_leases.drain(..) {
            self.ledger.release(lease)?;
        }
        if let Some(lease) = self.task_lease.take() {
            self.ledger.release(lease)?;
        }
        if let Some(lease) = self.child_draft_lease.take() {
            self.ledger.release(lease)?;
        }
        Ok(graph)
    }
}
