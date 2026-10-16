//! NSTextInputClient value conversion and owned pending composition projection.
use super::{
    ime::{utf8_to_utf16, utf16_to_utf8},
    route::RouteState,
    types::*,
};
use objc2::{AllocAnyThread, rc::Retained, runtime::AnyObject};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSMutableAttributedString, NSNumber,
    NSPoint, NSRange, NSRect, NSSize, NSString,
};
use std::ops::Range;
use std::ptr::NonNull;

// These baseline AppKit constants use the same &'static NSString declaration as
// the pinned generated NSAttributedString binding. That optional AppKit module
// is not selected; the Foundation string/attributed-string APIs are selected.
unsafe extern "C" {
    static NSUnderlineStyleAttributeName: &'static NSAttributedStringKey;
    static NSMarkedClauseSegmentAttributeName: &'static NSAttributedStringKey;
}
pub(crate) const NOT_FOUND: usize = isize::MAX as usize;
pub(crate) fn missing() -> NSRange {
    NSRange::new(NOT_FOUND, 0)
}
pub(crate) fn ns_range(range: Range<usize>) -> NSRange {
    NSRange::new(range.start, range.end - range.start)
}
pub(crate) fn requested(text: &str, range: NSRange) -> Option<Range<usize>> {
    if range.location == NOT_FOUND {
        return None;
    }
    utf16_to_utf8(
        text,
        range.location..range.location.checked_add(range.length)?,
    )
}
fn key(kind: TextAttributeKind) -> &'static NSString {
    // SAFETY: baseline immutable AppKit global NSString constants.
    unsafe {
        match kind {
            TextAttributeKind::Underline => NSUnderlineStyleAttributeName,
            _ => NSMarkedClauseSegmentAttributeName,
        }
    }
}
pub(crate) fn attributes() -> Retained<NSArray<NSAttributedStringKey>> {
    NSArray::from_slice(&[
        key(TextAttributeKind::Underline),
        key(TextAttributeKind::ClauseSegment),
    ])
}
fn incoming(route: &RouteState, object: &AnyObject) -> Option<(String, Vec<TextAttributeSpan>)> {
    let attributed = object.downcast_ref::<NSAttributedString>();
    let retained = attributed.map(|a| a.string());
    let string = object.downcast_ref::<NSString>().or(retained.as_deref())?;
    let count = string.length();
    if count > 64 * 1024 {
        route.fault(QueueFaultReason::InputOverflow);
        return None;
    }
    let mut units = Vec::new();
    if units.try_reserve_exact(count).is_err() {
        route.fail(QueueFaultReason::AllocationFailure);
        return None;
    }
    units.resize(count, 0u16);
    if count > 0 {
        // SAFETY: allocated buffer has count u16 slots, and the immutable NSString
        // range is exactly its validated UTF16 length. No pointer escapes.
        unsafe {
            string.getCharacters_range(
                NonNull::new(units.as_mut_ptr()).unwrap(),
                NSRange::new(0, count),
            );
        }
    }
    let mut text = String::new();
    for ch in char::decode_utf16(units) {
        let ch = ch.ok()?;
        if text.len() + ch.len_utf8() > 64 * 1024 {
            route.fault(QueueFaultReason::InputOverflow);
            return None;
        }
        if text.try_reserve(ch.len_utf8()).is_err() {
            route.fail(QueueFaultReason::AllocationFailure);
            return None;
        }
        text.push(ch);
    }
    let mut spans = Vec::new();
    if let Some(attributed) = attributed {
        let mut index = 0;
        while index < attributed.length() {
            let mut effective = missing();
            // SAFETY: index is strictly within the immutable string; stack output is valid.
            let dict =
                unsafe { attributed.attributesAtIndex_effectiveRange(index, &mut effective) };
            let end = effective.location.checked_add(effective.length)?;
            if effective.location > index
                || end <= index
                || end > attributed.length()
                || dict.count() > 2
            {
                return None;
            }
            for k in dict.allKeys().iter() {
                let kind = if k.isEqualToString(key(TextAttributeKind::Underline)) {
                    TextAttributeKind::Underline
                } else if k.isEqualToString(key(TextAttributeKind::ClauseSegment)) {
                    TextAttributeKind::ClauseSegment
                } else {
                    return None;
                };
                let number = dict.objectForKey(&k)?.downcast::<NSNumber>().ok()?;
                let value = number.longLongValue();
                if !(0..=65535).contains(&value) || spans.len() == 256 {
                    return None;
                }
                if spans.try_reserve(1).is_err() {
                    route.fail(QueueFaultReason::AllocationFailure);
                    return None;
                }
                spans.push(TextAttributeSpan {
                    range_utf8: requested(&text, effective)?,
                    kind,
                    value,
                });
            }
            index = end;
        }
    }
    if text.len() + spans.len() * std::mem::size_of::<TextAttributeSpan>() > 64 * 1024 {
        return None;
    }
    Some((text, spans))
}
pub(crate) fn replacement(
    route: &RouteState,
    snapshot: &TextInputState,
    range: NSRange,
) -> Option<Range<usize>> {
    if range.location == NOT_FOUND {
        if range.length != 0 {
            route.fault(QueueFaultReason::InvalidTextRange);
            return None;
        }
        Some(
            snapshot
                .marked_utf8
                .clone()
                .unwrap_or_else(|| snapshot.selection_utf8.clone()),
        )
    } else {
        let result = requested(&snapshot.text, range);
        if result.is_none() {
            route.fault(QueueFaultReason::InvalidTextRange);
        }
        result
    }
}
pub(crate) fn insert(route: &RouteState, object: &AnyObject, range: NSRange) {
    let Some(mut state) = route.snapshot() else {
        return;
    };
    let Some((text, _)) = incoming(route, object) else {
        route.fault(QueueFaultReason::InvalidTextRange);
        return;
    };
    let Some(replace) = replacement(route, &state, range) else {
        return;
    };
    if state.text.len() - replace.len() + text.len() > 64 * 1024 {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }
    state.text.replace_range(replace.clone(), &text);
    let end = replace.start + text.len();
    state.selection_utf8 = end..end;
    state.marked_utf8 = None;
    state.marked_attributes.clear();
    if !route.publish(state, None) {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }
    if range.location == NOT_FOUND {
        route.emit(WindowEvent::TextCommitted(text), false);
    } else {
        route.emit(
            WindowEvent::TextCommittedWithReplacement {
                text,
                replacement_utf8: replace,
            },
            false,
        );
    }
}
pub(crate) fn marked(
    route: &RouteState,
    object: &AnyObject,
    selection: NSRange,
    replacement_range: NSRange,
) {
    let Some(mut state) = route.snapshot() else {
        return;
    };
    let Some((text, mut attributes)) = incoming(route, object) else {
        route.fault(QueueFaultReason::InvalidTextRange);
        return;
    };
    let Some(selected) = requested(&text, selection) else {
        route.fault(QueueFaultReason::InvalidTextRange);
        return;
    };
    let Some(replace) = replacement(route, &state, replacement_range) else {
        return;
    };
    if state.text.len() - replace.len()
        + text.len()
        + attributes.len() * std::mem::size_of::<TextAttributeSpan>()
        > 64 * 1024
    {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }
    let composition = Composition {
        text: text.clone(),
        selection_utf8: selected.clone(),
        replacement_utf8: Some(replace.clone()),
        attributes: attributes.clone(),
    };
    state.text.replace_range(replace.clone(), &text);
    state.selection_utf8 = replace.start + selected.start..replace.start + selected.end;
    state.marked_utf8 = (!text.is_empty()).then_some(replace.start..replace.start + text.len());
    for a in &mut attributes {
        a.range_utf8.start += replace.start;
        a.range_utf8.end += replace.start;
    }
    state.marked_attributes = attributes;
    if !route.publish(state, Some(composition.clone())) {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }
    route.emit(WindowEvent::Composition(composition), false);
}
pub(crate) fn unmark(route: &RouteState) {
    let Some(mut state) = route.snapshot() else {
        return;
    };
    state.marked_utf8.take();
    state.marked_attributes.clear();
    if !route.publish(state, None) {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }
    route.emit(WindowEvent::CompositionEnded, false);
}
pub(crate) fn selected(route: &RouteState, marked: bool) -> NSRange {
    let Some(state) = route.snapshot() else {
        return missing();
    };
    let range = if marked {
        let Some(range) = state.marked_utf8 else {
            return missing();
        };
        range
    } else {
        state.selection_utf8
    };
    utf8_to_utf16(&state.text, range)
        .map(ns_range)
        .unwrap_or_else(missing)
}
pub(crate) fn substring(
    route: &RouteState,
    range: NSRange,
) -> Option<(Retained<NSAttributedString>, NSRange)> {
    let state = route.snapshot()?;
    let Some(bytes) = requested(&state.text, range) else {
        route.fault(QueueFaultReason::InvalidTextRange);
        return None;
    };
    let text = NSString::from_str(&state.text[bytes.clone()]);
    let result =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &text);
    for span in &state.marked_attributes {
        let start = span.range_utf8.start.max(bytes.start);
        let end = span.range_utf8.end.min(bytes.end);
        if start < end {
            let attr_range = utf8_to_utf16(
                &state.text[bytes.clone()],
                start - bytes.start..end - bytes.start,
            )?;
            let number = NSNumber::numberWithLongLong(span.value);
            // SAFETY: validated scalar-aligned range is within the owned substring;
            // NSNumber is a legal value for the two advertised marked attributes.
            unsafe {
                result.addAttribute_value_range(key(span.kind), &number, ns_range(attr_range));
            }
        }
    }
    Some((result.into_super(), range))
}
pub(crate) fn first_rect(route: &RouteState, range: NSRange) -> Option<(NSRect, NSRange)> {
    let state = route.snapshot()?;
    if requested(&state.text, range).is_none() {
        route.fault(QueueFaultReason::InvalidTextRange);
        return None;
    }
    let (window, view) = route.native()?;
    let rect = super::geometry::candidate_to_screen(&window, &view, state.candidate_rect_points);
    if [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        route.fault(QueueFaultReason::InvalidTextRange);
        return None;
    }
    Some((rect, range))
}
pub(crate) fn character_index(route: &RouteState, point: NSPoint) -> usize {
    let Some(state) = route.snapshot() else {
        return NOT_FOUND;
    };
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x.abs() > 1.0e9
        || point.y.abs() > 1.0e9
    {
        route.fault(QueueFaultReason::InvalidTextRange);
        return NOT_FOUND;
    }
    let Some((window, view)) = route.native() else {
        return NOT_FOUND;
    };
    let Some(point) = super::geometry::screen_to_content(&window, &view, point) else {
        route.fault(QueueFaultReason::InvalidTextRange);
        return NOT_FOUND;
    };
    let r = state.candidate_rect_points;
    // A candidate rectangle describes the insertion point, not glyph hit geometry.
    // Return that scalar-aligned insertion index only when the point hits its rect.
    if point.x < r.origin.x
        || point.y < r.origin.y
        || point.x > r.origin.x + r.size.width
        || point.y > r.origin.y + r.size.height
    {
        return NOT_FOUND;
    }
    utf8_to_utf16(
        &state.text,
        state.selection_utf8.start..state.selection_utf8.start,
    )
    .map_or(NOT_FOUND, |r| r.start)
}
pub(crate) fn zero_rect() -> NSRect {
    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0))
}
