use std::ops::Range;

pub(crate) fn utf8_to_utf16(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    if range.start > range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return None;
    }
    Some(text[..range.start].encode_utf16().count()..text[..range.end].encode_utf16().count())
}
pub(crate) fn utf16_to_utf8(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    if range.start > range.end {
        return None;
    }
    let mut units = 0usize;
    let mut start = None;
    let mut end = None;
    for (byte, ch) in text.char_indices() {
        if units == range.start {
            start = Some(byte);
        }
        if units == range.end {
            end = Some(byte);
        }
        units = units.checked_add(ch.len_utf16())?;
        if range.start > units.saturating_sub(ch.len_utf16()) && range.start < units {
            return None;
        }
        if range.end > units.saturating_sub(ch.len_utf16()) && range.end < units {
            return None;
        }
    }
    if units == range.start {
        start = Some(text.len());
    }
    if units == range.end {
        end = Some(text.len());
    }
    if range.end > units {
        return None;
    }
    Some(start?..end?)
}
