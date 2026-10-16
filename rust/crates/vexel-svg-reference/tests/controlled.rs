use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, ControlledError, ControlledStep, Stop, WorkControl, WorkKind};
use vexel_svg_reference::{
    REFERENCE_SLOT_CAPACITY, ReferenceBuildSession, ReferenceError, ReferenceFindSession,
    ReferenceIndex,
};
use vexel_xml::{XmlSpan, tokenize_large};

struct CountingControl {
    calls: u64,
    token_visits: u64,
    stop_after: Option<u64>,
    stopped: Option<Stop>,
}
impl CountingControl {
    fn new() -> Self {
        Self {
            calls: 0,
            token_visits: 0,
            stop_after: None,
            stopped: None,
        }
    }
}
impl WorkControl for CountingControl {
    fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop> {
        if let Some(stop) = self.stopped {
            return Err(stop);
        }
        if self
            .stop_after
            .is_some_and(|limit| self.calls.saturating_add(units) > limit)
        {
            self.stopped = Some(Stop::Limit);
            return Err(Stop::Limit);
        }
        if kind == WorkKind::TokenVisit {
            self.token_visits += units;
        }
        self.calls += units;
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        self.stopped.map_or(Ok(()), Err)
    }
}

fn tokens(source: &[u8]) -> Vec<vexel_xml::XmlToken> {
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, vexel_xml::XmlError::None);
    tokens
}

fn build_tiny<'a>(
    source: &'a [u8],
    tokens: &'a [vexel_xml::XmlToken],
) -> Result<ReferenceIndex<'a>, ControlledError<vexel_svg_reference::ReferenceDiagnostic>> {
    let mut session = ReferenceBuildSession::new(source, tokens);
    let mut control = CountingControl::new();
    loop {
        match session.step(4096, &mut control) {
            ControlledStep::Pending { charged_units } => {
                assert!((1..=4096).contains(&charged_units))
            }
            ControlledStep::Complete(()) => return session.finish(),
            ControlledStep::Failed(error) => return Err(error),
        }
    }
}

#[test]
fn tiny_build_matches_legacy_result_capacity_and_duplicate_errors() {
    for source in [
        &b"<svg/>"[..],
        &b"<svg><g id='a'/><g id='b'/></svg>"[..],
        &b"<svg><g id=''/></svg>"[..],
        &b"<svg><g id='x'/><g id='x'/></svg>"[..],
    ] {
        let input = tokens(source);
        let old = ReferenceIndex::create(source, &input);
        let controlled = build_tiny(source, &input);
        match (old, controlled) {
            (Ok(old), Ok(new)) => {
                assert_eq!(new.id_count(), old.id_count());
                assert_eq!(new.allocated_bytes(), old.storage_bytes());
            }
            (Err(old), Err(ControlledError::Original(new))) => assert_eq!(old, new),
            other => panic!("legacy/control mismatch: {other:?}"),
        }
    }
    let source = b"<svg><g id='controlled'/></svg>";
    let input = tokens(source);
    let cancel = AtomicBool::new(false);
    let mut budget = BudgetControl::new(&cancel, u64::MAX);
    let index = ReferenceIndex::create_controlled(source, &input, &mut budget).unwrap();
    assert_eq!(index.id_count(), 1);
    assert_eq!(
        index.allocated_bytes(),
        REFERENCE_SLOT_CAPACITY * core::mem::size_of::<u32>()
    );
}

