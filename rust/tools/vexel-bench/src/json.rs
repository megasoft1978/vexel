//! Minimal JSON value, writer and parser for bench results and baselines.

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj(pairs: Vec<(&str, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    pub fn text(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn pairs(&self) -> &[(String, Json)] {
        match self {
            Json::Obj(v) => v,
            _ => &[],
        }
    }

    /// Pretty JSON, two-space indent; arrays of scalars stay on one line.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => out.push_str(&number(*n)),
            Json::Str(s) => quote(s, out),
            Json::Arr(items)
                if items
                    .iter()
                    .all(|i| !matches!(i, Json::Arr(_) | Json::Obj(_))) =>
            {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    item.write(out, depth);
                }
                out.push(']');
            }
            Json::Arr(items) => block(out, depth, '[', ']', items.len(), |out, i| {
                items[i].write(out, depth + 1)
            }),
            Json::Obj(pairs) => block(out, depth, '{', '}', pairs.len(), |out, i| {
                quote(&pairs[i].0, out);
                out.push_str(": ");
                pairs[i].1.write(out, depth + 1);
            }),
        }
    }

    pub fn to_compact(&self) -> String {
        let mut out = String::new();
        self.compact(&mut out);
        out
    }

    fn compact(&self, out: &mut String) {
        match self {
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.compact(out);
                }
                out.push(']');
            }
            Json::Obj(pairs) => {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(k, out);
                    out.push(':');
                    v.compact(out);
                }
                out.push('}');
            }
            scalar => scalar.write(out, 0),
        }
    }
}

fn block(
    out: &mut String,
    depth: usize,
    open: char,
    close: char,
    len: usize,
    mut item: impl FnMut(&mut String, usize),
) {
    if len == 0 {
        out.push(open);
        out.push(close);
        return;
    }
    out.push(open);
    out.push('\n');
    for i in 0..len {
        out.push_str(&"  ".repeat(depth + 1));
        item(out, i);
        out.push_str(if i + 1 < len { ",\n" } else { "\n" });
    }
    out.push_str(&"  ".repeat(depth));
    out.push(close);
}

fn number(n: f64) -> String {
    if !n.is_finite() {
        "null".to_owned()
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        // Three decimals keep results diff-friendly without hiding sub-ms detail.
        let rounded = (n * 1000.0).round() / 1000.0;
        format!("{rounded}")
    }
}

fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
    };
    let value = p.value()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("trailing data at byte {}", p.i));
    }
    Ok(value)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }
    fn eat(&mut self, lit: &str) -> bool {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(_) if self.eat("true") => Ok(Json::Bool(true)),
            Some(_) if self.eat("false") => Ok(Json::Bool(false)),
            Some(_) if self.eat("null") => Ok(Json::Null),
            Some(c) if c.is_ascii_digit() || *c == b'-' => self.number(),
            _ => Err(format!("unexpected input at byte {}", self.i)),
        }
    }
    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        while self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c))
        {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i])
            .ok()
            .and_then(|t| t.parse().ok())
            .map(Json::Num)
            .ok_or_else(|| format!("bad number at byte {start}"))
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let c = *self.s.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("bad escape")?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = self.s.get(self.i..self.i + 4).ok_or("bad \\u escape")?;
                            let code = u32::from_str_radix(
                                std::str::from_utf8(hex).map_err(|e| e.to_string())?,
                                16,
                            )
                            .map_err(|e| e.to_string())?;
                            self.i += 4;
                            let ch = char::from_u32(code).unwrap_or('\u{fffd}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|e| e.to_string())
    }
    fn array(&mut self) -> Result<Json, String> {
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.ws();
            if self.s.get(self.i) == Some(&b']') {
                self.i += 1;
                return Ok(Json::Arr(items));
            }
            items.push(self.value()?);
            self.ws();
            if self.s.get(self.i) == Some(&b',') {
                self.i += 1;
            }
        }
    }
    fn object(&mut self) -> Result<Json, String> {
        self.i += 1;
        let mut pairs = Vec::new();
        loop {
            self.ws();
            match self.s.get(self.i) {
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(pairs));
                }
                Some(b'"') => {}
                _ => return Err(format!("expected key at byte {}", self.i)),
            }
            let key = self.string()?;
            self.ws();
            if self.s.get(self.i) != Some(&b':') {
                return Err(format!("expected ':' at byte {}", self.i));
            }
            self.i += 1;
            pairs.push((key, self.value()?));
            self.ws();
            if self.s.get(self.i) == Some(&b',') {
                self.i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_nested_values() {
        let v = Json::obj(vec![
            ("a", Json::Num(1.5)),
            ("s", Json::str("q\"\n")),
            (
                "l",
                Json::Arr(vec![Json::Num(1.0), Json::Bool(true), Json::Null]),
            ),
            ("o", Json::obj(vec![("k", Json::Arr(vec![]))])),
        ]);
        assert_eq!(parse(&v.to_pretty()).unwrap(), v);
        assert_eq!(parse(&v.to_compact()).unwrap(), v);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("{\"a\":}").is_err());
        assert!(parse("[1] x").is_err());
    }
}
