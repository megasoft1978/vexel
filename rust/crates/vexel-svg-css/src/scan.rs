//! One scanner iteration inspects at most a byte and one byte of lookahead.
//! Escape and comment delimiters consume two bytes without unbounded searches.
use crate::is_space;

pub(crate) struct Scan {
    pub cursor: usize,
    pub end: usize,
    pub quote: u8,
    comment: bool,
}
impl Scan {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            cursor: start,
            end,
            quote: 0,
            comment: false,
        }
    }
    pub fn done(&self) -> bool {
        self.cursor >= self.end
    }
    /// Returns only unquoted, uncommented byte events, just like the baseline scanners.
    pub fn tick(&mut self, source: &[u8]) -> Option<(usize, u8)> {
        if self.done() {
            return None;
        }
        let i = self.cursor;
        let c = source[i];
        if self.comment {
            if i + 1 >= self.end {
                self.cursor = self.end;
            } else if c == b'*' && source[i + 1] == b'/' {
                self.cursor += 2;
                self.comment = false;
            } else {
                self.cursor += 1;
            }
        } else if self.quote != 0 {
            if c == b'\\' {
                self.cursor += (self.end - i).min(2);
            } else {
                if c == self.quote {
                    self.quote = 0;
                }
                self.cursor += 1;
            }
        } else if matches!(c, b'\'' | b'"') {
            self.quote = c;
            self.cursor += 1;
        } else if c == b'/' && i + 1 < self.end && source[i + 1] == b'*' {
            self.comment = true;
            self.cursor += 2;
        } else {
            self.cursor += 1;
            return Some((i, c));
        }
        None
    }
}

pub(crate) struct Space {
    pub cursor: usize,
    end: usize,
    comment: bool,
}
impl Space {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            cursor: start,
            end,
            comment: false,
        }
    }
    /// True means the whitespace/comment run is complete. Each call advances at most two bytes.
    pub fn tick(&mut self, source: &[u8]) -> bool {
        let i = self.cursor;
        if i >= self.end {
            return true;
        }
        if self.comment {
            if i + 1 >= self.end {
                self.cursor = self.end;
                self.comment = false;
            } else if source[i] == b'*' && source[i + 1] == b'/' {
                self.cursor += 2;
                self.comment = false;
            } else {
                self.cursor += 1;
            }
        } else if is_space(source[i]) {
            self.cursor += 1;
        } else if source[i] == b'/' && i + 1 < self.end && source[i + 1] == b'*' {
            self.cursor += 2;
            self.comment = true;
        } else {
            return true;
        }
        false
    }
}