#[test]
fn long_raw_id_hash_and_lookup_resume_by_byte_and_keep_exact_bytes() {
    let long = format!("{}-left", "shared-prefix".repeat(420));
    let source = format!(
        "<svg><g id='{long}'/><g id='{}-right'/></svg>",
        "shared-prefix".repeat(80)
    );
    let source = source.into_bytes();
    let input = tokens(&source);
    let mut build = ReferenceBuildSession::new(&source, &input);
    let mut control = CountingControl::new();
    let mut pending = 0;
    while control.token_visits < (REFERENCE_SLOT_CAPACITY as u64 + 1) {
        match build.step(4096, &mut control) {
            ControlledStep::Pending { charged_units } => {
                assert!((1..=4096).contains(&charged_units))
            }
            ControlledStep::Complete(()) => panic!("build completed before long ID hashing"),
            ControlledStep::Failed(error) => panic!("build failed: {error:?}"),
        }
    }
    loop {
        match build.step(13, &mut control) {
            ControlledStep::Pending { charged_units } => {
                assert!((1..=13).contains(&charged_units));
                pending += 1;
            }
            ControlledStep::Complete(()) => break,
            ControlledStep::Failed(error) => panic!("build failed: {error:?}"),
        }
    }
    assert!(pending > 100);
    let mut index = build.finish().unwrap();
    let query = XmlSpan {
        offset: source
            .windows(long.len())
            .position(|window| window == long.as_bytes())
            .unwrap(),
        length: long.len(),
    };
    let before = index.lookup_probes();
    let mut find: ReferenceFindSession<'_, '_> = index.begin_find(query);
    let mut find_pending = 0;
    loop {
        match find.step(13, &mut control) {
            ControlledStep::Pending { charged_units } => {
                assert!((1..=13).contains(&charged_units));
                find_pending += 1;
            }
            ControlledStep::Complete(found) => {
                assert_eq!(found, Some(1));
                break;
            }
            ControlledStep::Failed(error) => panic!("find failed: {error:?}"),
        }
    }
    assert!(find_pending > 100);
    assert_eq!(find.finish().unwrap(), Some(1));
    assert!(index.lookup_probes() > before);
}

#[test]
fn zero_quantum_finish_drop_and_sticky_limit_are_real() {
    let source = b"<svg><g id='alpha'/></svg>";
    let input = tokens(source);
    let mut session = ReferenceBuildSession::new(source, &input);
    let mut control = CountingControl::new();
    assert_eq!(
        session.step(0, &mut control),
        ControlledStep::Pending { charged_units: 0 }
    );
    assert!(
        matches!(session.finish(), Err(ControlledError::Original(d)) if d.category == ReferenceError::InvalidArgument && d.offset == 0)
    );
    let _dropped = ReferenceBuildSession::new(source, &input);

    let mut limited = ReferenceBuildSession::new(source, &input);
    control.stop_after = Some(2);
    let first = limited.step(100, &mut control);
    assert!(matches!(
        first,
        ControlledStep::Failed(ControlledError::Limit)
    ));
    assert_eq!(limited.step(100, &mut control), first);
    assert!(matches!(limited.finish(), Err(ControlledError::Limit)));

    let mut index = ReferenceIndex::create(source, &input).unwrap();
    let unfinished = index.begin_find(XmlSpan {
        offset: 12,
        length: 5,
    });
    assert!(
        matches!(unfinished.finish(), Err(ControlledError::Original(d)) if d.category == ReferenceError::InvalidArgument && d.offset == 0)
    );
}

#[test]
fn controlled_lookup_and_budget_overflow_are_typed() {
    let source = b"<svg><g id='alpha'/></svg>";
    let input = tokens(source);
    let mut index = ReferenceIndex::create(source, &input).unwrap();
    let cancel = AtomicBool::new(false);
    let mut budget = BudgetControl::new(&cancel, u64::MAX);
    let span = XmlSpan {
        offset: source
            .windows(5)
            .position(|window| window == b"alpha")
            .unwrap(),
        length: 5,
    };
    assert_eq!(index.find_controlled(span, &mut budget), Ok(Some(1)));
    let probes = index.lookup_probes();
    assert_eq!(
        index.find_controlled(
            XmlSpan {
                offset: usize::MAX,
                length: 1
            },
            &mut budget
        ),
        Ok(None)
    );
    assert_eq!(
        index.find_controlled(
            XmlSpan {
                offset: span.offset,
                length: 0
            },
            &mut budget
        ),
        Ok(None)
    );
    assert_eq!(index.lookup_probes(), probes);

    struct Overflow;
    impl WorkControl for Overflow {
        fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
            Err(Stop::Overflow)
        }
        fn checkpoint(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }
    let mut find = index.begin_find(span);
    assert_eq!(
        find.step(3, &mut Overflow),
        ControlledStep::Failed(ControlledError::Overflow)
    );
    assert_eq!(
        find.step(3, &mut Overflow),
        ControlledStep::Failed(ControlledError::Overflow)
    );
}

