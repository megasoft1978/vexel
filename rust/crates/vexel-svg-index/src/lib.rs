#![forbid(unsafe_code)]
//! Bounded progressive SVG indexing with independent background XML validation.

use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use vexel_platform_tasks::{QueueConfig, TaskCallback, TaskOutput, TaskQueue};
use vexel_xml::{
    ReaderStatus, ReaderTag, TagKind, XmlDiagnostic, XmlError,
    XmlReader, XmlToken, XmlTokenStream, XmlSpan, StreamStatus, MAX_LARGE_INPUT_BYTES,
    MAX_READER_STEP_BYTES, MAX_TOKEN_NAME_BYTES,
};

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_STEP_BYTES: usize = 64 * 1024;
pub const MAX_ELEMENTS: usize = 1_000_000;
pub const MAX_ATTRIBUTES: usize = 1_000_000;
pub const MAX_STORAGE_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_COMPONENT_BYTES: usize = MAX_STORAGE_BYTES / 2;
pub const MAX_DEPTH: usize = 256;
pub const MAX_PREVIEW_SOURCE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStatus { Ok, Invalid, Limit, OutOfMemory, Malformed, Canceled, Complete }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementSupport { Unsupported, Rect, Path }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexElement {
    pub parent: usize,
    pub source_offset: usize,
    pub source_length: usize,
    pub name_offset: usize,
    pub name_length: usize,
    pub depth: usize,
    pub support: ElementSupport,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexProgress {
    pub bytes_read: usize,
    pub source_bytes: usize,
    pub element_count: usize,
    pub attribute_count: usize,
    pub token_count: usize,
    pub generation: u64,
    pub diagnostic: XmlDiagnostic,
    pub complete: bool,
    pub canceled: bool,
    pub first_preview_ready: bool,
}

#[derive(Clone, Copy)]
struct OpenElement { record: usize, name: XmlSpan }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementReserveError { Limit, OutOfMemory }

struct ValidationResult { diagnostic: XmlDiagnostic, tokens: Vec<XmlToken> }

struct WorkerState {
    frontier: AtomicUsize,
    failed: AtomicBool,
    error_offset: AtomicUsize,
    result: Mutex<Option<ValidationResult>>,
}

impl WorkerState {
    fn new() -> Self {
        Self { frontier: AtomicUsize::new(0), failed: AtomicBool::new(false),
            error_offset: AtomicUsize::new(0), result: Mutex::new(None) }
    }
}

/// A session borrows the caller's immutable source Arc. The validator owns an
/// Arc clone, while the resumable lexical reader borrows the original bytes.
pub struct IndexSession<'a> {
    source_owner: &'a Arc<Vec<u8>>,
    reader: XmlReader<'a>,
    queue: TaskQueue,
    worker: Arc<WorkerState>,
    elements: Vec<IndexElement>,
    element_storage: usize,
    attribute_count: usize,
    depth: usize,
    stack: [OpenElement; MAX_DEPTH],
    root_count: usize,
    root_open_length: usize,
    preview_record: usize,
    generation: u64,
    canceled: bool,
    complete: bool,
    preview: bool,
    preview_candidate: bool,
    preview_root_supported: bool,
    diagnostic: XmlDiagnostic,
    index_diagnostic: XmlDiagnostic,
    validation_diagnostic: XmlDiagnostic,
    reader_diagnostic: XmlDiagnostic,
    tokens: Vec<XmlToken>,
    token_count: usize,
    validation_complete: bool,
    source_scanned: bool,
}

