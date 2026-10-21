use crate::{is_name_char, is_name_start, is_xml_char, is_xml_whitespace};
use crate::tokenizer::{validate_declaration, MAX_ATTRIBUTES_PER_ELEMENT, MAX_DEPTH, MAX_LARGE_INPUT_BYTES,
    MAX_NODES, MAX_TOKEN_NAME_BYTES};
use crate::{StreamStatus, TokenKind, XmlDiagnostic, XmlError, XmlLimits, XmlResult, XmlSpan, XmlToken};
use core::mem::size_of;
use crate::utf8_scan::Utf8Scan;
use crate::EntityTable;

const STEP_CAP: usize = 64 * 1024;

#[derive(Clone, Copy)]
struct NameScan {
    start: usize, pos: usize, first_colon: Option<usize>, utf8: Utf8Scan,
    leading_colon: bool, trailing_colon: bool, local_first_valid: Option<bool>, classification: NameClass,
}
impl NameScan {
    fn new(start: usize) -> Self {
        Self { start, pos: start, first_colon: None, utf8: Utf8Scan::default(),
            leading_colon: false, trailing_colon: false, local_first_valid: None, classification: NameClass::new() }
    }
}

/// Every variable token-name source load, including comparison operands, uses
/// this budget gate. The legacy `reads` counter measures scanner loads only;
/// `total_reads` includes scanner and comparison loads without cursor inference.
#[derive(Default)]
struct NameWork {
    remaining: usize, spent: usize,
    #[cfg(test)] reads: usize,
    #[cfg(test)] decoder_inputs: usize,
    #[cfg(test)] comparison_reads: usize,
    #[cfg(test)] total_reads: usize,
}
impl NameWork {
    fn begin(&mut self, budget: usize) { self.remaining=budget;self.spent=0; }
    fn load(&mut self, source: &[u8], at: usize, comparison: bool) -> Option<u8> {
        if self.remaining==0{return None}
        self.remaining-=1;self.spent+=1;
        let byte=source[at];
        #[cfg(test)] {
            self.total_reads+=1;
            if comparison{self.comparison_reads+=1}else{self.reads+=1}
        }
        #[cfg(not(test))] let _=comparison;
        Some(byte)
    }
    fn read(&mut self, source: &[u8], at: usize) -> Option<u8> { self.load(source,at,false) }
    fn decode(&mut self, utf8: &mut Utf8Scan, byte: u8, at: usize) -> Result<Option<u32>, usize> {
        #[cfg(test)] { self.decoder_inputs += 1; }
        utf8.push(byte, at)
    }
}

const ENTITIES: [&[u8]; 5]=[b"lt",b"gt",b"amp",b"apos",b"quot"];
#[derive(Clone, Copy)]
struct NameClass { xml_exact: bool, xml_folded: bool, entities: u8 }
impl NameClass {
    fn new()->Self{Self{xml_exact:true,xml_folded:true,entities:31}}
    fn push(&mut self,cp:u32,offset:usize){
        self.xml_exact&=b"xml".get(offset).is_some_and(|v|u32::from(*v)==cp);
        self.xml_folded&=cp<128&&b"xml".get(offset).is_some_and(|v|*v==(cp as u8).to_ascii_lowercase());
        for (i,name) in ENTITIES.iter().enumerate(){
            if !name.get(offset).is_some_and(|v|u32::from(*v)==cp){self.entities&=!(1<<i)}
        }
    }
    fn known_entity(self,len:usize)->bool{ENTITIES.iter().enumerate().any(|(i,n)|self.entities&(1<<i)!=0&&n.len()==len)}
}

#[derive(Clone, Copy)]
struct NameCompare { left:XmlSpan, right:XmlSpan, index:usize, left_byte:Option<u8> }
#[derive(PartialEq, Eq)]
enum CompareStatus { Pending, Equal, Different, Canceled }
impl NameCompare {
    fn new(left:XmlSpan,right:XmlSpan)->Self{Self{left,right,index:0,left_byte:None}}
    fn step<C:FnMut()->bool>(&mut self,source:&[u8],work:&mut NameWork,cancel:&mut C)->CompareStatus{
        if self.left.length!=self.right.length{return CompareStatus::Different}
        while self.index<self.left.length{
            if work.remaining==0{return CompareStatus::Pending}
            if cancel(){return CompareStatus::Canceled}
            if self.left_byte.is_none(){
                self.left_byte=work.load(source,self.left.offset+self.index,true);
                continue
            }
            let Some(right)=work.load(source,self.right.offset+self.index,true)else{return CompareStatus::Pending};
            if self.left_byte.take()!=Some(right){return CompareStatus::Different}
            self.index+=1;
        }
        CompareStatus::Equal
    }
}
#[derive(Clone, Copy)]
enum RefMode { Start, Numeric { hex: bool, digits: usize, value: u32 }, Named(NameScan) }
#[derive(Clone, Copy)]
struct RefScan { start: usize, pos: usize, mode: RefMode }
#[derive(Clone, Copy, Default)]
struct NameParts { name: XmlSpan, prefix: XmlSpan, local: XmlSpan, xml_exact:bool, xml_folded:bool, entity_known:bool }