struct CheckpointControl {
    stop: Option<Stop>,
    checkpoints: u64,
    charges: u64,
}

impl WorkControl for CheckpointControl {
    fn charge(&mut self, _: WorkKind, units: u64) -> Result<(), Stop> {
        self.charges += units;
        Ok(())
    }

    fn checkpoint(&mut self) -> Result<(), Stop> {
        self.checkpoints += 1;
        self.stop.map_or(Ok(()), Err)
    }
}

#[test]
fn trivial_find_checks_control_before_pending_or_none_completion() {
    let source = b"<svg><g id='x'/></svg>";
    let input = tokens(source);
    let mut indexed = ReferenceIndex::create(source, &input).unwrap();
    let empty_source = b"<svg/>";
    let empty_tokens = tokens(empty_source);
    let mut empty_index = ReferenceIndex::create(empty_source, &empty_tokens).unwrap();

    check_trivial_find(
        &mut indexed,
        XmlSpan {
            offset: usize::MAX,
            length: 1,
        },
    );
    check_trivial_find(
        &mut indexed,
        XmlSpan {
            offset: 8,
            length: 0,
        },
    );
    check_trivial_find(
        &mut empty_index,
        XmlSpan {
            offset: 1,
            length: 1,
        },
    );
}

fn check_trivial_find(index: &mut ReferenceIndex<'_>, query: XmlSpan) {
    let probes = index.lookup_probes();
    {
        let mut session = index.begin_find(query);
        let mut control = CheckpointControl {
            stop: None,
            checkpoints: 0,
            charges: 0,
        };
        assert_eq!(
            session.step(0, &mut control),
            ControlledStep::Pending { charged_units: 0 }
        );
        assert_eq!(control.checkpoints, 1);
        assert_eq!(control.charges, 0);
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Complete(None)
        );
        assert_eq!(control.checkpoints, 2);
        assert_eq!(control.charges, 0);
        control.stop = Some(Stop::Limit);
        assert_eq!(
            session.step(0, &mut control),
            ControlledStep::Complete(None)
        );
        assert_eq!(control.checkpoints, 2);
        assert_eq!(session.finish(), Ok(None));
    }
    assert_eq!(index.find(query), None);
    assert_eq!(index.lookup_probes(), probes);

    for stop in [Stop::Canceled, Stop::Limit] {
        let mut session = index.begin_find(query);
        let mut control = CheckpointControl {
            stop: Some(stop),
            checkpoints: 0,
            charges: 0,
        };
        assert_eq!(
            session.step(0, &mut control),
            ControlledStep::Failed(stop.into())
        );
        assert_eq!(
            session.step(1, &mut control),
            ControlledStep::Failed(stop.into())
        );
        assert_eq!(control.checkpoints, 1);
    }

    for stop in [Stop::Canceled, Stop::Limit] {
        let mut control = CheckpointControl {
            stop: Some(stop),
            checkpoints: 0,
            charges: 0,
        };
        let expected = match stop {
            Stop::Canceled => ControlledError::Canceled,
            Stop::Limit => ControlledError::Limit,
            Stop::Overflow => unreachable!(),
        };
        assert_eq!(index.find_controlled(query, &mut control), Err(expected));
        assert_eq!(control.checkpoints, 1);
        assert_eq!(control.charges, 0);
    }
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Validation,
    Count,
    Zero,
    Hash,
    Probe,
    Compare,
}
struct PhaseGate {
    phase: Phase,
    tokens: u64,
    reads: u64,
    visits: u64,
    probes: u64,
    stopped: bool,
}
impl PhaseGate {
    fn new(phase: Phase, token_count: usize) -> Self {
        Self {
            phase,
            tokens: token_count as u64,
            reads: 0,
            visits: 0,
            probes: 0,
            stopped: false,
        }
    }
}
impl WorkControl for PhaseGate {
    fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop> {
        if self.stopped {
            return Err(Stop::Limit);
        }
        let hit = match self.phase {
            Phase::Validation => kind == WorkKind::SourceRead && self.reads == 0,
            Phase::Count => kind == WorkKind::TokenVisit && self.visits >= self.tokens,
            Phase::Zero => kind == WorkKind::TokenVisit && self.visits >= self.tokens + 1000,
            Phase::Hash => {
                kind == WorkKind::SourceRead
                    && self.visits >= self.tokens + REFERENCE_SLOT_CAPACITY as u64
            }
            Phase::Probe => kind == WorkKind::ReferenceProbe,
            Phase::Compare => kind == WorkKind::SourceRead && self.probes >= 2,
        };
        if hit {
            self.stopped = true;
            return Err(Stop::Limit);
        }
        match kind {
            WorkKind::SourceRead => self.reads += units,
            WorkKind::TokenVisit => self.visits += units,
            WorkKind::ReferenceProbe => self.probes += units,
            _ => {}
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        if self.stopped {
            Err(Stop::Limit)
        } else {
            Ok(())
        }
    }
}

#[test]
fn cancellation_is_sticky_at_each_real_build_phase() {
    let mut buckets = BTreeMap::<usize, Vec<String>>::new();
    let (left, right) = (0..100_000usize)
        .find_map(|n| {
            let a = format!("key{n:06}");
            let bucket = a.bytes().fold(1_469_598_103_934_665_603u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(1_099_511_628_211)
            }) as usize
                & (REFERENCE_SLOT_CAPACITY - 1);
            let values = buckets.entry(bucket).or_default();
            values.push(a);
            (values.len() == 2).then(|| (values[0].clone(), values[1].clone()))
        })
        .unwrap();
    let source = format!("<svg><g id='{left}'/><g id='{right}'/></svg>").into_bytes();
    let input = tokens(&source);
    for phase in [
        Phase::Validation,
        Phase::Count,
        Phase::Zero,
        Phase::Hash,
        Phase::Probe,
        Phase::Compare,
    ] {
        let mut session = ReferenceBuildSession::new(&source, &input);
        let mut gate = PhaseGate::new(phase, input.len());
        let failed = loop {
            match session.step(512, &mut gate) {
                ControlledStep::Pending { charged_units } => {
                    assert!((1..=512).contains(&charged_units))
                }
                ControlledStep::Complete(()) => panic!("phase gate was not reached: {phase:?}"),
                ControlledStep::Failed(error) => break error,
            }
        };
        assert_eq!(failed, ControlledError::Limit);
        assert_eq!(
            session.step(512, &mut gate),
            ControlledStep::Failed(ControlledError::Limit)
        );
        assert!(matches!(session.finish(), Err(ControlledError::Limit)));
    }
}

