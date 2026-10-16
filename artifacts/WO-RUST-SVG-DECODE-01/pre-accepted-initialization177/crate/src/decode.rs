//! Owned decoder lifecycle; source-only continuations never borrow this engine.
use crate::{
    ignored::IgnoredSink,
    invalid,
    tasks::{Context, Task},
};
use vexel_base::{Stop, WorkControl, WorkKind};
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
    pub(crate) child_draft_lease: Option<AllocationLease>,
    pub(crate) pending_pattern: Option<crate::definition_work::PendingPattern>,
    pub(crate) tickets: PatternTickets,
    pub(crate) blocked: Option<crate::ContractBlocker>,
    mode: Option<bool>,
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
        let mut leases = Vec::new();
        leases.try_reserve_exact(16).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
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
        let mut ends = Vec::new();
        ends.try_reserve_exact(tokens.len())
            .map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: 0,
            })?;
        ends.resize(tokens.len(), usize::MAX);
        let mut open = [None; 257];
        for (i, t) in tokens.iter().enumerate() {
            crate::source::charge(control, WorkKind::TokenVisit, 1, t.source.offset)?;
            if t.depth > 256 {
                return Err(Diagnostic {
                    kind: ErrorKind::Limit(LimitKind::Depth),
                    offset: t.name.offset,
                });
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
        leases.push(ledger.admit_vec(AllocationTag::None, &ends)?);
        let mut visited = Vec::new();
        visited
            .try_reserve_exact(tokens.len())
            .map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: 0,
            })?;
        visited.resize(tokens.len(), false);
        leases.push(ledger.admit_vec(AllocationTag::None, &visited)?);
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
                    })
                }
                TokenKind::Cdata => {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: t.value.offset,
                    })
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
        crate::style_display_none(&root_style, root_token.name.offset)?;
        let (root_local, transform_ops) =
            match crate::source::attribute(source, tokens, root_index, b"transform", None) {
                Some(v) => crate::transform::parse_transform_controlled(source, v, control)?,
                None => (vexel_geometry::affine_identity(), 0),
            };
        let (viewport, _, _) =
            crate::viewport::root_viewport_controlled(source, tokens, root_index, control)?;
        let animation =
            vexel_svg_animation::parse_owned_controlled(shared.source_owner(), tokens, control)
                .map_err(|e| match e {
                    vexel_base::ControlledError::Original(
                        vexel_svg_animation::ParseError::TrackLimit { offset },
                    ) => Diagnostic {
                        kind: ErrorKind::Limit(LimitKind::Animation),
                        offset,
                    },
                    vexel_base::ControlledError::Original(
                        vexel_svg_animation::ParseError::Allocation,
                    ) => Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset: 0,
                    },
                    vexel_base::ControlledError::Canceled => {
                        crate::source::stopped(Stop::Canceled, 0)
                    }
                    vexel_base::ControlledError::Limit => crate::source::stopped(Stop::Limit, 0),
                    vexel_base::ControlledError::Overflow => {
                        crate::source::stopped(Stop::Overflow, 0)
                    }
                    vexel_base::ControlledError::OutOfMemory => Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset: 0,
                    },
                })?;
        let mut builder = SceneBuilder::new(
            std::sync::Arc::clone(shared.source_owner()),
            animation,
            generation,
            viewport,
            options.limits,
        )?;
        let root = builder.root_id();
        let draft = builder.begin_draft(root)?;
        let context = Context {
            scene: root,
            style: CssPaint::initial(),
            transform: vexel_geometry::affine_identity(),
            viewport: [viewport.view_width, viewport.view_height],
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
        let metadata_lease = ledger.admit_vec(AllocationTag::None, &leases)?;
        leases.push(metadata_lease);
        let active = ActiveDefinitions {
            uses,
            markers,
            masks,
            patterns,
        };
        let mut value = Self {
            shared,
            builder: Some(builder),
            draft: Some(draft),
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
            child_draft_lease: None,
            pending_pattern: None,
            tickets: PatternTickets::new()?,
            blocked: None,
            mode: None,
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
        self.step_internal(
            work_quantum,
            &mut crate::operations::Operations::Fixture(operations),
            None,
            control,
        )
    }
    /// The caller retains the actual render job across pattern suspensions.
    pub fn step_production<'s>(
        &'s mut self,
        work_quantum: u64,
        operations: &mut dyn ProductionDecodeOperations,
        job: &RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'s>> {
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
        self.production_job = Some(job.owner_identity());
        self.step_internal(
            work_quantum,
            &mut crate::operations::Operations::Production(operations),
            Some(job),
            control,
        )
    }
    fn step_internal(
        &mut self,
        work_quantum: u64,
        operations: &mut crate::operations::Operations<'_>,
        job: Option<&RasterJob>,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'_>> {
        if self.state == DecoderState::Failed {
            return Err(invalid(0));
        }
        if self.state == DecoderState::Complete {
            return Ok(DecodeEvent::Complete);
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
            return self.pattern_event(job, control);
        }
        let mut quantum = DecodeQuantum::new(work_quantum)?;
        while quantum.work().total_work < work_quantum {
            let Some(task) = self.tasks.pop() else {
                quantum
                    .atomic(control, |c| {
                        self.ignored
                            .publish(self.builder.as_mut().ok_or_else(|| invalid(0))?, c)
                    })
                    .map_err(|e| self.abort(e))?;
                if !self.root_seen {
                    return Err(self.abort(Diagnostic {
                        kind: ErrorKind::Viewport,
                        offset: 0,
                    }));
                }
                self.state = DecoderState::Complete;
                return Ok(DecodeEvent::Complete);
            };
            if let Err(e) = quantum.atomic(control, |c| self.process(task, operations, c)) {
                return Err(self.abort(e));
            }
            if let Some(pending) = &self.pending_pattern {
                let offset = pending.descriptor.error_offset;
                control
                    .checkpoint()
                    .map_err(|s| self.abort(crate::source::stopped(s, offset)))?;
                return self.pattern_event(job, control);
            }
        }
        Ok(DecodeEvent::Progress {
            charged_units: quantum.work().total_work,
        })
    }
    pub(crate) fn abort(&mut self, error: Diagnostic) -> Diagnostic {
        self.tasks.clear();
        self.tickets.cancel();
        self.pending_pattern = None;
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