#[derive(Clone, Copy, PartialEq, Eq)]
enum DtdPhase { Start, Main, Quote, Subset, SubsetQuote, Lt, Bang, BangDash, Comment, Pi }
#[derive(Clone, Copy)]
struct Dtd { phase: DtdPhase, quote: u8, run: u8, subset_closed: bool }
impl Default for Dtd { fn default() -> Self { Self { phase: DtdPhase::Start, quote: 0, run: 0, subset_closed: false } } }
impl Dtd {
    fn step(&mut self, c: u8) -> Result<bool, ()> {
        loop { match self.phase {
            DtdPhase::Start => { if !is_xml_whitespace(c) { return Err(()) } self.phase=DtdPhase::Main; return Ok(false) },
            DtdPhase::Main => match c { b'"'|b'\''=>{self.quote=c;self.phase=DtdPhase::Quote},b'['=>{if self.subset_closed{return Err(())}self.phase=DtdPhase::Subset},b'>'=>return Ok(true),b'<'=>return Err(()),_=>{} },
            DtdPhase::Quote=>if c==self.quote{self.phase=DtdPhase::Main},
            DtdPhase::Subset=>match c {b'"'|b'\''=>{self.quote=c;self.phase=DtdPhase::SubsetQuote},b'<'=>self.phase=DtdPhase::Lt,b']'=>{self.subset_closed=true;self.phase=DtdPhase::Main},_=>{}},
            DtdPhase::SubsetQuote=>if c==self.quote{self.phase=DtdPhase::Subset},
            DtdPhase::Lt=>match c{b'!'=>{self.phase=DtdPhase::Bang;return Ok(false)},b'?'=>{self.run=0;self.phase=DtdPhase::Pi;return Ok(false)},_=>{self.phase=DtdPhase::Subset;continue}},
            DtdPhase::Bang=>if c==b'-'{self.phase=DtdPhase::BangDash;return Ok(false)}else{self.phase=DtdPhase::Subset;continue},
            DtdPhase::BangDash=>if c==b'-'{self.run=0;self.phase=DtdPhase::Comment;return Ok(false)}else{self.phase=DtdPhase::Subset;continue},
            DtdPhase::Comment=>if c==b'-'{self.run=(self.run+1).min(2)}else if c==b'>'&&self.run>=2{self.phase=DtdPhase::Subset}else{self.run=0},
            DtdPhase::Pi=>if c==b'>'&&self.run==1{self.phase=DtdPhase::Subset}else{self.run=u8::from(c==b'?')},
        } return Ok(false) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TagPhase { Prefix, Name, EndMatch, Between, EmptyClose, AttrName, AttrMatch, AttrEq, AttrQuote, AttrValue, EndBetween }
struct TagState { start:usize, pos:usize, closing:bool, phase:TagPhase, name_scan:Option<NameScan>, name:Option<NameParts>,
    attr_scan:Option<NameScan>, attr:Option<NameParts>, attr_start:usize, value_start:usize, quote:u8,
    local_attrs:usize, separated:bool, token_index:usize, reference:Option<RefScan>, comparison:Option<NameCompare>, duplicate_index:usize }
impl TagState { fn new(start:usize,closing:bool)->Self {Self{start,pos:start,closing,phase:TagPhase::Prefix,name_scan:None,name:None,attr_scan:None,attr:None,attr_start:0,value_start:0,quote:0,local_attrs:0,separated:false,token_index:0,reference:None,comparison:None,duplicate_index:0}} }

enum Pending {
    None,
    Text { start:usize, pos:usize, nonspace:bool, first_nonspace:usize, reference:Option<RefScan> },
    Comment { start:usize, pos:usize, dashes:u8 },
    Cdata { start:usize, pos:usize, brackets:u8 },
    Doctype { start:usize, pos:usize, state:Dtd },
    Pi { start:usize, pos:usize, name_scan:Option<NameScan>, name:Option<XmlSpan>, declaration:bool, data:Option<usize>, separating:bool, question:bool },
}

pub struct XmlTokenStream<'a> {
    source:&'a [u8], limits:XmlLimits, result:XmlResult, tokens:Vec<XmlToken>,
    tag_scratch:Vec<XmlToken>, token_capacity:usize, visitor_mode:bool,
    cursor:usize, parse_limit:usize, char_cursor:usize, utf8:Utf8Scan, name_work:NameWork,
    chars_complete:bool, depth:usize, roots:usize, total_attrs:usize, open_names:Vec<XmlSpan>,
    pending:Pending, tag:Option<TagState>, doctype_seen:bool, doctype_span:XmlSpan, entities:EntityTable, complete:bool, canceled:bool, failed:bool, taken:bool,
}

impl<'a> XmlTokenStream<'a> {
    pub fn new(source:&'a [u8],max_storage_bytes:usize)->Result<Self,StreamStatus>{
        if max_storage_bytes<size_of::<XmlToken>(){return Err(StreamStatus::Failed)}
        Self::construct(source,max_storage_bytes,false)
    }
    pub fn new_visitor(source:&'a [u8])->Result<Self,StreamStatus>{
        let scratch=MAX_ATTRIBUTES_PER_ELEMENT.checked_add(1).and_then(|n|n.checked_mul(size_of::<XmlToken>())).ok_or(StreamStatus::Failed)?;
        Self::construct(source,scratch,true)
    }
    fn construct(source:&'a [u8],max_storage_bytes:usize,visitor_mode:bool)->Result<Self,StreamStatus>{
        if source.len()>MAX_LARGE_INPUT_BYTES{return Err(StreamStatus::Failed)}
        let limits=XmlLimits{input_bytes:MAX_LARGE_INPUT_BYTES,..XmlLimits::default()};
        let mut tag_scratch=Vec::new();
        if visitor_mode {
            let scratch_capacity=MAX_ATTRIBUTES_PER_ELEMENT.checked_add(1).ok_or(StreamStatus::Failed)?;
            if tag_scratch.try_reserve_exact(scratch_capacity).is_err()
                || tag_scratch.capacity()>scratch_capacity || tag_scratch.capacity()<scratch_capacity {
                return Err(StreamStatus::Failed)
            }
        }
        let mut open_names=Vec::new();
        if open_names.try_reserve_exact(MAX_DEPTH).is_err()
            || open_names.capacity()>MAX_DEPTH || open_names.capacity()<MAX_DEPTH {
            return Err(StreamStatus::Failed)
        }
        let cursor=if source.starts_with(b"\xef\xbb\xbf"){3}else{0};
        Ok(Self{source,limits,result:XmlResult::default(),tokens:Vec::new(),tag_scratch,token_capacity:max_storage_bytes/size_of::<XmlToken>(),visitor_mode,
            cursor,parse_limit:cursor,char_cursor:0,utf8:Utf8Scan::default(),name_work:NameWork::default(),chars_complete:false,depth:0,roots:0,total_attrs:0,open_names,
            pending:Pending::None,tag:None,doctype_seen:false,doctype_span:XmlSpan::default(),entities:EntityTable::empty(),complete:false,canceled:false,failed:false,taken:false})
    }

    pub(crate) fn with_limits(source:&'a [u8],limits:XmlLimits,token_capacity:usize)->Result<Self,StreamStatus>{
        let mut stream=Self::construct(source,0,false)?;
        stream.limits=limits;
        stream.token_capacity=token_capacity.min((isize::MAX as usize)/size_of::<XmlToken>());
        Ok(stream)
    }

    pub fn step(&mut self,budget:usize,canceled:bool)->StreamStatus{
        let mut poll=||canceled;
        self.step_inner(budget,&mut poll,None)
    }
    pub fn step_with_visitor<F:FnMut(&XmlToken)->bool>(&mut self,budget:usize,canceled:bool,visitor:&mut F)->StreamStatus{
        let mut poll=||canceled;
        self.step_inner(budget,&mut poll,Some(visitor))
    }
    pub fn step_with_visitor_and_cancel<F:FnMut(&XmlToken)->bool,C:FnMut()->bool>(&mut self,budget:usize,visitor:&mut F,cancel:&mut C)->StreamStatus{
        self.step_inner(budget,cancel,Some(visitor))
    }
    pub fn step_with_cancel<C:FnMut()->bool>(&mut self,budget:usize,cancel:&mut C)->StreamStatus{
        self.step_inner(budget,cancel,None)
    }
    fn step_inner<C:FnMut()->bool>(&mut self,budget:usize,cancel:&mut C,mut visitor:Option<&mut dyn FnMut(&XmlToken)->bool>)->StreamStatus{
        let status=self.advance_step(budget,cancel,&mut visitor);
        if matches!(status,StreamStatus::Failed|StreamStatus::Canceled){
            self.failed|=status==StreamStatus::Failed;
            self.tokens.clear();self.tag_scratch.clear();
            self.result.token_count=0;self.result.element_count=0;
            self.result.attribute_count=0;self.result.node_count=0;
        }
        status
    }
    fn advance_step<C:FnMut()->bool>(&mut self,budget:usize,cancel:&mut C,visitor:&mut Option<&mut dyn FnMut(&XmlToken)->bool>)->StreamStatus{
        if self.failed{return StreamStatus::Failed}
        if self.canceled{return StreamStatus::Canceled} if self.result.diagnostic.category!=XmlError::None{return StreamStatus::Failed}
        if budget==0{return StreamStatus::Failed} if self.complete{return StreamStatus::Complete}
        if self.visitor_mode&&visitor.is_none(){return StreamStatus::Failed}
        if cancel(){self.canceled=true;return StreamStatus::Canceled}
        let budget=budget.min(STEP_CAP);let char_initial=self.char_cursor;
        if !self.scan_chars(budget,cancel){return self.failed_or_canceled()}
        if !self.chars_complete{return StreamStatus::Ok}
        let char_work=self.char_cursor.saturating_sub(char_initial);if char_work>=budget{return StreamStatus::Ok}
        let parse_budget=budget-char_work;self.name_work.begin(parse_budget);let initial=self.frontier();
        let limit=initial.saturating_add(parse_budget).min(self.source.len());self.parse_limit=limit;
        while self.cursor<self.source.len()&&self.frontier().saturating_sub(initial)<parse_budget {
            if cancel(){self.canceled=true;return StreamStatus::Canceled}
            let before=self.frontier();let work_before=self.name_work.spent;
            if !self.parse_next(limit,visitor,cancel){return self.failed_or_canceled()}
            if self.name_work.remaining==0||(self.frontier()==before&&self.name_work.spent==work_before){break}
        }
        if self.cursor==self.source.len()&&matches!(self.pending,Pending::None)&&self.tag.is_none(){
            if self.depth!=0||self.roots==0{self.fail(XmlError::IncompleteDocument,self.source.len());return self.failed_or_canceled()}
            self.complete=true;
        }
        if self.complete{StreamStatus::Complete}else{StreamStatus::Ok}
    }
    fn scan_chars<C:FnMut()->bool>(&mut self,budget:usize,cancel:&mut C)->bool{
        let end=self.char_cursor.saturating_add(budget).min(self.source.len());
        while self.char_cursor<end{
            if cancel(){self.canceled=true;return false}
            let at=self.char_cursor;
            match self.utf8.push(self.source[at],at){
                Err(start)=>{self.fail(XmlError::InvalidUtf8,start);return false},
                Ok(Some(cp)) if !is_xml_char(cp)=>{self.fail(XmlError::InvalidCharacter,self.utf8.start);return false},
                Ok(_)=>self.char_cursor+=1,
            }
        }
        if self.char_cursor==self.source.len(){
            if self.utf8.incomplete(){self.fail(XmlError::InvalidUtf8,self.utf8.start);return false}
            self.chars_complete=true
        }
        true
    }
    fn parse_next<C:FnMut()->bool>(&mut self,limit:usize,visitor:&mut Option<&mut dyn FnMut(&XmlToken)->bool>,cancel:&mut C)->bool{
        if self.tag.is_some(){return self.parse_tag(limit,visitor,cancel)}
        if !matches!(self.pending,Pending::None){return self.parse_pending(limit,visitor,cancel)}
        let p=self.cursor;if p>=limit{return true}
        if self.source[p]!=b'<'{self.pending=Pending::Text{start:p,pos:p,nonspace:false,first_nonspace:0,reference:None};return self.parse_pending(limit,visitor,cancel)}
        if self.source.get(p..p.saturating_add(4))==Some(b"<!--"){self.pending=Pending::Comment{start:p,pos:p+4,dashes:0};return self.parse_pending(limit,visitor,cancel)}
        if self.source.get(p..p.saturating_add(9))==Some(b"<![CDATA["){self.pending=Pending::Cdata{start:p,pos:p+9,brackets:0};return self.parse_pending(limit,visitor,cancel)}
        if self.source.get(p..p.saturating_add(9))==Some(b"<!DOCTYPE"){
            if self.depth!=0||self.roots!=0||self.doctype_seen{self.fail(XmlError::MalformedSyntax,p);return false}
            self.pending=Pending::Doctype{start:p,pos:p+9,state:Dtd::default()};return self.parse_pending(limit,visitor,cancel)
        }
        if self.source.get(p..p.saturating_add(8))==Some(b"<!ENTITY"){self.fail(XmlError::UnsupportedDtd,p);return false}
        if self.source.get(p..p.saturating_add(2))==Some(b"<?"){self.pending=Pending::Pi{start:p,pos:p+2,name_scan:None,name:None,declaration:false,data:None,separating:false,question:false};return self.parse_pending(limit,visitor,cancel)}
        if self.source.get(p..p.saturating_add(2))==Some(b"</"){self.tag=Some(TagState::new(p,true));return self.parse_tag(limit,visitor,cancel)}
        if self.source.get(p..p.saturating_add(2))==Some(b"<!"){self.fail(XmlError::MalformedSyntax,p);return false}
        self.tag=Some(TagState::new(p,false));self.parse_tag(limit,visitor,cancel)
    }
    fn parse_pending<C:FnMut()->bool>(&mut self,limit:usize,visitor:&mut Option<&mut dyn FnMut(&XmlToken)->bool>,cancel:&mut C)->bool{
        let pending=core::mem::replace(&mut self.pending,Pending::None);
        match pending{
            Pending::None=>true,
            Pending::Text{start,mut pos,mut nonspace,mut first_nonspace,mut reference}=>{
                while pos<limit&&pos<self.source.len(){if cancel(){self.pending=Pending::Text{start,pos,nonspace,first_nonspace,reference};self.canceled=true;return false}
                    if self.source[pos]==b'<'{break}
                    if self.source.get(pos..pos+3)==Some(b"]]>"){self.fail(XmlError::MalformedSyntax,pos);return false}
                    if self.source[pos]==b'&'{if !nonspace{nonspace=true;first_nonspace=pos}let scan=reference.get_or_insert_with(||RefScan::new(pos));match reference_step(self.source,&mut self.entities,scan,limit,&mut self.name_work){Ok(Some(end))=>{pos=end;reference=None},Ok(None)=>break,Err((e,at))=>{self.fail(e,at);return false}}}
                    else{if !is_xml_whitespace(self.source[pos])&&!nonspace{nonspace=true;first_nonspace=pos}pos+=1}
                }
                if pos<self.source.len()&&self.source[pos]==b'<'{return self.finish_text(start,pos,nonspace,first_nonspace,visitor)}
                if pos==self.source.len(){return self.finish_text(start,pos,nonspace,first_nonspace,visitor)}
                self.pending=Pending::Text{start,pos,nonspace,first_nonspace,reference};true
            }
            Pending::Comment{start,mut pos,mut dashes}=>{
                while pos<limit&&pos<self.source.len(){if cancel(){self.pending=Pending::Comment{start,pos,dashes};self.canceled=true;return false}
                    let c=self.source[pos];if dashes==2{if c!=b'>'{self.fail(XmlError::MalformedSyntax,pos-2);return false}let end=pos+1;let t=XmlToken{kind:TokenKind::Comment,source:span(start,end),value:span(start+4,pos-2),depth:self.depth,..XmlToken::default()};if !self.count_node(start){return false}if !self.emit(t,start,visitor){return false}self.cursor=end;return true}
                    dashes=if c==b'-'{dashes+1}else{0};pos+=1;
                }
                if pos==self.source.len(){self.fail(XmlError::MalformedSyntax,start);return false}self.pending=Pending::Comment{start,pos,dashes};true
            }
            Pending::Cdata{start,mut pos,mut brackets}=>{
                while pos<limit&&pos<self.source.len(){if cancel(){self.pending=Pending::Cdata{start,pos,brackets};self.canceled=true;return false}
                    let c=self.source[pos];if c==b'>'&&brackets>=2{if self.depth==0{self.fail(XmlError::MalformedSyntax,start);return false}
                        if pos-2-(start+9)>self.limits.text_run_bytes{self.fail(XmlError::TextLimit,start+9);return false}let end=pos+1;let t=XmlToken{kind:TokenKind::Cdata,source:span(start,end),value:span(start+9,pos-2),depth:self.depth,..XmlToken::default()};if !self.count_node(start){return false}if !self.emit(t,start,visitor){return false}self.cursor=end;return true}brackets=if c==b']'{(brackets+1).min(2)}else{0};pos+=1;
                }
                if pos==self.source.len(){self.fail(XmlError::MalformedSyntax,start);return false}self.pending=Pending::Cdata{start,pos,brackets};true
            }
            Pending::Doctype{start,mut pos,mut state}=>{
                while pos<limit&&pos<self.source.len(){if cancel(){self.pending=Pending::Doctype{start,pos,state};self.canceled=true;return false}
                    match state.step(self.source[pos]){Ok(done)=>{pos+=1;if done{let t=XmlToken{kind:TokenKind::Doctype,source:span(start,pos),depth:self.depth,..XmlToken::default()};if !self.count_node(start){return false}self.doctype_seen=true;self.doctype_span=span(start,pos);match EntityTable::parse(self.source,start){Ok(t)=>self.entities=t,Err((e,at))=>{self.fail(e,at);return false}}if !self.emit(t,start,visitor){return false}self.cursor=pos;return true}},Err(())=>{self.fail(XmlError::MalformedSyntax,pos);return false}}
                }
                if pos==self.source.len(){self.fail(XmlError::MalformedSyntax,start);return false}self.pending=Pending::Doctype{start,pos,state};true
            }
            Pending::Pi{start,mut pos,mut name_scan,mut name,mut declaration,mut data,mut separating,mut question}=>{
                if name.is_none(){let scan=name_scan.get_or_insert(NameScan::new(pos));match scan_name(self.source,scan,limit,false,&mut self.name_work){Ok(Some(parts))=>{name=Some(parts.name);pos=scan.pos;declaration=parts.xml_exact;if !declaration&&parts.xml_folded{self.fail(XmlError::MalformedSyntax,start);return false}if declaration&&start!=0&&!(start==3&&self.source.starts_with(b"\xef\xbb\xbf")){self.fail(XmlError::MalformedSyntax,start);return false}},Ok(None)=>{if pos==self.source.len(){self.fail(XmlError::InvalidName,pos);return false}self.pending=Pending::Pi{start,pos:scan.pos,name_scan,name,declaration,data,separating,question};return true},Err((e,at))=>{self.fail(e,at);return false}}}
                if data.is_none(){if pos>=limit{if pos==self.source.len(){self.fail(XmlError::MalformedSyntax,start);return false}self.pending=Pending::Pi{start,pos,name_scan,name,declaration,data,separating,question};return true}
                    if !separating&&self.source.get(pos..pos.saturating_add(2))==Some(b"?>"){if declaration{self.fail(XmlError::MalformedSyntax,pos);return false}data=Some(pos)}
                    else if !separating&&!is_xml_whitespace(self.source[pos]){self.fail(XmlError::MalformedSyntax,pos);return false}
                    else{separating=true;while pos<limit&&pos<self.source.len()&&is_xml_whitespace(self.source[pos]){pos+=1}if pos==limit&&pos<self.source.len(){self.pending=Pending::Pi{start,pos,name_scan,name,declaration,data,separating,question};return true}data=Some(pos)}
                }
                let begin=data.unwrap_or(pos);if data.is_none(){pos=begin}
                let mut end=None;while pos<limit&&pos<self.source.len(){if cancel(){self.pending=Pending::Pi{start,pos,name_scan,name,declaration,data,separating,question};self.canceled=true;return false}let c=self.source[pos];if c==b'>'&&question{end=Some(pos+1);break}question=c==b'?';pos+=1}
                let Some(end)=end else{if pos==self.source.len(){self.fail(XmlError::MalformedSyntax,start);return false}self.pending=Pending::Pi{start,pos,name_scan,name,declaration,data,separating,question};return true};
                let end_start=end-2;let n=name.unwrap_or_default();let t=XmlToken{kind:if declaration{TokenKind::Declaration}else{TokenKind::ProcessingInstruction},source:span(start,end),name:n,value:span(begin,end_start),depth:self.depth,..XmlToken::default()};
                if declaration{if let Err(crate::tokenizer::Fault(e,at))=validate_declaration(self.source,t.value){self.fail(e,at);return false}}else if !self.count_node(start){return false}
                if !self.emit(t,start,visitor){return false}self.cursor=end;true
            }
        }
    }
    fn finish_text(&mut self,start:usize,end:usize,nonspace:bool,first:usize,visitor:&mut Option<&mut dyn FnMut(&XmlToken)->bool>)->bool{
        if end-start>self.limits.text_run_bytes{self.fail(XmlError::TextLimit,start);return false}
        if self.depth==0&&nonspace{self.fail(XmlError::MalformedSyntax,first);return false}
        if (self.depth!=0||nonspace)&&!self.count_node(start){return false}let source=span(start,end);let t=XmlToken{kind:TokenKind::Text,source,value:source,depth:self.depth,..XmlToken::default()};
        if !self.emit(t,start,visitor){return false}self.cursor=end;true
    }

    fn parse_tag<C:FnMut()->bool>(&mut self,limit:usize,visitor:&mut Option<&mut dyn FnMut(&XmlToken)->bool>,cancel:&mut C)->bool{
        let Some(mut t)=self.tag.take()else{return true};
        while t.pos<limit&&t.pos<self.source.len(){if cancel(){self.tag=Some(t);self.canceled=true;return false}
            match t.phase{
                TagPhase::Prefix=>{let expected=if t.closing{if t.pos==t.start{b'<' }else{b'/'} }else{b'<'};if self.source[t.pos]!=expected{self.fail(XmlError::MalformedSyntax,t.pos);return false}t.pos+=1;if (t.closing&&t.pos==t.start+2)||(!t.closing&&t.pos==t.start+1){t.phase=TagPhase::Name;t.name_scan=Some(NameScan::new(t.pos));}},
                TagPhase::Name=>{let qname=true;let Some(scan)=t.name_scan.as_mut()else{self.fail(XmlError::InvalidName,t.pos);return false};match scan_name(self.source,scan,limit,qname,&mut self.name_work){Ok(Some(parts))=>{t.pos=scan.pos;t.name=Some(parts);if t.closing{let Some(expected)=self.open_names.last().copied().filter(|_|self.depth!=0)else{self.fail(XmlError::MismatchedTag,t.start);return false};t.comparison=Some(NameCompare::new(expected,parts.name));t.phase=TagPhase::EndMatch}else{
                    if self.depth==0{if self.roots!=0{self.fail(XmlError::MultipleRoots,t.start);return false}self.roots+=1}
                    if self.depth>=self.limits.nesting_depth||self.depth>=MAX_DEPTH{self.fail(XmlError::DepthLimit,t.start);return false}if !self.count_node(t.start){return false}
                    let tok=XmlToken{kind:TokenKind::StartElement,source:span(t.start,t.pos),name:parts.name,prefix:parts.prefix,local_name:parts.local,depth:self.depth,..XmlToken::default()};
                    t.token_index=self.tag_len();if !self.stage_tag(tok,t.start){return false}t.local_attrs=0;t.phase=TagPhase::Between;t.separated=false;}}
                    Ok(None)=>{self.tag=Some(t);return true},Err((e,at))=>{self.fail(e,at);return false}}},
                TagPhase::EndMatch=>{
                    let Some(comparison)=t.comparison.as_mut()else{self.fail(XmlError::MismatchedTag,t.start);return false};
                    match comparison.step(self.source,&mut self.name_work,cancel){
                        CompareStatus::Pending=>{self.tag=Some(t);return true},
                        CompareStatus::Canceled=>{self.tag=Some(t);self.canceled=true;return false},
                        CompareStatus::Different=>{self.fail(XmlError::MismatchedTag,t.start);return false},
                        CompareStatus::Equal=>{t.comparison=None;t.phase=TagPhase::EndBetween;if self.name_work.remaining==0{self.tag=Some(t);return true}},
                    }
                },
                TagPhase::Between=>{while t.pos<limit&&t.pos<self.source.len()&&is_xml_whitespace(self.source[t.pos]){t.pos+=1;t.separated=true}if t.pos>=limit&&t.pos<self.source.len(){self.tag=Some(t);return true}
                    if t.pos>=self.source.len(){self.fail(XmlError::MalformedSyntax,t.start);return false}
                    if self.source[t.pos]==b'>'{let end=t.pos+1;if !self.finish_start(&mut t,end,false,visitor){return false}return true}
                    if self.source.get(t.pos..t.pos+2)==Some(b"/>"){t.pos+=1;t.phase=TagPhase::EmptyClose;continue}
                    if !t.separated{self.fail(XmlError::MalformedSyntax,t.pos);return false}
                    if t.local_attrs>=self.limits.attributes_per_element||self.total_attrs>=self.limits.total_attributes{self.fail(XmlError::AttributeLimit,t.pos);return false}
                    t.attr_start=t.pos;t.attr_scan=Some(NameScan::new(t.pos));t.phase=TagPhase::AttrName;},
                TagPhase::EmptyClose=>{let end=t.pos+1;if !self.finish_start(&mut t,end,true,visitor){return false}return true},
                TagPhase::AttrName=>{let Some(scan)=t.attr_scan.as_mut()else{self.fail(XmlError::InvalidName,t.pos);return false};match scan_name(self.source,scan,limit,true,&mut self.name_work){Ok(Some(parts))=>{t.pos=scan.pos;
                    t.attr=Some(parts);t.duplicate_index=0;t.comparison=None;t.phase=TagPhase::AttrMatch;},Ok(None)=>{self.tag=Some(t);return true},Err((e,at))=>{self.fail(e,at);return false}}},
                TagPhase::AttrMatch=>{
                    if t.duplicate_index>=t.local_attrs{t.phase=TagPhase::AttrEq;if self.name_work.remaining==0{self.tag=Some(t);return true}continue}
                    if t.comparison.is_none(){
                        let Some(previous)=self.attribute_name(t.token_index,t.duplicate_index)else{self.fail(XmlError::OutputExhausted,t.attr_start);return false};
                        t.comparison=Some(NameCompare::new(previous,t.attr.unwrap_or_default().name));
                    }
                    match t.comparison.as_mut().unwrap().step(self.source,&mut self.name_work,cancel){
                        CompareStatus::Pending=>{self.tag=Some(t);return true},
                        CompareStatus::Canceled=>{self.tag=Some(t);self.canceled=true;return false},
                        CompareStatus::Equal=>{self.fail(XmlError::DuplicateAttribute,t.attr_start);return false},
                        CompareStatus::Different=>{t.comparison=None;t.duplicate_index+=1;if self.name_work.remaining==0{self.tag=Some(t);return true}},
                    }
                },
                TagPhase::AttrEq=>{while t.pos<limit&&t.pos<self.source.len()&&is_xml_whitespace(self.source[t.pos]){t.pos+=1}if t.pos>=limit&&t.pos<self.source.len(){self.tag=Some(t);return true}if t.pos>=self.source.len()||self.source[t.pos]!=b'='{self.fail(XmlError::MalformedSyntax,t.pos);return false}t.pos+=1;t.phase=TagPhase::AttrQuote;},
                TagPhase::AttrQuote=>{while t.pos<limit&&t.pos<self.source.len()&&is_xml_whitespace(self.source[t.pos]){t.pos+=1}if t.pos>=limit&&t.pos<self.source.len(){self.tag=Some(t);return true}if t.pos>=self.source.len()||!matches!(self.source[t.pos],b'\''|b'"'){self.fail(XmlError::MalformedSyntax,t.pos);return false}t.quote=self.source[t.pos];t.pos+=1;t.value_start=t.pos;t.phase=TagPhase::AttrValue;},
                TagPhase::AttrValue=>{let mut paused=false;while t.pos<limit&&t.pos<self.source.len()&&self.source[t.pos]!=t.quote{if cancel(){self.tag=Some(t);self.canceled=true;return false}if self.source[t.pos]==b'<'{self.fail(XmlError::MalformedSyntax,t.pos);return false}if self.source[t.pos]==b'&'{let scan=t.reference.get_or_insert_with(||RefScan::new(t.pos));match reference_step(self.source,&mut self.entities,scan,limit,&mut self.name_work){Ok(Some(end))=>{t.pos=end;t.reference=None},Ok(None)=>{paused=true;break},Err((e,at))=>{self.fail(e,at);return false}}}else{t.pos+=1}}
                    if paused||(t.pos==limit&&t.pos<self.source.len()){self.tag=Some(t);return true}if t.pos>=self.source.len(){self.fail(XmlError::MalformedSyntax,t.value_start-1);return false}
                    let Some(attr)=t.attr else{self.fail(XmlError::InvalidName,t.attr_start);return false};let tok=XmlToken{kind:TokenKind::Attribute,name:attr.name,prefix:attr.prefix,local_name:attr.local,source:span(t.attr_start,t.pos+1),value:span(t.value_start,t.pos),depth:self.depth,..XmlToken::default()};
                    if !self.stage_tag(tok,t.attr_start){return false}t.local_attrs+=1;self.total_attrs+=1;self.result.attribute_count+=1;t.pos+=1;t.attr=None;t.attr_scan=None;t.reference=None;t.phase=TagPhase::Between;t.separated=false;},
                TagPhase::EndBetween=>{while t.pos<limit&&t.pos<self.source.len()&&is_xml_whitespace(self.source[t.pos]){t.pos+=1}if t.pos>=limit&&t.pos<self.source.len(){self.tag=Some(t);return true}if t.pos>=self.source.len()||self.source[t.pos]!=b'>'{self.fail(XmlError::MalformedSyntax,t.pos);return false}
                    let Some(n)=t.name else{self.fail(XmlError::InvalidName,t.start);return false};let tok=XmlToken{kind:TokenKind::EndElement,source:span(t.start,t.pos+1),name:n.name,prefix:n.prefix,local_name:n.local,depth:self.depth-1,..XmlToken::default()};
                    if !self.emit(tok,t.start,visitor){return false}self.depth-=1;self.open_names.pop();self.cursor=t.pos+1;return true;}
            }
        }
        if t.pos==self.source.len(){let at=if t.phase==TagPhase::AttrValue{t.value_start.saturating_sub(1)}else{t.start};self.fail(XmlError::MalformedSyntax,at);return false}
        self.tag=Some(t);true
    }
    fn finish_start(&mut self,t:&mut TagState,end:usize,empty:bool,visitor:&mut Option<&mut dyn FnMut(&XmlToken)->bool>)->bool{
        let ok=if self.visitor_mode{if let Some(tok)=self.tag_scratch.first_mut(){tok.source.length=end-t.start;if empty{tok.kind=TokenKind::EmptyElement}}
            for tok in &self.tag_scratch{if let Some(cb)=visitor.as_deref_mut(){if !cb(tok){self.canceled=true;return false}}}self.tag_scratch.clear();true
        }else{let Some(tok)=self.tokens.get_mut(t.token_index)else{self.fail(XmlError::OutputExhausted,t.start);return false};tok.source.length=end-t.start;if empty{tok.kind=TokenKind::EmptyElement}true};
        if !ok{return false}if !empty{
            if self.depth>=MAX_DEPTH || self.open_names.len()>=MAX_DEPTH {
                self.fail(XmlError::DepthLimit,t.start);return false
            }
            if !Self::reserve_bounded(&mut self.open_names,MAX_DEPTH) {
                self.fail(XmlError::OutputExhausted,t.start);return false
            }
            self.open_names.push(t.name.unwrap_or_default().name);self.depth+=1
        }
        self.result.element_count+=1;self.cursor=end;true
    }
    fn tag_len(&self)->usize{if self.visitor_mode{self.tag_scratch.len()}else{self.tokens.len()}}
    fn attribute_name(&self,start_index:usize,index:usize)->Option<XmlSpan>{
        let token=if self.visitor_mode{self.tag_scratch.get(1+index)}else{self.tokens.get(start_index+1+index)};
        token.map(|t|t.name)
    }
    fn stage_tag(&mut self,t:XmlToken,at:usize)->bool{
        if self.visitor_mode{
            if self.tag_scratch.len()>=MAX_ATTRIBUTES_PER_ELEMENT+1
                || !Self::reserve_bounded(&mut self.tag_scratch,MAX_ATTRIBUTES_PER_ELEMENT+1) {
                self.fail(XmlError::OutputExhausted,at);return false
            }
            self.tag_scratch.push(t)
        }
        else if !self.push_token(t,at){return false}
        self.result.token_count+=1;true
    }
    fn emit(&mut self,t:XmlToken,at:usize,visitor:&mut Option<&mut dyn FnMut(&XmlToken)->bool>)->bool{
        if self.visitor_mode{if let Some(cb)=visitor.as_deref_mut(){if !cb(&t){self.canceled=true;return false}}}
        else if !self.push_token(t,at){return false}
        self.result.token_count+=1;true
    }
    fn reserve_bounded<T>(storage:&mut Vec<T>,capacity_limit:usize)->bool{
        if storage.len()>=capacity_limit{return false}
        if storage.len()==storage.capacity(){
            if storage.try_reserve_exact(1).is_err() || storage.capacity()>capacity_limit {
                *storage=Vec::new();
                return false
            }
        }
        storage.capacity()<=capacity_limit
    }
    fn push_token(&mut self,t:XmlToken,at:usize)->bool{
        if !Self::reserve_bounded(&mut self.tokens,self.token_capacity){
            self.fail(XmlError::OutputExhausted,at);return false
        }
        self.tokens.push(t);true
    }
    fn count_node(&mut self,at:usize)->bool{let Some(n)=self.result.node_count.checked_add(1)else{self.fail(XmlError::NodeLimit,at);return false};if n>self.limits.nodes.min(MAX_NODES){self.fail(XmlError::NodeLimit,at);return false}self.result.node_count=n;true}
    fn fail(&mut self,e:XmlError,at:usize){if self.result.diagnostic.category==XmlError::None{self.result.diagnostic=XmlDiagnostic{category:e,offset:at.min(self.source.len())}}}
    fn failed_or_canceled(&mut self)->StreamStatus{if self.canceled{StreamStatus::Canceled}else{StreamStatus::Failed}}
    fn frontier(&self)->usize{
        let position=if let Some(t)=&self.tag{let mut p=t.pos;if let Some(r)=t.reference{p=p.max(ref_frontier(r))}if let Some(n)=t.name_scan{p=p.max(n.pos)}if let Some(n)=t.attr_scan{p=p.max(n.pos)}p}else{match &self.pending{Pending::None=>self.cursor,Pending::Text{pos,reference,..}=>reference.map_or(*pos,|r|(*pos).max(ref_frontier(r))),Pending::Pi{pos,name_scan,..}=>name_scan.map_or(*pos,|n|(*pos).max(n.pos)),Pending::Comment{pos,..}|Pending::Cdata{pos,..}|Pending::Doctype{pos,..}=>*pos}};
        position.min(self.parse_limit)
    }
    pub fn result(&self)->XmlResult{self.result}
    pub fn bytes_read(&self)->usize{self.frontier()}
    pub fn tokens(&self)->&[XmlToken]{if self.failed||self.canceled{&[]}else{&self.tokens}}
    pub fn take(&mut self)->Option<Vec<XmlToken>>{if self.complete&&!self.failed&&!self.canceled&&!self.visitor_mode&&!self.taken{self.taken=true;Some(core::mem::take(&mut self.tokens))}else{None}}
    pub fn visitor_storage_bytes(&self)->usize{if self.visitor_mode{self.tag_scratch.capacity().saturating_mul(size_of::<XmlToken>())}else{0}}
    /// Bytes in the token Vec's retained, observable capacity.
    pub fn token_storage_bytes(&self)->usize{self.tokens.capacity().saturating_mul(size_of::<XmlToken>())}
    /// Bytes in the open-element name stack's retained, observable capacity.
    pub fn open_name_storage_bytes(&self)->usize{self.open_names.capacity().saturating_mul(size_of::<XmlSpan>())}
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "name_budget_tests.rs"]
mod name_budget_tests;

fn span(start:usize,end:usize)->XmlSpan{XmlSpan{offset:start,length:end.saturating_sub(start)}}
fn scan_name(source:&[u8],scan:&mut NameScan,limit:usize,qname:bool,work:&mut NameWork)->Result<Option<NameParts>,(XmlError,usize)>{
    while scan.pos<limit&&scan.pos<source.len(){
        let p=scan.pos;
        let Some(byte)=work.read(source,p)else{return Ok(None)};
        // ASCII delimiters terminate a name without consuming their byte. A
        // multibyte scalar is decoded incrementally from its saved state.
        if !scan.utf8.incomplete()&&byte<0x80 {
            let allowed=if p==scan.start{is_name_start(u32::from(byte))}else{is_name_char(u32::from(byte))};
            if !allowed {if p==scan.start{return Err((XmlError::InvalidName,p))}break}
        }
        let decoded=work.decode(&mut scan.utf8,byte,p).map_err(|at|(XmlError::InvalidUtf8,at))?;
        scan.pos+=1;
        if scan.pos-scan.start>MAX_TOKEN_NAME_BYTES{return Err((XmlError::MalformedSyntax,scan.start))}
        let Some(cp)=decoded else{continue};
        let at=scan.utf8.start;let first=at==scan.start;
        if !(if first{is_name_start(cp)}else{is_name_char(cp)}) {
            if first{return Err((XmlError::InvalidName,at))}
            scan.pos=at;break
        }
        scan.classification.push(cp,at-scan.start);
        if first{scan.leading_colon=cp==b':' as u32}
        if let Some(colon)=scan.first_colon {
            if at==colon+1{scan.local_first_valid=Some(is_name_start(cp))}
        }
        if cp==b':' as u32 {
            if qname&&scan.first_colon.is_some(){return Err((XmlError::InvalidName,scan.first_colon.unwrap_or(at)))}
            if scan.first_colon.is_none(){scan.first_colon=Some(at)}
        }
        scan.trailing_colon=cp==b':' as u32;
    }
    let p=scan.pos;
    if p==source.len()&&scan.utf8.incomplete(){return Err((XmlError::InvalidUtf8,scan.utf8.start))}
    if p==scan.start{return if p==source.len(){Err((XmlError::InvalidName,scan.start))}else{Ok(None)}}
    if p==limit&&p<source.len(){return Ok(None)}
    if qname&&(scan.leading_colon||scan.trailing_colon){return Err((XmlError::InvalidName,scan.start))}
    let name=span(scan.start,p);let (prefix,local)=if let Some(c)=scan.first_colon{(span(scan.start,c),span(c+1,p))}else{(XmlSpan{offset:scan.start,length:0},name)};
    if qname&&scan.local_first_valid==Some(false){return Err((XmlError::InvalidName,local.offset))}
    Ok(Some(NameParts{name,prefix,local,xml_exact:p-scan.start==3&&scan.classification.xml_exact,xml_folded:p-scan.start==3&&scan.classification.xml_folded,entity_known:scan.classification.known_entity(p-scan.start)}))
}

impl RefScan { fn new(start:usize)->Self{Self{start,pos:start+1,mode:RefMode::Start}} }
fn ref_frontier(scan:RefScan)->usize{match scan.mode{RefMode::Start|RefMode::Numeric{..}=>scan.pos,RefMode::Named(name)=>name.pos}}
/// Diagnostics for a reference point at its `&`; an unsupported reference's
/// entity name is `src[offset+1..]` up to the `;`. `ents` holds the DOCTYPE's
/// internal entities; each reference to one is charged against its limits.
fn reference_step(src:&[u8],ents:&mut EntityTable,scan:&mut RefScan,limit:usize,work:&mut NameWork)->Result<Option<usize>,(XmlError,usize)>{
    if matches!(scan.mode,RefMode::Start){if scan.pos>=limit{return if scan.pos==src.len(){Err((XmlError::MalformedSyntax,scan.start))}else{Ok(None)}}
        if src[scan.pos]==b'#'{scan.pos+=1;scan.mode=RefMode::Numeric{hex:false,digits:0,value:0}}
        else{scan.mode=RefMode::Named(NameScan::new(scan.pos))}}
    match &mut scan.mode{
        RefMode::Start=>unreachable!(),
        RefMode::Numeric{hex,digits,value}=>{
            if !*hex&&*digits==0&&scan.pos<limit&&src[scan.pos]==b'x'{*hex=true;scan.pos+=1}
            while scan.pos<limit{let c=src[scan.pos];let d=match c{b'0'..=b'9'=>Some((c-b'0')as u32),b'a'..=b'f' if *hex=>Some((c-b'a'+10)as u32),b'A'..=b'F' if *hex=>Some((c-b'A'+10)as u32),_=>None};let Some(d)=d else{break};*value=value.checked_mul(if *hex{16}else{10}).and_then(|v|v.checked_add(d)).filter(|v|*v<=0x10ffff).ok_or((XmlError::MalformedSyntax,scan.start))?;*digits+=1;scan.pos+=1}
            if scan.pos==limit&&limit<src.len(){return Ok(None)}if *digits==0||src.get(scan.pos)!=Some(&b';'){return Err((XmlError::MalformedSyntax,scan.start))}if !is_xml_char(*value){return Err((XmlError::InvalidCharacter,scan.start))}Ok(Some(scan.pos+1))
        },
        RefMode::Named(name)=>{let Some(parts)=scan_name(src,name,limit,false,work).map_err(|_|(XmlError::MalformedSyntax,scan.start))?else{return Ok(None)};scan.pos=name.pos;if scan.pos==limit&&limit<src.len(){return Ok(None)}if src.get(scan.pos)!=Some(&b';'){return Err((XmlError::MalformedSyntax,scan.start))}if !parts.entity_known{match ents.charge_reference(src,parts.name.slice(src).unwrap_or(&[])){Ok(true)=>{},Ok(false)=>return Err((XmlError::MalformedSyntax,scan.start)),Err(e)=>return Err((e,scan.start))}}Ok(Some(scan.pos+1))}
    }
}
