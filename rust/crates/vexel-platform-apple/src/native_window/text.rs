//! Bounded CoreText rasterization into caller-owned RGBA pixels.
//!
//! Native CoreText/CoreGraphics handles stay private to this module. Text is
//! shaped as CoreText lines, which supplies font fallback and shaping for the
//! selected system face while the clipped bitmap remains caller-owned.
use objc2_core_foundation::{
    CFDictionaryKeyCallBacks, CFDictionaryValueCallBacks, CFStringBuiltInEncodings, kCFBooleanTrue,
};
use objc2_core_graphics::{
    CGBitmapInfo, CGImageAlphaInfo, CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_core_text::{
    kCTFontAttributeName, kCTFontTraitsAttribute, kCTForegroundColorFromContextAttributeName,
};
use std::ffi::c_void;
use std::ptr;

const MAX_AXIS: u32 = 4096;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 8192;
const MAX_GLYPHS: usize = 4096;
const SEMIBOLD_WEIGHT: f64 = 0.3;
const CF_NUMBER_CGFLOAT_TYPE: isize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFace {
    System,
    Monospaced,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEmphasis {
    Regular,
    Semibold,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextError {
    InvalidBuffer,
    InvalidBounds,
    InvalidText,
    InvalidSize,
    NativeUnavailable,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeRect {
    origin: Point,
    size: NativeSize,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct AffineTransform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}
#[repr(C)]
struct CfString {
    _private: [u8; 0],
}
#[repr(C)]
struct Dictionary {
    _private: [u8; 0],
}
#[repr(C)]
struct AttributedString {
    _private: [u8; 0],
}
#[repr(C)]
struct Font {
    _private: [u8; 0],
}
#[repr(C)]
struct FontDescriptor {
    _private: [u8; 0],
}
#[repr(C)]
struct Line {
    _private: [u8; 0],
}
#[repr(C)]
struct ColorSpace {
    _private: [u8; 0],
}
#[repr(C)]
struct Context {
    _private: [u8; 0],
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithBytes(
        allocator: *const c_void,
        bytes: *const u8,
        count: isize,
        encoding: u32,
        external: bool,
    ) -> *const CfString;
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: isize,
        key_callbacks: *const CFDictionaryKeyCallBacks,
        value_callbacks: *const CFDictionaryValueCallBacks,
    ) -> *const Dictionary;
    fn CFAttributedStringCreate(
        allocator: *const c_void,
        string: *const CfString,
        attributes: *const Dictionary,
    ) -> *const AttributedString;
    fn CFRelease(value: *const c_void);
    fn CFNumberCreate(
        allocator: *const c_void,
        number_type: isize,
        value: *const c_void,
    ) -> *const c_void;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGColorSpaceCreateWithName(name: *const CfString) -> *mut ColorSpace;
    fn CGColorSpaceRelease(space: *mut ColorSpace);
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits: usize,
        bytes_per_row: usize,
        space: *mut ColorSpace,
        bitmap_info: u32,
    ) -> *mut Context;
    fn CGContextSaveGState(context: *mut Context);
    fn CGContextRestoreGState(context: *mut Context);
    fn CGContextTranslateCTM(context: *mut Context, tx: f64, ty: f64);
    fn CGContextScaleCTM(context: *mut Context, sx: f64, sy: f64);
    fn CGContextClipToRect(context: *mut Context, rect: NativeRect);
    fn CGContextSetRGBFillColor(context: *mut Context, red: f64, green: f64, blue: f64, alpha: f64);
    fn CGContextSetTextMatrix(context: *mut Context, transform: AffineTransform);
    fn CGContextSetTextPosition(context: *mut Context, x: f64, y: f64);
}
#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    static kCTFontWeightTrait: &'static objc2_core_foundation::CFString;
    fn CTFontCreateWithName(name: *const CfString, size: f64, matrix: *const c_void) -> *mut Font;
    fn CTFontDescriptorCreateWithAttributes(attributes: *const Dictionary) -> *mut FontDescriptor;
    fn CTFontCreateCopyWithAttributes(
        font: *mut Font,
        size: f64,
        matrix: *const c_void,
        descriptor: *mut FontDescriptor,
    ) -> *mut Font;
    fn CTLineCreateWithAttributedString(string: *const AttributedString) -> *mut Line;
    fn CTLineGetTypographicBounds(
        line: *mut Line,
        ascent: *mut f64,
        descent: *mut f64,
        leading: *mut f64,
    ) -> f64;
    fn CTLineDraw(line: *mut Line, context: *mut Context);
}

/// A private owner for a retained Core Foundation / Core Text object.
struct Owned<T>(*mut T);
impl<T> Owned<T> {
    fn new(value: *mut T) -> Option<Self> {
        (!value.is_null()).then_some(Self(value))
    }
    fn as_ptr(&self) -> *mut T {
        self.0
    }
}
impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        // SAFETY: constructors used here return one owned reference, released once.
        unsafe { CFRelease(self.0.cast()) }
    }
}

