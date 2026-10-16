//! Thin helpers over `vexel_json` for reading, plus string escaping for writing.

use vexel_json::{JsonDocument, JsonType};

pub struct Json<'a> {
    pub doc: &'a JsonDocument<'a>,
}

impl<'a> Json<'a> {
    pub fn new(doc: &'a JsonDocument<'a>) -> Self {
        Self { doc }
    }

    pub fn root(&self) -> usize {
        self.doc.root()
    }

    pub fn kind(&self, tok: usize) -> Option<JsonType> {
        self.doc.token(tok).map(|t| t.kind)
    }

    /// Array elements, in order.
    pub fn elements(&self, tok: usize) -> Vec<usize> {
        let mut out = Vec::new();
        if self.kind(tok) != Some(JsonType::Array) {
            return out;
        }
        let mut next = self.doc.token(tok).and_then(|t| t.first_child);
        while let Some(i) = next {
            out.push(i);
            next = self.doc.token(i).and_then(|t| t.next_sibling);
        }
        out
    }

    pub fn get(&self, obj: usize, key: &str) -> Option<usize> {
        self.doc.object_get(obj, key.as_bytes())
    }

    pub fn string(&self, tok: usize) -> Option<String> {
        String::from_utf8(self.doc.decoded_string(tok)?).ok()
    }

    pub fn str_at(&self, obj: usize, key: &str) -> Option<String> {
        self.string(self.get(obj, key)?)
    }

    pub fn u64_at(&self, obj: usize, key: &str) -> Option<u64> {
        self.doc.uint(self.get(obj, key)?)
    }
}

/// JSON string literal with escaping.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
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
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_escapes() {
        assert_eq!(quote("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
    }

    #[test]
    fn reads_objects_and_arrays() {
        let doc = JsonDocument::parse(br#"[{"id":"a","bytes":7},{"id":"b"}]"#).unwrap();
        let j = Json::new(&doc);
        let els = j.elements(j.root());
        assert_eq!(els.len(), 2);
        assert_eq!(j.str_at(els[0], "id").as_deref(), Some("a"));
        assert_eq!(j.u64_at(els[0], "bytes"), Some(7));
        assert_eq!(j.u64_at(els[1], "bytes"), None);
    }
}
