use crate::declarations::Declarations;
use crate::matching::{Attribute, SelectorMatcher};
use crate::{
    CssError, CssPaint, CssSheet, CssValue, Declaration, MAX_BYTES, MAX_DECLARATIONS,
    MAX_RULE_MATCHES, PROPERTY_COUNT, Property, Winner, checked_slice, control, inherited,
    is_element, offer, span_eq,
};
use vexel_base::{ControlledError, ControlledStep, WorkControl, WorkKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Initialize,
    Reset,
    Rule,
    Match,
    Offer,
    Presentation,
    InlineAttribute,
    InlineParse,
    InlineOffer,
    Winner,
    Done,
}

/// An exclusive temporary sheet borrow with independent source and paint lifetime.
/// The parent is copied on entry; the only public paint result is produced by successful finish.
/// Units count token visits, byte comparisons, parser operations, rule tests and cascade entries.
pub struct CssComputeSession<'ctx, 'src> {
    sheet: &'ctx mut CssSheet<'src>,
    element: usize,
    parent: Option<CssPaint<'src>>,
    paint: CssPaint<'src>,
    winners: [Option<Winner>; PROPERTY_COUNT],
    terminal: Option<Result<(), ControlledError<CssError>>>,
    phase: Phase,
    property: usize,
    rule: usize,
    declaration: usize,
    matcher: Option<SelectorMatcher<'src>>,
    attribute: Option<Attribute>,
    inline: Option<Declarations>,
}

#[cfg(test)]
#[path = "compute_tests.rs"]
mod tests;

impl<'ctx, 'src> CssComputeSession<'ctx, 'src> {
    pub(crate) fn new(
        sheet: &'ctx mut CssSheet<'src>,
        element: usize,
        parent: Option<CssPaint<'src>>,
    ) -> Self {
        Self {
            sheet,
            element,
            parent,
            paint: parent.unwrap_or_else(CssPaint::initial),
            winners: [None; PROPERTY_COUNT],
            terminal: None,
            phase: Phase::Initialize,
            property: 0,
            rule: 0,
            declaration: 0,
            matcher: None,
            attribute: None,
            inline: None,
        }
    }