/// Draw clipped, shaped system or SF Mono text into a top-left-origin RGBA8 buffer.
/// Dimensions, stride, text length, and font size are bounded before native calls.
pub fn draw_text_rgba(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
    backing_scale: u32,
    text: &str,
    clip: (u32, u32, u32, u32),
    color: u32,
    face: TextFace,
    emphasis: TextEmphasis,
    size_pixels: u32,
) -> Result<(), TextError> {
    validate(
        width,
        height,
        stride,
        rgba.len(),
        backing_scale,
        text,
        clip,
        size_pixels,
    )?;
    if text.is_empty() {
        return Ok(());
    }
    let glyph_count = text.encode_utf16().count();
    if glyph_count == 0 || glyph_count > MAX_GLYPHS {
        return Err(TextError::InvalidText);
    }
    let (x, y, clip_width, clip_height) = clip;
    let clip_stride = (clip_width as usize)
        .checked_mul(4)
        .ok_or(TextError::InvalidBounds)?;
    let clip_len = clip_stride
        .checked_mul(clip_height as usize)
        .filter(|len| *len <= MAX_BYTES)
        .ok_or(TextError::InvalidBounds)?;
    let mut clipped = zeroed(clip_len)?;
    copy_to_premultiplied(
        rgba,
        stride,
        (x, y, clip_width, clip_height),
        clip_stride,
        &mut clipped,
    );

    let font_name: &[u8] = match face {
        TextFace::System => b".AppleSystemUIFont",
        TextFace::Monospaced => b"SFMono-Regular",
    };
    // SAFETY: bounded static UTF-8 name and encoding from the pinned CoreFoundation binding.
    let name = Owned::new(unsafe {
        CFStringCreateWithBytes(
            ptr::null(),
            font_name.as_ptr(),
            font_name.len() as isize,
            CFStringBuiltInEncodings::EncodingUTF8.0,
            false,
        ) as *mut CfString
    })
    .ok_or(TextError::NativeUnavailable)?;
    // SAFETY: name stays retained until font creation returns.
    let regular =
        Owned::new(unsafe { CTFontCreateWithName(name.as_ptr(), size_pixels as f64, ptr::null()) })
            .ok_or(TextError::NativeUnavailable)?;
    let emphasized = if emphasis == TextEmphasis::Semibold {
        semibold_font(regular.as_ptr(), size_pixels as f64)
    } else {
        None
    };
    let font = if use_semibold(emphasis, emphasized.is_some()) {
        emphasized.as_ref().map_or(regular.as_ptr(), Owned::as_ptr)
    } else {
        regular.as_ptr()
    };
    // SAFETY: CoreText exports this process-lifetime CFString key.
    // SAFETY: CoreText exports process-lifetime immutable CFString attribute keys.
    let (attributed_key, foreground_key) = unsafe {
        (
            kCTFontAttributeName as *const objc2_core_foundation::CFString as *const c_void,
            kCTForegroundColorFromContextAttributeName as *const objc2_core_foundation::CFString
                as *const c_void,
        )
    };
    let foreground_value = foreground_context_value().ok_or(TextError::NativeUnavailable)?;

    let lines = wrap_lines(
        text,
        clip_width as f64,
        font,
        attributed_key,
        foreground_key,
        foreground_value,
    )?;
    if lines.is_empty() {
        return Ok(());
    }
    let mut shaped_lines = Vec::new();
    shaped_lines
        .try_reserve_exact(lines.len())
        .map_err(|_| TextError::NativeUnavailable)?;
    let (mut max_ascent, mut max_descent, mut max_leading) = (0.0_f64, 0.0_f64, 0.0_f64);
    for text in lines {
        let shaped = create_line(
            &text,
            font,
            attributed_key,
            foreground_key,
            foreground_value,
        )?;
        let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
        // SAFETY: all metric pointers are writable locals and the shaped line is live.
        let _ = unsafe {
            CTLineGetTypographicBounds(shaped.as_ptr(), &mut ascent, &mut descent, &mut leading)
        };
        if !ascent.is_finite() || !descent.is_finite() || !leading.is_finite() {
            return Err(TextError::NativeUnavailable);
        }
        max_ascent = max_ascent.max(ascent);
        max_descent = max_descent.max(descent);
        max_leading = max_leading.max(leading.max(0.0));
        shaped_lines.push(shaped);
    }
    let line_height = (max_ascent + max_descent + max_leading).max(1.0);
    if !line_height.is_finite() {
        return Err(TextError::NativeUnavailable);
    }
    let visible_lines = (clip_height as f64 / line_height).floor() as usize;
    shaped_lines.truncate(visible_lines);
    let block_height = shaped_lines.len() as f64 * line_height;
    let first_baseline = ((clip_height as f64 - block_height) / 2.0).max(0.0) + max_ascent;

    // SAFETY: named sRGB space matches the frame's documented sRGB RGBA8 pixels.
    let space = unsafe {
        CGColorSpaceCreateWithName(
            (kCGColorSpaceSRGB as *const objc2_core_foundation::CFString).cast::<CfString>(),
        )
    };
    if space.is_null() {
        return Err(TextError::NativeUnavailable);
    }
    // CoreGraphics expects premultiplied-last byte RGBA. Combine named SDK values.
    let bitmap = CGBitmapInfo::from_bits_retain(
        CGImageAlphaInfo::PremultipliedLast.0 | CGImageByteOrderInfo::Order32Big.0,
    );
    // SAFETY: clipped has the exact checked row layout, is exclusively borrowed, and context stays local.
    let context = unsafe {
        CGBitmapContextCreate(
            clipped.as_mut_ptr().cast(),
            clip_width as usize,
            clip_height as usize,
            8,
            clip_stride,
            space,
            bitmap.bits(),
        )
    };
    unsafe { CGColorSpaceRelease(space) };
    let context = Owned::new(context).ok_or(TextError::NativeUnavailable)?;
    let (red, green, blue, alpha) = color_components(color);
    // SAFETY: all handles are live and the explicit clip is local to the copied pixel rectangle.
    unsafe {
        CGContextSaveGState(context.as_ptr());
        CGContextTranslateCTM(context.as_ptr(), 0.0, clip_height as f64);
        CGContextScaleCTM(context.as_ptr(), 1.0, -1.0);
        // The bitmap uses a top-left pixel origin, so the context is flipped
        // for geometry. CoreText glyphs use their own text matrix and need an
        // inverse y flip to stay upright in that coordinate system.
        CGContextSetTextMatrix(context.as_ptr(), upright_text_matrix());
        CGContextClipToRect(
            context.as_ptr(),
            NativeRect {
                origin: Point { x: 0.0, y: 0.0 },
                size: NativeSize {
                    width: clip_width as f64,
                    height: clip_height as f64,
                },
            },
        );
        CGContextSetRGBFillColor(context.as_ptr(), red, green, blue, alpha);
        for (index, line) in shaped_lines.iter().enumerate() {
            let baseline = first_baseline + index as f64 * line_height;
            CGContextSetTextPosition(context.as_ptr(), 0.0, baseline);
            CTLineDraw(line.as_ptr(), context.as_ptr());
        }
        CGContextRestoreGState(context.as_ptr());
    }
    copy_from_premultiplied(
        &mut clipped,
        clip_stride,
        rgba,
        stride,
        (x, y, clip_width, clip_height),
    );
    Ok(())
}

