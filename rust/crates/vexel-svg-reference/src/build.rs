use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuildPhase {
    Validate,
    Count,
    Zero,
    Insert,
    Complete,
    Failed,
}

/// Incremental index builder. It borrows the original input and owns only its
/// bounded slot table; unfinished sessions can be dropped at any point.
pub struct ReferenceBuildSession<'src> {
    source: &'src [u8],
    tokens: &'src [XmlToken],
    phase: BuildPhase,
    token: usize,
    span: usize,
    token_charged: bool,
    attr: usize,
    active_element: Option<usize>,
    id_count: usize,
    slots: Vec<u32>,
    zeroed: usize,
    current_id: Option<XmlSpan>,
    hash: u64,
    hash_pos: usize,
    hash_ready: bool,
    slot: usize,
    compare_id: Option<XmlSpan>,
    compare_pos: usize,
    compare_attr: usize,
    compare_search: bool,
    terminal: Option<Result<(), ControlledError<ReferenceDiagnostic>>>,
}

impl<'src> ReferenceBuildSession<'src> {
    pub fn new(source: &'src [u8], tokens: &'src [XmlToken]) -> Self {
        Self {
            source,
            tokens,
            phase: BuildPhase::Validate,
            token: 0,
            span: 0,
            token_charged: false,
            attr: 0,
            active_element: None,
            id_count: 0,
            slots: Vec::new(),
            zeroed: 0,
            current_id: None,
            hash: 1_469_598_103_934_665_603,
            hash_pos: 0,
            hash_ready: false,
            slot: 0,
            compare_id: None,
            compare_pos: 0,
            compare_attr: 0,
            compare_search: false,
            terminal: None,
        }
    }

    fn fail(
        &mut self,
        error: ControlledError<ReferenceDiagnostic>,
    ) -> ControlledStep<(), ReferenceDiagnostic> {
        self.phase = BuildPhase::Failed;
        self.terminal = Some(Err(error));
        ControlledStep::Failed(error)
    }

    pub fn step(
        &mut self,
        quantum: u64,
        control: &mut dyn WorkControl,
    ) -> ControlledStep<(), ReferenceDiagnostic> {
        if self.phase == BuildPhase::Complete {
            return ControlledStep::Complete(());
        }
        if self.phase == BuildPhase::Failed {
            return ControlledStep::Failed(self.terminal.unwrap().unwrap_err());
        }
        if quantum == 0 {
            return match control.checkpoint() {
                Ok(()) => ControlledStep::Pending { charged_units: 0 },
                Err(s) => self.fail(s.into()),
            };
        }
        if let Err(stop) = control.checkpoint() {
            return self.fail(stop.into());
        }
        let mut charged = 0u64;
        while charged < quantum {
            let result = self.build_one(control, &mut charged, quantum);
            match result {
                Ok(true) => {
                    self.phase = BuildPhase::Complete;
                    self.terminal = Some(Ok(()));
                    return ControlledStep::Complete(());
                }
                Ok(false) => {}
                Err(error) => return self.fail(error),
            }
            if charged > 0 && charged % 256 == 0 {
                if let Err(stop) = control.checkpoint() {
                    return self.fail(stop.into());
                }
            }
        }
        if let Err(stop) = control.checkpoint() {
            return self.fail(stop.into());
        }
        ControlledStep::Pending {
            charged_units: charged,
        }
    }

    fn charge(
        &self,
        control: &mut dyn WorkControl,
        charged: &mut u64,
        quantum: u64,
        kind: WorkKind,
    ) -> Result<bool, ControlledError<ReferenceDiagnostic>> {
        if *charged >= quantum {
            return Ok(false);
        }
        control.charge(kind, 1).map_err(ControlledError::from)?;
        *charged += 1;
        Ok(true)
    }