impl<'a> IndexSession<'a> {
    pub fn start(source: &'a Arc<Vec<u8>>, generation: u64) -> Result<Self, IndexStatus> {
        let bytes = source.as_slice();
        if source_exceeds_limit(bytes.len()) {
            return Err(IndexStatus::Limit);
        }
        let reader = XmlReader::new(bytes).map_err(|status| match status {
            ReaderStatus::Limit => IndexStatus::Limit,
            _ => IndexStatus::Invalid,
        })?;
        let queue = TaskQueue::new(QueueConfig { capacity: 1, workers: 1 })
            .map_err(|_| IndexStatus::OutOfMemory)?;
        let worker = Arc::new(WorkerState::new());
        let worker_context = Arc::clone(&worker);
        let worker_source = Arc::clone(source);
        let callback = TaskCallback::new((worker_context, worker_source), |context, token| {
            validate_document(context, token)
        });
        queue.submit(1, 1, callback).map_err(|_| IndexStatus::OutOfMemory)?;
        Ok(Self {
            source_owner: source, reader, queue, worker, elements: Vec::new(),
            element_storage: size_of::<[OpenElement; MAX_DEPTH]>(),
            attribute_count: 0, depth: 0,
            stack: [OpenElement { record: 0, name: XmlSpan { offset: 0, length: 0 } }; MAX_DEPTH],
            root_count: 0, root_open_length: 0, preview_record: 0, generation,
            canceled: false, complete: false, preview: false, preview_candidate: false,
            preview_root_supported: false, diagnostic: XmlDiagnostic::default(),
            index_diagnostic: XmlDiagnostic::default(), validation_diagnostic: XmlDiagnostic::default(),
            reader_diagnostic: XmlDiagnostic::default(), tokens: Vec::new(), token_count: 0,
            validation_complete: false, source_scanned: false,
        })
    }

    pub fn step(&mut self, byte_budget: usize) -> IndexStatus {
        if byte_budget == 0 { return IndexStatus::Invalid; }
        if self.canceled { return IndexStatus::Canceled; }
        if self.complete { return IndexStatus::Complete; }
        self.harvest_validation();
        self.publish_validated_preview();
        // Preserve the C ordering: a completed validator diagnostic is returned
        // before another lexical reader step, even when its offset is ahead.
        if self.validation_complete && self.validation_diagnostic.category != XmlError::None {
            self.diagnostic = self.validation_diagnostic;
            return IndexStatus::Malformed;
        }
        if self.index_diagnostic.category != XmlError::None && self.validation_complete {
            self.diagnostic = if self.validation_diagnostic.category != XmlError::None {
                self.validation_diagnostic
            } else { self.index_diagnostic };
            return IndexStatus::Malformed;
        }
        if self.preview_candidate && !self.preview && self.validation_diagnostic.category == XmlError::None {
            return IndexStatus::Ok;
        }
        let source = self.source_owner.as_slice();
        let mut elements = &mut self.elements;
        let mut element_storage = &mut self.element_storage;
        let mut attribute_count = &mut self.attribute_count;
        let mut depth = &mut self.depth;
        let mut stack = &mut self.stack;
        let mut root_count = &mut self.root_count;
        let mut root_open_length = &mut self.root_open_length;
        let mut preview_record = &mut self.preview_record;
        let mut preview_candidate = &mut self.preview_candidate;
        let mut preview_root_supported = &mut self.preview_root_supported;
        let mut diagnostic = &mut self.index_diagnostic;
        let preview = self.preview;
        let reader_status = self.reader.step(byte_budget.min(MAX_STEP_BYTES), |tag| {
            index_tag(source, tag, &mut elements, &mut element_storage, &mut attribute_count,
                &mut depth, &mut stack, &mut root_count, &mut root_open_length,
                &mut preview_record, &mut preview_candidate, &mut preview_root_supported,
                &mut diagnostic, preview)
        });
        let reader_progress = self.reader.progress();
        if reader_progress.diagnostic.category != XmlError::None {
            self.reader_diagnostic = XmlDiagnostic { category: reader_progress.diagnostic.category,
                offset: reader_progress.diagnostic.offset };
        }
        if reader_status == ReaderStatus::Malformed {
            self.harvest_validation();
            self.publish_validated_preview();
            if !self.validation_complete { return IndexStatus::Ok; }
            self.diagnostic = if self.validation_diagnostic.category != XmlError::None {
                self.validation_diagnostic
            } else if self.index_diagnostic.category != XmlError::None { self.index_diagnostic }
            else { self.reader_diagnostic };
            return IndexStatus::Malformed;
        }
        if reader_status == ReaderStatus::Complete {
            if self.depth != 0 || self.root_count != 1 {
                set_error(&mut self.index_diagnostic, XmlError::IncompleteDocument,
                    self.source_owner.len());
            } else { self.source_scanned = true; }
        }
        self.harvest_validation();
        self.publish_validated_preview();
        if self.validation_complete {
            if self.validation_diagnostic.category != XmlError::None {
                self.diagnostic = self.validation_diagnostic;
                return IndexStatus::Malformed;
            }
            if self.index_diagnostic.category != XmlError::None {
                self.diagnostic = if self.validation_diagnostic.category != XmlError::None {
                    self.validation_diagnostic
                } else { self.index_diagnostic };
                return IndexStatus::Malformed;
            }
            if self.reader_diagnostic.category != XmlError::None {
                self.diagnostic = if self.validation_diagnostic.category != XmlError::None {
                    self.validation_diagnostic
                } else { self.reader_diagnostic };
                return IndexStatus::Malformed;
            }
        }
        if self.source_scanned && self.validation_complete { self.complete = true; }
        if self.complete { IndexStatus::Complete } else { IndexStatus::Ok }
    }

