//! The stylesheet and inline declaration parser share these real cursors.
use crate::scan::{Scan, Space};
use crate::{
    CssError, Declaration, MAX_DECLARATIONS, PROPERTY_COUNT, Property, XmlSpan, checked_slice,
    ident, is_space,
};

#[derive(Clone, Copy)]
enum Phase {
    Skip,
    Segment,
    NameLeft,
    NameRight,
    ValueLeft,
    ValueRight,
    BackComment,
    Property,
    Mark,
    Word,
    Bang,
    Inside,
    ImportantTrim,
    Push,
    Done,
}

pub(crate) struct Declarations {
    phase: Phase,
    end: usize,
    next: usize,
    scan: Scan,
    space: Space,
    start: usize,
    colon: Option<usize>,
    parens: usize,
    ns: usize,
    ne: usize,
    vs: usize,
    ve: usize,
    back: usize,
    property_index: usize,
    property: Property,
    mark: usize,
    word: usize,
    bang: usize,
    important: bool,
    pub output: Vec<Declaration>,
    pub unsupported: bool,
    pub hit_limit: bool,
}

impl Declarations {
    pub fn new(source: &[u8], span: XmlSpan) -> Result<Self, CssError> {
        let end = span.end().ok_or(CssError::Limit)?;
        checked_slice(source, span)?;
        Ok(Self {
            phase: Phase::Skip,
            end,
            next: span.offset,
            scan: Scan::new(span.offset, end),
            space: Space::new(span.offset, end),
            start: 0,
            colon: None,
            parens: 0,
            ns: 0,
            ne: 0,
            vs: 0,
            ve: 0,
            back: 0,
            property_index: 0,
            property: Property::Fill,
            mark: 0,
            word: 0,
            bang: 0,
            important: false,
            output: Vec::new(),
            unsupported: false,
            hit_limit: false,
        })
    }
    fn next_segment(&mut self) {
        self.space = Space::new(self.next, self.end);
        self.phase = Phase::Skip;
    }
    fn segment(&mut self, end: usize) {
        self.next = self.scan.cursor;
        if let Some(colon) = self.colon.filter(|&c| c < end) {
            self.ns = self.start;
            self.ne = colon;
            self.vs = colon + 1;
            self.ve = end;
            self.phase = Phase::NameLeft;
        } else {
            self.next_segment();
        }
    }
    fn after_value(&mut self) {
        if self.ns == self.ne || self.vs == self.ve {
            self.next_segment();
        } else {
            self.property_index = 0;
            self.phase = Phase::Property;
        }
    }
    /// One bounded scanner, trim, name lookup, or retained declaration append operation.
    pub fn tick(&mut self, source: &[u8]) -> Result<bool, CssError> {
        match self.phase {
            Phase::Skip => {
                if self.space.tick(source) {
                    if self.space.cursor >= self.end {
                        self.phase = Phase::Done;
                    } else {
                        self.start = self.space.cursor;
                        self.colon = None;
                        self.parens = 0;
                        self.scan = Scan::new(self.start, self.end);
                        self.phase = Phase::Segment;
                    }
                }
            }
            Phase::Segment => {
                if self.scan.done() {
                    self.segment(self.end);
                } else if let Some((i, c)) = self.scan.tick(source) {
                    match c {
                        b'(' => self.parens = self.parens.saturating_add(1),
                        b')' => self.parens = self.parens.saturating_sub(1),
                        b':' if self.colon.is_none() => self.colon = Some(i),
                        b';' if self.parens == 0 => self.segment(i),
                        _ => {}
                    }
                }
            }
            Phase::NameLeft => {
                if self.ns < self.ne && is_space(source[self.ns]) {
                    self.ns += 1;
                } else {
                    self.phase = Phase::NameRight;
                }
            }
            Phase::NameRight => {
                if self.ne > self.ns && is_space(source[self.ne - 1]) {
                    self.ne -= 1;
                } else {
                    self.space = Space::new(self.vs, self.ve);
                    self.phase = Phase::ValueLeft;
                }
            }
            Phase::ValueLeft => {
                if self.space.tick(source) {
                    self.vs = self.space.cursor;
                    self.phase = Phase::ValueRight;
                }
            }
            Phase::ValueRight => {
                if self.ve > self.vs && is_space(source[self.ve - 1]) {
                    self.ve -= 1;
                } else if self.ve - self.vs >= 2 && &source[self.ve - 2..self.ve] == b"*/" {
                    self.back = self.ve - 2;
                    self.phase = Phase::BackComment;
                } else {
                    self.after_value();
                }
            }
            Phase::BackComment => {
                // Preserve the baseline reverse search, including its unmatched-comment behavior.
                if self.back > self.vs {
                    if &source[self.back - 1..self.back + 1] == b"/*" {
                        self.ve = self.back - 1;
                        self.phase = Phase::ValueRight;
                    } else {
                        self.back -= 1;
                    }
                } else {
                    self.after_value();
                }
            }
            Phase::Property => {
                if self.property_index == PROPERTY_COUNT {
                    self.unsupported = true;
                    self.next_segment();
                } else {
                    let p = Property::ALL[self.property_index];
                    // A property name is at most 26 bytes; unequal lengths require no byte walk.
                    let name = &source[self.ns..self.ne];
                    if name.eq_ignore_ascii_case(p.name()) {
                        self.property = p;
                        self.mark = self.ve;
                        self.important = false;
                        self.phase = Phase::Mark;
                    } else {
                        self.property_index += 1;
                    }
                }
            }
            Phase::Mark => {
                if self.mark > self.vs && is_space(source[self.mark - 1]) {
                    self.mark -= 1;
                } else {
                    self.word = self.mark;
                    self.phase = Phase::Word;
                }
            }
            Phase::Word => {
                if self.word > self.vs && ident(source[self.word - 1]) {
                    self.word -= 1;
                } else {
                    self.bang = self.word;
                    self.phase = Phase::Bang;
                }
            }
            Phase::Bang => {
                if self.bang > self.vs && is_space(source[self.bang - 1]) {
                    self.bang -= 1;
                } else if self.bang > self.vs
                    && source[self.bang - 1] == b'!'
                    && source[self.word..self.mark].eq_ignore_ascii_case(b"important")
                {
                    self.scan = Scan::new(self.vs, self.bang - 1);
                    self.phase = Phase::Inside;
                } else {
                    self.phase = Phase::Push;
                }
            }
            Phase::Inside => {
                if self.scan.done() {
                    if self.scan.quote == 0 {
                        self.ve = self.bang - 1;
                        self.important = true;
                        self.phase = Phase::ImportantTrim;
                    } else {
                        self.phase = Phase::Push;
                    }
                } else {
                    self.scan.tick(source);
                }
            }
            Phase::ImportantTrim => {
                if self.ve > self.vs && is_space(source[self.ve - 1]) {
                    self.ve -= 1;
                } else {
                    self.phase = Phase::Push;
                }
            }
            Phase::Push => {
                if self.ve > self.vs {
                    if self.output.len() >= MAX_DECLARATIONS {
                        self.hit_limit = true;
                        return Err(CssError::Limit);
                    }
                    crate::reserve(&mut self.output, 1, false)?;
                    self.output.push(Declaration {
                        property: self.property,
                        value: XmlSpan {
                            offset: self.vs,
                            length: self.ve - self.vs,
                        },
                        important: self.important,
                    });
                }
                self.next_segment();
            }
            Phase::Done => return Ok(true),
        }
        Ok(matches!(self.phase, Phase::Done))
    }
}
