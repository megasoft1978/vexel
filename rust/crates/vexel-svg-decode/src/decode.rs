//! Owned decoder lifecycle; source-only continuations never borrow this engine.
use crate::{
    ignored::IgnoredSink,
    invalid,
    tasks::{Context, Task},
};
use vexel_base::{WorkControl, WorkKind};
use vexel_scene::*;
use vexel_xml::TokenKind;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderState {
    Running,
    Complete,
    Failed,
}
pub(crate) enum Shared<'src, 'ctx> {
    Owned(DecodeSource<'src>),
    Borrowed(&'ctx mut DecodeSource<'src>),
}
impl<'src> std::ops::Deref for Shared<'src, '_> {
    type Target = DecodeSource<'src>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(s) => s,
            Self::Borrowed(s) => s,
        }
    }
}
impl std::ops::DerefMut for Shared<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(s) => s,
            Self::Borrowed(s) => s,
        }
    }
}
#[derive(Debug)]
pub enum TraversalDecodeEvent<'a> {
    Progress { charged_units: u64 },
    NeedPattern(PatternRequest<'a, Option<Viewport>, Option<AnimationView<'a>>>),
    Complete,
}
pub(crate) enum Advance {
    Progress { charged_units: u64 },
    Pattern,
    Complete,
}
pub struct Decoder<'src, 'ctx> {
    pub(crate) shared: Shared<'src, 'ctx>,
    pub(crate) builder: Option<SceneBuilder>,
    pub(crate) draft: Option<DraftToken>,
    pub(crate) tasks: Vec<Task<'src>>,
    pub(crate) ends: Vec<usize>,
    pub(crate) root: SceneId,
    pub(crate) state: DecoderState,
    pub(crate) root_seen: bool,
    pub(crate) language: &'ctx [u8],
    pub(crate) media: Option<&'ctx mut dyn MediaProvider>,
    pub(crate) cache: PaintCache,
    pub(crate) text_cache: TextCache,
    pub(crate) ledger: ChargeLedger,
    pub(crate) active: ActiveDefinitions,
    pub(crate) ignored: IgnoredSink,
    pub(crate) visited: Vec<bool>,
    pub(crate) text_states: [TextState; 65],
    pub(crate) visits: usize,
    pub(crate) transform_ops: usize,
    pub(crate) use_instances: usize,
    pub(crate) reference_bytes: usize,
    pub(crate) point_count: usize,
    pub(crate) root_style: Option<CssPaint<'src>>,
    pub(crate) root_local: Affine,
    pub(crate) root_index: usize,
    pub(crate) child_drafts: Vec<(SceneId, DraftToken)>,
    pub(crate) batch_root_shapes: bool,
    pub(crate) shape_requests: [Option<(SceneId, RequestToken, usize)>; 64],
    pub(crate) shape_request_depth: usize,
    pub(crate) custom_request: Option<RequestToken>,
    pub(crate) pending_custom_lease: Option<AllocationLease>,
    pub(crate) child_draft_lease: Option<AllocationLease>,
    pub(crate) pending_pattern: Option<crate::definition_work::PendingPattern>,
    pub(crate) tickets: PatternTickets,
    pub(crate) blocked: Option<crate::ContractBlocker>,
    mode: Option<bool>,
    pub(crate) scoped_static: bool,
    production_job: Option<LedgerIdentity>,
    pub(crate) task_lease: Option<AllocationLease>,
    pub(crate) retained_leases: Vec<AllocationLease>,
    generation: u64,
}
impl<'src, 'ctx> Decoder<'src, 'ctx> {
    pub fn new(
        document: &'src LexicalDocument,
        generation: u64,
        options: DecodeOptions<'ctx>,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        let shared = Shared::Owned(DecodeSource::new(document, control)?);
        Self::construct(document, shared, generation, options, control)
    }
    pub fn from_source(
        document: &'src LexicalDocument,
        shared: &'ctx mut DecodeSource<'src>,
        generation: u64,
        options: DecodeOptions<'ctx>,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        if !std::sync::Arc::ptr_eq(document.source_owner(), shared.source_owner())
            || document.tokens().as_ptr() != shared.tokens().as_ptr()
            || document.tokens().len() != shared.tokens().len()
        {
            return Err(invalid(0));
        }
        Self::construct(
            document,
            Shared::Borrowed(shared),
            generation,
            options,
            control,
        )
    }
    fn construct(
        document: &'src LexicalDocument,
        mut shared: Shared<'src, 'ctx>,
        generation: u64,
        options: DecodeOptions<'ctx>,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        validate_language(options.system_language)?;
        control
            .checkpoint()
            .map_err(|s| crate::source::stopped(s, 0))?;
        let source = shared.source();
        let tokens = shared.tokens();
        let mut ledger = ChargeLedger::new(
            shared.source_owner(),
            options.limits.reserved_bytes,
            1_048_576,
        )?;
        let (mut leases, metadata_lease) =
            crate::storage::allocate(&mut ledger, 16, AllocationTag::None, control)?;
        leases.push(metadata_lease);
        for (tag, bytes) in [
            (AllocationTag::TokenArray, document.token_storage_bytes()?),
            (
                AllocationTag::ReferenceIndex,
                shared.reference_storage_bytes(),
            ),
            (AllocationTag::None, shared.css_storage_bytes()),
        ] {
            leases.push(ledger.admit(tag, bytes)?)
        }
        let (mut ends, ends_lease) =
            crate::storage::allocate(&mut ledger, tokens.len(), AllocationTag::None, control)?;
        ends.resize(tokens.len(), usize::MAX);
        let mut open = [None; 257];
        let mut direct_shapes = 0usize;
        let mut batching_boundary = false;
        for (i, t) in tokens.iter().enumerate() {
            crate::source::charge(control, WorkKind::TokenVisit, 1, t.source.offset)?;
            if t.depth > 256 {
                return Err(Diagnostic {
                    kind: ErrorKind::Limit(LimitKind::Depth),
                    offset: t.name.offset,
                });
            }
            if t.depth == 1
                && matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && crate::source::basic_shape(source, t)
            {
                direct_shapes = direct_shapes.checked_add(1).ok_or(Diagnostic {
                    kind: ErrorKind::Limit(LimitKind::Arithmetic),
                    offset: t.name.offset,
                })?;
            }
            if matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && [
                    b"g".as_slice(),
                    b"defs",
                    b"use",
                    b"switch",
                    b"symbol",
                    b"clipPath",
                    b"pattern",
                    b"mask",
                    b"marker",
                    b"filter",
                    b"style",
                ]
                .iter()
                .any(|name| t.local_name.slice(source) == Some(*name))
            {
                batching_boundary = true;
            }
            if t.kind == TokenKind::Attribute
                && (t.local_name.slice(source) == Some(b"href".as_slice())
                    || t.value
                        .slice(source)
                        .is_some_and(|value| value.windows(5).any(|window| window == b"url(#")))
            {
                batching_boundary = true;
            }
            match t.kind {
                TokenKind::StartElement => open[t.depth] = Some(i),
                TokenKind::EmptyElement => ends[i] = i,
                TokenKind::EndElement => {
                    let first = open[t.depth].take().ok_or(Diagnostic {
                        kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
                        offset: t.name.offset,
                    })?;
                    ends[first] = i;
                }
                _ => {}
            }
        }
        leases.push(ends_lease);
        let (mut visited, visited_lease) =
            crate::storage::allocate(&mut ledger, tokens.len(), AllocationTag::None, control)?;
        visited.resize(tokens.len(), false);
        leases.push(visited_lease);
        let mut ignored = IgnoredSink::default();
        ignored.scan(source, tokens, control)?;
        let root_index = tokens
            .iter()
            .position(|t| matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement))
            .ok_or(Diagnostic {
                kind: ErrorKind::Viewport,
                offset: 0,
            })?;
        for t in &tokens[..root_index] {
            match t.kind {
                TokenKind::ProcessingInstruction => {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: t.name.offset,
                    });
                }
                TokenKind::Cdata => {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: t.value.offset,
                    });
                }
                TokenKind::Text => {
                    for (i, b) in t
                        .value
                        .slice(source)
                        .ok_or_else(|| invalid(t.value.offset))?
                        .iter()
                        .enumerate()
                    {
                        crate::source::charge(
                            control,
                            WorkKind::SourceRead,
                            1,
                            t.value.offset + i,
                        )?;
                        if !matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
                            return Err(Diagnostic {
                                kind: ErrorKind::Unsupported,
                                offset: t.value.offset + i,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        let root_token = tokens[root_index];
        if root_token.depth != 0 || !crate::source::element_name(source, &root_token, b"svg") {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: root_token.name.offset,
            });
        }
        if !crate::ignored::attributes_supported(
            source,
            tokens,
            root_index,
            true,
            &mut visited,
            &mut ignored,
            control,
        )? {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: root_token.name.offset,
            });
        }
        let root_style = shared.compute_style(ElementId(root_index), None, control)?;
        let hidden = crate::style_display_none(&root_style, root_token.name.offset)?;
        let (root_local, transform_ops) = if hidden {
            (vexel_geometry::affine_identity(), 0)
        } else {
            match crate::source::attribute(source, tokens, root_index, b"transform", None) {
                Some(v) => crate::transform::parse_transform_controlled(source, v, control)?,
                None => (vexel_geometry::affine_identity(), 0),
            }
        };
        let mut builder = SceneBuilder::provisional(
            std::sync::Arc::clone(shared.source_owner()),
            generation,
            options.limits,
        )?;
        if hidden {
            builder.initialize_hidden_root(&mut shared, ElementId(root_index), control)?;
        } else {
            builder.initialize_root(&mut shared, ElementId(root_index), control)?;
        }
        let viewport = builder.traversal_scene(builder.root_id())?.viewport;
        let root = builder.root_id();
        let draft = if hidden {
            None
        } else {
            Some(builder.begin_draft(root)?)
        };
        let context = Context {
            scene: root,
            style: CssPaint::initial(),
            transform: vexel_geometry::affine_identity(),
            viewport: viewport.map_or([0.0, 0.0], |v| [v.view_width, v.view_height]),
            depth: 0,
            node: None,
            group: None,
            host: None,
            text: None,
            preserve: false,
            flavor: DecodeFlavor::Normal,
        };
        let (uses, uses_lease) =
            crate::storage::allocate(&mut ledger, 16, AllocationTag::None, control)?;
        let (markers, markers_lease) =
            crate::storage::allocate(&mut ledger, 8, AllocationTag::None, control)?;
        let (masks, masks_lease) =
            crate::storage::allocate(&mut ledger, 8, AllocationTag::None, control)?;
        let (patterns, patterns_lease) =
            crate::storage::allocate(&mut ledger, 8, AllocationTag::None, control)?;
        leases.extend([uses_lease, markers_lease, masks_lease, patterns_lease]);
        let active = ActiveDefinitions {
            uses,
            markers,
            masks,
            patterns,
        };
        let mut value = Self {
            shared,
            builder: Some(builder),
            draft,
            tasks: Vec::new(),
            ends,
            root,
            state: DecoderState::Running,
            root_seen: false,
            language: options.system_language,
            media: options.media,
            cache: PaintCache::default(),
            text_cache: TextCache::default(),
            ledger,
            active,
            ignored,
            visited,
            text_states: [TextState::default(); 65],
            visits: 0,
            transform_ops,
            use_instances: 0,
            reference_bytes: 0,
            point_count: 0,
            root_style: Some(root_style),
            root_local,
            root_index,
            child_drafts: Vec::new(),
            batch_root_shapes: direct_shapes >= 128 && !batching_boundary,
            shape_requests: [None; 64],
            shape_request_depth: 0,
            custom_request: None,
            pending_custom_lease: None,
            child_draft_lease: None,
            pending_pattern: None,
            tickets: PatternTickets::new()?,
            blocked: None,
            mode: None,
            scoped_static: false,
            production_job: None,
            task_lease: None,
            retained_leases: leases,
            generation,
        };
        value.push(
            Task::Walk {
                cursor: 0,
                limit: tokens.len(),
                context,
            },
            control,
        )?;
        Ok(value)
    }
    pub fn blocked_contract(&self) -> Option<crate::ContractBlocker> {
        self.blocked
    }
    pub(crate) fn contract_blocked(
        &mut self,
        kind: crate::ContractBlocker,
        offset: usize,
    ) -> Diagnostic {
        self.blocked = Some(kind);
        Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset,
        }
    }
    pub fn state(&self) -> DecoderState {
        self.state
    }
    /// Actual decoder lease journal, excluding the separately owned builder.
    /// No allocator bookkeeping or RSS estimate is represented by this snapshot.
    pub fn lease_stats(&self) -> RenderMemoryStats {
        self.ledger.stats()
    }
    /// Cumulative caller payload and retained lease-registry capacities for
    /// allocations owned by this decoder. The scene builder is accounted
    /// separately, and this is not allocator or RSS instrumentation.
    pub fn lease_allocation_history(&self) -> [usize; 5] {
        self.ledger.allocation_history()
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn builder(&self) -> Option<&SceneBuilder> {
        self.builder.as_ref()
    }
    pub fn has_media_provider(&self) -> bool {
        self.media.is_some()
    }
    pub(crate) fn builder_mut(&mut self) -> Result<&mut SceneBuilder> {
        self.builder.as_mut().ok_or_else(|| invalid(0))
    }
    pub(crate) fn push(&mut self, task: Task<'src>, control: &mut dyn WorkControl) -> Result<()> {
        crate::storage::reserve(
            &mut self.tasks,
            &mut self.task_lease,
            &mut self.ledger,
            1,
            1_048_576,
            control,
        )?;
        self.tasks.push(task);
        Ok(())
    }
    pub fn step(
        &mut self,
        work_quantum: u64,
        operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'_>> {
        if self.mode == Some(true) {
            return Err(self.abort(invalid(0)));
        }
        self.mode = Some(false);
        match self.step_internal(
            work_quantum,
            &mut crate::operations::Operations::Fixture(operations),
            None,
            control,
        )? {
            Advance::Complete => Ok(DecodeEvent::Complete),
            Advance::Progress { charged_units } => Ok(DecodeEvent::Progress { charged_units }),
            Advance::Pattern => self.pattern_event(None, control),
        }
    }
    /// Explicit legacy delegates with an honest pending traversal view.
    pub fn step_traversal<'s>(
        &'s mut self,
        work_quantum: u64,
        operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<TraversalDecodeEvent<'s>> {
        if self.mode == Some(true) {
            return Err(self.abort(invalid(0)));
        }
        self.mode = Some(false);
        match self.step_internal(
            work_quantum,
            &mut crate::operations::Operations::Fixture(operations),
            None,
            control,
        )? {
            Advance::Complete => Ok(TraversalDecodeEvent::Complete),
            Advance::Progress { charged_units } => {
                Ok(TraversalDecodeEvent::Progress { charged_units })
            }
            Advance::Pattern => self.pattern_event_traversal(None, control),
        }
    }
    /// The caller retains the actual render job across pattern suspensions.
    pub fn step_production<'s>(
        &'s mut self,
        work_quantum: u64,
        operations: &mut dyn crate::DecoderProductionOperations,
        job: &RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<TraversalDecodeEvent<'s>> {
        job.validate_source(self.shared.source_owner())
            .map_err(|e| self.abort(e))?;
        if self.mode == Some(false)
            || self
                .production_job
                .is_some_and(|id| id != job.owner_identity())
        {
            return Err(self.abort(invalid(0)));
        }
        self.mode = Some(true);
        self.scoped_static = false;
        self.production_job = Some(job.owner_identity());
        match self.step_internal(
            work_quantum,
            &mut crate::operations::Operations::Production(operations),
            Some(job),
            control,
        )? {
            Advance::Complete => Ok(TraversalDecodeEvent::Complete),
            Advance::Progress { charged_units } => {
                Ok(TraversalDecodeEvent::Progress { charged_units })
            }
            Advance::Pattern => self.pattern_event_traversal(Some(job), control),
        }
    }
    /// Production route whose suspension carries the accepted static pattern
    /// capability, bound to the open parent request and the caller's job.
    pub fn step_scoped_production<'s>(
        &'s mut self,
        work_quantum: u64,
        operations: &mut dyn ScopedProductionDecodeOperations,
        job: &RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<ScopedDecodeEvent<'s>> {
        job.validate_source(self.shared.source_owner())
            .map_err(|e| self.abort(e))?;
        if self.mode == Some(false)
            || self
                .production_job
                .is_some_and(|id| id != job.owner_identity())
        {
            return Err(self.abort(invalid(0)));
        }
        self.mode = Some(true);
        self.scoped_static = true;
        self.production_job = Some(job.owner_identity());
        match self.step_internal(
            work_quantum,
            &mut crate::operations::Operations::ScopedProduction(operations),
            Some(job),
            control,
        )? {
            Advance::Complete => Ok(ScopedDecodeEvent::Complete),
            Advance::Progress { .. } => Ok(ScopedDecodeEvent::Progress),
            Advance::Pattern => self.static_pattern_event(job, control),
        }
    }
    fn static_pattern_event(
        &mut self,
        job: &RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<ScopedDecodeEvent<'_>> {
        let pending_data = self.pending_pattern.as_ref().map(|pending| {
            (
                pending.offered,
                pending.mapping,
                pending.parent,
                pending.child,
                pending.descriptor,
                pending.owner,
                pending.channel,
            )
        });
        let Some((offered, mapping, parent, child, descriptor, owner, channel)) = pending_data
        else {
            return Err(self.abort(invalid(0)));
        };
        if offered {
            return Err(self.abort(invalid(descriptor.error_offset)));
        }
        let Some(mapping) = mapping else {
            return Err(self.abort(invalid(descriptor.error_offset)));
        };
        let draft = match draft_token(parent, self.root, &self.draft, &self.child_drafts) {
            Ok(draft) => draft,
            Err(error) => return Err(self.abort(error)),
        };
        let builder = self.builder.as_ref().ok_or_else(|| invalid(0))?;
        let request = self.tickets.request_mapped_static(
            builder,
            &draft,
            &self.shared,
            child,
            descriptor,
            mapping,
            owner,
            channel,
            job,
            control,
        )?;
        if let Some(pending) = self.pending_pattern.as_mut() {
            pending.offered = true;
        }
        Ok(ScopedDecodeEvent::Pattern(request))
    }
    fn step_internal(
        &mut self,
        work_quantum: u64,
        operations: &mut crate::operations::Operations<'_>,
        _job: Option<&RasterJob>,
        control: &mut dyn WorkControl,
    ) -> Result<Advance> {
        if self.state == DecoderState::Failed {
            return Err(invalid(0));
        }
        if self.state == DecoderState::Complete {
            return Ok(Advance::Complete);
        }
        if work_quantum == 0 {
            return Err(invalid(0));
        }
        if let Some(pending) = &self.pending_pattern {
            let offset = pending.descriptor.error_offset;
            if pending.offered {
                return Err(self.abort(invalid(offset)));
            }
            control
                .checkpoint()
                .map_err(|s| self.abort(crate::source::stopped(s, offset)))?;
            return Ok(Advance::Pattern);
        }
        let mut quantum = DecodeQuantum::new(work_quantum)?;
        while quantum.work().total_work < work_quantum {
            let Some(task) = self.tasks.pop() else {
                if !self.root_seen {
                    return Err(self.abort(Diagnostic {
                        kind: ErrorKind::Viewport,
                        offset: 0,
                    }));
                }
                // This loop runs only while the quantum has budget, so this one
                // atomic step always executes. Splitting the publish and the
                // finalization into two atomic steps would let the second become a
                // boundary that is skipped, completing an un-finalized scene.
                quantum
                    .atomic(control, |c| {
                        self.flush_shape_request(c)?;
                        self.ignored
                            .publish(self.builder.as_mut().ok_or_else(|| invalid(0))?, c)?;
                        self.finalize_animation(c)
                    })
                    .map_err(|e| self.abort(e))?;
                self.state = DecoderState::Complete;
                return Ok(Advance::Complete);
            };
            if let Err(e) = quantum.atomic(control, |c| self.process(task, operations, c)) {
                return Err(self.abort(e));
            }
            if let Some(pending) = &self.pending_pattern {
                let offset = pending.descriptor.error_offset;
                control
                    .checkpoint()
                    .map_err(|s| self.abort(crate::source::stopped(s, offset)))?;
                return Ok(Advance::Pattern);
            }
        }
        Ok(Advance::Progress {
            charged_units: quantum.work().total_work,
        })
    }
    pub(crate) fn abort(&mut self, error: Diagnostic) -> Diagnostic {
        self.tasks.clear();
        if let Some(lease) = self.pending_custom_lease.take() {
            let _ = self.ledger.release(lease);
        }
        self.tickets.cancel();
        self.pending_pattern = None;
        self.custom_request = None;
        while self.shape_request_depth != 0 {
            let index = self.shape_request_depth - 1;
            let Some((scene, request, _)) = self.shape_requests[index].take() else {
                self.shape_request_depth = index;
                continue;
            };
            if let (Some(builder), Ok(draft)) = (
                self.builder.as_mut(),
                draft_token(scene, self.root, &self.draft, &self.child_drafts),
            ) {
                let _ = builder.rollback_request(draft, request);
            }
            self.shape_request_depth = index;
        }
        while let Some((_, token)) = self.child_drafts.pop() {
            if let Some(builder) = self.builder.as_mut() {
                let _ = builder.rollback_draft(token);
            }
        }
        self.active.uses.clear();
        if let (Some(b), Some(t)) = (self.builder.as_mut(), self.draft.take()) {
            let _ = b.rollback_draft(t);
        }
        self.state = DecoderState::Failed;
        error
    }
}
impl DecodeSession for Decoder<'_, '_> {
    fn step<'s>(
        &'s mut self,
        q: u64,
        o: &mut dyn DecodeOperations,
        c: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'s>> {
        Decoder::step(self, q, o, c)
    }
    fn resume_pattern(
        &mut self,
        t: PatternTicket,
        r: Result<vexel_raster::Tile>,
        c: &mut dyn WorkControl,
    ) -> Result<()> {
        Decoder::resume_pattern(self, t, r, c)
    }
    fn finish(self, c: &mut dyn WorkControl) -> Result<SceneGraph> {
        Decoder::finish(self, c)
    }
}
impl ScopedProductionDecodeSession for Decoder<'_, '_> {
    fn step<'s>(
        &'s mut self,
        quantum: u64,
        job: &RasterJob,
        operations: &mut dyn ScopedProductionDecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<ScopedDecodeEvent<'s>> {
        Decoder::step_scoped_production(self, quantum, operations, job, control)
    }
    fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.resume_pattern_production(ticket, job, result, control)
    }
    fn finish(self, control: &mut dyn WorkControl) -> Result<SceneGraph> {
        Decoder::finish(self, control)
    }
}
fn validate_language(language: &[u8]) -> Result<()> {
    if language.is_empty() || language.len() > 64 {
        return Err(invalid(0));
    }
    for (i, b) in language.iter().copied().enumerate() {
        if !(b.is_ascii_alphanumeric() || b == b'-')
            || b == b'-' && (i == 0 || i + 1 == language.len() || language[i - 1] == b'-')
        {
            return Err(invalid(i));
        }
    }
    Ok(())
}
pub(crate) fn draft_token<'a>(
    scene: SceneId,
    root: SceneId,
    root_token: &'a Option<DraftToken>,
    children: &'a [(SceneId, DraftToken)],
) -> Result<&'a DraftToken> {
    if let Some((child, token)) = children.last() {
        if *child == scene {
            return Ok(token);
        }
    }
    if scene == root {
        return root_token.as_ref().ok_or_else(|| crate::invalid(0));
    }
    Err(crate::invalid(0))
}