    pub fn progress(&self) -> IndexProgress {
        IndexProgress {
            bytes_read: self.reader.progress().bytes_read,
            source_bytes: self.source_owner.len(), element_count: self.elements.len(),
            attribute_count: self.attribute_count, token_count: self.token_count,
            generation: self.generation, diagnostic: self.diagnostic, complete: self.complete,
            canceled: self.canceled, first_preview_ready: self.preview,
        }
    }

    pub fn advance_generation(&mut self, generation: u64) -> IndexStatus {
        if generation <= self.generation { return IndexStatus::Invalid; }
        self.generation = generation;
        self.preview = false;
        self.preview_candidate = false;
        IndexStatus::Ok
    }

    pub fn preview_generation_is_current(&self, generation: u64) -> bool {
        !self.canceled && self.preview && generation == self.generation
    }

    pub fn cancel(&mut self) {
        if self.canceled { return; }
        self.canceled = true;
        let _ = self.queue.cancel(1);
    }

    pub fn element_at(&self, index: usize) -> Option<IndexElement> { self.elements.get(index).copied() }

    /// Token storage is visible only after full lexical and semantic completion.
    pub fn token_at(&self, index: usize) -> Option<XmlToken> {
        if self.complete { self.tokens.get(index).copied() } else { None }
    }

    pub fn make_preview_source(&self) -> Option<Vec<u8>> {
        if !self.preview || self.root_open_length == 0 || self.preview_record >= self.elements.len() {
            return None;
        }
        let source = self.source_owner.as_slice();
        let root = self.elements.first()?;
        let shape = self.elements.get(self.preview_record)?;
        if span_slice(source, XmlSpan { offset: root.name_offset, length: root.name_length })? != b"svg"
            || shape.source_length == 0 { return None; }
        let total = self.root_open_length.checked_add(shape.source_length)?.checked_add(6)?;
        if total > MAX_PREVIEW_SOURCE_BYTES { return None; }
        let mut output = Vec::new();
        output.try_reserve_exact(total).ok()?;
        output.extend_from_slice(source.get(root.source_offset..root.source_offset.checked_add(self.root_open_length)?)?);
        output.extend_from_slice(source.get(shape.source_offset..shape.source_offset.checked_add(shape.source_length)?)?);
        output.extend_from_slice(b"</svg>");
        Some(output)
    }

    fn harvest_validation(&mut self) {
        if self.validation_complete { return; }
        let Some(result) = self.queue.poll() else { return; };
        self.accept_validation_result(result.succeeded);
    }

    fn accept_validation_result(&mut self, succeeded: bool) {
        if self.validation_complete { return; }
        self.validation_complete = true;
        let mut guard = lock(&self.worker.result);
        if let Some(validated) = guard.take() {
            self.token_count = validated.tokens.len();
            self.tokens = validated.tokens;
            if validated.diagnostic.category != XmlError::None {
                self.validation_diagnostic = validated.diagnostic;
            } else if !succeeded {
                self.validation_diagnostic = XmlDiagnostic { category: XmlError::OutputExhausted, offset: 0 };
            } else if self.reader_diagnostic.category != XmlError::None {
                self.validation_diagnostic = self.reader_diagnostic;
            }
        } else if !succeeded {
            self.validation_diagnostic = XmlDiagnostic { category: XmlError::OutputExhausted, offset: 0 };
        }
    }

