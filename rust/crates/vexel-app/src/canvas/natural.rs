//! The natural (100%) size of a document, read from its root `<svg>` tag.

/// `width`/`height` of the root element in user units (`px` or unitless), else
/// the `viewBox` size; `None` when neither gives a positive finite size.
pub fn natural_size(source: &[u8]) -> Option<(f64, f64)> {
    let tag = root_tag(source)?;
    let (mut width, mut height, mut view) = (None, None, None);
    for (name, value) in attributes(tag) {
        match name {
            "width" => width = length(value),
            "height" => height = length(value),
            "viewBox" => view = view_size(value),
            _ => {}
        }
    }
    match (width, height, view) {
        (Some(w), Some(h), _) => Some((w, h)),
        (Some(w), None, Some((vw, vh))) => Some((w, w * vh / vw)),
        (None, Some(h), Some((vw, vh))) => Some((h * vw / vh, h)),
        (_, _, view) => view,
    }
}

fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

/// The text of the first start tag named `svg`, after prolog items.
fn root_tag(source: &[u8]) -> Option<&str> {
    let mut at = 0;
    loop {
        at = find(source, at, b"<")?;
        let rest = source.get(at..)?;
        if rest.starts_with(b"<?") {
            at = find(source, at, b"?>")? + 2;
        } else if rest.starts_with(b"<!--") {
            at = find(source, at, b"-->")? + 3;
        } else if rest.starts_with(b"<!") {
            at = find(source, at, b">")? + 1;
        } else if rest.starts_with(b"<svg")
            && rest
                .get(4)
                .is_some_and(|c| c.is_ascii_whitespace() || *c == b'>' || *c == b'/')
        {
            let mut quote = None;
            for (i, c) in rest.iter().enumerate().skip(4) {
                match (quote, *c) {
                    (None, b'>') => return std::str::from_utf8(&rest[4..i]).ok(),
                    (None, b'"' | b'\'') => quote = Some(*c),
                    (Some(q), c) if c == q => quote = None,
                    _ => {}
                }
            }
            return None;
        } else {
            return None;
        }
    }
}

fn attributes(mut tag: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    while let Some(eq) = tag.find('=') {
        let name = tag[..eq].trim();
        let after = tag[eq + 1..].trim_start();
        let Some(quote) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            break;
        };
        let Some(end) = after[1..].find(quote) else {
            break;
        };
        let name = name.rsplit(char::is_whitespace).next().unwrap_or(name);
        out.push((name, &after[1..1 + end]));
        tag = &after[end + 2..];
    }
    out
}

fn length(value: &str) -> Option<f64> {
    let text = value.trim();
    let number = text.strip_suffix("px").unwrap_or(text).trim();
    number
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0e6)
}

fn view_size(value: &str) -> Option<(f64, f64)> {
    let mut numbers = value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(str::parse::<f64>);
    let (_, _, w, h) = (
        numbers.next()?.ok()?,
        numbers.next()?.ok()?,
        numbers.next()?.ok()?,
        numbers.next()?.ok()?,
    );
    (w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0 && w <= 1.0e6 && h <= 1.0e6)
        .then_some((w, h))
}

#[cfg(test)]
mod tests {
    use super::natural_size;

    #[test]
    fn reads_width_height_then_viewbox() {
        let doc = br#"<?xml version="1.0"?><!-- <svg width="9"> --><svg xmlns="x" width="160" height='120px' viewBox="0 0 80 60">"#;
        assert_eq!(natural_size(doc), Some((160.0, 120.0)));
        assert_eq!(
            natural_size(br#"<svg viewBox="0,0,80,60"/>"#),
            Some((80.0, 60.0))
        );
        assert_eq!(
            natural_size(br#"<svg width="100%" height="50%" viewBox="0 0 8 6">"#),
            Some((8.0, 6.0))
        );
        assert_eq!(
            natural_size(br#"<svg width="40" viewBox="0 0 8 6">"#),
            Some((40.0, 30.0))
        );
        assert_eq!(natural_size(b"<svg>"), None);
        assert_eq!(natural_size(b"not xml"), None);
        assert_eq!(natural_size(br#"<svg width="0" height="5">"#), None);
    }
}
