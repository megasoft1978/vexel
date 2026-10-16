use super::*;

#[test]
fn completed_validator_diagnostic_returns_before_lexical_progress_like_c_step() {
    let source = Arc::new(b"<svg>bad & raw<rect width='1' height='1'/></svg>".to_vec());
    let mut session = IndexSession::start(&source, 1).unwrap();

    // Deterministically wait for the independent task's published completion,
    // outside IndexSession::step; this mirrors C's harvest-before-reader order.
    let mut completion = None;
    for _ in 0..100_000 {
        if let Some(result) = session.queue.poll() {
            completion = Some(result);
            break;
        }
        std::thread::yield_now();
    }
    let completion = completion.expect("bounded background-completion wait");
    assert!(!completion.succeeded);
    session.accept_validation_result(completion.succeeded);
    let before = session.progress().bytes_read;
    assert_eq!(before, 0);

    // svg_index_step harvests a completed diagnostic and returns MALFORMED
    // before invoking svg_xml_reader_step, regardless of the error offset.
    assert_eq!(session.step(3), IndexStatus::Malformed);
    assert_eq!(session.progress().bytes_read, before);
    assert_eq!(
        session.progress().diagnostic.category,
        XmlError::MalformedSyntax
    );
}

#[test]
fn completed_malformed_tail_keeps_prior_candidate_preview_without_more_reader_work() {
    let source =
        Arc::new(b"<svg width='2' height='2'><rect width='1' height='1'/></broken>".to_vec());
    let reader = XmlReader::new(source.as_slice()).unwrap();
    let queue = TaskQueue::new(QueueConfig {
        capacity: 1,
        workers: 1,
    })
    .unwrap();
    let worker = Arc::new(WorkerState::new());
    let root_open_length = b"<svg width='2' height='2'>".len();
    let shape_source = b"<rect width='1' height='1'/>";
    let shape_offset = root_open_length;
    let error_offset = source
        .windows(b"</broken>".len())
        .position(|part| part == b"</broken>")
        .unwrap();
    let elements = vec![
        IndexElement {
            parent: usize::MAX,
            source_offset: 0,
            source_length: source.len(),
            name_offset: 1,
            name_length: 3,
            depth: 0,
            support: ElementSupport::Unsupported,
            closed: false,
        },
        IndexElement {
            parent: 0,
            source_offset: shape_offset,
            source_length: shape_source.len(),
            name_offset: shape_offset + 1,
            name_length: 4,
            depth: 1,
            support: ElementSupport::Rect,
            closed: true,
        },
    ];
    let mut session = IndexSession {
        source_owner: &source,
        reader,
        queue,
        worker: Arc::clone(&worker),
        element_storage: size_of::<[OpenElement; MAX_DEPTH]>()
            + elements.capacity() * size_of::<IndexElement>(),
        elements,
        attribute_count: 4,
        depth: 0,
        stack: [OpenElement {
            record: 0,
            name: XmlSpan::default(),
        }; MAX_DEPTH],
        root_count: 1,
        root_open_length,
        preview_record: 1,
        generation: 1,
        canceled: false,
        complete: false,
        preview: false,
        preview_candidate: true,
        preview_root_supported: true,
        diagnostic: XmlDiagnostic::default(),
        index_diagnostic: XmlDiagnostic::default(),
        validation_diagnostic: XmlDiagnostic::default(),
        reader_diagnostic: XmlDiagnostic::default(),
        tokens: Vec::new(),
        token_count: 0,
        validation_complete: false,
        source_scanned: false,
    };
    *lock(&worker.result) = Some(ValidationResult {
        diagnostic: XmlDiagnostic {
            category: XmlError::MismatchedTag,
            offset: error_offset,
        },
        tokens: Vec::new(),
    });
    worker.error_offset.store(error_offset, Ordering::Relaxed);
    worker.failed.store(true, Ordering::Release);
    session.accept_validation_result(false);

    assert_eq!(session.step(3), IndexStatus::Malformed);
    assert_eq!(session.progress().bytes_read, 0);
    assert!(session.progress().first_preview_ready);
    assert_eq!(
        session.progress().diagnostic,
        XmlDiagnostic {
            category: XmlError::MismatchedTag,
            offset: error_offset,
        }
    );
    assert_eq!(
        session.make_preview_source().unwrap(),
        b"<svg width='2' height='2'><rect width='1' height='1'/></svg>"
    );
}

#[test]
fn oversized_owned_element_candidate_is_rejected_without_replacing_partial_index() {
    let original = IndexElement {
        parent: usize::MAX,
        source_offset: 11,
        source_length: 17,
        name_offset: 12,
        name_length: 3,
        depth: 0,
        support: ElementSupport::Unsupported,
        closed: false,
    };
    let mut elements = vec![original];
    let original_capacity = elements.capacity();

    let mut detached = Vec::new();
    detached.try_reserve_exact(1).unwrap();
    assert!(detached.capacity() >= 1);
    let result = install_element_candidate(&mut elements, detached, 0);

    assert_eq!(result, Err(ElementReserveError::Limit));
    assert_eq!(elements.as_slice(), &[original]);
    assert_eq!(elements.capacity(), original_capacity);
}