    fn publish_validated_preview(&mut self) {
        if !self.preview_candidate || self.preview { return; }
        let Some(shape) = self.elements.get(self.preview_record) else { return; };
        let Some(end) = shape.source_offset.checked_add(shape.source_length) else { return; };
        let frontier = if self.worker.failed.load(Ordering::Acquire) {
            self.worker.error_offset.load(Ordering::Relaxed)
        } else { self.worker.frontier.load(Ordering::Acquire) };
        if frontier >= end { self.preview = true; }
    }
}

impl Drop for IndexSession<'_> {
    fn drop(&mut self) {
        self.cancel();
        self.queue.shutdown();
    }
}

fn validate_document(context: &mut (Arc<WorkerState>, Arc<Vec<u8>>), token: vexel_platform_tasks::CancellationToken<'_>) -> TaskOutput {
    let (worker, source) = context;
    let mut stream = match XmlTokenStream::new_visitor(source.as_slice()) {
        Ok(stream) => stream,
        Err(_) => {
            *lock(&worker.result) = Some(ValidationResult {
                diagnostic: XmlDiagnostic { category: XmlError::OutputExhausted, offset: 0 },
                tokens: Vec::new(),
            });
            return TaskOutput::failure(XmlError::OutputExhausted as i64);
        }
    };
    match checked_validator_storage_bytes(&stream, 0) {
        Some(bytes) if bytes <= MAX_COMPONENT_BYTES => (),
        _ => {
            *lock(&worker.result) = Some(ValidationResult {
                diagnostic: XmlDiagnostic { category: XmlError::OutputExhausted, offset: 0 },
                tokens: Vec::new(),
            });
            return TaskOutput::failure(XmlError::OutputExhausted as i64);
        }
    };
    let mut tokens: Vec<XmlToken> = Vec::new();
    let mut allocation_failed = false;
    let mut status = StreamStatus::Ok;
    while status == StreamStatus::Ok {
        let xml_storage = match checked_validator_storage_bytes(&stream, 0) {
            Some(bytes) if bytes <= MAX_COMPONENT_BYTES => bytes,
            _ => { allocation_failed = true; break; }
        };
        status = stream.step_with_visitor_and_cancel(MAX_READER_STEP_BYTES,
            &mut |item| {
                if tokens.len() == tokens.capacity() {
                    let next = match next_token_capacity(tokens.capacity(), xml_storage, MAX_COMPONENT_BYTES) {
                        Ok(value) => value,
                        Err(_) => { allocation_failed = true; return false; }
                    };
                    if reserve_tokens(&mut tokens, next, xml_storage).is_err() {
                        allocation_failed = true; return false;
                    }
                }
                tokens.push(*item);
                true
            },
            &mut || token.is_cancelled());
        match checked_validator_storage_bytes(&stream, tokens.capacity()) {
            Some(bytes) if bytes <= MAX_COMPONENT_BYTES => (),
            _ => { allocation_failed = true; break; }
        }
        worker.frontier.store(stream.bytes_read(), Ordering::Release);
        let partial = stream.result();
        if partial.diagnostic.category != XmlError::None {
            worker.error_offset.store(partial.diagnostic.offset, Ordering::Relaxed);
            worker.failed.store(true, Ordering::Release);
        }
        if token.is_cancelled() { break; }
    }
    if token.is_cancelled() || status == StreamStatus::Canceled {
        return TaskOutput::failure(XmlError::OutputExhausted as i64);
    }
    let mut diagnostic = stream.result().diagnostic;
    if allocation_failed {
        diagnostic = XmlDiagnostic { category: XmlError::OutputExhausted, offset: worker.frontier.load(Ordering::Acquire) };
    }
    let succeeded = status == StreamStatus::Complete && diagnostic.category == XmlError::None;
    if !succeeded { tokens = Vec::new(); }
    *lock(&worker.result) = Some(ValidationResult { diagnostic, tokens });
    if succeeded { TaskOutput::success(0) }
    else { TaskOutput::failure(diagnostic.category as i64) }
}

fn checked_validator_storage_bytes(stream: &XmlTokenStream, collected_capacity: usize) -> Option<usize> {
    collected_capacity.checked_mul(size_of::<XmlToken>())?
        .checked_add(stream.token_storage_bytes())?
        .checked_add(stream.open_name_storage_bytes())?
        .checked_add(stream.visitor_storage_bytes())
}

