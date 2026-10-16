//! CoreText glyph-outline layout for SVG `<text>` (macOS only).
//!
//! A port of the legacy C provider (`platform_text.m`) combined with the
//! glyph-local conversion of `app_text_layout`: shaped glyph runs are walked,
//! each glyph outline is read through `CGPathApply`, quadratic segments are
//! elevated to cubics exactly as the C does, and command coordinates are
//! returned relative to the glyph origin (y up, in points).
//!
//! Threading: Core Text fonts, lines and runs created here are private to one
//! call, so no main-thread proof is required; the module neither acquires nor
//! fabricates [`crate::MainThread`] and touches no AppKit state.
//!
//! Every entry runs inside [`guard_native_call`]; the `CGPathApply` callback
//! additionally guards itself and latches a fault. All raw Core Foundation
//! ownership is held by private [`Cf`] owners that release exactly once.
//! The two unexported Core Text constants (`kCTFontWeightTrait`,
//! `kCTFontSymbolicTrait`) are declared locally because the pinned binding
//! feature `CTFontTraits` is not enabled (the existing text module does the
//! same); `kCTFontItalicTrait` is the documented value `1 << 0`.
#![cfg(target_os = "macos")]

use crate::native_boundary::{NativeFault, guard_native_call};
use objc2_core_foundation::{
    kCFTypeArrayCallBacks, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use objc2_core_text::{
    kCTFontAttributeName, kCTFontCascadeListAttribute, kCTFontFamilyNameAttribute,
    kCTFontSizeAttribute, kCTFontTraitsAttribute, kCTKernAttributeName, kCTLigatureAttributeName,
};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

/// C `PLATFORM_TEXT_MAX_BYTES`.
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
/// C `PLATFORM_TEXT_MAX_GLYPHS`.
pub const MAX_GLYPHS: usize = 16384;
/// C `PLATFORM_TEXT_MAX_PATH_SEGMENTS`.
pub const MAX_PATH_SEGMENTS: usize = 262144;
/// C `PLATFORM_TEXT_MAX_FAMILIES`.
pub const MAX_FAMILIES: usize = 32;
/// C `PLATFORM_TEXT_MAX_FAMILY_BYTES`.
pub const MAX_FAMILY_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextVerb {
    Move,
    Line,
    Cubic,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextCommand {
    pub verb: TextVerb,
    /// Move/Line: x,y. Cubic: c1x,c1y,c2x,c2y,x,y. Close: unused (zero).
    pub values: [f64; 6],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextGlyph {
    pub glyph_id: u16,
    pub x: f64,
    pub y: f64,
    pub advance: f64,
    pub command_start: usize,
    pub command_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextLayout {
    pub glyphs: Vec<TextGlyph>,
    pub commands: Vec<TextCommand>,
    pub advance_width: f64,
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
}

/// Borrowed request; `font_families` are UTF-8 in fallback order.
#[derive(Clone, Copy, Debug)]
pub struct TextRequest<'a> {
    pub text: &'a [u8],
    pub font_families: &'a [&'a [u8]],
    pub size_points: f64,
    pub weight: f64,
    pub style: TextStyle,
    pub letter_spacing_points: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextLayoutError {
    InvalidArgument,
    InvalidUtf8,
    Limit,
    Unsupported,
    OutOfMemory,
    /// A Rust panic was caught at the native boundary.
    Fault(NativeFault),
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CfRange {
    location: isize,
    length: isize,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Size {
    width: f64,
    height: f64,
}
#[repr(C)]
struct PathElement {
    kind: i32,
    points: *const Point,
}
type Applier = extern "C" fn(*mut c_void, *const PathElement);

const UTF8: u32 = 0x0800_0100;
const NUMBER_SINT32: isize = 3;
const NUMBER_DOUBLE: isize = 13;
const ITALIC_TRAIT: i32 = 1;

#[allow(clashing_extern_declarations)]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithBytes(
        alloc: *const c_void,
        bytes: *const u8,
        count: isize,
        encoding: u32,
        external: bool,
    ) -> *const c_void;
    fn CFStringGetLength(s: *const c_void) -> isize;
    fn CFNumberCreate(alloc: *const c_void, kind: isize, value: *const c_void) -> *const c_void;
    fn CFDictionaryCreateMutable(
        alloc: *const c_void,
        capacity: isize,
        key_cb: *const c_void,
        value_cb: *const c_void,
    ) -> *const c_void;
    fn CFDictionarySetValue(d: *const c_void, key: *const c_void, value: *const c_void);
    fn CFDictionaryGetValue(d: *const c_void, key: *const c_void) -> *const c_void;
    fn CFArrayCreateMutable(
        alloc: *const c_void,
        capacity: isize,
        cb: *const c_void,
    ) -> *const c_void;
    fn CFArrayAppendValue(a: *const c_void, value: *const c_void);
    fn CFArrayGetCount(a: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(a: *const c_void, index: isize) -> *const c_void;
    fn CFAttributedStringCreateMutable(alloc: *const c_void, max: isize) -> *const c_void;
    fn CFAttributedStringReplaceString(a: *const c_void, range: CfRange, s: *const c_void);
    fn CFAttributedStringSetAttributes(
        a: *const c_void,
        range: CfRange,
        attrs: *const c_void,
        clear: bool,
    );
    fn CFRelease(v: *const c_void);
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPathApply(path: *const c_void, info: *mut c_void, applier: Applier);
}
// Opaque pointer types differ from the editing module's private declarations
// of the same ABI; the signatures are ABI-identical.
#[allow(clashing_extern_declarations)]
#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    static kCTFontWeightTrait: &'static objc2_core_foundation::CFString;
    static kCTFontSymbolicTrait: &'static objc2_core_foundation::CFString;
    fn CTFontDescriptorCreateWithAttributes(attrs: *const c_void) -> *const c_void;
    fn CTFontDescriptorCreateCopyWithAttributes(
        d: *const c_void,
        attrs: *const c_void,
    ) -> *const c_void;
    fn CTFontCreateWithFontDescriptor(
        d: *const c_void,
        size: f64,
        matrix: *const c_void,
    ) -> *const c_void;
    fn CTFontGetAscent(f: *const c_void) -> f64;
    fn CTFontGetDescent(f: *const c_void) -> f64;
    fn CTFontGetLeading(f: *const c_void) -> f64;
    fn CTFontCreatePathForGlyph(
        f: *const c_void,
        glyph: u16,
        matrix: *const c_void,
    ) -> *const c_void;
    fn CTLineCreateWithAttributedString(s: *const c_void) -> *const c_void;
    fn CTLineGetTypographicBounds(l: *const c_void, a: *mut f64, d: *mut f64, g: *mut f64) -> f64;
    fn CTLineGetGlyphRuns(l: *const c_void) -> *const c_void;
    fn CTRunGetGlyphCount(r: *const c_void) -> isize;
    fn CTRunGetGlyphs(r: *const c_void, range: CfRange, out: *mut u16);
    fn CTRunGetPositions(r: *const c_void, range: CfRange, out: *mut Point);
    fn CTRunGetAdvances(r: *const c_void, range: CfRange, out: *mut Size);
    fn CTRunGetAttributes(r: *const c_void) -> *const c_void;
}

/// Owned retained Core Foundation object, released once on drop.
struct Cf(*const c_void);
impl Cf {
    fn new(p: *const c_void) -> Option<Self> {
        // Not `then_some`: that would build (and drop, releasing NULL) eagerly.
        if p.is_null() { None } else { Some(Self(p)) }
    }
}
impl Drop for Cf {
    fn drop(&mut self) {
        // SAFETY: every `Cf` wraps exactly one +1 reference from a Create/Copy call.
        unsafe { CFRelease(self.0) }
    }
}

fn cf_string(bytes: &[u8]) -> Option<Cf> {
    // SAFETY: pointer/length come from a live slice; CF copies the bytes.
    Cf::new(unsafe {
        CFStringCreateWithBytes(
            ptr::null(),
            bytes.as_ptr(),
            bytes.len() as isize,
            UTF8,
            false,
        )
    })
}
fn cf_number(kind: isize, value: *const c_void) -> Option<Cf> {
    // SAFETY: `value` points at a live f64/i32 matching `kind`.
    Cf::new(unsafe { CFNumberCreate(ptr::null(), kind, value) })
}
fn cf_f64(v: f64) -> Option<Cf> {
    cf_number(NUMBER_DOUBLE, ptr::addr_of!(v).cast())
}
fn cf_i32(v: i32) -> Option<Cf> {
    cf_number(NUMBER_SINT32, ptr::addr_of!(v).cast())
}
fn cf_dict() -> Option<Cf> {
    // SAFETY: the callback tables are process-lifetime CoreFoundation statics.
    Cf::new(unsafe {
        CFDictionaryCreateMutable(
            ptr::null(),
            0,
            ptr::addr_of!(kCFTypeDictionaryKeyCallBacks).cast(),
            ptr::addr_of!(kCFTypeDictionaryValueCallBacks).cast(),
        )
    })
}
fn key(k: &'static objc2_core_foundation::CFString) -> *const c_void {
    (k as *const objc2_core_foundation::CFString).cast()
}
fn set(d: &Cf, k: *const c_void, v: &Cf) {
    // SAFETY: both are live CF objects; the dictionary retains the value.
    unsafe { CFDictionarySetValue(d.0, k, v.0) }
}

fn ascii_eq(bytes: &[u8], literal: &str) -> bool {
    bytes.eq_ignore_ascii_case(literal.as_bytes())
}

fn family_string(family: &[u8]) -> Option<Cf> {
    if ascii_eq(family, "serif") {
        cf_string(b"Times")
    } else if ascii_eq(family, "sans-serif") {
        cf_string(b"Helvetica Neue")
    } else if ascii_eq(family, "monospace") {
        cf_string(b"Menlo")
    } else {
        cf_string(family)
    }
}

fn descriptor(family: &[u8], weight: f64, style: TextStyle) -> Option<Cf> {
    let name = family_string(family)?;
    let core_weight = ((weight - 400.0) / 500.0).clamp(-0.8, 0.8);
    let weight_number = cf_f64(core_weight)?;
    let symbolic = cf_i32(if style == TextStyle::Normal {
        0
    } else {
        ITALIC_TRAIT
    })?;
    let traits = cf_dict()?;
    let attributes = cf_dict()?;
    // SAFETY: Core Text exports these process-lifetime immutable CFString keys.
    unsafe {
        set(&traits, key(kCTFontWeightTrait), &weight_number);
        set(&traits, key(kCTFontSymbolicTrait), &symbolic);
        set(&attributes, key(kCTFontFamilyNameAttribute), &name);
        set(&attributes, key(kCTFontTraitsAttribute), &traits);
        Cf::new(CTFontDescriptorCreateWithAttributes(attributes.0))
    }
}

fn is_emoji(c: char) -> bool {
    let c = c as u32;
    (0x1f000..=0x1faff).contains(&c) || (0x2600..=0x27bf).contains(&c)
}

fn validate(req: &TextRequest<'_>) -> Result<(), TextLayoutError> {
    use TextLayoutError::*;
    if req.text.len() > MAX_TEXT_BYTES || req.font_families.len() > MAX_FAMILIES {
        return Err(Limit);
    }
    if !req.size_points.is_finite()
        || req.size_points <= 0.0
        || req.size_points > 4096.0
        || !req.weight.is_finite()
        || !(100.0..=900.0).contains(&req.weight)
        || !req.letter_spacing_points.is_finite()
        || req.letter_spacing_points.abs() > 4096.0
    {
        return Err(InvalidArgument);
    }
    for family in req.font_families {
        if family.len() > MAX_FAMILY_BYTES {
            return Err(Limit);
        }
        if family.is_empty() || family.contains(&0) {
            return Err(InvalidArgument);
        }
        if std::str::from_utf8(family).is_err() {
            return Err(InvalidUtf8);
        }
    }
    let text = std::str::from_utf8(req.text).map_err(|_| InvalidUtf8)?;
    if text.chars().any(is_emoji) {
        return Err(Unsupported);
    }
    Ok(())
}

struct PathContext<'a> {
    commands: &'a mut Vec<TextCommand>,
    error: Option<TextLayoutError>,
    offset: Point,
    current: Point,
}

impl PathContext<'_> {
    fn append(&mut self, verb: TextVerb, values: &[f64]) {
        if self.error.is_some() {
            return;
        }
        if self.commands.len() >= MAX_PATH_SEGMENTS {
            self.error = Some(TextLayoutError::Limit);
            return;
        }
        if self.commands.try_reserve(1).is_err() {
            self.error = Some(TextLayoutError::OutOfMemory);
            return;
        }
        let mut out = [0.0; 6];
        for (slot, v) in out.iter_mut().zip(values) {
            if !v.is_finite() {
                self.error = Some(TextLayoutError::Unsupported);
                return;
            }
            *slot = *v;
        }
        self.commands.push(TextCommand { verb, values: out });
    }

    fn apply(&mut self, element: &PathElement) {
        if self.error.is_some() {
            return;
        }
        let count = match element.kind {
            0 | 1 => 1,
            2 => 2,
            3 => 3,
            _ => 0,
        };
        let mut pts = [Point::default(); 3];
        for (i, slot) in pts.iter_mut().enumerate().take(count) {
            // SAFETY: CGPathApply guarantees `count` points for this element type.
            let p = unsafe { *element.points.add(i) };
            if !p.x.is_finite() || !p.y.is_finite() {
                self.error = Some(TextLayoutError::Unsupported);
                return;
            }
            *slot = Point {
                x: p.x + self.offset.x,
                y: p.y + self.offset.y,
            };
        }
        match element.kind {
            0 | 1 => {
                self.current = pts[0];
                let verb = if element.kind == 0 {
                    TextVerb::Move
                } else {
                    TextVerb::Line
                };
                self.append(verb, &[pts[0].x, pts[0].y]);
            }
            2 => {
                let (q, e, c) = (pts[0], pts[1], self.current);
                let v = [
                    c.x + (2.0 / 3.0) * (q.x - c.x),
                    c.y + (2.0 / 3.0) * (q.y - c.y),
                    e.x + (2.0 / 3.0) * (q.x - e.x),
                    e.y + (2.0 / 3.0) * (q.y - e.y),
                    e.x,
                    e.y,
                ];
                self.current = e;
                self.append(TextVerb::Cubic, &v);
            }
            3 => {
                let v = [pts[0].x, pts[0].y, pts[1].x, pts[1].y, pts[2].x, pts[2].y];
                self.current = pts[2];
                self.append(TextVerb::Cubic, &v);
            }
            _ => self.append(TextVerb::Close, &[]),
        }
    }
}

extern "C" fn path_apply(info: *mut c_void, element: *const PathElement) {
    // SAFETY: `info` is the `PathContext` passed to CGPathApply below, alive and
    // exclusively used for the duration of the call; `element` is valid per CG.
    let (context, element) = unsafe { (&mut *info.cast::<PathContext<'_>>(), &*element) };
    // No unwind may cross this inbound callback: latch a fault instead.
    if catch_unwind(AssertUnwindSafe(|| context.apply(element))).is_err() {
        context.error = Some(TextLayoutError::Unsupported);
    }
}

fn layout_native(req: &TextRequest<'_>) -> Result<TextLayout, TextLayoutError> {
    use TextLayoutError::*;
    validate(req)?;
    let default_family: &[u8] = b"sans-serif";
    let primary_family = req.font_families.first().copied().unwrap_or(default_family);
    let cascade = {
        // SAFETY: the array callback table is a process-lifetime static.
        Cf::new(unsafe {
            CFArrayCreateMutable(ptr::null(), 0, ptr::addr_of!(kCFTypeArrayCallBacks).cast())
        })
        .ok_or(OutOfMemory)?
    };
    let primary = descriptor(primary_family, req.weight, req.style).ok_or(OutOfMemory)?;
    for family in req.font_families.iter().skip(1) {
        let d = descriptor(family, req.weight, req.style).ok_or(OutOfMemory)?;
        // SAFETY: live array and descriptor; the array retains the value.
        unsafe { CFArrayAppendValue(cascade.0, d.0) };
    }
    let font_attributes = cf_dict().ok_or(OutOfMemory)?;
    let size = cf_f64(req.size_points).ok_or(OutOfMemory)?;
    // SAFETY: process-lifetime keys; live values; live descriptor copies below.
    let font = unsafe {
        set(&font_attributes, key(kCTFontSizeAttribute), &size);
        if CFArrayGetCount(cascade.0) != 0 {
            set(&font_attributes, key(kCTFontCascadeListAttribute), &cascade);
        }
        let d = Cf::new(CTFontDescriptorCreateCopyWithAttributes(
            primary.0,
            font_attributes.0,
        ))
        .ok_or(OutOfMemory)?;
        Cf::new(CTFontCreateWithFontDescriptor(
            d.0,
            req.size_points,
            ptr::null(),
        ))
        .ok_or(Unsupported)?
    };
    let text = cf_string(req.text).ok_or(InvalidUtf8)?;
    let kern = cf_f64(req.letter_spacing_points).ok_or(OutOfMemory)?;
    let ligature = cf_i32(1).ok_or(OutOfMemory)?;
    let attributes = cf_dict().ok_or(OutOfMemory)?;
    // SAFETY: live objects throughout; attribute keys are process-lifetime statics.
    let line = unsafe {
        set(&attributes, key(kCTFontAttributeName), &font);
        set(&attributes, key(kCTKernAttributeName), &kern);
        set(&attributes, key(kCTLigatureAttributeName), &ligature);
        let attributed =
            Cf::new(CFAttributedStringCreateMutable(ptr::null(), 0)).ok_or(OutOfMemory)?;
        CFAttributedStringReplaceString(
            attributed.0,
            CfRange {
                location: 0,
                length: 0,
            },
            text.0,
        );
        let len = CFStringGetLength(text.0);
        if len > 0 {
            CFAttributedStringSetAttributes(
                attributed.0,
                CfRange {
                    location: 0,
                    length: len,
                },
                attributes.0,
                true,
            );
        }
        Cf::new(CTLineCreateWithAttributedString(attributed.0)).ok_or(OutOfMemory)?
    };
    let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
    // SAFETY: live line; metric pointers are writable locals.
    let width =
        unsafe { CTLineGetTypographicBounds(line.0, &mut ascent, &mut descent, &mut leading) };
    if ![width, ascent, descent, leading]
        .iter()
        .all(|v| v.is_finite())
        || width < 0.0
    {
        return Err(Unsupported);
    }
    // SAFETY: live line; the run array is owned by the line (get rule).
    let runs = unsafe { CTLineGetGlyphRuns(line.0) };
    // SAFETY: `runs` is the live array just returned by the line.
    let run_count = unsafe { CFArrayGetCount(runs) };
    let mut total = 0usize;
    for i in 0..run_count {
        // SAFETY: index is within the array count.
        let n = unsafe { CTRunGetGlyphCount(CFArrayGetValueAtIndex(runs, i)) };
        let n = usize::try_from(n).map_err(|_| Limit)?;
        total = total
            .checked_add(n)
            .filter(|t| *t <= MAX_GLYPHS)
            .ok_or(Limit)?;
    }
    if total == 0 {
        // SAFETY: live font.
        unsafe {
            ascent = CTFontGetAscent(font.0);
            descent = CTFontGetDescent(font.0);
            leading = CTFontGetLeading(font.0);
        }
    }
    let mut glyphs: Vec<TextGlyph> = Vec::new();
    glyphs.try_reserve_exact(total).map_err(|_| OutOfMemory)?;
    let mut commands: Vec<TextCommand> = Vec::new();
    for i in 0..run_count {
        // SAFETY: index within count; run owned by the line.
        let run = unsafe { CFArrayGetValueAtIndex(runs, i) };
        // SAFETY: `run` is a live element of the line's run array.
        let n = unsafe { CTRunGetGlyphCount(run) } as usize;
        if n == 0 {
            continue;
        }
        let mut ids = vec![0u16; n];
        let mut positions = vec![Point::default(); n];
        let mut advances = vec![Size::default(); n];
        let range = CfRange {
            location: 0,
            length: n as isize,
        };
        // SAFETY: buffers hold exactly `n` elements for the full range.
        let run_font = unsafe {
            CTRunGetGlyphs(run, range, ids.as_mut_ptr());
            CTRunGetPositions(run, range, positions.as_mut_ptr());
            CTRunGetAdvances(run, range, advances.as_mut_ptr());
            let f = CFDictionaryGetValue(CTRunGetAttributes(run), key(kCTFontAttributeName));
            if f.is_null() { font.0 } else { f }
        };
        for g in 0..n {
            let (p, a) = (positions[g], advances[g]);
            if !p.x.is_finite() || !p.y.is_finite() || !a.width.is_finite() || !a.height.is_finite()
            {
                return Err(Unsupported);
            }
            let start = commands.len();
            // SAFETY: live font and glyph id; a null path is allowed.
            let path = unsafe { CTFontCreatePathForGlyph(run_font, ids[g], ptr::null()) };
            if let Some(path) = Cf::new(path) {
                let mut context = PathContext {
                    commands: &mut commands,
                    error: None,
                    offset: p,
                    current: p,
                };
                // SAFETY: `context` outlives the synchronous apply call.
                unsafe { CGPathApply(path.0, ptr::addr_of_mut!(context).cast(), path_apply) };
                if let Some(e) = context.error {
                    return Err(e);
                }
            }
            glyphs.push(TextGlyph {
                glyph_id: ids[g],
                x: p.x,
                y: p.y,
                advance: a.width,
                command_start: start,
                command_count: commands.len() - start,
            });
        }
    }
    // Glyph-local conversion exactly as `app_text_layout`.
    for glyph in &glyphs {
        for command in &mut commands[glyph.command_start..glyph.command_start + glyph.command_count]
        {
            let pairs = match command.verb {
                TextVerb::Cubic => 3,
                TextVerb::Close => 0,
                _ => 1,
            };
            for k in 0..pairs {
                command.values[2 * k] -= glyph.x;
                command.values[2 * k + 1] -= glyph.y;
            }
        }
    }
    Ok(TextLayout {
        glyphs,
        commands,
        advance_width: width,
        ascent,
        descent,
        leading,
    })
}

fn layout_guarded(
    req: &TextRequest<'_>,
    before_native: impl FnOnce(),
) -> Result<TextLayout, TextLayoutError> {
    match guard_native_call(|| {
        before_native();
        layout_native(req)
    }) {
        Ok(result) => result,
        Err(fault) => Err(TextLayoutError::Fault(fault)),
    }
}

/// Shapes `request` with CoreText and returns glyph-local outlines.
///
/// Kerning and ligatures are enabled; emoji return `Unsupported`; empty text
/// succeeds with font metrics. Rust panics are returned as
/// [`TextLayoutError::Fault`].
pub fn text_layout(request: &TextRequest<'_>) -> Result<TextLayout, TextLayoutError> {
    layout_guarded(request, || {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_panic_is_a_typed_fault() {
        let request = TextRequest {
            text: b"A",
            font_families: &[],
            size_points: 12.0,
            weight: 400.0,
            style: TextStyle::Normal,
            letter_spacing_points: 0.0,
        };
        let result = layout_guarded(&request, || panic!("injected"));
        assert!(matches!(result, Err(TextLayoutError::Fault(_))));
    }
}
