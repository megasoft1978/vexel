use vexel_base::{WorkControl, WorkKind};
use vexel_scene::{
    Diagnostic, ErrorKind, LimitKind, MediaError, MediaProvider, Result, TextLayout, TextRequest,
};

pub const TEXT_CACHE_ENTRIES: usize = 32;
pub const TEXT_CACHE_BYTES: usize = 256 * 1024;
pub const MAX_PATH_COMMANDS: usize = 262_144;
pub const MAX_GLYPHS: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub struct TextCacheKey {
    pub text: Vec<u8>,
    pub families: Vec<Vec<u8>>,
    pub size_points: f64,
    pub weight: f64,
    pub letter_spacing_points: f64,
    pub style: vexel_scene::TextStyle,
}
impl TextCacheKey {
    pub fn matches(&self, request: TextRequest<'_>) -> bool {
        self.text == request.text
            && self.families.len() == request.font_families.len()
            && self
                .families
                .iter()
                .zip(request.font_families)
                .all(|(a, b)| a.as_slice() == *b)
            && self.size_points == request.size_points
            && self.weight == request.weight
            && self.letter_spacing_points == request.letter_spacing_points
            && self.style == request.style
    }
}

#[derive(Debug)]
struct Entry {
    key: TextCacheKey,
    layout: TextLayout,
}
#[derive(Debug, Default)]
pub struct TextCache {
    entries: Vec<Entry>,
    text_bytes: usize,
    commands: usize,
}

/// Borrowed cache hit or an owned provider result when admission prevents caching.
pub enum LayoutResult<'a> {
    Cached(&'a TextLayout),
    Owned(TextLayout),
}
impl LayoutResult<'_> {
    pub fn layout(&self) -> &TextLayout {
        match self {
            Self::Cached(v) => v,
            Self::Owned(v) => v,
        }
    }
    pub fn is_cached(&self) -> bool {
        matches!(self, Self::Cached(_))
    }
}

impl TextCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn stored_text_bytes(&self) -> usize {
        self.text_bytes
    }
    pub fn stored_commands(&self) -> usize {
        self.commands
    }

    /// Run explicit provider layout and validate the owned output before caching.
    pub fn layout<'a>(
        &'a mut self,
        request: TextRequest<'_>,
        provider: &mut dyn MediaProvider,
        control: &mut dyn WorkControl,
        offset: usize,
    ) -> Result<LayoutResult<'a>> {
        request.validate(control)?;
        if let Some(i) = self
            .entries
            .iter()
            .position(|entry| entry.key.matches(request))
        {
            return Ok(LayoutResult::Cached(&self.entries[i].layout));
        }
        let layout = provider
            .layout_text(request, control)
            .map_err(|e| provider_error(e, offset))?;
        validate_layout(&layout, offset, control)?;
        let text_ok = request.text.len() <= TEXT_CACHE_BYTES.saturating_sub(self.text_bytes);
        let command_ok = layout.commands.len() <= MAX_PATH_COMMANDS.saturating_sub(self.commands);
        if self.entries.len() < TEXT_CACHE_ENTRIES && text_ok && command_ok {
            let key = try_key(request);
            if let Some(key) = key {
                if self.entries.try_reserve(1).is_ok() {
                    self.text_bytes += request.text.len();
                    self.commands += layout.commands.len();
                    self.entries.push(Entry { key, layout });
                    let last = self.entries.len() - 1;
                    return Ok(LayoutResult::Cached(&self.entries[last].layout));
                }
            }
        }
        Ok(LayoutResult::Owned(layout))
    }
}