#[test]
fn over_cap_error_and_validation_precedence_match_legacy() {
    let mut source = String::from("<svg>");
    for _ in 0..=vexel_svg_reference::MAX_REFERENCE_IDS {
        source.push_str("<g id='x'/>");
    }
    source.push_str("</svg>");
    let source = source.into_bytes();
    let input = tokens(&source);
    let legacy = ReferenceIndex::create(&source, &input).unwrap_err();
    let mut control = CountingControl::new();
    let controlled = match ReferenceIndex::create_controlled(&source, &input, &mut control) {
        Err(ControlledError::Original(error)) => error,
        other => panic!("expected original limit error, got {other:?}"),
    };
    assert_eq!(controlled, legacy);
    assert_eq!(legacy.category, ReferenceError::Limit);

    let mut malformed = input;
    let last = malformed.len() - 1;
    malformed[last].value = XmlSpan {
        offset: usize::MAX,
        length: 1,
    };
    let legacy = ReferenceIndex::create(&source, &malformed).unwrap_err();
    let mut session = ReferenceBuildSession::new(&source, &malformed);
    let mut control = CountingControl::new();
    let controlled = loop {
        match session.step(4096, &mut control) {
            ControlledStep::Pending { .. } => {}
            ControlledStep::Complete(()) => panic!("invalid span accepted"),
            ControlledStep::Failed(ControlledError::Original(error)) => break error,
            ControlledStep::Failed(error) => panic!("unexpected control failure: {error:?}"),
        }
    };
    assert_eq!(controlled, legacy);
    assert_eq!(legacy.category, ReferenceError::InvalidArgument);
    assert_eq!(legacy.offset, usize::MAX);
}
