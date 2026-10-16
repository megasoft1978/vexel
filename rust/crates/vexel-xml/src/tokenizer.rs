use crate::{is_name_char, is_name_start, is_xml_char, is_xml_whitespace, utf8_next};

pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LARGE_INPUT_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_DEPTH: usize = 256;
pub const MAX_NODES: usize = 1_000_000;
pub const MAX_ATTRIBUTES_PER_ELEMENT: usize = 4096;
pub const MAX_TOTAL_ATTRIBUTES: usize = 1_000_000;
pub const MAX_TEXT_RUN_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TOKEN_NAME_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenKind { Declaration = 1, ProcessingInstruction, Comment, StartElement, EndElement,
    EmptyElement, Attribute, Text, Cdata, Doctype }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum XmlError { None, InvalidArgument, InputLimit, InvalidUtf8, InvalidCharacter,
    UnsupportedEncoding, InvalidName, MalformedSyntax, DuplicateAttribute, MismatchedTag,
    DepthLimit, NodeLimit, AttributeLimit, TextLimit, OutputExhausted, UnsupportedDtd,
    MultipleRoots, IncompleteDocument }

impl XmlError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none", Self::InvalidArgument => "invalid_argument", Self::InputLimit => "input_limit",
            Self::InvalidUtf8 => "invalid_utf8", Self::InvalidCharacter => "invalid_character",
            Self::UnsupportedEncoding => "unsupported_encoding", Self::InvalidName => "invalid_name",
            Self::MalformedSyntax => "malformed_syntax", Self::DuplicateAttribute => "duplicate_attribute",
            Self::MismatchedTag => "mismatched_tag", Self::DepthLimit => "depth_limit", Self::NodeLimit => "node_limit",
            Self::AttributeLimit => "attribute_limit", Self::TextLimit => "text_limit", Self::OutputExhausted => "output_exhausted",
            Self::UnsupportedDtd => "unsupported_dtd", Self::MultipleRoots => "multiple_roots", Self::IncompleteDocument => "incomplete_document",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XmlSpan { pub offset: usize, pub length: usize }
impl XmlSpan {
    pub fn end(self) -> Option<usize> { self.offset.checked_add(self.length) }
    pub fn slice<'a>(self, source: &'a [u8]) -> Option<&'a [u8]> { source.get(self.offset..self.end()?) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XmlToken { pub kind: TokenKind, pub source: XmlSpan, pub name: XmlSpan, pub value: XmlSpan,
    pub prefix: XmlSpan, pub local_name: XmlSpan, pub depth: usize }
impl Default for TokenKind { fn default() -> Self { Self::Declaration } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlLimits { pub input_bytes: usize, pub nesting_depth: usize, pub nodes: usize,
    pub attributes_per_element: usize, pub total_attributes: usize, pub text_run_bytes: usize }
impl Default for XmlLimits { fn default() -> Self { Self { input_bytes: MAX_INPUT_BYTES, nesting_depth: MAX_DEPTH,
    nodes: MAX_NODES, attributes_per_element: MAX_ATTRIBUTES_PER_ELEMENT, total_attributes: MAX_TOTAL_ATTRIBUTES,
    text_run_bytes: MAX_TEXT_RUN_BYTES } } }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XmlDiagnostic { pub category: XmlError, pub offset: usize }
impl Default for XmlError { fn default() -> Self { Self::None } }
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XmlResult { pub token_count: usize, pub element_count: usize, pub attribute_count: usize,
    pub node_count: usize, pub diagnostic: XmlDiagnostic }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fault(pub(crate) XmlError, pub(crate) usize);

struct Parser<'a> { src: &'a [u8], pos: usize, depth: usize, roots: usize, attrs: usize,
    limits: XmlLimits, tokens: Vec<XmlToken>, result: XmlResult, open_names: Vec<XmlSpan>, token_capacity: usize }
