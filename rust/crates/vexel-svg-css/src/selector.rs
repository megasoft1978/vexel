use crate::{MAX_CLASSES, MAX_SELECTOR_TERMS, Selector, Term, XmlSpan, ident, is_space};

#[derive(Clone, Copy)]
enum Phase {
    Right,
    Left,
    Begin,
    Identifier,
    Compound,
    FinishTerm,
    Space,
    Done,
}

pub(crate) struct SelectorParser {
    phase: Phase,
    start: usize,
    end: usize,
    cursor: usize,
    begin: usize,
    marker: u8,
    spaced: bool,
    term: Term,
    selector: Selector,
    valid: bool,
}
impl SelectorParser {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            phase: Phase::Right,
            start,
            end,
            cursor: start,
            begin: start,
            marker: 0,
            spaced: false,
            term: Term::EMPTY,
            selector: Selector::EMPTY,
            valid: true,
        }
    }
    fn reject(&mut self) {
        self.valid = false;
        self.phase = Phase::Done;
    }
    pub fn result(&self) -> Option<Selector> {
        (self.valid && self.selector.count > 0).then_some(self.selector)
    }
    pub fn tick(&mut self, source: &[u8]) -> bool {
        match self.phase {
            Phase::Right => {
                if self.end > self.start && is_space(source[self.end - 1]) {
                    self.end -= 1;
                } else {
                    self.phase = Phase::Left;
                }
            }
            Phase::Left => {
                if self.start < self.end && is_space(source[self.start]) {
                    self.start += 1;
                } else {
                    self.cursor = self.start;
                    self.phase = Phase::Begin;
                }
            }
            Phase::Begin => {
                if self.cursor >= self.end {
                    self.phase = Phase::Done;
                } else if self.selector.count == MAX_SELECTOR_TERMS {
                    self.reject();
                } else {
                    self.term = Term::EMPTY;
                    if matches!(source[self.cursor], b'#' | b'.') {
                        self.phase = Phase::Compound;
                    } else {
                        self.marker = 0;
                        self.begin = self.cursor;
                        self.phase = Phase::Identifier;
                    }
                }
            }
            Phase::Identifier => {
                if self.cursor < self.end && ident(source[self.cursor]) {
                    self.cursor += 1;
                } else if self.begin == self.cursor {
                    self.reject();
                } else {
                    let span = XmlSpan {
                        offset: self.begin,
                        length: self.cursor - self.begin,
                    };
                    match self.marker {
                        0 => {
                            self.term.element = Some(span);
                            self.term.specificity += 1;
                        }
                        b'#' if self.term.id.is_none() => {
                            self.term.id = Some(span);
                            self.term.specificity += 10_000;
                        }
                        b'.' if self.term.class_count < MAX_CLASSES => {
                            self.term.classes[self.term.class_count] = span;
                            self.term.class_count += 1;
                            self.term.specificity += 100;
                        }
                        _ => {
                            self.reject();
                            return true;
                        }
                    }
                    self.phase = Phase::Compound;
                }
            }
            Phase::Compound => {
                if self.cursor < self.end && matches!(source[self.cursor], b'#' | b'.') {
                    self.marker = source[self.cursor];
                    self.cursor += 1;
                    self.begin = self.cursor;
                    self.phase = Phase::Identifier;
                } else {
                    self.phase = Phase::FinishTerm;
                }
            }
            Phase::FinishTerm => {
                if self.term.element.is_none()
                    && self.term.id.is_none()
                    && self.term.class_count == 0
                {
                    self.reject();
                } else if let Some(specificity) =
                    self.selector.specificity.checked_add(self.term.specificity)
                {
                    self.selector.specificity = specificity;
                    self.selector.terms[self.selector.count] = self.term;
                    self.selector.count += 1;
                    self.spaced = false;
                    self.phase = Phase::Space;
                } else {
                    self.reject();
                }
            }
            Phase::Space => {
                if self.cursor < self.end && is_space(source[self.cursor]) {
                    self.cursor += 1;
                    self.spaced = true;
                } else if self.cursor < self.end
                    && (!self.spaced || matches!(source[self.cursor], b'>' | b'+' | b'~'))
                {
                    self.reject();
                } else {
                    self.phase = Phase::Begin;
                }
            }
            Phase::Done => return true,
        }
        matches!(self.phase, Phase::Done)
    }
}