    fn build_one(
        &mut self,
        control: &mut dyn WorkControl,
        charged: &mut u64,
        quantum: u64,
    ) -> Result<bool, ControlledError<ReferenceDiagnostic>> {
        match self.phase {
            BuildPhase::Validate => {
                if self.token == self.tokens.len() {
                    self.phase = BuildPhase::Count;
                    self.token = 0;
                    return Ok(false);
                }
                if !self.token_charged {
                    if !self.charge(control, charged, quantum, WorkKind::TokenVisit)? {
                        return Ok(false);
                    }
                    self.token_charged = true;
                }
                let spans = [
                    self.tokens[self.token].source,
                    self.tokens[self.token].name,
                    self.tokens[self.token].value,
                    self.tokens[self.token].prefix,
                    self.tokens[self.token].local_name,
                ];
                if self.span < spans.len() {
                    if !self.charge(control, charged, quantum, WorkKind::SourceRead)? {
                        return Ok(false);
                    }
                    let span = spans[self.span];
                    self.span += 1;
                    if span_bytes(self.source, span).is_none() {
                        return Err(ControlledError::Original(ReferenceDiagnostic::new(
                            ReferenceError::InvalidArgument,
                            span.offset,
                        )));
                    }
                } else {
                    self.token += 1;
                    self.span = 0;
                    self.token_charged = false;
                }
            }
            BuildPhase::Count => {
                if self.token == self.tokens.len() {
                    if self.id_count == 0 {
                        self.phase = BuildPhase::Complete;
                        return Ok(true);
                    }
                    let mut candidate = Vec::new();
                    candidate
                        .try_reserve_exact(REFERENCE_SLOT_CAPACITY)
                        .map_err(|_| ControlledError::OutOfMemory)?;
                    self.slots = retain_exact_slot_candidate(candidate).ok_or_else(|| {
                        ControlledError::Original(ReferenceDiagnostic::new(
                            ReferenceError::Limit,
                            0,
                        ))
                    })?;
                    self.phase = BuildPhase::Zero;
                    return Ok(false);
                }
                if self.active_element.is_none() {
                    if !self.charge(control, charged, quantum, WorkKind::TokenVisit)? {
                        return Ok(false);
                    }
                    let i = self.token;
                    self.token += 1;
                    if is_element(self.tokens[i].kind) {
                        self.active_element = Some(i);
                        self.attr = i + 1;
                    }
                } else {
                    let i = self.active_element.unwrap();
                    if !self.charge(control, charged, quantum, WorkKind::TokenVisit)? {
                        return Ok(false);
                    }
                    if self
                        .tokens
                        .get(self.attr)
                        .is_some_and(|a| a.kind == TokenKind::Attribute)
                    {
                        let a = self.attr;
                        self.attr += 1;
                        if self.tokens[a].prefix.length == 0
                            && span_eq(self.source, self.tokens[a].local_name, b"id")
                        {
                            self.id_count = self.id_count.checked_add(1).ok_or_else(|| {
                                ControlledError::Original(ReferenceDiagnostic::new(
                                    ReferenceError::Limit,
                                    self.tokens[i].name.offset,
                                ))
                            })?;
                            if self.id_count > MAX_REFERENCE_IDS {
                                return Err(ControlledError::Original(ReferenceDiagnostic::new(
                                    ReferenceError::Limit,
                                    self.tokens[i].name.offset,
                                )));
                            }
                            self.active_element = None;
                        }
                    } else {
                        self.active_element = None;
                    }
                }
            }
            BuildPhase::Zero => {
                if self.zeroed == REFERENCE_SLOT_CAPACITY {
                    self.phase = BuildPhase::Insert;
                    self.token = 0;
                    return Ok(false);
                }
                let units = (quantum - *charged)
                    .min((REFERENCE_SLOT_CAPACITY - self.zeroed) as u64)
                    .min(256);
                control
                    .charge(WorkKind::TokenVisit, units)
                    .map_err(ControlledError::from)?;
                *charged += units;
                let next = self.zeroed + units as usize;
                self.slots.resize(next, 0);
                self.zeroed = next;
            }
            BuildPhase::Insert => {
                if self.current_id.is_none() && self.hash_pos == 0 && self.compare_id.is_none() {
                    if self.active_element.is_none() {
                        if self.token == self.tokens.len() {
                            return Ok(true);
                        }
                        if !self.charge(control, charged, quantum, WorkKind::TokenVisit)? {
                            return Ok(false);
                        }
                        let i = self.token;
                        self.token += 1;
                        if is_element(self.tokens[i].kind) {
                            self.active_element = Some(i);
                            self.attr = i + 1;
                        }
                    } else {
                        let i = self.active_element.unwrap();
                        if !self.charge(control, charged, quantum, WorkKind::TokenVisit)? {
                            return Ok(false);
                        }
                        if self
                            .tokens
                            .get(self.attr)
                            .is_some_and(|a| a.kind == TokenKind::Attribute)
                        {
                            let a = self.attr;
                            self.attr += 1;
                            if self.tokens[a].prefix.length == 0
                                && span_eq(self.source, self.tokens[a].local_name, b"id")
                            {
                                let id = self.tokens[a].value;
                                if id.length == 0 {
                                    return Err(ControlledError::Original(
                                        ReferenceDiagnostic::new(
                                            ReferenceError::Unsupported,
                                            id.offset,
                                        ),
                                    ));
                                }
                                if i >= u32::MAX as usize {
                                    return Err(ControlledError::Original(
                                        ReferenceDiagnostic::new(
                                            ReferenceError::Limit,
                                            self.tokens[i].name.offset,
                                        ),
                                    ));
                                }
                                self.current_id = Some(id);
                                self.hash = 1_469_598_103_934_665_603;
                                self.hash_pos = 0;
                                self.hash_ready = false;
                            }
                        } else {
                            self.active_element = None;
                        }
                    }
                    return Ok(false);
                }
                if let Some(id) = self.current_id.filter(|_| !self.hash_ready) {
                    if self.hash_pos < id.length {
                        if !self.charge(control, charged, quantum, WorkKind::SourceRead)? {
                            return Ok(false);
                        }
                        let byte = self.source[id.offset + self.hash_pos];
                        self.hash = (self.hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211);
                        self.hash_pos += 1;
                    } else {
                        self.slot = self.hash as usize & (REFERENCE_SLOT_CAPACITY - 1);
                        self.hash_ready = true;
                    }
                    return Ok(false);
                }
                if self.compare_search {
                    if !self.charge(control, charged, quantum, WorkKind::TokenVisit)? {
                        return Ok(false);
                    }
                    if self
                        .tokens
                        .get(self.compare_attr)
                        .is_some_and(|a| a.kind == TokenKind::Attribute)
                    {
                        let at = self.compare_attr;
                        self.compare_attr += 1;
                        let attr = &self.tokens[at];
                        if attr.prefix.length == 0 && span_eq(self.source, attr.local_name, b"id") {
                            self.compare_search = false;
                            self.compare_id = Some(attr.value);
                            self.compare_pos = 0;
                            if attr.value.length != self.current_id.unwrap().length {
                                self.compare_id = None;
                                self.slot = (self.slot + 1) & (REFERENCE_SLOT_CAPACITY - 1);
                            }
                        }
                    } else {
                        self.compare_search = false;
                        return Err(ControlledError::Original(ReferenceDiagnostic::new(
                            ReferenceError::InvalidArgument,
                            self.tokens[self.active_element.unwrap()].name.offset,
                        )));
                    }
                    return Ok(false);
                }
                if let Some(other) = self.compare_id {
                    if self.compare_pos < other.length {
                        if !self.charge(control, charged, quantum, WorkKind::SourceRead)? {
                            return Ok(false);
                        }
                        let incoming = self.current_id.unwrap();
                        if self.source[other.offset + self.compare_pos]
                            != self.source[incoming.offset + self.compare_pos]
                        {
                            self.compare_id = None;
                            self.compare_pos = 0;
                            self.slot = (self.slot + 1) & (REFERENCE_SLOT_CAPACITY - 1);
                        } else {
                            self.compare_pos += 1;
                        }
                    } else {
                        return Err(ControlledError::Original(ReferenceDiagnostic::new(
                            ReferenceError::Unsupported,
                            self.current_id.unwrap().offset,
                        )));
                    }
                    return Ok(false);
                }
                if !self.charge(control, charged, quantum, WorkKind::ReferenceProbe)? {
                    return Ok(false);
                }
                let occupied = self.slots[self.slot];
                if occupied == 0 {
                    self.slots[self.slot] = (self.active_element.unwrap() + 1) as u32;
                    self.active_element = None;
                    self.current_id = None;
                    self.hash_pos = 0;
                    self.hash_ready = false;
                } else {
                    let previous = occupied as usize - 1;
                    self.compare_attr = previous + 1;
                    self.compare_search = true;
                }
            }
            BuildPhase::Complete => return Ok(true),
            BuildPhase::Failed => unreachable!(),
        }
        Ok(false)
    }

    pub fn finish(self) -> Result<ReferenceIndex<'src>, ControlledError<ReferenceDiagnostic>> {
        match self.terminal {
            Some(Ok(())) if self.phase == BuildPhase::Complete => Ok(ReferenceIndex {
                source: self.source,
                tokens: self.tokens,
                slots: self.slots,
                id_count: self.id_count,
                lookup_probes: 0,
            }),
            Some(Err(error)) => Err(error),
            _ => Err(ControlledError::Original(ReferenceDiagnostic::new(
                ReferenceError::InvalidArgument,
                0,
            ))),
        }
    }
}