impl<'a> Parser<'a> {
    fn fault<T>(&self, e: XmlError, at: usize) -> Result<T, Fault> { Err(Fault(e, at.min(self.src.len()))) }
    fn emit(&mut self, token: XmlToken, at: usize) -> Result<(), Fault> {
        if self.tokens.len() >= self.token_capacity { return self.fault(XmlError::OutputExhausted, at); }
        if self.tokens.try_reserve(1).is_err() { return self.fault(XmlError::OutputExhausted, at); }
        self.tokens.push(token); self.result.token_count += 1; Ok(())
    }
    fn node(&mut self, at: usize) -> Result<(), Fault> {
        let n = self.result.node_count.checked_add(1).ok_or(Fault(XmlError::NodeLimit, at))?;
        if n > self.limits.nodes { return self.fault(XmlError::NodeLimit, at); } self.result.node_count = n; Ok(())
    }
    fn starts(&self, b: usize, lit: &[u8]) -> bool { self.src.get(b..b.saturating_add(lit.len())) == Some(lit) }
    fn ws(&self, p: &mut usize) { while self.src.get(*p).is_some_and(|b| is_xml_whitespace(*b)) { *p += 1; } }
    fn name(&self, p: &mut usize, qname: bool) -> Result<(XmlSpan, XmlSpan, XmlSpan), Fault> {
        let start = *p; let Some((cp, mut q)) = utf8_next(self.src, *p) else { return self.fault(XmlError::InvalidName, start) };
        if !is_name_start(cp) { return self.fault(XmlError::InvalidName, start); }
        let mut colon = None;
        while q < self.src.len() {
            let before = q; let Some((cp, next)) = utf8_next(self.src, q) else { return self.fault(XmlError::InvalidUtf8, q) };
            if !is_name_char(cp) { break; }
            if cp == 0x3a { if qname && colon.is_some() { return self.fault(XmlError::InvalidName, colon.unwrap_or(before)); } if colon.is_none() { colon = Some(before); } }
            q = next;
        }
        if q - start > MAX_TOKEN_NAME_BYTES { return self.fault(XmlError::MalformedSyntax, start); }
        if qname && (self.src.get(start) == Some(&b':') || self.src.get(q.saturating_sub(1)) == Some(&b':')) {
            return self.fault(XmlError::InvalidName, start);
        }
        let name = XmlSpan { offset: start, length: q - start };
        let (prefix, local) = match colon {
            Some(c) => (XmlSpan { offset: start, length: c - start }, XmlSpan { offset: c + 1, length: q - c - 1 }),
            None => (XmlSpan { offset: start, length: 0 }, name),
        };
        if qname && local.length > 0 {
            let Some((local_first, _)) = utf8_next(self.src, local.offset) else { return self.fault(XmlError::InvalidUtf8, local.offset) };
            if !is_name_start(local_first) { return self.fault(XmlError::InvalidName, local.offset); }
        }
        *p = q; Ok((name, prefix, local))
    }
    fn reference(&self, p: &mut usize) -> Result<(), Fault> {
        let at = *p; let mut q = at + 1;
        if q >= self.src.len() { return self.fault(XmlError::MalformedSyntax, at); }
        if self.src[q] == b'#' {
            q += 1; let hex = self.src.get(q) == Some(&b'x'); if hex { q += 1; }
            let begin = q; let mut n: u32 = 0;
            while let Some(&c) = self.src.get(q) {
                let digit = match c { b'0'..=b'9' => Some((c - b'0') as u32), b'a'..=b'f' if hex => Some((c-b'a'+10) as u32), b'A'..=b'F' if hex => Some((c-b'A'+10) as u32), _ => None };
                let Some(d) = digit else { break };
                n = n.checked_mul(if hex {16} else {10}).and_then(|v| v.checked_add(d)).filter(|v| *v <= 0x10ffff)
                    .ok_or(Fault(XmlError::MalformedSyntax, at))?; q += 1;
            }
            if q == begin || self.src.get(q) != Some(&b';') { return self.fault(XmlError::MalformedSyntax, at); }
            if !is_xml_char(n) { return self.fault(XmlError::InvalidCharacter, at); }
        } else {
            let begin = q; let (_, _, _) = self.name(&mut q, false).map_err(|_| Fault(XmlError::MalformedSyntax, at))?;
            if self.src.get(q) != Some(&b';') || !matches!(&self.src[begin..q], b"lt"|b"gt"|b"amp"|b"apos"|b"quot") { return self.fault(XmlError::MalformedSyntax, at); }
        }
        *p = q + 1; Ok(())
    }
    fn tag(&mut self, closing: bool) -> Result<(), Fault> {
        let start = self.pos; let mut p = start + if closing { 2 } else { 1 };
        let (name, prefix, local) = self.name(&mut p, true)?;
        if closing {
            self.ws(&mut p); if self.src.get(p) != Some(&b'>') { return self.fault(XmlError::MalformedSyntax, p); }
            if self.depth == 0 { return self.fault(XmlError::MismatchedTag, start); }
            // Stack is maintained separately by matching starts in depth order.
            let expected = self.open_names.last().copied().unwrap_or_default();
            if expected.slice(self.src) != name.slice(self.src) { return self.fault(XmlError::MismatchedTag, start); }
            self.emit(XmlToken { kind: TokenKind::EndElement, source: XmlSpan { offset: start, length: p + 1 - start }, name, prefix, local_name: local, depth: self.depth - 1, ..XmlToken::default() }, start)?;
            self.depth -= 1; self.open_names.pop(); self.pos = p + 1; return Ok(());
        }
        if self.depth == 0 { if self.roots != 0 { return self.fault(XmlError::MultipleRoots, start); } self.roots += 1; }
        if self.depth >= self.limits.nesting_depth || self.depth >= MAX_DEPTH { return self.fault(XmlError::DepthLimit, start); }
        self.node(start)?;
        let token_index = self.tokens.len(); self.emit(XmlToken { kind: TokenKind::StartElement, source: XmlSpan { offset: start, length: p - start }, name, prefix, local_name: local, depth: self.depth, ..XmlToken::default() }, start)?;
        let mut local_attrs = 0;
        loop {
            let before = p; self.ws(&mut p); let separated = p != before;
            match self.src.get(p).copied() {
                Some(b'>') => { self.tokens[token_index].source.length = p + 1 - start; if self.open_names.try_reserve(1).is_err() { return self.fault(XmlError::OutputExhausted, start); } self.open_names.push(name); self.depth += 1; self.pos = p + 1; return Ok(()); }
                Some(b'/') if self.src.get(p + 1) == Some(&b'>') => { self.tokens[token_index].kind = TokenKind::EmptyElement; self.tokens[token_index].source.length = p + 2 - start; self.pos = p + 2; return Ok(()); }
                None => return self.fault(XmlError::MalformedSyntax, start),
                _ if !separated => return self.fault(XmlError::MalformedSyntax, p),
                _ => {}
            }
            if local_attrs >= self.limits.attributes_per_element || self.attrs >= self.limits.total_attributes { return self.fault(XmlError::AttributeLimit, p); }
            let attr_start = p; let (an, ap, al) = self.name(&mut p, true)?;
            if self.tokens[token_index + 1..].iter().any(|t| t.kind == TokenKind::Attribute && an.slice(self.src) == t.name.slice(self.src)) { return self.fault(XmlError::DuplicateAttribute, attr_start); }
            self.ws(&mut p); if self.src.get(p) != Some(&b'=') { return self.fault(XmlError::MalformedSyntax, p); } p += 1; self.ws(&mut p);
            let Some(&quote @ (b'\'' | b'"')) = self.src.get(p) else { return self.fault(XmlError::MalformedSyntax, p) }; p += 1; let value_start = p;
            while p < self.src.len() && self.src[p] != quote {
                if self.src[p] == b'<' { return self.fault(XmlError::MalformedSyntax, p); }
                if self.src[p] == b'&' { self.reference(&mut p)?; } else { p += 1; }
            }
            if p == self.src.len() { return self.fault(XmlError::MalformedSyntax, value_start - 1); }
            self.emit(XmlToken { kind: TokenKind::Attribute, source: XmlSpan { offset: attr_start, length: p + 1 - attr_start }, name: an, prefix: ap, local_name: al, value: XmlSpan { offset: value_start, length: p - value_start }, depth: self.depth, ..XmlToken::default() }, attr_start)?;
            p += 1; local_attrs += 1; self.attrs += 1; self.result.attribute_count += 1;
        }
    }
    fn open_tag(&mut self) -> Result<(), Fault> { self.tag(false)?; self.result.element_count += 1; Ok(()) }
    fn doctype_end(&self, start: usize) -> Result<usize, Fault> {
        #[derive(Clone, Copy, PartialEq)] enum S { Start, Main, Quote, Sub, SubQuote, Lt, Bang, Dash, Comment, Pi }
        let mut s=S::Start; let mut q=0; let mut run=0; let mut closed=false; let mut i=start+9;
        while i < self.src.len() { let c=self.src[i]; match s {
            S::Start => { if !is_xml_whitespace(c) { return self.fault(XmlError::MalformedSyntax,i) } s=S::Main; },
            S::Main => match c { b'"'|b'\''=>{q=c;s=S::Quote}, b'['=>{if closed{return self.fault(XmlError::MalformedSyntax,i)}s=S::Sub}, b'>'=>return Ok(i+1), b'<'=>return self.fault(XmlError::MalformedSyntax,i), _=>{} },
            S::Quote=>if c==q{s=S::Main}, S::Sub=>match c { b'"'|b'\''=>{q=c;s=S::SubQuote},b'<'=>s=S::Lt,b']'=>{closed=true;s=S::Main},_=>{} },
            S::SubQuote=>if c==q{s=S::Sub}, S::Lt=>match c {b'!'=>s=S::Bang,b'?'=>{run=0;s=S::Pi},_=>s=S::Sub},
            S::Bang=>if c==b'-'{s=S::Dash}else{s=S::Sub}, S::Dash=>if c==b'-'{run=0;s=S::Comment}else{s=S::Sub},
            S::Comment=>if c==b'-'{run=(run+1).min(2)}else if c==b'>'&&run>=2{s=S::Sub}else{run=0},
            S::Pi=>if c==b'>'&&run==1{s=S::Sub}else{run=usize::from(c==b'?')},
        } i+=1; }
        self.fault(XmlError::MalformedSyntax,start)
    }
    fn run(mut self) -> Result<(Vec<XmlToken>, XmlResult), Fault> {
        // Validate every source scalar before parsing, preserving C tokenizer error precedence.
        let mut p=0; while p < self.src.len() { let at=p; let Some((cp,n))=utf8_next(self.src,p) else{return self.fault(XmlError::InvalidUtf8,at)}; if !is_xml_char(cp){return self.fault(XmlError::InvalidCharacter,at)} p=n; }
        if self.starts(0,b"\xef\xbb\xbf") { self.pos=3; }
        let mut doctype=false;
        while self.pos < self.src.len() {
            let start=self.pos;
            if self.src[start] != b'<' {
                let mut p=start; while p<self.src.len() && self.src[p]!=b'<' { if self.starts(p,b"]]>" ) { return self.fault(XmlError::MalformedSyntax,p) } if self.src[p]==b'&' { self.reference(&mut p)?; } else { p+=1; } }
                if p-start > self.limits.text_run_bytes { return self.fault(XmlError::TextLimit,start); }
                let nonspace=self.src[start..p].iter().any(|b| !is_xml_whitespace(*b)); if self.depth==0 && nonspace { let i=self.src[start..p].iter().position(|b| !is_xml_whitespace(*b)).unwrap()+start; return self.fault(XmlError::MalformedSyntax,i); }
                let counted=self.depth!=0||nonspace; if counted { self.node(start)?; }
                self.emit(XmlToken {kind:TokenKind::Text,source:XmlSpan{offset:start,length:p-start},value:XmlSpan{offset:start,length:p-start},depth:self.depth,..XmlToken::default()},start)?; self.pos=p; continue;
            }
            if self.starts(start,b"<!--") { let mut p=start+4; while p+2<self.src.len() && !self.starts(p,b"-->") { if self.starts(p,b"--") { return self.fault(XmlError::MalformedSyntax,p) } p+=1; } if p+2>=self.src.len(){return self.fault(XmlError::MalformedSyntax,start)} self.node(start)?; self.emit(XmlToken{kind:TokenKind::Comment,source:XmlSpan{offset:start,length:p+3-start},value:XmlSpan{offset:start+4,length:p-start-4},depth:self.depth,..XmlToken::default()},start)?;self.pos=p+3;continue; }
            if self.starts(start,b"<![CDATA[") { let mut p=start+9;while p+2<self.src.len()&&!self.starts(p,b"]]>"){p+=1} if p+2>=self.src.len()||self.depth==0{return self.fault(XmlError::MalformedSyntax,start)} if p-(start+9)>self.limits.text_run_bytes{return self.fault(XmlError::TextLimit,start+9)} self.node(start)?;self.emit(XmlToken{kind:TokenKind::Cdata,source:XmlSpan{offset:start,length:p+3-start},value:XmlSpan{offset:start+9,length:p-start-9},depth:self.depth,..XmlToken::default()},start)?;self.pos=p+3;continue; }
            if self.starts(start,b"<!DOCTYPE") { if doctype||self.depth!=0||self.roots!=0{return self.fault(XmlError::MalformedSyntax,start)} let end=self.doctype_end(start)?; self.node(start)?; self.emit(XmlToken{kind:TokenKind::Doctype,source:XmlSpan{offset:start,length:end-start},..XmlToken::default()},start)?;doctype=true;self.pos=end;continue; }
            if self.starts(start,b"<!ENTITY") {return self.fault(XmlError::UnsupportedDtd,start)}
            if self.starts(start,b"<?") { let mut p=start+2; let (name,_,_)=self.name(&mut p,false)?;let declaration=name.slice(self.src)==Some(b"xml");let folded=name.slice(self.src).unwrap_or_default();if !declaration&&folded.eq_ignore_ascii_case(b"xml"){return self.fault(XmlError::MalformedSyntax,start)}if declaration&&start!=0&&!(start==3&&self.starts(0,b"\xef\xbb\xbf")){return self.fault(XmlError::MalformedSyntax,start)} if p>=self.src.len()||(!is_xml_whitespace(self.src[p])&&!self.starts(p,b"?>")){return self.fault(XmlError::MalformedSyntax,p)} if declaration&&self.starts(p,b"?>"){return self.fault(XmlError::MalformedSyntax,p)} while p<self.src.len()&&is_xml_whitespace(self.src[p]){p+=1}let data=p;while p+1<self.src.len()&&!self.starts(p,b"?>"){p+=1}if p+1>=self.src.len(){return self.fault(XmlError::MalformedSyntax,start)} let kind=if declaration{TokenKind::Declaration}else{self.node(start)?;TokenKind::ProcessingInstruction};if declaration{validate_declaration(self.src,XmlSpan{offset:data,length:p-data})?;}self.emit(XmlToken{kind,source:XmlSpan{offset:start,length:p+2-start},name,value:XmlSpan{offset:data,length:p-data},depth:self.depth,..XmlToken::default()},start)?;self.pos=p+2;continue; }
            if self.starts(start,b"</") {self.tag(true)?;continue}
            if self.starts(start,b"<!") {return self.fault(XmlError::MalformedSyntax,start)}
            self.open_tag()?;
        }
        if self.depth!=0||self.roots==0{return self.fault(XmlError::IncompleteDocument,self.src.len())}
        Ok((self.tokens,self.result))
    }
}