    fn kind(&self) -> WorkKind {
        match self.phase {
            Phase::Match => self.matcher.as_ref().expect("selector matching").kind(),
            Phase::Presentation | Phase::InlineAttribute => WorkKind::TokenVisit,
            Phase::InlineParse => WorkKind::SourceRead,
            _ => WorkKind::PaintVisit,
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

    pub fn finish(self) -> Result<CssPaint<'src>, ControlledError<CssError>> {
        self.terminal
            .unwrap_or(Err(ControlledError::Original(CssError::Limit)))?;
        Ok(self.paint)
    }

    fn presentation(&mut self) {
        self.property = 0;
        self.attribute = Some(Attribute::new(self.element, Property::ALL[0].name()));
        self.phase = Phase::Presentation;
    }

    fn tick(&mut self) -> Result<bool, CssError> {
        let source = self.sheet.source;
        let tokens = self.sheet.tokens;
        match self.phase {
            Phase::Initialize => {
                if self
                    .parent
                    .and_then(|paint| paint.source)
                    .is_some_and(|p| !std::ptr::eq(p, source))
                {
                    return Err(CssError::Limit);
                }
                if !tokens.get(self.element).is_some_and(is_element) {
                    return Err(CssError::Limit);
                }
                self.paint.source = Some(source);
                self.property = 0;
                self.phase = Phase::Reset;
            }
            Phase::Reset => {
                let property = Property::ALL[self.property];
                if !inherited(property) {
                    self.paint.values[self.property] = None;
                }
                self.property += 1;
                if self.property == PROPERTY_COUNT {
                    self.phase = Phase::Rule;
                }
            }
            Phase::Rule => {
                if self.rule == self.sheet.rules.len() {
                    self.presentation();
                } else {
                    if self.sheet.match_tests >= MAX_RULE_MATCHES {
                        self.sheet.hit_limit = true;
                        return Err(CssError::Limit);
                    }
                    self.sheet.match_tests += 1;
                    self.matcher = Some(SelectorMatcher::new(
                        self.sheet.rules[self.rule].selector,
                        self.element,
                    ));
                    self.phase = Phase::Match;
                }
            }
            Phase::Match => {
                let result = self.matcher.as_mut().expect("selector matching").tick(
                    source,
                    tokens,
                    &mut self.sheet.selector_visits,
                    &mut self.sheet.hit_limit,
                )?;
                if let Some(matches) = result {
                    self.matcher = None;
                    if matches {
                        let token = &tokens[self.element];
                        if self.sheet.rules[self.rule].unsupported
                            && token.prefix.length == 0
                            && span_eq(source, token.local_name, b"stop")
                        {
                            return Err(CssError::Unsupported);
                        }
                        self.declaration = 0;
                        self.phase = Phase::Offer;
                    } else {
                        self.rule += 1;
                        self.phase = Phase::Rule;
                    }
                }
            }
            Phase::Offer => {
                let rule = &self.sheet.rules[self.rule];
                if self.declaration == rule.declarations.len() {
                    self.rule += 1;
                    self.phase = Phase::Rule;
                } else {
                    offer(
                        &mut self.winners,
                        rule.declarations[self.declaration],
                        rule.selector.specificity,
                        rule.order
                            .saturating_mul(MAX_DECLARATIONS)
                            .saturating_add(self.declaration),
                    );
                    self.declaration += 1;
                }
            }
            Phase::Presentation => {
                let attr = self.attribute.as_mut().expect("presentation attribute");
                if attr.tick(source, tokens) {
                    if let Some(value) = attr.result {
                        offer(
                            &mut self.winners,
                            Declaration {
                                property: Property::ALL[self.property],
                                value,
                                important: false,
                            },
                            0,
                            0,
                        );
                    }
                    self.property += 1;
                    if self.property == PROPERTY_COUNT {
                        self.attribute = Some(Attribute::new(self.element, b"style"));
                        self.phase = Phase::InlineAttribute;
                    } else {
                        self.attribute = Some(Attribute::new(
                            self.element,
                            Property::ALL[self.property].name(),
                        ));
                    }
                }
            }
            Phase::InlineAttribute => {
                let attr = self.attribute.as_mut().expect("inline attribute");
                if attr.tick(source, tokens) {
                    if let Some(style) = attr.result {
                        if style.length > MAX_BYTES {
                            return Err(CssError::Limit);
                        }
                        self.inline = Some(Declarations::new(source, style)?);
                        self.phase = Phase::InlineParse;
                    } else {
                        self.property = 0;
                        self.phase = Phase::Winner;
                    }
                }
            }
            Phase::InlineParse => {
                let parser = self.inline.as_mut().expect("inline parser");
                let result = parser.tick(source);
                self.sheet.hit_limit |= parser.hit_limit;
                if result? {
                    self.declaration = 0;
                    self.phase = Phase::InlineOffer;
                }
            }
            Phase::InlineOffer => {
                let output = &self.inline.as_ref().expect("inline parser").output;
                if self.declaration == output.len() {
                    self.inline = None;
                    self.property = 0;
                    self.phase = Phase::Winner;
                } else {
                    offer(
                        &mut self.winners,
                        output[self.declaration],
                        1_000_000,
                        self.declaration,
                    );
                    self.declaration += 1;
                }
            }
            Phase::Winner => {
                let property = Property::ALL[self.property];
                if let Some(winner) = self.winners[self.property] {
                    let value = checked_slice(source, winner.value)?;
                    // These three fixed keyword comparisons inspect at most 19 bytes in total.
                    // All long value scanning is retained in the declaration parser above.
                    if value.eq_ignore_ascii_case(b"inherit")
                        || (value.eq_ignore_ascii_case(b"unset") && inherited(property))
                    {
                        self.paint.values[self.property] =
                            self.parent.and_then(|p| p.value(property));
                    } else if value.eq_ignore_ascii_case(b"initial")
                        || value.eq_ignore_ascii_case(b"unset")
                    {
                        self.paint.values[self.property] = None;
                    } else {
                        self.paint.values[self.property] = Some(CssValue {
                            span: winner.value,
                            bytes: value,
                        });
                    }
                }
                self.property += 1;
                if self.property == PROPERTY_COUNT {
                    self.phase = Phase::Done;
                }
            }
            Phase::Done => return Ok(true),
        }
        Ok(matches!(self.phase, Phase::Done))
    }
}
