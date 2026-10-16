//! Source-span CSS color parsing used by SVG filter primitives.
// Copied as a local std-only port of vexel-svg-color APIs; no external provenance.
#![forbid(unsafe_code)]
//! Source-borrowing CSS/SVG color parser. `currentColor` is context dependent.

use named_colors::NAMED_COLORS;
use vexel_base::Span;

const MAX_VALUE_BYTES: usize = 256;

/// Parse one CSS/SVG color value within an original source span.
/// The source is borrowed and never copied; malformed spans and values return `None`.
pub fn parse_color(source: &[u8], span: Span) -> Option<[u8; 4]> {
    let raw = span.slice(source)?;
    if raw.is_empty() || raw.len() > MAX_VALUE_BYTES {
        return None;
    }
    let start = raw.iter().position(|b| !ws(*b))?;
    let end = raw.iter().rposition(|b| !ws(*b))? + 1;
    let value = &raw[start..end];
    if eq_ci(value, b"transparent") {
        return Some([0, 0, 0, 0]);
    }
    if value[0] == b'#' {
        return parse_hex(value);
    }
    if value.contains(&b'(') {
        return parse_function(value);
    }
    NAMED_COLORS.iter().find_map(|(name, rgb)| {
        eq_ci(value, name.as_bytes()).then_some([
            (rgb >> 16) as u8,
            (rgb >> 8) as u8,
            *rgb as u8,
            255,
        ])
    })
}

fn ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0c)
}
fn eq_ci(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_ascii_lowercase() == *y)
}
fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
fn parse_hex(v: &[u8]) -> Option<[u8; 4]> {
    if !matches!(v.len(), 4 | 5 | 7 | 9) {
        return None;
    }
    let short = v.len() <= 5;
    let count = if matches!(v.len(), 5 | 9) { 4 } else { 3 };
    let mut out = [0; 4];
    for (i, c) in out.iter_mut().take(count).enumerate() {
        let at = 1 + i * (if short { 1 } else { 2 });
        let hi = hex(*v.get(at)?)?;
        *c = if short {
            hi * 17
        } else {
            hi * 16 + hex(*v.get(at + 1)?)?
        };
    }
    if count == 3 {
        out[3] = 255;
    }
    Some(out)
}