fn validate(
    width: u32,
    height: u32,
    stride: usize,
    len: usize,
    backing_scale: u32,
    text: &str,
    clip: (u32, u32, u32, u32),
    size_pixels: u32,
) -> Result<(), TextError> {
    if width == 0
        || height == 0
        || width > MAX_AXIS
        || height > MAX_AXIS
        || !(1..=3).contains(&backing_scale)
    {
        return Err(TextError::InvalidBuffer);
    }
    let row = (width as usize)
        .checked_mul(4)
        .ok_or(TextError::InvalidBuffer)?;
    let bytes = stride
        .checked_mul(height as usize)
        .ok_or(TextError::InvalidBuffer)?;
    if stride < row || bytes > MAX_BYTES || len != bytes {
        return Err(TextError::InvalidBuffer);
    }
    let (x, y, w, h) = clip;
    if w == 0
        || h == 0
        || x.checked_add(w).is_none_or(|v| v > width)
        || y.checked_add(h).is_none_or(|v| v > height)
    {
        return Err(TextError::InvalidBounds);
    }
    if text.len() > MAX_TEXT_BYTES
        || text.contains('\0')
        || text.encode_utf16().count() > MAX_GLYPHS
    {
        return Err(TextError::InvalidText);
    }
    if !(1..=256).contains(&size_pixels) {
        return Err(TextError::InvalidSize);
    }
    Ok(())
}