pub(crate) fn validate_declaration(src: &[u8], span: XmlSpan) -> Result<(), Fault> {
    let end=span.end().unwrap_or(src.len());let mut p=span.offset;let(mut version,mut encoding,mut standalone)=(false,false,false);
    while p<end { while p<end&&is_xml_whitespace(src[p]){p+=1} if p==end{break} let ns=p;while p<end&&src[p].is_ascii_alphabetic(){p+=1}let key=&src[ns..p];while p<end&&is_xml_whitespace(src[p]){p+=1}if p>=end||src[p]!=b'='{return Err(Fault(XmlError::MalformedSyntax,p))}p+=1;while p<end&&is_xml_whitespace(src[p]){p+=1}if p>=end||!matches!(src[p],b'\''|b'"'){return Err(Fault(XmlError::MalformedSyntax,p))}let q=src[p];p+=1;let vs=p;while p<end&&src[p]!=q{p+=1}if p>=end{return Err(Fault(XmlError::MalformedSyntax,vs-1))}let value=&src[vs..p];p+=1;if p<end&&!is_xml_whitespace(src[p]){return Err(Fault(XmlError::MalformedSyntax,p))}
        match key { b"version" if !version&&!encoding&&!standalone&&value==b"1.0"=>version=true,
            b"encoding" if version&&!encoding&&!standalone&&!value.is_empty()&&value[0].is_ascii_alphabetic()=>{if !value.iter().all(|c|c.is_ascii_alphanumeric()||matches!(c,b'.'|b'_'|b'-')){return Err(Fault(XmlError::MalformedSyntax,vs))}if !value.eq_ignore_ascii_case(b"utf-8"){return Err(Fault(XmlError::UnsupportedEncoding,vs))}encoding=true},
            b"standalone" if version&&!standalone&&matches!(value,b"yes"|b"no")=>standalone=true,
            _=>return Err(Fault(XmlError::MalformedSyntax,ns)) }
    }
    if !version{return Err(Fault(XmlError::MalformedSyntax,span.offset))}Ok(())
}