fn checked_total_storage_bytes(metadata_bytes: usize, validator_bytes: usize) -> Option<usize> {
    if metadata_bytes > MAX_COMPONENT_BYTES || validator_bytes > MAX_COMPONENT_BYTES { return None; }
    let total = metadata_bytes.checked_add(validator_bytes)?;
    (total <= MAX_STORAGE_BYTES).then_some(total)
}

fn source_exceeds_limit(source_bytes: usize) -> bool {
    source_bytes > MAX_SOURCE_BYTES || source_bytes > MAX_LARGE_INPUT_BYTES
}

fn reserve_tokens(tokens: &mut Vec<XmlToken>, requested_capacity: usize,
    xml_storage_bytes: usize) -> Result<usize, ElementReserveError> {
    reserve_tokens_with_budget(tokens, requested_capacity, xml_storage_bytes, MAX_COMPONENT_BYTES)
}

fn next_token_capacity(current_capacity: usize, xml_storage_bytes: usize,
    component_budget: usize) -> Result<usize, ElementReserveError> {
    let legal_capacity = component_budget.checked_sub(xml_storage_bytes)
        .ok_or(ElementReserveError::Limit)? / size_of::<XmlToken>();
    if current_capacity >= legal_capacity { return Err(ElementReserveError::Limit); }
    let preferred = if current_capacity == 0 { 64 } else {
        current_capacity.checked_mul(2).unwrap_or(legal_capacity)
    };
    Ok(preferred.min(legal_capacity))
}

fn reserve_tokens_with_budget(tokens: &mut Vec<XmlToken>, requested_capacity: usize,
    xml_storage_bytes: usize, component_budget: usize) -> Result<usize, ElementReserveError> {
    let legal_capacity = component_budget.checked_sub(xml_storage_bytes)
        .ok_or(ElementReserveError::Limit)? / size_of::<XmlToken>();
    let requested_capacity = requested_capacity.min(legal_capacity);
    if requested_capacity < tokens.len() { return Err(ElementReserveError::Limit); }
    let mut candidate = Vec::new();
    candidate.try_reserve_exact(requested_capacity).map_err(|_| ElementReserveError::OutOfMemory)?;
    install_token_candidate(tokens, candidate, xml_storage_bytes, component_budget)
}

fn install_token_candidate(tokens: &mut Vec<XmlToken>, mut candidate: Vec<XmlToken>,
    xml_storage_bytes: usize, component_budget: usize) -> Result<usize, ElementReserveError> {
    let candidate_bytes = candidate.capacity().checked_mul(size_of::<XmlToken>())
        .ok_or(ElementReserveError::Limit)?;
    let total = candidate_bytes.checked_add(xml_storage_bytes).ok_or(ElementReserveError::Limit)?;
    if candidate.capacity() < tokens.len() || total > component_budget {
        return Err(ElementReserveError::Limit);
    }
    candidate.extend_from_slice(tokens);
    *tokens = candidate;
    Ok(total)
}