fn zeroed(len: usize) -> Result<Vec<u8>, TextError> {
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| TextError::NativeUnavailable)?;
    out.resize(len, 0);
    Ok(out)
}

fn copy_to_premultiplied(
    source: &[u8],
    stride: usize,
    rect: (u32, u32, u32, u32),
    row: usize,
    out: &mut [u8],
) {
    let (x, y, _width, height) = rect;
    for line in 0..height as usize {
        let src = (y as usize + line) * stride + x as usize * 4;
        let dst = line * row;
        out[dst..dst + row].copy_from_slice(&source[src..src + row]);
        for pixel in out[dst..dst + row].chunks_exact_mut(4) {
            let alpha = u16::from(pixel[3]);
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
    }
}

fn copy_from_premultiplied(
    source: &mut [u8],
    row: usize,
    target: &mut [u8],
    stride: usize,
    rect: (u32, u32, u32, u32),
) {
    let (x, y, width, height) = rect;
    let copy_len = width as usize * 4;
    for line in 0..height as usize {
        let src = line * row;
        for pixel in source[src..src + copy_len].chunks_exact_mut(4) {
            let alpha = u16::from(pixel[3]);
            if alpha == 0 {
                pixel[..3].fill(0);
            } else {
                for channel in &mut pixel[..3] {
                    *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
        let dst = (y as usize + line) * stride + x as usize * 4;
        target[dst..dst + copy_len].copy_from_slice(&source[src..src + copy_len]);
    }
}

fn create_attributed(
    text: &str,
    font: *mut Font,
    font_key: *const c_void,
    foreground_key: *const c_void,
    foreground_value: *const c_void,
) -> Result<Owned<AttributedString>, TextError> {
    let native = Owned::new(unsafe {
        CFStringCreateWithBytes(
            ptr::null(),
            text.as_ptr(),
            text.len() as isize,
            CFStringBuiltInEncodings::EncodingUTF8.0,
            false,
        ) as *mut CfString
    })
    .ok_or(TextError::NativeUnavailable)?;
    let pairs = text_attribute_pairs(
        font_key,
        font.cast::<c_void>() as *const c_void,
        foreground_key,
        foreground_value,
    );
    let keys = [pairs[0].0, pairs[1].0];
    let values = [pairs[0].1, pairs[1].1];
    // SAFETY: system keys, live font, and process-lifetime CFBoolean are valid CF objects.
    let attributes = Owned::new(unsafe {
        CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            2,
            ptr::addr_of!(objc2_core_foundation::kCFTypeDictionaryKeyCallBacks),
            ptr::addr_of!(objc2_core_foundation::kCFTypeDictionaryValueCallBacks),
        ) as *mut Dictionary
    })
    .ok_or(TextError::NativeUnavailable)?;
    // SAFETY: CoreFoundation copies the string and attributes into the returned value.
    Owned::new(unsafe {
        CFAttributedStringCreate(ptr::null(), native.as_ptr(), attributes.as_ptr())
            as *mut AttributedString
    })
    .ok_or(TextError::NativeUnavailable)
}

fn create_line(
    text: &str,
    font: *mut Font,
    font_key: *const c_void,
    foreground_key: *const c_void,
    foreground_value: *const c_void,
) -> Result<Owned<Line>, TextError> {
    let attributed = create_attributed(text, font, font_key, foreground_key, foreground_value)?;
    // SAFETY: the attributed string remains live for the copying line constructor.
    Owned::new(unsafe { CTLineCreateWithAttributedString(attributed.as_ptr()) })
        .ok_or(TextError::NativeUnavailable)
}

fn wrap_lines(
    text: &str,
    max_width: f64,
    font: *mut Font,
    font_key: *const c_void,
    foreground_key: *const c_void,
    foreground_value: *const c_void,
) -> Result<Vec<String>, TextError> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            let line = create_line(&candidate, font, font_key, foreground_key, foreground_value)?;
            let width = unsafe {
                CTLineGetTypographicBounds(
                    line.as_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if !width.is_finite() || width < 0.0 {
                return Err(TextError::NativeUnavailable);
            }
            if !current.is_empty() && width > max_width {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
            } else {
                current = candidate;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    Ok(lines)
}

fn semibold_font(regular: *mut Font, size: f64) -> Option<Owned<Font>> {
    // SAFETY: CoreText exports immutable process-lifetime descriptor keys.
    let (weight_key, traits_key) = unsafe {
        (
            kCTFontWeightTrait as *const objc2_core_foundation::CFString as *const c_void,
            kCTFontTraitsAttribute as *const objc2_core_foundation::CFString as *const c_void,
        )
    };
    let weight = semibold_descriptor_weight();
    // SAFETY: CGFloat is f64 on the supported Apple Silicon ABI; the pointer is a live local.
    let number = Owned::new(unsafe {
        CFNumberCreate(
            ptr::null(),
            CF_NUMBER_CGFLOAT_TYPE,
            (&weight as *const f64).cast(),
        ) as *mut c_void
    })?;
    let trait_value = number.as_ptr().cast::<c_void>() as *const c_void;
    // SAFETY: CoreText trait key and retained numeric value are valid CF objects.
    let traits = Owned::new(unsafe {
        CFDictionaryCreate(
            ptr::null(),
            &weight_key,
            &trait_value,
            1,
            ptr::addr_of!(objc2_core_foundation::kCFTypeDictionaryKeyCallBacks),
            ptr::addr_of!(objc2_core_foundation::kCFTypeDictionaryValueCallBacks),
        ) as *mut Dictionary
    })?;
    let traits_value = traits.as_ptr().cast::<c_void>() as *const c_void;
    // SAFETY: descriptor attribute key and retained traits dictionary are valid CF objects.
    let attributes = Owned::new(unsafe {
        CFDictionaryCreate(
            ptr::null(),
            &traits_key,
            &traits_value,
            1,
            ptr::addr_of!(objc2_core_foundation::kCFTypeDictionaryKeyCallBacks),
            ptr::addr_of!(objc2_core_foundation::kCFTypeDictionaryValueCallBacks),
        ) as *mut Dictionary
    })?;
    // SAFETY: the dictionary owns copies of its CF entries until descriptor creation returns.
    let descriptor =
        Owned::new(unsafe { CTFontDescriptorCreateWithAttributes(attributes.as_ptr()) })?;
    // SAFETY: regular and descriptor remain live through the copy operation.
    Owned::new(unsafe {
        CTFontCreateCopyWithAttributes(regular, size, ptr::null(), descriptor.as_ptr())
    })
}

fn color_components(color: u32) -> (f64, f64, f64, f64) {
    (
        ((color >> 24) & 255) as f64 / 255.0,
        ((color >> 16) & 255) as f64 / 255.0,
        ((color >> 8) & 255) as f64 / 255.0,
        (color & 255) as f64 / 255.0,
    )
}

fn foreground_context_value() -> Option<*const c_void> {
    // SAFETY: CoreFoundation's kCFBooleanTrue is an immutable process-lifetime singleton.
    unsafe { kCFBooleanTrue.map(|value| value as *const _ as *const c_void) }
}

fn text_attribute_pairs(
    font_key: *const c_void,
    font_value: *const c_void,
    foreground_key: *const c_void,
    foreground_value: *const c_void,
) -> [(*const c_void, *const c_void); 2] {
    [(font_key, font_value), (foreground_key, foreground_value)]
}

fn upright_text_matrix() -> AffineTransform {
    AffineTransform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: -1.0,
        tx: 0.0,
        ty: 0.0,
    }
}

fn semibold_descriptor_weight() -> f64 {
    SEMIBOLD_WEIGHT
}
fn use_semibold(emphasis: TextEmphasis, descriptor_available: bool) -> bool {
    emphasis == TextEmphasis::Semibold && descriptor_available
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
