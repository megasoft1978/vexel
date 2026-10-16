use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FindPhase {
    Trivial,
    Hash,
    Probe,
    CandidateAttribute,
    Compare,
    Complete,
    Failed,
}

/// Incremental exact-raw-byte lookup borrowing the index mutably so actual
/// probes remain accounted even if the caller later cancels the operation.
pub struct ReferenceFindSession<'idx, 'src> {
    index: &'idx mut ReferenceIndex<'src>,
    query: XmlSpan,
    phase: FindPhase,
    hash: u64,
    byte: usize,
    slot: usize,
    candidate: usize,
    attr: usize,
    candidate_id: Option<XmlSpan>,
    compare: usize,
    terminal: Option<Result<Option<usize>, ControlledError<ReferenceDiagnostic>>>,
}

impl<'idx, 'src> ReferenceFindSession<'idx, 'src> {
    pub(super) fn new(index: &'idx mut ReferenceIndex<'src>, query: XmlSpan) -> Self {
        let valid = span_bytes(index.source, query).is_some_and(|bytes| !bytes.is_empty());
        let phase = if !valid || index.slots.is_empty() {
            FindPhase::Trivial
        } else {
            FindPhase::Hash
        };
        Self {
            index,
            query,
            phase,
            hash: 1_469_598_103_934_665_603,
            byte: 0,
            slot: 0,
            candidate: 0,
            attr: 0,
            candidate_id: None,
            compare: 0,
            terminal: None,
        }
    }

    fn fail(
        &mut self,
        error: ControlledError<ReferenceDiagnostic>,
    ) -> ControlledStep<Option<usize>, ReferenceDiagnostic> {
        self.phase = FindPhase::Failed;
        self.terminal = Some(Err(error));
        ControlledStep::Failed(error)
    }

    pub fn step(
        &mut self,
        quantum: u64,
        control: &mut dyn WorkControl,
    ) -> ControlledStep<Option<usize>, ReferenceDiagnostic> {
        match self.phase {
            FindPhase::Complete => {
                return ControlledStep::Complete(self.terminal.unwrap().unwrap());
            }
            FindPhase::Failed => {
                return ControlledStep::Failed(self.terminal.unwrap().unwrap_err());
            }
            _ => {}
        }
        if quantum == 0 {
            return match control.checkpoint() {
                Ok(()) => ControlledStep::Pending { charged_units: 0 },
                Err(e) => self.fail(e.into()),
            };
        }
        if let Err(e) = control.checkpoint() {
            return self.fail(e.into());
        }
        if self.phase == FindPhase::Trivial {
            self.phase = FindPhase::Complete;
            self.terminal = Some(Ok(None));
            return ControlledStep::Complete(None);
        }
        let mut charged = 0u64;
        while charged < quantum {
            let result = self.find_one(control, &mut charged, quantum);
            match result {
                Ok(Some(found)) => {
                    self.phase = FindPhase::Complete;
                    self.terminal = Some(Ok(found));
                    return ControlledStep::Complete(found);
                }
                Ok(None) => {}
                Err(error) => return self.fail(error),
            }
            if charged > 0 && charged % 256 == 0 {
                if let Err(e) = control.checkpoint() {
                    return self.fail(e.into());
                }
            }
            if self.phase == FindPhase::Complete {
                self.terminal = Some(Ok(None));
                return ControlledStep::Complete(None);
            }
        }
        if let Err(e) = control.checkpoint() {
            return self.fail(e.into());
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

    fn find_one(
        &mut self,
        control: &mut dyn WorkControl,
        charged: &mut u64,
        quantum: u64,
    ) -> Result<Option<Option<usize>>, ControlledError<ReferenceDiagnostic>> {
        match self.phase {
            FindPhase::Trivial => unreachable!("trivial lookup completes before source work"),
            FindPhase::Hash => {
                if self.byte == self.query.length {
                    self.slot = self.hash as usize & (REFERENCE_SLOT_CAPACITY - 1);
                    self.phase = FindPhase::Probe;
                    return Ok(None);
                }
                if !self.charge(control, charged, quantum, WorkKind::SourceRead)? {
                    return Ok(None);
                }
                let byte = self.index.source[self.query.offset + self.byte];
                self.hash = (self.hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211);
                self.byte += 1;
            }
            FindPhase::Probe => {
                if !self.charge(control, charged, quantum, WorkKind::ReferenceProbe)? {
                    return Ok(None);
                }
                let occupied = self.index.slots[self.slot];
                self.index.lookup_probes = self.index.lookup_probes.saturating_add(1);
                if occupied == 0 {
                    self.phase = FindPhase::Complete;
                    return Ok(None);
                }
                self.candidate = occupied as usize - 1;
                let Some(token) = self.index.tokens.get(self.candidate) else {
                    self.phase = FindPhase::Complete;
                    return Ok(None);
                };
                if !is_element(token.kind) {
                    self.phase = FindPhase::Complete;
                    return Ok(None);
                }
                self.attr = self.candidate + 1;
                self.phase = FindPhase::CandidateAttribute;
            }
            FindPhase::CandidateAttribute => {
                if !self.charge(control, charged, quantum, WorkKind::TokenVisit)? {
                    return Ok(None);
                }
                if self
                    .index
                    .tokens
                    .get(self.attr)
                    .is_some_and(|t| t.kind == TokenKind::Attribute)
                {
                    let at = self.attr;
                    self.attr += 1;
                    let token = &self.index.tokens[at];
                    if token.prefix.length == 0
                        && span_eq(self.index.source, token.local_name, b"id")
                    {
                        self.candidate_id = Some(token.value);
                        if token.value.length != self.query.length {
                            self.next_probe();
                        } else {
                            self.phase = FindPhase::Compare;
                            self.compare = 0;
                        }
                    }
                } else {
                    self.phase = FindPhase::Complete;
                    self.terminal = Some(Ok(None));
                }
            }
            FindPhase::Compare => {
                let id = self.candidate_id.unwrap();
                if self.compare == self.query.length {
                    self.phase = FindPhase::Complete;
                    return Ok(Some(Some(self.candidate)));
                }
                if !self.charge(control, charged, quantum, WorkKind::SourceRead)? {
                    return Ok(None);
                }
                if self.index.source[id.offset + self.compare]
                    == self.index.source[self.query.offset + self.compare]
                {
                    self.compare += 1;
                } else {
                    self.next_probe();
                }
            }
            FindPhase::Complete => return Ok(None),
            FindPhase::Failed => unreachable!(),
        }
        Ok(None)
    }

    fn next_probe(&mut self) {
        self.slot = (self.slot + 1) & (REFERENCE_SLOT_CAPACITY - 1);
        self.phase = FindPhase::Probe;
        self.candidate_id = None;
        self.compare = 0;
    }

    pub fn finish(self) -> Result<Option<usize>, ControlledError<ReferenceDiagnostic>> {
        match self.terminal {
            Some(result) => result,
            None => Err(ControlledError::Original(ReferenceDiagnostic::new(
                ReferenceError::InvalidArgument,
                0,
            ))),
        }
    }
}