/// Layout through the accepted scene request cache, keeping source-only cache
/// tests useful while production requests share the coordinator-owned cache.
pub(crate) fn layout_shared<'a>(
    cache: &'a mut vexel_scene::TextCache,
    request: TextRequest<'_>,
    family_spans: &[vexel_base::Span],
    provider: &mut dyn MediaProvider,
    control: &mut dyn WorkControl,
    offset: usize,
) -> Result<LayoutResult<'a>> {
    request.validate(control)?;
    let cached = cache.entries.iter().position(|entry| {
        entry.text == request.text
            && entry.owned_families.len() == request.font_families.len()
            && entry
                .owned_families
                .iter()
                .zip(request.font_families)
                .all(|(a, b)| a.as_slice() == *b)
            && entry.size_points == request.size_points
            && entry.weight == request.weight
            && entry.letter_spacing_points == request.letter_spacing_points
            && entry.style == request.style
    });
    if let Some(index) = cached {
        return Ok(LayoutResult::Cached(&cache.entries[index].layout));
    }
    let layout = provider
        .layout_text(request, control)
        .map_err(|e| provider_error(e, offset))?;
    validate_layout(&layout, offset, control)?;
    let text_bytes = cache
        .entries
        .iter()
        .try_fold(0usize, |sum, entry| sum.checked_add(entry.text.len()));
    let commands = cache.entries.iter().try_fold(0usize, |sum, entry| {
        sum.checked_add(entry.layout.commands.len())
    });
    let text_bytes = text_bytes.ok_or_else(|| Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Arithmetic),
        offset,
    })?;
    let commands = commands.ok_or_else(|| Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Arithmetic),
        offset,
    })?;
    let can_cache = cache.entries.len() < TEXT_CACHE_ENTRIES
        && request.text.len() <= TEXT_CACHE_BYTES.saturating_sub(text_bytes)
        && layout.commands.len() <= MAX_PATH_COMMANDS.saturating_sub(commands);
    if can_cache {
        let mut text = Vec::new();
        let mut owned_families = Vec::new();
        let mut stored_family_spans = Vec::new();
        if text.try_reserve_exact(request.text.len()).is_ok()
            && owned_families
                .try_reserve_exact(request.font_families.len())
                .is_ok()
            && stored_family_spans
                .try_reserve_exact(family_spans.len())
                .is_ok()
        {
            text.extend_from_slice(request.text);
            stored_family_spans.extend_from_slice(family_spans);
            let mut complete = true;
            for family in request.font_families {
                let mut owned = Vec::new();
                if owned.try_reserve_exact(family.len()).is_err() {
                    complete = false;
                    break;
                }
                owned.extend_from_slice(family);
                owned_families.push(owned);
            }
            if complete && cache.entries.try_reserve(1).is_ok() {
                cache.text_bytes = text_bytes + request.text.len();
                cache.commands = commands + layout.commands.len();
                cache.entries.push(vexel_scene::TextCacheEntry {
                    text,
                    family_spans: stored_family_spans,
                    owned_families,
                    size_points: request.size_points,
                    weight: request.weight,
                    letter_spacing_points: request.letter_spacing_points,
                    style: request.style,
                    layout,
                });
                return Ok(LayoutResult::Cached(
                    &cache.entries.last().expect("inserted").layout,
                ));
            }
        }
    }
    Ok(LayoutResult::Owned(layout))
}

fn try_key(request: TextRequest<'_>) -> Option<TextCacheKey> {
    let mut text = Vec::new();
    text.try_reserve_exact(request.text.len()).ok()?;
    text.extend_from_slice(request.text);
    let mut families = Vec::new();
    families
        .try_reserve_exact(request.font_families.len())
        .ok()?;
    for family in request.font_families {
        let mut copy = Vec::new();
        copy.try_reserve_exact(family.len()).ok()?;
        copy.extend_from_slice(family);
        families.push(copy);
    }
    Some(TextCacheKey {
        text,
        families,
        size_points: request.size_points,
        weight: request.weight,
        letter_spacing_points: request.letter_spacing_points,
        style: request.style,
    })
}
fn validate_layout(
    layout: &TextLayout,
    offset: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let invalid = || Diagnostic {
        kind: ErrorKind::Unsupported,
        offset,
    };
    if layout.glyphs.len() > MAX_GLYPHS
        || layout.commands.len() > MAX_PATH_COMMANDS
        || !layout.advance_width.is_finite()
    {
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::TextCommands),
            offset,
        });
    }
    for glyph in &layout.glyphs {
        control
            .charge(WorkKind::Media, 1)
            .map_err(|stop| work_error(stop, offset))?;
        let end = glyph
            .commands
            .first
            .checked_add(glyph.commands.count)
            .ok_or_else(|| invalid())?;
        if !glyph.x.is_finite()
            || !glyph.y.is_finite()
            || !glyph.advance.is_finite()
            || end > layout.commands.len()
        {
            return Err(invalid());
        }
    }
    for command in &layout.commands {
        control
            .charge(WorkKind::Media, 1)
            .map_err(|stop| work_error(stop, offset))?;
        if command.values.iter().any(|v| !v.is_finite()) {
            return Err(invalid());
        }
    }
    Ok(())
}
fn work_error(stop: vexel_base::Stop, offset: usize) -> Diagnostic {
    Diagnostic {
        kind: match stop {
            vexel_base::Stop::Canceled => ErrorKind::Canceled,
            vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset,
    }
}
fn provider_error(error: MediaError, offset: usize) -> Diagnostic {
    let kind = match error {
        MediaError::Limit => ErrorKind::Limit(LimitKind::TextCommands),
        MediaError::OutOfMemory => ErrorKind::OutOfMemory,
        MediaError::InvalidArgument => ErrorKind::InvalidArgument,
        MediaError::Unsupported
        | MediaError::Invalid
        | MediaError::InvalidUtf8
        | MediaError::Animated
        | MediaError::Corrupt => ErrorKind::Unsupported,
    };
    Diagnostic { kind, offset }
}

/// Release semantics are Rust ownership: cached layouts drop once with the cache.
pub fn release_layout(layout: TextLayout) {
    drop(layout);
}

/// Work-accounting helper for consumers that traverse outline verbs.
pub fn charge_outline(control: &mut dyn WorkControl, units: usize) -> Result<()> {
    control
        .charge(
            WorkKind::Media,
            u64::try_from(units).map_err(|_| Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Arithmetic),
                offset: 0,
            })?,
        )
        .map_err(|stop| Diagnostic {
            kind: match stop {
                vexel_base::Stop::Canceled => ErrorKind::Canceled,
                vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
                vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
            },
            offset: 0,
        })
}