pub fn tokenize(source: &[u8], mut limits: XmlLimits) -> (Vec<XmlToken>, XmlResult) { tokenize_impl(source, &mut limits, MAX_INPUT_BYTES, usize::MAX) }
pub fn tokenize_large(source: &[u8]) -> (Vec<XmlToken>, XmlResult) { tokenize_impl(source, &mut XmlLimits { input_bytes: MAX_LARGE_INPUT_BYTES, ..XmlLimits::default() }, MAX_LARGE_INPUT_BYTES, usize::MAX) }
pub fn tokenize_with_capacity(source: &[u8], limits: XmlLimits, token_capacity: usize) -> (Vec<XmlToken>, XmlResult) { tokenize_impl(source, &mut limits.clone(), MAX_INPUT_BYTES, token_capacity) }
pub fn tokenize_large_with_capacity(source: &[u8], token_capacity: usize) -> (Vec<XmlToken>, XmlResult) {
    tokenize_impl(source, &mut XmlLimits { input_bytes: MAX_LARGE_INPUT_BYTES, ..XmlLimits::default() }, MAX_LARGE_INPUT_BYTES, token_capacity)
}
pub fn tokenize_cancelable<F: FnMut() -> bool>(source: &[u8], limits: XmlLimits, token_capacity: usize, canceled: F) -> (Vec<XmlToken>, XmlResult, StreamStatus) {
    tokenize_cancelable_impl(source, limits, MAX_INPUT_BYTES, token_capacity, canceled)
}
pub fn tokenize_large_cancelable<F: FnMut() -> bool>(source: &[u8], token_capacity: usize, canceled: F) -> (Vec<XmlToken>, XmlResult, StreamStatus) {
    tokenize_cancelable_impl(source, XmlLimits { input_bytes: MAX_LARGE_INPUT_BYTES, ..XmlLimits::default() }, MAX_LARGE_INPUT_BYTES, token_capacity, canceled)
}
fn normalize_limits(mut limits: XmlLimits, hard: usize) -> XmlLimits {
    limits.input_bytes=limits.input_bytes.min(hard);limits.nesting_depth=limits.nesting_depth.min(MAX_DEPTH);
    limits.nodes=limits.nodes.min(MAX_NODES);limits.attributes_per_element=limits.attributes_per_element.min(MAX_ATTRIBUTES_PER_ELEMENT);
    limits.total_attributes=limits.total_attributes.min(MAX_TOTAL_ATTRIBUTES);limits.text_run_bytes=limits.text_run_bytes.min(MAX_TEXT_RUN_BYTES);
    limits
}
fn tokenize_cancelable_impl<F: FnMut() -> bool>(source: &[u8], limits: XmlLimits, hard: usize, token_capacity: usize, mut canceled: F) -> (Vec<XmlToken>, XmlResult, StreamStatus) {
    if canceled() { return (Vec::new(), XmlResult::default(), StreamStatus::Canceled); }
    let limits=normalize_limits(limits,hard);
    if source.len()>limits.input_bytes {
        return (Vec::new(),XmlResult{diagnostic:XmlDiagnostic{category:XmlError::InputLimit,offset:limits.input_bytes},..XmlResult::default()},StreamStatus::Failed);
    }
    let mut stream=match crate::XmlTokenStream::with_limits(source,limits,token_capacity){
        Ok(stream)=>stream,
        Err(_)=>return (Vec::new(),XmlResult{diagnostic:XmlDiagnostic{category:XmlError::OutputExhausted,offset:0},..XmlResult::default()},StreamStatus::Failed),
    };
    loop {
        match stream.step_with_cancel(64*1024,&mut canceled){
            StreamStatus::Ok=>{},
            StreamStatus::Complete=>{let result=stream.result();return (stream.take().unwrap_or_default(),result,StreamStatus::Complete)},
            status=>return (Vec::new(),stream.result(),status),
        }
    }
}