#[test]
fn exact_validator_capacity_and_combined_storage_budgets_are_charged() {
    let stream = XmlTokenStream::new_visitor(b"<svg><rect/></svg>").unwrap();
    let xml_storage = stream
        .token_storage_bytes()
        .checked_add(stream.open_name_storage_bytes())
        .unwrap()
        .checked_add(stream.visitor_storage_bytes())
        .unwrap();
    assert_eq!(
        checked_validator_storage_bytes(&stream, 0),
        Some(xml_storage)
    );
    assert!(xml_storage <= MAX_COMPONENT_BYTES);
    assert_eq!(
        checked_total_storage_bytes(MAX_COMPONENT_BYTES, MAX_COMPONENT_BYTES),
        Some(MAX_STORAGE_BYTES)
    );
    assert_eq!(
        checked_total_storage_bytes(MAX_COMPONENT_BYTES + 1, 0),
        None
    );
    assert_eq!(checked_total_storage_bytes(usize::MAX, 1), None);

    let mut tokens = Vec::new();
    let exact_candidate_capacity = 64;
    let charged = reserve_tokens(&mut tokens, exact_candidate_capacity, xml_storage).unwrap();
    assert_eq!(
        charged,
        tokens.capacity() * size_of::<XmlToken>() + xml_storage
    );
    assert!(charged <= MAX_COMPONENT_BYTES);
}

#[test]
fn exact_metadata_capacity_and_source_limit_boundaries_are_preserved() {
    let mut candidate = Vec::new();
    candidate.try_reserve_exact(1).unwrap();
    let exact_capacity_bytes = candidate
        .capacity()
        .checked_mul(size_of::<IndexElement>())
        .unwrap();
    let mut elements = Vec::new();
    assert_eq!(
        install_element_candidate(&mut elements, candidate, exact_capacity_bytes),
        Ok(exact_capacity_bytes)
    );
    assert_eq!(
        elements.capacity().checked_mul(size_of::<IndexElement>()),
        Some(exact_capacity_bytes)
    );
    let metadata_bytes = size_of::<[OpenElement; MAX_DEPTH]>() + exact_capacity_bytes;
    assert_eq!(
        checked_total_storage_bytes(metadata_bytes, MAX_COMPONENT_BYTES),
        Some(metadata_bytes + MAX_COMPONENT_BYTES)
    );
    assert_eq!(
        source_exceeds_limit(MAX_SOURCE_BYTES.min(MAX_LARGE_INPUT_BYTES)),
        false
    );
    assert_eq!(
        source_exceeds_limit(MAX_SOURCE_BYTES.min(MAX_LARGE_INPUT_BYTES) + 1),
        true
    );
}

#[test]
fn collector_growth_clamps_to_last_legal_token_and_drops_oversized_candidates() {
    let token_bytes = size_of::<XmlToken>();
    let xml_storage = 17;
    let token = vexel_xml::tokenize(b"<svg/>", vexel_xml::XmlLimits::default()).0[0];
    for legal_capacity in [1, 3, 5, 65] {
        for remainder in [0, 1, token_bytes - 1] {
            let budget = xml_storage + legal_capacity * token_bytes + remainder;
            let mut tokens = Vec::new();
            let mut previous_capacity = 0;
            while tokens.len() < legal_capacity {
                if tokens.len() == tokens.capacity() {
                    let next = next_token_capacity(tokens.capacity(), xml_storage, budget).unwrap();
                    assert!(next > tokens.capacity());
                    assert!(next <= legal_capacity);
                    let prior_tokens = tokens.clone();
                    let charged =
                        reserve_tokens_with_budget(&mut tokens, next, xml_storage, budget).unwrap();
                    assert_eq!(tokens, prior_tokens);
                    assert_eq!(charged, xml_storage + tokens.capacity() * token_bytes);
                    assert!(charged <= budget);
                    assert!(tokens.capacity() > previous_capacity);
                    previous_capacity = tokens.capacity();
                }
                tokens.push(token);
            }
            assert_eq!(tokens.capacity(), legal_capacity);
            assert_eq!(
                next_token_capacity(tokens.capacity(), xml_storage, budget),
                Err(ElementReserveError::Limit)
            );

            let original_pointer = tokens.as_ptr();
            let original_capacity = tokens.capacity();
            let mut oversized = Vec::new();
            oversized.try_reserve_exact(legal_capacity + 1).unwrap();
            assert_eq!(
                install_token_candidate(&mut tokens, oversized, xml_storage, budget),
                Err(ElementReserveError::Limit)
            );
            assert_eq!(tokens.as_ptr(), original_pointer);
            assert_eq!(tokens.capacity(), original_capacity);
            assert_eq!(tokens.as_slice(), vec![token; legal_capacity]);
        }
    }
    assert_eq!(
        next_token_capacity(4, xml_storage, xml_storage + 5 * token_bytes),
        Ok(5)
    );
    assert_eq!(
        next_token_capacity(0, xml_storage, xml_storage + token_bytes - 1),
        Err(ElementReserveError::Limit)
    );
    assert_eq!(
        next_token_capacity(0, xml_storage, xml_storage - 1),
        Err(ElementReserveError::Limit)
    );
    assert_eq!(
        next_token_capacity(usize::MAX, 0, usize::MAX),
        Err(ElementReserveError::Limit)
    );
}
