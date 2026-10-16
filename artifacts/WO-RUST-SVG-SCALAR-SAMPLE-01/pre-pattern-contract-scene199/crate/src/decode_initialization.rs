//! Explicit source-owned decode lifecycle. Traversal never receives a ready view.
use crate::memory::{add, arithmetic};
use crate::model::SceneData;
use crate::types::{invalid, limit, stopped};
use crate::*;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeInitialization {
    /// Root traversal has not established a viewport or hidden disposition.
    Unresolved,
    /// Root viewport is validated; actual same-source animation is still absent.
    Traversing,
    /// Root display:none skipped viewport and descendants; animation is absent.
    Hidden,
    /// Empty hidden document with its actual same-source animation installed.
    HiddenFinalized,
    Finalized,
}
/// Failure returns the actual input owner unchanged; callers can retry or drop it.
#[derive(Debug)]
pub struct FinalizationError {
    pub diagnostic: Diagnostic,
    pub animation: OwnedAnimation,
}
/// No apparently completed empty animation exists during traversal.
#[derive(Debug)]
pub(crate) enum AnimationStorage { Pending, Parsed(OwnedAnimation) }
impl AnimationStorage {
    pub(crate) fn parsed(&self) -> Option<&OwnedAnimation> {
        match self { Self::Pending => None, Self::Parsed(value) => Some(value) }
    }
    pub(crate) fn into_parsed(self) -> Option<OwnedAnimation> {
        match self { Self::Pending => None, Self::Parsed(value) => Some(value) }
    }
    pub(crate) fn storage_bytes(&self) -> Option<usize> {
        self.parsed()?.storage_bytes()
    }
}
impl SceneBuilder {
    pub fn provisional(source: SourceOwner, generation: u64, limits: Limits) -> Result<Self> {
        let limits = limits.effective();
        if source.len() > limits.source_bytes { return Err(limit(LimitKind::SourceBytes)); }
        let live = source.capacity();
        if live > limits.reserved_bytes { return Err(limit(LimitKind::ReservedBytes)); }
        let owner = crate::builder::NEXT_OWNER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| arithmetic())?;
        let mut value = Self {
            owner, source, generation, scenes: Vec::new(), animation: AnimationStorage::Pending,
            initialization: DecodeInitialization::Unresolved,
            ignored: [Ignored::default(); 4],
            stats: MemoryStats { source_bytes: live, live_bytes: live,
                peak_live_bytes: live, ..Default::default() },
            limits, drafts: [None; 64], draft_depth: 0, request: None,
            text_commands: 0, next_serial: 1,
            root: SceneId { owner, slot: 0, serial: 1 },
        };
        let root = value.reserve_scene()?;
        value.scenes.push(SceneData::new(root.serial, None));
        value.root = root;
        Ok(value)
    }
    pub fn initialization(&self) -> DecodeInitialization { self.initialization }
    pub fn generation(&self) -> u64 { self.generation }
    pub(crate) fn require_finalized(&self) -> Result<()> {
        if self.initialization != DecodeInitialization::Finalized || self.animation.parsed().is_none() {
            return Err(invalid(0));
        }
        Ok(())
    }
    pub fn traversal_scene(&self, id: SceneId) -> Result<TraversalView<'_>> {
        let i = self.index(id)?;
        Ok(self.scenes[i].view(self.owner, i, &self.scenes, &self.source,
            self.animation.parsed().map(OwnedAnimation::view), self.scenes[i].viewport,
            self.ignored))
    }
    fn root_witness(&self, source: &DecodeSource<'_>, root: ElementId,
        control: &mut dyn WorkControl) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        if self.initialization != DecodeInitialization::Unresolved
            || !std::sync::Arc::ptr_eq(&self.source, source.source_owner()) {
            return Err(invalid(0));
        }
        let element = source.element(root, add(root.0, 1)?, control)?;
        let token = &element.tokens()[root.0];
        if token.depth != 0 || token.local_name.slice(&self.source) != Some(b"svg") {
            return Err(invalid(token.name.offset));
        }
        Ok(())
    }
    // Guard the hidden bypass with the canonical CSS engine, including original
    // invalid-display priority. This is a root query, never a second source parse.
    fn root_display(&self, source: &mut DecodeSource<'_>, root: ElementId,
        control: &mut dyn WorkControl) -> Result<bool> {
        self.root_witness(source, root, control)?;
        let style = source.compute_style(root, None, control)?;
        let Some(value) = style.value(vexel_svg_css::Property::Display) else { return Ok(false) };
        if style.keyword_equals(vexel_svg_css::Property::Display, b"none") { return Ok(true); }
        if [b"inline".as_slice(), b"block", b"contents"].iter()
            .any(|word| style.keyword_equals(vexel_svg_css::Property::Display, word)) {
            return Ok(false);
        }
        Err(Diagnostic { kind: ErrorKind::Unsupported, offset: value.span.offset })
    }
    /// Called after original root attribute/style/display/transform checks.
    /// Parses only canonical root attributes with the accepted values engine.
    /// Caller-supplied viewport values cannot authorize provisional publication.
    /// ```compile_fail
    /// use vexel_scene::{SceneBuilder, DecodeSource, ElementId, Viewport, WorkControl};
    /// fn forged(b: &mut SceneBuilder, s: &mut DecodeSource<'_>, v: Viewport,
    ///           c: &mut dyn WorkControl) {
    ///     b.initialize_root(s, ElementId(0), v, c).unwrap();
    /// }
    /// ```
    pub fn initialize_root(&mut self, source: &mut DecodeSource<'_>, root: ElementId,
        control: &mut dyn WorkControl) -> Result<()> {
        if self.root_display(source, root, control)? { return Err(invalid(0)); }
        let viewport = self.canonical_root_viewport(source, root, control)?;
        crate::validate::viewport(viewport)?;
        control.checkpoint().map_err(stopped)?;
        self.scenes[0].viewport = Some(viewport);
        self.initialization = DecodeInitialization::Traversing;
        Ok(())
    }
    // Only these five unprefixed attributes affect the original root parser.
    // Fixed stack storage borrows canonical spans; there is no attribute Vec,
    // alternate source, tokenizer replay or public unchecked proof conversion.
    fn canonical_root_viewport(&self, source: &DecodeSource<'_>, root: ElementId,
        control: &mut dyn WorkControl) -> Result<Viewport> {
        use vexel_svg_values::{Attribute, AttributeList, ValueLimits, MappingErrorKind};
        let empty = Attribute { name: Span { start: 0, end: 0 },
            value: Span { start: 0, end: 0 }, prefix: None };
        let mut attributes = [empty; 5];
        let mut count = 0;
        let token = source.tokens().get(root.0).ok_or_else(|| invalid(0))?;
        let first = add(root.0, 1)?;
        for attribute in &source.tokens()[first..] {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            if attribute.kind != vexel_xml::TokenKind::Attribute { break; }
            if attribute.prefix.length != 0 { continue; }
            let name = attribute.name.slice(source.source()).ok_or_else(|| invalid(attribute.name.offset))?;
            if ![b"xmlns".as_slice(), b"width", b"height", b"viewBox", b"preserveAspectRatio"]
                .contains(&name) { continue; }
            if count == attributes.len() { return Err(limit(LimitKind::Attributes)); }
            let end = attribute.value.end().ok_or_else(arithmetic)?;
            let value = source.source().get(attribute.value.offset..end)
                .ok_or_else(|| invalid(attribute.value.offset))?;
            // The accepted values parser is synchronous. Poll controlled source
            // admission in bounded batches before its indivisible bounded call.
            for chunk in value.chunks(256) {
                control.charge(WorkKind::SourceRead, chunk.len() as u64).map_err(stopped)?;
            }
            attributes[count] = Attribute {
                name: Span { start: attribute.name.offset, end: attribute.name.end().ok_or_else(arithmetic)? },
                value: Span { start: attribute.value.offset, end }, prefix: None,
            };
            count += 1;
        }
        control.checkpoint().map_err(stopped)?;
        let list = AttributeList::new_with_limits(source.source(),
            Span { start: token.name.offset, end: token.name.end().ok_or_else(arithmetic)? },
            &attributes[..count], ValueLimits { source_bytes: self.limits.source_bytes, attributes: 5 })
            .map_err(|_| invalid(token.name.offset))?;
        let parsed = vexel_svg_values::parse_root_viewport(list).map_err(|error| Diagnostic {
            kind: match error.kind {
                MappingErrorKind::InvalidArgument => ErrorKind::InvalidArgument,
                MappingErrorKind::Number => ErrorKind::Number,
                MappingErrorKind::Viewport => ErrorKind::Viewport,
                MappingErrorKind::Unsupported => ErrorKind::Unsupported,
            }, offset: error.offset,
        })?;
        control.checkpoint().map_err(stopped)?;
        Ok(Viewport { width: parsed.width, height: parsed.height,
            view_x: parsed.view_x, view_y: parsed.view_y,
            view_width: parsed.view_width, view_height: parsed.view_height,
            aspect: parsed.preserve_aspect })
    }
    /// Called only after original attribute/style/display validation establishes
    /// root display:none. Transform/viewport parsing and descendants are skipped.
    pub fn initialize_hidden_root(&mut self, source: &mut DecodeSource<'_>, root: ElementId,
        control: &mut dyn WorkControl) -> Result<()> {
        if !self.root_display(source, root, control)? { return Err(invalid(0)); }
        self.require_empty_hidden()?;
        control.checkpoint().map_err(stopped)?;
        self.initialization = DecodeInitialization::Hidden;
        Ok(())
    }
    fn require_empty_hidden(&self) -> Result<()> {
        if self.scenes.len() != 1 || self.scenes[0].lengths().iter().any(|n| *n != 0)
            || !self.scenes[0].publications.is_empty() || self.draft_depth != 0
            || self.request.is_some() || self.text_commands != 0 {
            return Err(invalid(0));
        }
        Ok(())
    }
    /// Admit the actual late parser result after successful traversal/root_seen.
    /// No replacement is allowed. Admission counts source once and all actual
    /// animation capacities, including nested spare capacity. Failed admission
    /// leaves initialization and all scene identities intact and returns input.
    /// Active drafts/requests survive installation so finalized child renders can
    /// finish deferred publication. Freeze still rejects every pending scope.
    pub fn finalize_decode(&mut self, animation: OwnedAnimation, generation: u64,
        control: &mut dyn WorkControl) -> std::result::Result<(), FinalizationError> {
        let result = (|| {
            control.checkpoint().map_err(stopped)?;
            if self.animation.parsed().is_some() || generation != self.generation
                || !matches!(self.initialization, DecodeInitialization::Traversing | DecodeInitialization::Hidden)
                || !std::sync::Arc::ptr_eq(&self.source, animation.source_owner()) {
                return Err(invalid(0));
            }
            if self.initialization == DecodeInitialization::Hidden { self.require_empty_hidden()?; }
            // storage_bytes performs fixed arithmetic per track; poll at each
            // actual retained track before that bounded measurement, not per key.
            for _ in animation.tracks() {
                control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            }
            let owned = animation.storage_bytes().ok_or_else(arithmetic)?;
            let incoming = owned.checked_sub(self.source.capacity()).ok_or_else(arithmetic)?;
            let live = add(self.stats.live_bytes, incoming)?;
            let history = add(self.stats.total_allocated_bytes, incoming)?;
            let count = self.stats.allocation_count.checked_add(usize::from(incoming != 0))
                .ok_or_else(arithmetic)?;
            if live > self.limits.reserved_bytes { return Err(limit(LimitKind::ReservedBytes)); }
            control.checkpoint().map_err(stopped)?;
            self.stats.live_bytes = live;
            self.stats.peak_live_bytes = self.stats.peak_live_bytes.max(live);
            self.stats.total_allocated_bytes = history;
            self.stats.allocation_count = count;
            Ok(())
        })();
        if let Err(diagnostic) = result { return Err(FinalizationError { diagnostic, animation }); }
        self.animation = AnimationStorage::Parsed(animation);
        if self.initialization == DecodeInitialization::Traversing {
            self.initialization = DecodeInitialization::Finalized;
        } else {
            self.initialization = DecodeInitialization::HiddenFinalized;
        }
        Ok(())
    }
    /// Complete decode with an explicit hidden result; ordinary freeze never
    /// publishes a drawable graph without a positive validated viewport.
    pub fn freeze_decode(mut self, control: &mut dyn WorkControl) -> Result<DecodedScene> {
        if !matches!(self.initialization, DecodeInitialization::Hidden | DecodeInitialization::HiddenFinalized) {
            return self.freeze_controlled(control).map(DecodedScene::Drawable);
        }
        control.checkpoint().map_err(stopped)?;
        self.require_empty_hidden()?;
        let animation = self.animation.parsed().ok_or_else(|| invalid(0))?;
        let measured = add(animation.storage_bytes().ok_or_else(arithmetic)?,
            crate::memory::bytes(&self.scenes)?)?;
        let measured = add(measured, self.scenes[0].storage_bytes_controlled(control)?)?;
        if measured != self.stats.live_bytes || measured > self.limits.reserved_bytes {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)?;
        self.scenes[0].sealed = true;
        Ok(DecodedScene::Hidden(HiddenScene { storage: self }))
    }
}
#[derive(Debug)]
pub enum DecodedScene { Drawable(SceneGraph), Hidden(HiddenScene) }
/// Immutable completed empty document with no viewport and actual parsed animation.
/// It has no conversion to a drawable graph or scalar/frame render input.
#[derive(Debug)]
pub struct HiddenScene { storage: SceneBuilder }
impl HiddenScene {
    pub fn source_owner(&self) -> &SourceOwner { self.storage.source_owner() }
    pub fn source(&self) -> &[u8] { self.storage.source_owner() }
    pub fn generation(&self) -> u64 { self.storage.generation }
    pub fn memory_stats(&self) -> MemoryStats { self.storage.stats }
    pub fn view(&self) -> TraversalView<'_> {
        self.storage.traversal_scene(self.storage.root_id()).expect("validated hidden root")
    }
    pub fn animation(&self) -> AnimationView<'_> {
        self.storage.animation.parsed().expect("finalized hidden animation").view()
    }
    /// A hidden frame is an immutable empty query, with no dimensions to rasterize.
    pub fn frame(&self) -> HiddenFrame<'_> { HiddenFrame { view: self.view() } }
}
#[derive(Debug, Clone, Copy)]
pub struct HiddenFrame<'a> { view: TraversalView<'a> }
impl<'a> HiddenFrame<'a> { pub fn view(&self) -> TraversalView<'a> { self.view } }