// Units are matched from the source without allocating by keeping offsets separately.
#[derive(Clone, Copy, Default)]
struct Part {
    value: f64,
    percent: bool,
    suffix: (usize, usize),
}
struct Reader<'a> {
    b: &'a [u8],
    i: usize,
    end: usize,
}
impl<'a> Reader<'a> {
    fn skip(&mut self) {
        while self.i < self.end && ws(self.b[self.i]) {
            self.i += 1
        }
    }
    fn number(&mut self) -> Option<Part> {
        self.skip();
        let start = self.i;
        let negative = if self.peek() == Some(b'-') {
            self.i += 1;
            true
        } else {
            if self.peek() == Some(b'+') {
                self.i += 1;
            }
            false
        };
        let mut value = 0.0;
        let mut digits = 0;
        while let Some(b @ b'0'..=b'9') = self.peek() {
            value = value * 10.0 + f64::from(b - b'0');
            self.i += 1;
            digits += 1;
            if !value.is_finite() || value > 1e100 {
                return None;
            }
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            let mut place = 0.1;
            while let Some(b @ b'0'..=b'9') = self.peek() {
                value += f64::from(b - b'0') * place;
                place *= 0.1;
                self.i += 1;
                digits += 1
            }
        }
        if digits == 0 {
            self.i = start;
            return None;
        }
        let mut exp = 0i32;
        let mut eneg = false;
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                eneg = self.peek() == Some(b'-');
                self.i += 1;
            }
            let mut n = 0;
            while let Some(b @ b'0'..=b'9') = self.peek() {
                if exp < 1000 {
                    exp = exp * 10 + i32::from(b - b'0');
                }
                self.i += 1;
                n += 1;
            }
            if n == 0 {
                return None;
            }
        }
        if eneg {
            exp = -exp
        }
        if !(-324..=308).contains(&exp) {
            return None;
        }
        value *= 10f64.powi(exp);
        if negative {
            value = -value
        }
        if !value.is_finite() {
            return None;
        }
        let s = self.i;
        while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'%')) {
            self.i += 1
        }
        Some(Part {
            value,
            percent: self.i - s == 1 && self.b[s] == b'%',
            suffix: (s, self.i),
        })
    }
    fn peek(&self) -> Option<u8> {
        (self.i < self.end).then(|| self.b[self.i])
    }
    fn spaces(&mut self) -> bool {
        let i = self.i;
        self.skip();
        self.i != i
    }
}
fn part_unit<'a>(p: Part, b: &'a [u8]) -> &'a [u8] {
    &b[p.suffix.0..p.suffix.1]
}
fn parse_function(v: &[u8]) -> Option<[u8; 4]> {
    let open = v.iter().position(|b| *b == b'(')?;
    if open == 0 || v.last() != Some(&b')') {
        return None;
    }
    let name = &v[..open];
    let rgb = eq_ci(name, b"rgb") || eq_ci(name, b"rgba");
    let hsl = eq_ci(name, b"hsl") || eq_ci(name, b"hsla");
    if !rgb && !hsl {
        return None;
    }
    let mut r = Reader {
        b: v,
        i: open + 1,
        end: v.len() - 1,
    };
    let mut a = [Part::default(); 4];
    let mut comma = false;
    for i in 0..3 {
        a[i] = r.number()?;
        if i == 0 {
            let spaced = r.spaces();
            if r.peek() == Some(b',') {
                comma = true;
                r.i += 1;
                r.skip()
            } else if !spaced {
                return None;
            }
        } else if i == 1 {
            let spaced = r.spaces();
            if comma {
                if r.peek() != Some(b',') {
                    return None;
                }
                r.i += 1;
                r.skip()
            } else if !spaced {
                return None;
            }
        }
    }
    let count = 3;
    let spaced = r.spaces();
    let alpha = if comma && r.peek() == Some(b',') {
        r.i += 1;
        r.skip();
        true
    } else if !comma && r.peek() == Some(b'/') {
        r.i += 1;
        true
    } else if !comma && spaced && r.peek().is_some() && r.peek() != Some(b')') {
        if r.peek() == Some(b'/') {
            r.i += 1
        }
        true
    } else {
        false
    };
    let count = if alpha {
        a[3] = r.number()?;
        4
    } else {
        count
    };
    if hsl {
        let u = part_unit(a[0], v);
        if !u.is_empty() {
            if eq_ci(u, b"deg") {
            } else if eq_ci(u, b"rad") {
                a[0].value *= 57.29577951308232
            } else if eq_ci(u, b"grad") {
                a[0].value *= 0.9
            } else if eq_ci(u, b"turn") {
                a[0].value *= 360.0
            } else {
                return None;
            }
        }
        if !a[1].percent || !a[2].percent {
            return None;
        }
    } else if a[..3]
        .iter()
        .any(|x| x.suffix.0 != x.suffix.1 && !x.percent)
    {
        return None;
    }
    if count == 4 && a[3].suffix.0 != a[3].suffix.1 && !a[3].percent {
        return None;
    }
    r.skip();
    if r.i != r.end {
        return None;
    }
    let alpha = if count == 4 {
        if a[3].percent {
            a[3].value / 100.0
        } else {
            a[3].value
        }
    } else {
        1.0
    };
    let mut out = [0; 4];
    if hsl {
        hsl_rgb(a[0].value, a[1].value / 100.0, a[2].value / 100.0, &mut out)
    } else {
        for i in 0..3 {
            out[i] = byte(if a[i].percent {
                a[i].value * 2.55
            } else {
                a[i].value
            })
        }
    }
    out[3] = byte(clamp(alpha, 0.0, 1.0) * 255.0);
    Some(out)
}
fn clamp(v: f64, l: f64, h: f64) -> f64 {
    v.max(l).min(h)
}
fn byte(v: f64) -> u8 {
    (clamp(v, 0.0, 255.0) + 0.5).floor() as u8
}
fn hsl_rgb(mut h: f64, s: f64, l: f64, o: &mut [u8; 4]) {
    h = h.rem_euclid(360.0);
    let s = clamp(s, 0.0, 1.0);
    let l = clamp(l, 0.0, 1.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c * 0.5;
    o[0] = byte((r + m) * 255.0);
    o[1] = byte((g + m) * 255.0);
    o[2] = byte((b + m) * 255.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn c(s: &[u8]) -> Option<[u8; 4]> {
        parse_color(
            s,
            Span {
                start: 0,
                end: s.len(),
            },
        )
    }
    #[test]
    fn named_table_and_alias_fingerprint() {
        assert_eq!(NAMED_COLORS.len(), 148);
        let mut h = 0xcbf29ce484222325u64;
        for (n, v) in NAMED_COLORS {
            for b in n.bytes().chain(v.to_be_bytes()) {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x100000001b3)
            }
        }
        assert_eq!(h, 0x8141e50e0c774f03);
    }
    #[test]
    fn original_css_color_assertions() {
        assert_eq!(c(b" ReBeccAPurple "), Some([102, 51, 153, 255]));
        assert_eq!(c(b"rgb(100% 0% 0%)"), Some([255, 0, 0, 255]));
        assert_eq!(c(b"rgba(0, 0, 255, 0.5)"), Some([0, 0, 255, 128]));
        assert_eq!(c(b"hsl(120,100%,50%)"), Some([0, 255, 0, 255]));
        assert_eq!(c(b"hsla(240 100% 50% / 25%)"), Some([0, 0, 255, 64]));
        assert_eq!(c(b"#0f08"), Some([0, 255, 0, 136]));
        assert_eq!(c(b"#33669980"), Some([51, 102, 153, 128]));
        assert_eq!(c(b"transparent"), Some([0, 0, 0, 0]));
        assert_eq!(c(b"currentColor"), None);
    }
    #[test]
    fn malformed_bounds_and_grammar() {
        assert_eq!(
            parse_color(
                b"x#fff",
                Span {
                    start: usize::MAX,
                    end: usize::MAX
                }
            ),
            None
        );
        for s in [
            &b"#12"[..],
            b"rgb(1 2 3 /)",
            b"rgb(1e 2 3)",
            b"hsl(0foo 50% 50%)",
            b"red trailing",
        ] {
            assert_eq!(c(s), None, "{s:?}")
        }
        let src = b"x red y";
        assert_eq!(
            parse_color(src, Span { start: 2, end: 5 }),
            Some([255, 0, 0, 255])
        );
        let mut long = vec![b'x'; 100_000];
        long[50_000..50_003].copy_from_slice(b"red");
        assert_eq!(
            parse_color(
                &long,
                Span {
                    start: 50_000,
                    end: 50_003
                }
            ),
            Some([255, 0, 0, 255])
        );
        assert_eq!(parse_color(&long, Span { start: 0, end: 257 }), None);
    }
}

mod named_colors {
    // Exact CSS Color 4 table ported from src/svg/css/color.c; aliases retained.
    pub(super) const NAMED_COLORS: &[(&str, u32)] = &[
        ("aliceblue", 0xf0f8ff),
        ("antiquewhite", 0xfaebd7),
        ("aqua", 0x00ffff),
        ("aquamarine", 0x7fffd4),
        ("azure", 0xf0ffff),
        ("beige", 0xf5f5dc),
        ("bisque", 0xffe4c4),
        ("black", 0x000000),
        ("blanchedalmond", 0xffebcd),
        ("blue", 0x0000ff),
        ("blueviolet", 0x8a2be2),
        ("brown", 0xa52a2a),
        ("burlywood", 0xdeb887),
        ("cadetblue", 0x5f9ea0),
        ("chartreuse", 0x7fff00),
        ("chocolate", 0xd2691e),
        ("coral", 0xff7f50),
        ("cornflowerblue", 0x6495ed),
        ("cornsilk", 0xfff8dc),
        ("crimson", 0xdc143c),
        ("cyan", 0x00ffff),
        ("darkblue", 0x00008b),
        ("darkcyan", 0x008b8b),
        ("darkgoldenrod", 0xb8860b),
        ("darkgray", 0xa9a9a9),
        ("darkgreen", 0x006400),
        ("darkgrey", 0xa9a9a9),
        ("darkkhaki", 0xbdb76b),
        ("darkmagenta", 0x8b008b),
        ("darkolivegreen", 0x556b2f),
        ("darkorange", 0xff8c00),
        ("darkorchid", 0x9932cc),
        ("darkred", 0x8b0000),
        ("darksalmon", 0xe9967a),
        ("darkseagreen", 0x8fbc8f),
        ("darkslateblue", 0x483d8b),
        ("darkslategray", 0x2f4f4f),
        ("darkslategrey", 0x2f4f4f),
        ("darkturquoise", 0x00ced1),
        ("darkviolet", 0x9400d3),
        ("deeppink", 0xff1493),
        ("deepskyblue", 0x00bfff),
        ("dimgray", 0x696969),
        ("dimgrey", 0x696969),
        ("dodgerblue", 0x1e90ff),
        ("firebrick", 0xb22222),
        ("floralwhite", 0xfffaf0),
        ("forestgreen", 0x228b22),
        ("fuchsia", 0xff00ff),
        ("gainsboro", 0xdcdcdc),
        ("ghostwhite", 0xf8f8ff),
        ("gold", 0xffd700),
        ("goldenrod", 0xdaa520),
        ("gray", 0x808080),
        ("green", 0x008000),
        ("greenyellow", 0xadff2f),
        ("grey", 0x808080),
        ("honeydew", 0xf0fff0),
        ("hotpink", 0xff69b4),
        ("indianred", 0xcd5c5c),
        ("indigo", 0x4b0082),
        ("ivory", 0xfffff0),
        ("khaki", 0xf0e68c),
        ("lavender", 0xe6e6fa),
        ("lavenderblush", 0xfff0f5),
        ("lawngreen", 0x7cfc00),
        ("lemonchiffon", 0xfffacd),
        ("lightblue", 0xadd8e6),
        ("lightcoral", 0xf08080),
        ("lightcyan", 0xe0ffff),
        ("lightgoldenrodyellow", 0xfafad2),
        ("lightgray", 0xd3d3d3),
        ("lightgreen", 0x90ee90),
        ("lightgrey", 0xd3d3d3),
        ("lightpink", 0xffb6c1),
        ("lightsalmon", 0xffa07a),
        ("lightseagreen", 0x20b2aa),
        ("lightskyblue", 0x87cefa),
        ("lightslategray", 0x778899),
        ("lightslategrey", 0x778899),
        ("lightsteelblue", 0xb0c4de),
        ("lightyellow", 0xffffe0),
        ("lime", 0x00ff00),
        ("limegreen", 0x32cd32),
        ("linen", 0xfaf0e6),
        ("magenta", 0xff00ff),
        ("maroon", 0x800000),
        ("mediumaquamarine", 0x66cdaa),
        ("mediumblue", 0x0000cd),
        ("mediumorchid", 0xba55d3),
        ("mediumpurple", 0x9370db),
        ("mediumseagreen", 0x3cb371),
        ("mediumslateblue", 0x7b68ee),
        ("mediumspringgreen", 0x00fa9a),
        ("mediumturquoise", 0x48d1cc),
        ("mediumvioletred", 0xc71585),
        ("midnightblue", 0x191970),
        ("mintcream", 0xf5fffa),
        ("mistyrose", 0xffe4e1),
        ("moccasin", 0xffe4b5),
        ("navajowhite", 0xffdead),
        ("navy", 0x000080),
        ("oldlace", 0xfdf5e6),
        ("olive", 0x808000),
        ("olivedrab", 0x6b8e23),
        ("orange", 0xffa500),
        ("orangered", 0xff4500),
        ("orchid", 0xda70d6),
        ("palegoldenrod", 0xeee8aa),
        ("palegreen", 0x98fb98),
        ("paleturquoise", 0xafeeee),
        ("palevioletred", 0xdb7093),
        ("papayawhip", 0xffefd5),
        ("peachpuff", 0xffdab9),
        ("peru", 0xcd853f),
        ("pink", 0xffc0cb),
        ("plum", 0xdda0dd),
        ("powderblue", 0xb0e0e6),
        ("purple", 0x800080),
        ("rebeccapurple", 0x663399),
        ("red", 0xff0000),
        ("rosybrown", 0xbc8f8f),
        ("royalblue", 0x4169e1),
        ("saddlebrown", 0x8b4513),
        ("salmon", 0xfa8072),
        ("sandybrown", 0xf4a460),
        ("seagreen", 0x2e8b57),
        ("seashell", 0xfff5ee),
        ("sienna", 0xa0522d),
        ("silver", 0xc0c0c0),
        ("skyblue", 0x87ceeb),
        ("slateblue", 0x6a5acd),
        ("slategray", 0x708090),
        ("slategrey", 0x708090),
        ("snow", 0xfffafa),
        ("springgreen", 0x00ff7f),
        ("steelblue", 0x4682b4),
        ("tan", 0xd2b48c),
        ("teal", 0x008080),
        ("thistle", 0xd8bfd8),
        ("tomato", 0xff6347),
        ("turquoise", 0x40e0d0),
        ("violet", 0xee82ee),
        ("wheat", 0xf5deb3),
        ("white", 0xffffff),
        ("whitesmoke", 0xf5f5f5),
        ("yellow", 0xffff00),
        ("yellowgreen", 0x9acd32),
    ];
}
