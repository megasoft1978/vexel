use crate::{
    CssError, MAX_RULE_MATCHES, Selector, Term, TokenKind, XmlSpan, XmlToken, checked_slice,
    is_element, is_space, span_eq,
};
use vexel_base::WorkKind;

/// Attribute names are compared only to fixed literals (max 26 bytes). One tick visits one token.
pub(crate) struct Attribute {
    cursor: Option<usize>,
    name: &'static [u8],
    pub result: Option<XmlSpan>,
}
impl Attribute {
    pub fn new(element: usize, name: &'static [u8]) -> Self {
        Self {
            cursor: element.checked_add(1),
            name,
            result: None,
        }
    }
    pub fn tick(&mut self, source: &[u8], tokens: &[XmlToken]) -> bool {
        let Some(i) = self.cursor else { return true };
        let Some(token) = tokens.get(i).filter(|t| t.kind == TokenKind::Attribute) else {
            self.cursor = None;
            return true;
        };
        if token.prefix.length == 0 && span_eq(source, token.local_name, self.name) {
            self.result = Some(token.value);
            self.cursor = None;
            return true;
        }
        self.cursor = i.checked_add(1);
        false
    }
}

/// Each tick compares one byte. Invalid ID spans preserve the baseline Option equality.
struct Compare<'src> {
    a: Option<&'src [u8]>,
    b: Option<&'src [u8]>,
    cursor: usize,
    invalid_equal: bool,
}
impl<'src> Compare<'src> {
    fn new(a: Option<&'src [u8]>, b: Option<&'src [u8]>, invalid_equal: bool) -> Self {
        Self {
            a,
            b,
            cursor: 0,
            invalid_equal,
        }
    }
    fn tick(&mut self) -> Option<bool> {
        match (self.a, self.b) {
            (Some(a), Some(b)) if a.len() == b.len() => {
                if self.cursor == a.len() {
                    return Some(true);
                }
                if a[self.cursor] != b[self.cursor] {
                    return Some(false);
                }
                self.cursor += 1;
                (self.cursor == a.len()).then_some(true)
            }
            (None, None) => Some(self.invalid_equal),
            _ => Some(false),
        }
    }
}

#[derive(Clone, Copy)]
enum ClassPhase {
    Skip,
    Start,
    Word,
    Compare,
}
struct Class<'src> {
    value: Option<&'src [u8]>,
    wanted: Option<&'src [u8]>,
    cursor: usize,
    start: usize,
    count: usize,
    phase: ClassPhase,
    compare: Option<Compare<'src>>,
}
impl<'src> Class<'src> {
    fn new(source: &'src [u8], value: XmlSpan, wanted: XmlSpan) -> Self {
        Self {
            value: checked_slice(source, value).ok(),
            wanted: checked_slice(source, wanted).ok(),
            cursor: 0,
            start: 0,
            count: 0,
            phase: ClassPhase::Skip,
            compare: None,
        }
    }
    fn tick(&mut self) -> Option<bool> {
        let (Some(value), Some(wanted)) = (self.value, self.wanted) else {
            return Some(false);
        };
        match self.phase {
            ClassPhase::Skip => {
                if self.cursor == value.len() {
                    return Some(false);
                }
                if is_space(value[self.cursor]) {
                    self.cursor += 1;
                } else {
                    self.phase = ClassPhase::Start;
                }
            }
            ClassPhase::Start => {
                self.count += 1;
                if self.count > 256 {
                    return Some(false);
                }
                self.start = self.cursor;
                self.phase = ClassPhase::Word;
            }
            ClassPhase::Word => {
                if self.cursor < value.len() && !is_space(value[self.cursor]) {
                    self.cursor += 1;
                } else {
                    self.compare = Some(Compare::new(
                        Some(&value[self.start..self.cursor]),
                        Some(wanted),
                        false,
                    ));
                    self.phase = ClassPhase::Compare;
                }
            }
            ClassPhase::Compare => {
                if let Some(matches) = self.compare.as_mut().expect("class comparison").tick() {
                    if matches {
                        return Some(true);
                    }
                    self.compare = None;
                    self.phase = ClassPhase::Skip;
                }
            }
        }
        None
    }
}