fn index_tag(source: &[u8], tag: ReaderTag, elements: &mut Vec<IndexElement>, element_storage: &mut usize,
    attribute_count: &mut usize, depth: &mut usize, stack: &mut [OpenElement; MAX_DEPTH], root_count: &mut usize,
    root_open_length: &mut usize, preview_record: &mut usize, preview_candidate: &mut bool,
    preview_root_supported: &mut bool, diagnostic: &mut XmlDiagnostic, preview: bool) -> bool {
    if diagnostic.category != XmlError::None { return true; }
    let closing = tag.kind == TagKind::EndElement;
    let self_closing = tag.kind == TagKind::EmptyElement;
    if tag.attribute_count > MAX_ATTRIBUTES.saturating_sub(*attribute_count) {
        set_error(diagnostic, XmlError::AttributeLimit, tag.source.start); return true;
    }
    *attribute_count += tag.attribute_count;
    if closing {
        if *depth == 0 { set_error(diagnostic, XmlError::MismatchedTag, tag.source.start); return true; }
        let top = stack[*depth - 1];
        if top.name.length != tag.name.end.saturating_sub(tag.name.start) || span_slice(source, top.name) != span_slice(source, reader_span(tag.name)) {
            set_error(diagnostic, XmlError::MismatchedTag, tag.source.start); return true;
        }
        *depth -= 1;
        if let Some(element) = elements.get_mut(top.record) {
            element.source_length = tag.source.end.saturating_sub(element.source_offset);
            element.closed = true;
            if !*preview_candidate && !preview && *depth == 1 && *preview_root_supported
                && element.support != ElementSupport::Unsupported {
                *preview_candidate = true; *preview_record = top.record;
            }
        }
        return true;
    }
    if tag.name.end.saturating_sub(tag.name.start) == 0 || tag.name.end.saturating_sub(tag.name.start) == MAX_TOKEN_NAME_BYTES {
        set_error(diagnostic, XmlError::InvalidName, tag.name.start); return true;
    }
    if *depth >= MAX_DEPTH { set_error(diagnostic, XmlError::DepthLimit, tag.source.start); return true; }
    if *depth == 0 {
        *root_count += 1;
        if *root_count > 1 { set_error(diagnostic, XmlError::MultipleRoots, tag.source.start); return true; }
    }
    if elements.len() >= MAX_ELEMENTS { set_error(diagnostic, XmlError::NodeLimit, tag.source.start); return true; }
    if elements.len() == elements.capacity() {
        let next = if elements.capacity() == 0 { 256 } else { elements.capacity().saturating_mul(2) }.min(MAX_ELEMENTS);
        let fixed_stack_bytes = size_of::<[OpenElement; MAX_DEPTH]>();
        let Some(element_budget) = MAX_COMPONENT_BYTES.checked_sub(fixed_stack_bytes) else {
            set_error(diagnostic, XmlError::NodeLimit, tag.source.start); return true;
        };
        match reserve_elements(elements, next, element_budget) {
            Ok(actual_capacity_bytes) => {
                let Some(actual_storage) = fixed_stack_bytes.checked_add(actual_capacity_bytes) else {
                    set_error(diagnostic, XmlError::NodeLimit, tag.source.start); return true;
                };
                if checked_total_storage_bytes(actual_storage, MAX_COMPONENT_BYTES).is_none() {
                    set_error(diagnostic, XmlError::NodeLimit, tag.source.start); return true;
                }
                *element_storage = actual_storage;
            }
            Err(ElementReserveError::Limit | ElementReserveError::OutOfMemory) => {
                set_error(diagnostic, XmlError::NodeLimit, tag.source.start); return true;
            }
        }
        if *element_storage > MAX_COMPONENT_BYTES {
            set_error(diagnostic, XmlError::NodeLimit, tag.source.start); return true;
        }
    }
    let name = reader_span(tag.name);
    let parent = if *depth == 0 { usize::MAX } else { stack[*depth - 1].record };
    let support = match span_slice(source, name) {
        Some(b"rect") => ElementSupport::Rect,
        Some(b"path") => ElementSupport::Path,
        _ => ElementSupport::Unsupported,
    };
    let index = elements.len();
    elements.push(IndexElement { parent, source_offset: tag.source.start,
        source_length: tag.source.end.saturating_sub(tag.source.start), name_offset: name.offset,
        name_length: name.length, depth: *depth, support, closed: self_closing });
    if *depth == 0 {
        *root_open_length = tag.source.end.saturating_sub(tag.source.start);
        *preview_root_supported = span_slice(source, name) == Some(b"svg")
            && preview_attributes_supported(source, tag, ElementSupport::Unsupported);
    }
    if *depth == 1 && elements[index].support != ElementSupport::Unsupported
        && !preview_attributes_supported(source, tag, elements[index].support) {
        elements[index].support = ElementSupport::Unsupported;
    }
    if !*preview_candidate && !preview && *depth == 1 && *preview_root_supported && self_closing
        && elements[index].support != ElementSupport::Unsupported {
        *preview_candidate = true; *preview_record = index;
    }
    if !self_closing {
        stack[*depth] = OpenElement { record: index, name };
        *depth += 1;
    }
    true
}

/// Reserve through a detached owned candidate. The existing partial index is
/// unchanged unless the candidate's actual capacity fits before copying.
fn reserve_elements(elements: &mut Vec<IndexElement>, requested_capacity: usize,
    max_capacity_bytes: usize) -> Result<usize, ElementReserveError> {
    if requested_capacity < elements.len() { return Err(ElementReserveError::Limit); }
    let mut candidate = Vec::new();
    candidate.try_reserve_exact(requested_capacity).map_err(|_| ElementReserveError::OutOfMemory)?;
    install_element_candidate(elements, candidate, max_capacity_bytes)
}

