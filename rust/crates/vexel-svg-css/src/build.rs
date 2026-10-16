use crate::declarations::Declarations;
use crate::scan::{Scan, Space};
use crate::selector::SelectorParser;
use crate::{
    CssError, CssSheet, Declaration, MAX_BYTES, MAX_DECLARATIONS, MAX_RULES, Rule, Selector,
    TokenKind, XmlSpan, XmlToken, checked_slice, control, is_style_element,
};
use std::mem::size_of;
use vexel_base::{ControlledError, ControlledStep, WorkControl, WorkKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Outer,
    Inner,
    Space,
    AtRule,
    Brace,
    Close,
    Declarations,
    Comma,
    Selector,
    Reserve,
    Copy,
    Push,
    Done,
}

/// A source-borrowing build transaction. Each step advances retained parser cursors.
/// Units count bounded token visits, scanner iterations, grammar transitions and Vec entries.
/// No sheet is published until completion; dropping a session releases its temporary buffers.
pub struct CssBuildSession<'src> {
    sheet: CssSheet<'src>,
    terminal: Option<Result<(), ControlledError<CssError>>>,
    phase: Phase,
    outer: usize,
    inner: usize,
    end: usize,
    space: Space,
    scan: Scan,
    parens: usize,
    depth: usize,
    at_rule: bool,
    selector_start: usize,
    brace: usize,
    close: usize,
    part: usize,
    next_part: usize,
    declarations: Option<Declarations>,
    selector_parser: Option<SelectorParser>,
    selector: Selector,
    copy: Vec<Declaration>,
    copy_index: usize,
}

impl<'src> CssBuildSession<'src> {
    pub fn new(source: &'src [u8], tokens: &'src [XmlToken]) -> Self {
        Self {
            sheet: CssSheet {
                source,
                tokens,
                rules: Vec::new(),
                declaration_count: 0,
                css_bytes: 0,
                match_tests: 0,
                selector_visits: 0,
                hit_limit: false,
                allocated_bytes: 0,
            },
            terminal: None,
            phase: Phase::Outer,
            outer: 0,
            inner: 0,
            end: 0,
            space: Space::new(0, 0),
            scan: Scan::new(0, 0),
            parens: 0,
            depth: 0,
            at_rule: false,
            selector_start: 0,
            brace: 0,
            close: 0,
            part: 0,
            next_part: 0,
            declarations: None,
            selector_parser: None,
            selector: Selector::EMPTY,
            copy: Vec::new(),
            copy_index: 0,
        }
    }

    fn kind(&self) -> WorkKind {
        match self.phase {
            Phase::Outer | Phase::Inner => WorkKind::TokenVisit,
            Phase::Reserve | Phase::Copy | Phase::Push => WorkKind::Expansion,
            _ => WorkKind::SourceRead,
        }
    }

    pub fn step(
        &mut self,
        quantum: u64,
        control: &mut dyn WorkControl,
    ) -> ControlledStep<(), CssError> {
        if let Some(result) = self.terminal {
            return match result {
                Ok(()) => ControlledStep::Complete(()),
                Err(e) => ControlledStep::Failed(e),
            };
        }
        if let Err(stop) = control.checkpoint() {
            return self.fail(stop.into());
        }
        let mut charged = 0;
        while charged < quantum {
            if let Err(stop) = control.charge(self.kind(), 1) {
                return self.fail(stop.into());
            }
            charged += 1;
            match self.tick() {
                Ok(true) => {
                    self.terminal = Some(Ok(()));
                    return ControlledStep::Complete(());
                }
                Ok(false) => {}
                Err(e) => return self.fail(control::original(e)),
            }
        }
        ControlledStep::Pending {
            charged_units: charged,
        }
    }

    fn fail(&mut self, error: ControlledError<CssError>) -> ControlledStep<(), CssError> {
        self.terminal = Some(Err(error));
        ControlledStep::Failed(error)
    }