fn tokenize_impl(source:&[u8],limits:&mut XmlLimits,hard:usize,token_capacity:usize)->(Vec<XmlToken>,XmlResult){
    *limits=normalize_limits(*limits,hard);
    if source.len()>limits.input_bytes { return (Vec::new(),XmlResult{diagnostic:XmlDiagnostic{category:XmlError::InputLimit,offset:limits.input_bytes},..XmlResult::default()}); }
    let parser=Parser{src:source,pos:0,depth:0,roots:0,attrs:0,limits:*limits,tokens:Vec::new(),result:XmlResult::default(),open_names:Vec::new(),token_capacity};
    match parser.run(){Ok((tokens,result))=>(tokens,result),Err(Fault(category,offset))=>(Vec::new(),XmlResult{diagnostic:XmlDiagnostic{category,offset},..XmlResult::default()})}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatus { Ok, Complete, Failed, Canceled }

pub fn visit_tokens<F:FnMut(&XmlToken)->bool>(source:&[u8],limits:XmlLimits,visitor:&mut F)->StreamStatus{
    let (tokens,result)=tokenize(source,limits);if result.diagnostic.category!=XmlError::None{return StreamStatus::Failed}
    for token in &tokens {if !visitor(token){return StreamStatus::Canceled}}StreamStatus::Complete
}

pub fn decode_attribute_value(source:&[u8],raw:XmlSpan,output:&mut [u8])->Result<usize,usize>{
    let bytes=raw.slice(source).ok_or(0usize)?;let mut p=0;let mut out=0;
    while p<bytes.len(){let at=p;if bytes[p]!=b'&'{let Some((cp,end))=utf8_next(bytes,p)else{return Err(at)};if !is_xml_char(cp)||end-p>output.len().saturating_sub(out){return Err(at)}output[out..out+end-p].copy_from_slice(&bytes[p..end]);out+=end-p;p=end;continue}
        let semi=bytes[p+1..].iter().take(16).position(|b|*b==b';').map(|v|p+1+v).ok_or(at)?;let body=&bytes[p+1..semi];let cp=match body{b"amp"=>38,b"lt"=>60,b"gt"=>62,b"quot"=>34,b"apos"=>39,_ if body.starts_with(b"#")=>{let (digits,radix)=if body.get(1)==Some(&b'x'){(&body[2..],16)}else{(&body[1..],10)};if digits.is_empty(){return Err(at)}let mut n=0u32;for c in digits{let d=match c{b'0'..=b'9'=>(c-b'0')as u32,b'a'..=b'f'=>(c-b'a'+10)as u32,b'A'..=b'F'=>(c-b'A'+10)as u32,_=>return Err(at)};n=n.checked_mul(radix).and_then(|v|v.checked_add(d)).ok_or(at)?}n},_=>return Err(at)};if !is_xml_char(cp){return Err(at)}let ch=char::from_u32(cp).ok_or(at)?;let mut tmp=[0;4];let enc=ch.encode_utf8(&mut tmp).as_bytes();if enc.len()>output.len().saturating_sub(out){return Err(at)}output[out..out+enc.len()].copy_from_slice(enc);out+=enc.len();p=semi+1;
    }Ok(out)
}