fn install_element_candidate(elements: &mut Vec<IndexElement>, mut candidate: Vec<IndexElement>,
    max_capacity_bytes: usize) -> Result<usize, ElementReserveError> {
    let candidate_bytes = candidate.capacity().checked_mul(size_of::<IndexElement>())
        .ok_or(ElementReserveError::Limit)?;
    let Some(metadata_bytes) = size_of::<[OpenElement; MAX_DEPTH]>().checked_add(candidate_bytes) else {
        return Err(ElementReserveError::Limit);
    };
    if candidate.capacity() < elements.len() || candidate_bytes > max_capacity_bytes
        || checked_total_storage_bytes(metadata_bytes, MAX_COMPONENT_BYTES).is_none() {
        return Err(ElementReserveError::Limit);
    }
    candidate.extend_from_slice(elements);
    *elements = candidate;
    Ok(candidate_bytes)
}

fn preview_attributes_supported(source: &[u8], tag: ReaderTag, shape: ElementSupport) -> bool {
    if tag.source.end.saturating_sub(tag.source.start) > MAX_STEP_BYTES { return false; }
    let Some(mut at) = tag.name.end.checked_add(0) else { return false; };
    let Some(end) = tag.source.end.checked_sub(0) else { return false; };
    let Some(bytes) = source.get(..end) else { return false; };
    let mut width = false;
    let mut height = false;
    let mut data = false;
    while at < end {
        while at < end && is_xml_space(bytes[at]) { at += 1; }
        if at >= end || matches!(bytes[at], b'>' | b'/') { break; }
        let name_start = at;
        while at < end && !is_xml_space(bytes[at]) && !matches!(bytes[at], b'=' | b'/' | b'>') { at += 1; }
        let name = &bytes[name_start..at];
        if name.is_empty() || name.contains(&b':') { return false; }
        while at < end && is_xml_space(bytes[at]) { at += 1; }
        if bytes.get(at) != Some(&b'=') { return false; }
        at += 1;
        while at < end && is_xml_space(bytes[at]) { at += 1; }
        let Some(&quote @ (b'\'' | b'"')) = bytes.get(at) else { return false; };
        at += 1;
        let value_start = at;
        while at < end && bytes[at] != quote { at += 1; }
        if at >= end { return false; }
        let value = &bytes[value_start..at];
        let accepted = match shape {
            ElementSupport::Unsupported => {
                if name == b"width" { width = true; }
                if name == b"height" { height = true; }
                if name == b"xmlns" && value != b"http://www.w3.org/2000/svg" { return false; }
                matches!(name, b"xmlns" | b"version" | b"width" | b"height" | b"viewBox")
            }
            ElementSupport::Rect => {
                if name == b"width" { width = true; }
                if name == b"height" { height = true; }
                matches!(name, b"fill" | b"fill-rule" | b"x" | b"y" | b"width" | b"height")
            }
            ElementSupport::Path => {
                if name == b"d" { data = true; }
                matches!(name, b"fill" | b"fill-rule" | b"d")
            }
        };
        if !accepted { return false; }
        at += 1;
    }
    match shape {
        ElementSupport::Unsupported => width && height,
        ElementSupport::Rect => width && height,
        ElementSupport::Path => data,
    }
}

fn is_xml_space(byte: u8) -> bool { matches!(byte, b' ' | b'\t' | b'\r' | b'\n') }
fn span_slice(source: &[u8], span: XmlSpan) -> Option<&[u8]> { source.get(span.offset..span.offset.checked_add(span.length)?) }
fn reader_span(span: vexel_base::Span) -> XmlSpan { XmlSpan { offset: span.start, length: span.end.saturating_sub(span.start) } }
fn set_error(diagnostic: &mut XmlDiagnostic, category: XmlError, offset: usize) {
    if diagnostic.category == XmlError::None { *diagnostic = XmlDiagnostic { category, offset }; }
}
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> { mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner) }

#[cfg(test)]
mod index_tests;