    pub fn finish(self) -> Result<CssSheet<'src>, ControlledError<CssError>> {
        self.terminal
            .unwrap_or(Err(ControlledError::Original(CssError::Limit)))?;
        Ok(self.sheet)
    }

    fn next_child(&mut self) {
        self.inner += 1;
        self.phase = Phase::Inner;
    }
    fn space_at(&mut self, cursor: usize) {
        self.space = Space::new(cursor, self.end);
        self.phase = Phase::Space;
    }
    fn next_selector(&mut self) {
        self.part = self.next_part;
        self.selector_parser = None;
        self.scan = Scan::new(self.part, self.brace);
        self.phase = Phase::Comma;
    }
    fn reserve_rule(&mut self) -> Result<(), CssError> {
        let count = self
            .declarations
            .as_ref()
            .expect("declaration phase")
            .output
            .len();
        let new_rules = self
            .sheet
            .rules
            .len()
            .checked_add(1)
            .ok_or(CssError::Limit)?;
        if new_rules > MAX_RULES
            || count > MAX_DECLARATIONS.saturating_sub(self.sheet.declaration_count)
        {
            self.sheet.hit_limit = true;
            return Err(CssError::Limit);
        }
        let old = self
            .sheet
            .rules
            .capacity()
            .checked_mul(size_of::<Rule>())
            .ok_or(CssError::Limit)?;
        crate::reserve(&mut self.sheet.rules, 1, false)?;
        let new = self
            .sheet
            .rules
            .capacity()
            .checked_mul(size_of::<Rule>())
            .ok_or(CssError::Limit)?;
        self.sheet.allocated_bytes = self
            .sheet
            .allocated_bytes
            .checked_sub(old)
            .and_then(|n| n.checked_add(new))
            .ok_or(CssError::Limit)?;
        self.copy = Vec::new();
        crate::reserve(&mut self.copy, count, true)?;
        self.copy_index = 0;
        self.phase = Phase::Copy;
        Ok(())
    }

    /// Every arm performs bounded work; all potentially long loops live in session cursors.
    fn tick(&mut self) -> Result<bool, CssError> {
        let source = self.sheet.source;
        match self.phase {
            Phase::Outer => {
                if self.outer == self.sheet.tokens.len() {
                    self.phase = Phase::Done;
                } else {
                    let index = self.outer;
                    self.outer += 1;
                    let token = &self.sheet.tokens[index];
                    if token.kind == TokenKind::StartElement && is_style_element(source, token) {
                        self.inner = index + 1;
                        self.phase = Phase::Inner;
                    }
                }
            }
            Phase::Inner => match self.sheet.tokens.get(self.inner) {
                None => self.phase = Phase::Outer,
                Some(token) if token.kind == TokenKind::EndElement => self.phase = Phase::Outer,
                Some(token) if matches!(token.kind, TokenKind::Text | TokenKind::Cdata) => {
                    let bytes = checked_slice(source, token.value)?;
                    self.sheet.css_bytes = self
                        .sheet
                        .css_bytes
                        .checked_add(bytes.len())
                        .ok_or(CssError::Limit)?;
                    if self.sheet.css_bytes > MAX_BYTES {
                        return Err(CssError::Limit);
                    }
                    self.end = token.value.end().ok_or(CssError::Limit)?;
                    self.space_at(token.value.offset);
                }
                Some(_) => self.inner += 1,
            },
            Phase::Space => {
                if self.space.tick(source) {
                    let cursor = self.space.cursor;
                    if cursor >= self.end {
                        self.next_child();
                    } else if source[cursor] == b'@' {
                        self.scan = Scan::new(cursor, self.end);
                        self.parens = 0;
                        self.phase = Phase::AtRule;
                    } else {
                        self.selector_start = cursor;
                        self.scan = Scan::new(cursor, self.end);
                        self.phase = Phase::Brace;
                    }
                }
            }
            Phase::AtRule => {
                if self.scan.done() {
                    self.next_child();
                } else if let Some((i, c)) = self.scan.tick(source) {
                    match c {
                        b'(' => self.parens = self.parens.checked_add(1).ok_or(CssError::Limit)?,
                        b')' => self.parens = self.parens.saturating_sub(1),
                        b';' if self.parens == 0 => self.space_at(i + 1),
                        b'{' if self.parens == 0 => {
                            self.scan = Scan::new(i + 1, self.end);
                            self.depth = 1;
                            self.at_rule = true;
                            self.phase = Phase::Close;
                        }
                        _ => {}
                    }
                }
            }
            Phase::Brace => {
                if self.scan.done() {
                    self.next_child();
                } else if let Some((i, b'{')) = self.scan.tick(source) {
                    self.brace = i;
                    self.scan = Scan::new(i + 1, self.end);
                    self.depth = 1;
                    self.at_rule = false;
                    self.phase = Phase::Close;
                }
            }
            Phase::Close => {
                if self.scan.done() {
                    self.next_child();
                } else if let Some((i, c)) = self.scan.tick(source) {
                    if c == b'{' {
                        self.depth = self.depth.checked_add(1).ok_or(CssError::Limit)?;
                    } else if c == b'}' {
                        self.depth -= 1;
                        if self.depth == 0 {
                            if self.at_rule {
                                self.space_at(i + 1);
                            } else {
                                self.close = i;
                                self.declarations = Some(Declarations::new(
                                    source,
                                    XmlSpan {
                                        offset: self.brace + 1,
                                        length: i - self.brace - 1,
                                    },
                                )?);
                                self.phase = Phase::Declarations;
                            }
                        }
                    }
                }
            }
            Phase::Declarations => {
                let parser = self.declarations.as_mut().expect("declaration phase");
                let result = parser.tick(source);
                self.sheet.hit_limit |= parser.hit_limit;
                if result? {
                    self.part = self.selector_start;
                    self.scan = Scan::new(self.part, self.brace);
                    self.phase = Phase::Comma;
                }
            }
            Phase::Comma => {
                if self.part >= self.brace {
                    self.declarations = None;
                    self.space_at(self.close + 1);
                } else {
                    let comma = if self.scan.done() {
                        Some(self.brace)
                    } else {
                        self.scan
                            .tick(source)
                            .and_then(|(i, c)| (c == b',').then_some(i))
                    };
                    if let Some(comma) = comma {
                        self.selector_parser = Some(SelectorParser::new(self.part, comma));
                        self.next_part = if comma < self.brace {
                            comma + 1
                        } else {
                            self.brace
                        };
                        self.phase = Phase::Selector;
                    }
                }
            }
            Phase::Selector => {
                let parser = self.selector_parser.as_mut().expect("selector phase");
                if parser.tick(source) {
                    let decls = self.declarations.as_ref().expect("declaration phase");
                    if let Some(selector) = parser
                        .result()
                        .filter(|_| !decls.output.is_empty() || decls.unsupported)
                    {
                        self.selector = selector;
                        self.phase = Phase::Reserve;
                    } else {
                        self.next_selector();
                    }
                }
            }
            Phase::Reserve => self.reserve_rule()?,
            Phase::Copy => {
                let declarations = &self
                    .declarations
                    .as_ref()
                    .expect("declaration phase")
                    .output;
                if self.copy_index == declarations.len() {
                    self.phase = Phase::Push;
                } else {
                    self.copy.push(declarations[self.copy_index]);
                    self.copy_index += 1;
                }
            }
            Phase::Push => {
                let bytes = self
                    .copy
                    .capacity()
                    .checked_mul(size_of::<Declaration>())
                    .ok_or(CssError::Limit)?;
                self.sheet.allocated_bytes = self
                    .sheet
                    .allocated_bytes
                    .checked_add(bytes)
                    .ok_or(CssError::Limit)?;
                self.sheet.declaration_count = self
                    .sheet
                    .declaration_count
                    .checked_add(self.copy.len())
                    .ok_or(CssError::Limit)?;
                let rule = Rule {
                    selector: self.selector,
                    declarations: std::mem::take(&mut self.copy),
                    order: self.sheet.rules.len(),
                    unsupported: self
                        .declarations
                        .as_ref()
                        .expect("declaration phase")
                        .unsupported,
                };
                self.sheet.rules.push(rule);
                self.next_selector();
            }
            Phase::Done => return Ok(true),
        }
        Ok(matches!(self.phase, Phase::Done))
    }
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