#[derive(Clone, Copy)]
enum TermPhase {
    Begin,
    Element,
    FindId,
    Id,
    FindClass,
    Class,
    Done,
}
struct TermMatcher<'src> {
    term: Term,
    element: usize,
    phase: TermPhase,
    attribute: Option<Attribute>,
    compare: Option<Compare<'src>>,
    class: Option<Class<'src>>,
    class_index: usize,
    class_value: XmlSpan,
    result: bool,
}
impl<'src> TermMatcher<'src> {
    fn new(term: Term, element: usize) -> Self {
        Self {
            term,
            element,
            phase: TermPhase::Begin,
            attribute: None,
            compare: None,
            class: None,
            class_index: 0,
            class_value: XmlSpan {
                offset: 0,
                length: 0,
            },
            result: false,
        }
    }
    fn kind(&self) -> WorkKind {
        match self.phase {
            TermPhase::FindId | TermPhase::FindClass => WorkKind::TokenVisit,
            _ => WorkKind::SourceRead,
        }
    }
    fn finish(&mut self, result: bool) {
        self.result = result;
        self.phase = TermPhase::Done;
    }
    fn id(&mut self) {
        if self.term.id.is_some() {
            self.attribute = Some(Attribute::new(self.element, b"id"));
            self.phase = TermPhase::FindId;
        } else {
            self.classes();
        }
    }
    fn classes(&mut self) {
        if self.term.class_count > 0 {
            self.attribute = Some(Attribute::new(self.element, b"class"));
            self.phase = TermPhase::FindClass;
        } else {
            self.finish(true);
        }
    }
    fn tick(&mut self, source: &'src [u8], tokens: &[XmlToken]) -> Option<bool> {
        match self.phase {
            TermPhase::Begin => {
                if let Some(name) = self.term.element {
                    let local =
                        checked_slice(source, tokens[self.element].local_name).unwrap_or_default();
                    self.compare = Some(Compare::new(
                        checked_slice(source, name).ok(),
                        Some(local),
                        false,
                    ));
                    self.phase = TermPhase::Element;
                } else {
                    self.id();
                }
            }
            TermPhase::Element => {
                if let Some(matches) = self.compare.as_mut().expect("element comparison").tick() {
                    if matches {
                        self.id();
                    } else {
                        self.finish(false);
                    }
                }
            }
            TermPhase::FindId => {
                let attr = self.attribute.as_mut().expect("id lookup");
                if attr.tick(source, tokens) {
                    if let Some(value) = attr.result {
                        self.compare = Some(Compare::new(
                            checked_slice(source, value).ok(),
                            checked_slice(source, self.term.id.expect("id span")).ok(),
                            true,
                        ));
                        self.phase = TermPhase::Id;
                    } else {
                        self.finish(false);
                    }
                }
            }
            TermPhase::Id => {
                if let Some(matches) = self.compare.as_mut().expect("id comparison").tick() {
                    if matches {
                        self.classes();
                    } else {
                        self.finish(false);
                    }
                }
            }
            TermPhase::FindClass => {
                let attr = self.attribute.as_mut().expect("class lookup");
                if attr.tick(source, tokens) {
                    if let Some(value) = attr.result {
                        self.class_value = value;
                        self.class_index = 0;
                        self.class = Some(Class::new(source, value, self.term.classes[0]));
                        self.phase = TermPhase::Class;
                    } else {
                        self.finish(false);
                    }
                }
            }
            TermPhase::Class => {
                if let Some(matches) = self.class.as_mut().expect("class scan").tick() {
                    if !matches {
                        self.finish(false);
                    } else {
                        self.class_index += 1;
                        if self.class_index == self.term.class_count {
                            self.finish(true);
                        } else {
                            self.class = Some(Class::new(
                                source,
                                self.class_value,
                                self.term.classes[self.class_index],
                            ));
                        }
                    }
                }
            }
            TermPhase::Done => return Some(self.result),
        }
        matches!(self.phase, TermPhase::Done).then_some(self.result)
    }
}

#[derive(Clone, Copy)]
enum MatchPhase {
    Begin,
    Term,
    Ancestor,
    Done,
}
pub(crate) struct SelectorMatcher<'src> {
    selector: Selector,
    phase: MatchPhase,
    term_index: usize,
    descendant: usize,
    cursor: usize,
    candidate: usize,
    rightmost: bool,
    term: Option<TermMatcher<'src>>,
    result: bool,
}
impl<'src> SelectorMatcher<'src> {
    pub fn new(selector: Selector, element: usize) -> Self {
        Self {
            selector,
            phase: MatchPhase::Begin,
            term_index: 0,
            descendant: element,
            cursor: element,
            candidate: element,
            rightmost: true,
            term: None,
            result: false,
        }
    }
    pub fn kind(&self) -> WorkKind {
        if matches!(self.phase, MatchPhase::Term) {
            self.term.as_ref().expect("term matching").kind()
        } else {
            WorkKind::TokenVisit
        }
    }
    fn finish(&mut self, result: bool) {
        self.result = result;
        self.phase = MatchPhase::Done;
    }
    pub fn tick(
        &mut self,
        source: &'src [u8],
        tokens: &[XmlToken],
        selector_visits: &mut usize,
        hit_limit: &mut bool,
    ) -> Result<Option<bool>, CssError> {
        match self.phase {
            MatchPhase::Begin => {
                if self.selector.count == 0 {
                    self.finish(false);
                } else {
                    self.term_index = self.selector.count - 1;
                    self.term = Some(TermMatcher::new(
                        self.selector.terms[self.term_index],
                        self.descendant,
                    ));
                    self.phase = MatchPhase::Term;
                }
            }
            MatchPhase::Term => {
                if let Some(matches) = self
                    .term
                    .as_mut()
                    .expect("term matching")
                    .tick(source, tokens)
                {
                    if matches {
                        self.descendant = self.candidate;
                        if self.term_index == 0 {
                            self.finish(true);
                        } else {
                            self.term_index -= 1;
                            self.cursor = self.descendant;
                            self.rightmost = false;
                            self.phase = MatchPhase::Ancestor;
                        }
                    } else if self.rightmost {
                        self.finish(false);
                    } else {
                        self.phase = MatchPhase::Ancestor;
                    }
                }
            }
            MatchPhase::Ancestor => {
                if self.cursor == 0 {
                    self.finish(false);
                } else {
                    self.cursor -= 1;
                    let token = &tokens[self.cursor];
                    if is_element(token) && token.depth < tokens[self.descendant].depth {
                        if *selector_visits >= MAX_RULE_MATCHES {
                            *hit_limit = true;
                            return Err(CssError::Limit);
                        }
                        *selector_visits += 1;
                        self.candidate = self.cursor;
                        self.term = Some(TermMatcher::new(
                            self.selector.terms[self.term_index],
                            self.candidate,
                        ));
                        self.phase = MatchPhase::Term;
                    }
                }
            }
            MatchPhase::Done => return Ok(Some(self.result)),
        }
        Ok(matches!(self.phase, MatchPhase::Done).then_some(self.result))
    }
}
