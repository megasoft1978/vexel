//! Owned, bounded interaction state corresponding to the legacy widget input API.
use crate::ControlState;

pub const MAX_EDIT_BYTES: usize = 127;
pub const MAX_LABEL_BYTES: usize = 63;
pub const MAX_UNIT_BYTES: usize = 8;
pub const MAX_OPTIONS: usize = 16;
pub const MAX_SEGMENTS: usize = 8;
pub const MOD_SHIFT: u32 = 1;
pub const MOD_COMMAND: u32 = 2;
pub const MOD_OPTION: u32 = 4;
pub const MOD_CONTROL: u32 = 8;
pub const KEY_TAB: u32 = 0x110000;
pub const KEY_RETURN: u32 = KEY_TAB + 1;
pub const KEY_SPACE: u32 = KEY_TAB + 2;
pub const KEY_ESCAPE: u32 = KEY_TAB + 3;
pub const KEY_LEFT: u32 = KEY_TAB + 4;
pub const KEY_RIGHT: u32 = KEY_TAB + 5;
pub const KEY_UP: u32 = KEY_TAB + 6;
pub const KEY_DOWN: u32 = KEY_TAB + 7;
pub const KEY_HOME: u32 = KEY_TAB + 8;
pub const KEY_END: u32 = KEY_TAB + 9;
pub const KEY_BACKSPACE: u32 = KEY_TAB + 10;
pub const KEY_DELETE: u32 = KEY_TAB + 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputResult { Unhandled, Changed, Committed, Cancelled, Invalid }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone { Neutral, Success, Warning, Error }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerEvent { Enter, Leave, Move, Down, UpInside, UpOutside, Cancel }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerState { pub enabled: bool, pub hovered: bool, pub pressed: bool, pub focused: bool, pub selected: bool }
impl Default for PointerState { fn default() -> Self { Self { enabled: false, hovered:false, pressed:false, focused:false, selected:false } } }
impl PointerState {
    pub fn visual_state(self) -> ControlState { if !self.enabled {ControlState::Disabled} else if self.pressed {ControlState::Pressed} else if self.focused {ControlState::Focus} else if self.selected {ControlState::Selected} else if self.hovered {ControlState::Hover} else {ControlState::Rest} }
    pub fn transition(&mut self, event: PointerEvent) -> bool {
        if !self.enabled { self.hovered=false; self.pressed=false; return false; }
        match event { PointerEvent::Enter=>self.hovered=true, PointerEvent::Leave=>{self.hovered=false;if !self.pressed{self.focused=false}}, PointerEvent::Move=>{}, PointerEvent::Down=>{if !self.hovered{return false}self.pressed=true;self.focused=true}, PointerEvent::UpInside=>{let activated=self.pressed&&self.hovered;self.pressed=false;return activated}, PointerEvent::UpOutside|PointerEvent::Cancel=>self.pressed=false }
        false
    }
}

fn valid_label(s:&str, nonempty:bool)->bool { s.len() <= MAX_LABEL_BYTES && (!nonempty || !s.is_empty()) }
fn bounded_copy(s:&str, max:usize)->Option<String>{ if s.len()>max{return None} let mut out=String::new();out.try_reserve_exact(s.len()).ok()?;out.push_str(s);Some(out) }
fn finite_range(v:f64,min:f64,max:f64)->bool { v.is_finite()&&min.is_finite()&&max.is_finite()&&min<=max&&v>=min&&v<=max }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit { pub semantic_label:String, pub value:String, pub original:String, pub cursor:usize, pub anchor:usize, pub editing:bool, pub invalid:bool }
impl TextEdit {
    pub fn new(label:&str,value:&str)->Option<Self>{if !valid_label(label,true)||value.len()>MAX_EDIT_BYTES{return None}Some(Self{semantic_label:bounded_copy(label,MAX_LABEL_BYTES)?,value:bounded_copy(value,MAX_EDIT_BYTES)?,original:bounded_copy(value,MAX_EDIT_BYTES)?,cursor:value.len(),anchor:value.len(),editing:false,invalid:false})}
    pub fn begin(&mut self)->bool{self.original=match bounded_copy(&self.value,MAX_EDIT_BYTES){Some(s)=>s,None=>return false};self.editing=true;self.invalid=false;true}
    pub fn commit(&mut self)->bool{if !self.editing||self.invalid{return false}self.original=match bounded_copy(&self.value,MAX_EDIT_BYTES){Some(s)=>s,None=>return false};self.editing=false;true}
    pub fn cancel(&mut self)->bool{if !self.editing{return false}self.value=match bounded_copy(&self.original,MAX_EDIT_BYTES){Some(s)=>s,None=>return false};self.cursor=self.value.len();self.anchor=self.cursor;self.editing=false;self.invalid=false;true}
    pub fn pointer(&mut self,byte_offset:usize,extend:bool)->bool{if byte_offset>self.value.len()||!self.value.is_char_boundary(byte_offset){return false}if !self.editing&&!self.begin(){return false}self.cursor=byte_offset;if !extend{self.anchor=byte_offset}true}
    fn selection(&self)->(usize,usize){(self.cursor.min(self.anchor),self.cursor.max(self.anchor))}
    fn insert(&mut self,text:&str)->bool{let (start,end)=self.selection();if text.len()>MAX_EDIT_BYTES-(self.value.len()-(end-start)){return false}let mut next=String::new();if next.try_reserve_exact(self.value.len()-(end-start)+text.len()).is_err(){return false}next.push_str(&self.value[..start]);next.push_str(text);next.push_str(&self.value[end..]);self.cursor=start+text.len();self.anchor=self.cursor;self.value=next;self.invalid=false;true}
    pub fn key(&mut self,key:u32,mods:u32)->InputResult{
        if key==KEY_ESCAPE{return if self.cancel(){InputResult::Cancelled}else{InputResult::Unhandled}}
        if key==KEY_RETURN{return if self.invalid{InputResult::Invalid}else if self.commit(){InputResult::Committed}else{InputResult::Unhandled}}
        if mods&MOD_COMMAND!=0{return InputResult::Unhandled} if !self.editing&&!self.begin(){return InputResult::Invalid}
        let extend=mods&MOD_SHIFT!=0;
        if matches!(key,KEY_LEFT|KEY_RIGHT|KEY_HOME|KEY_END){let to=match key{KEY_LEFT=>previous_boundary(&self.value,self.cursor),KEY_RIGHT=>next_boundary(&self.value,self.cursor),KEY_HOME=>0,_=>self.value.len()};self.cursor=to;if !extend{self.anchor=to}return InputResult::Changed}
        if key==KEY_BACKSPACE||key==KEY_DELETE{let (mut a,mut b)=self.selection();if a==b{if key==KEY_BACKSPACE{a=previous_boundary(&self.value,a)}else{b=next_boundary(&self.value,b)}}if a!=b{self.value.replace_range(a..b,"");self.cursor=a;self.anchor=a}return InputResult::Changed}
        if key<0x20||key>=KEY_TAB{return InputResult::Unhandled} let Some(ch)=char::from_u32(key) else{return InputResult::Invalid};if key==0x7f||!self.insert(&ch.to_string()){self.invalid=true;return InputResult::Invalid}InputResult::Changed
    }
}
fn previous_boundary(s:&str,at:usize)->usize{if at==0{0}else{s[..at].char_indices().next_back().map_or(0,|(i,_)|i)}}
fn next_boundary(s:&str,at:usize)->usize{if at>=s.len(){s.len()}else{at+s[at..].chars().next().map_or(0,char::len_utf8)}}

pub fn number_parse(text:&str)->Option<f64>{if text.is_empty()||text.len()>MAX_EDIT_BYTES{return None}let s=text.trim_matches([' ','\t']);if s.is_empty(){return None}let bytes=s.as_bytes();let mut i=usize::from(matches!(bytes.first(),Some(b'+')|Some(b'-')));let mut digits=0;while i<bytes.len()&&bytes[i].is_ascii_digit(){i+=1;digits+=1}if i<bytes.len()&&bytes[i]==b'.'{i+=1;while i<bytes.len()&&bytes[i].is_ascii_digit(){i+=1;digits+=1}}if digits==0{return None}if i<bytes.len()&&matches!(bytes[i],b'e'|b'E'){i+=1;if i<bytes.len()&&matches!(bytes[i],b'+'|b'-'){i+=1}let start=i;while i<bytes.len()&&bytes[i].is_ascii_digit(){i+=1}if start==i{return None}}if i!=bytes.len(){return None}let nonzero_significand=bytes.iter().take_while(|&&b|!matches!(b,b'e'|b'E')).any(|&b|b.is_ascii_digit()&&b!=b'0');let value=s.parse::<f64>().ok().filter(|v|v.is_finite())?;if nonzero_significand&&(value==0.0||value.abs()<f64::MIN_POSITIVE){None}else{Some(value)}}
pub fn number_format(v:f64)->Option<String>{if !v.is_finite(){return None}if v==0.0{return Some(if v.is_sign_negative(){"-0"}else{"0"}.into())}let scientific=format!("{v:.16e}");let split=scientific.find('e')?;let mut mant=scientific[..split].to_owned();let exponent=scientific[split+1..].parse::<i32>().ok()?;while mant.ends_with('0'){mant.pop();}if mant.ends_with('.') {mant.pop();}let s=if exponent < -4||exponent>=17{format!("{mant}e{}{abs:02}",if exponent>=0{"+"}else{"-"},abs=exponent.abs())}else{let decimals=(16-exponent).max(0)as usize;let mut fixed=format!("{v:.decimals$}");while fixed.contains('.')&&fixed.ends_with('0'){fixed.pop();}if fixed.ends_with('.') {fixed.pop();}fixed};if s.len()<=MAX_EDIT_BYTES{Some(s)}else{None}}

#[derive(Debug,Clone,PartialEq)] pub struct NumberField{pub edit:TextEdit,pub unit:String,pub value:f64,pub original:f64,pub minimum:f64,pub maximum:f64,pub step:f64,pub scrub_origin_value:f64,pub scrub_origin_x:i32,pub scrubbing:bool}
impl NumberField{
 pub fn new(label:&str,value:f64,min:f64,max:f64,step:f64,unit:&str)->Option<Self>{if !finite_range(value,min,max)||!step.is_finite()||step<=0.0||unit.len()>MAX_UNIT_BYTES{return None}Some(Self{edit:TextEdit::new(label,&number_format(value)?)?,unit:bounded_copy(unit,MAX_UNIT_BYTES)?,value,original:value,minimum:min,maximum:max,step,scrub_origin_value:0.0,scrub_origin_x:0,scrubbing:false})}
 pub fn set_draft(&mut self,s:&str)->bool{if s.len()>MAX_EDIT_BYTES||!self.edit.editing&&!self.edit.begin(){return false}self.edit.value=match bounded_copy(s,MAX_EDIT_BYTES){Some(v)=>v,None=>return false};self.edit.cursor=s.len();self.edit.anchor=s.len();match number_parse(s).filter(|v|*v>=self.minimum&&*v<=self.maximum){Some(v)=>{self.edit.invalid=false;self.value=v},None=>self.edit.invalid=true}true}
 fn set_value(&mut self,v:f64)->bool{if !v.is_finite(){return false}let v=v.clamp(self.minimum,self.maximum);let Some(s)=number_format(v)else{return false};self.edit.value=s;self.edit.cursor=self.edit.value.len();self.edit.anchor=self.edit.cursor;self.edit.invalid=false;self.value=v;true}
 pub fn key(&mut self,key:u32,mods:u32)->InputResult{if key==KEY_UP||key==KEY_DOWN{let Some(v)=number_parse(&self.edit.value).filter(|v|*v>=self.minimum&&*v<=self.maximum)else{self.edit.invalid=true;return InputResult::Invalid};let factor=if mods&MOD_SHIFT!=0{10.0}else if mods&MOD_OPTION!=0{0.1}else{1.0};if !self.edit.editing&&!self.edit.begin()||!self.set_value(v+if key==KEY_UP{1.0}else{-1.0}*self.step*factor){return InputResult::Invalid}return InputResult::Changed}if key==KEY_RETURN{let Some(v)=number_parse(&self.edit.value).filter(|v|*v>=self.minimum&&*v<=self.maximum)else{self.edit.invalid=true;return InputResult::Invalid};self.value=v;self.edit.invalid=false;if self.edit.commit(){self.original=v;return InputResult::Committed}return InputResult::Unhandled}if key==KEY_ESCAPE{if !self.edit.cancel(){return InputResult::Unhandled}self.scrubbing=false;self.value=number_parse(&self.edit.value).unwrap_or(self.original);return InputResult::Cancelled}let r=self.edit.key(key,mods);if r==InputResult::Changed{match number_parse(&self.edit.value).filter(|v|*v>=self.minimum&&*v<=self.maximum){Some(v)=>{self.value=v;self.edit.invalid=false},None=>self.edit.invalid=true}}r}
 pub fn pointer(&mut self,e:PointerEvent,x:i32,mods:u32)->InputResult{if e==PointerEvent::Down{if !self.edit.begin(){return InputResult::Invalid}self.scrub_origin_x=x;self.scrub_origin_value=self.value;self.scrubbing=true;return InputResult::Changed}if matches!(e,PointerEvent::Enter|PointerEvent::Leave)||e==PointerEvent::Move&&!self.scrubbing||!self.scrubbing{return InputResult::Unhandled}if matches!(e,PointerEvent::Cancel|PointerEvent::UpOutside){self.scrubbing=false;self.edit.cancel();self.value=self.scrub_origin_value;return InputResult::Cancelled}if e==PointerEvent::UpInside{self.scrubbing=false;self.original=self.value;return if self.edit.commit(){InputResult::Committed}else{InputResult::Unhandled}}let factor=if mods&MOD_SHIFT!=0{10.0}else if mods&MOD_OPTION!=0{0.1}else{1.0};if self.set_value(self.scrub_origin_value+(x as f64-self.scrub_origin_x as f64)*0.25*factor){InputResult::Changed}else{InputResult::Invalid}}
}

pub fn hex_parse(text:&str)->Option<u32>{let s=text.strip_prefix('#').unwrap_or(text);if !matches!(s.len(),3|4|6|8){return None}let mut c=[0u8,0,0,255];if s.len()<=4{for(i,b)in s.bytes().enumerate(){c[i]=b.to_digit_hex()?.checked_mul(17)?}}else{for(i,pair)in s.as_bytes().chunks_exact(2).enumerate(){c[i]=pair[0].to_digit_hex()?.checked_mul(16)?.checked_add(pair[1].to_digit_hex()?)?}}Some((c[0]as u32)<<24|(c[1]as u32)<<16|(c[2]as u32)<<8|c[3]as u32)}
trait Hex {fn to_digit_hex(self)->Option<u8>;} impl Hex for u8{fn to_digit_hex(self)->Option<u8>{match self{b'0'..=b'9'=>Some(self-b'0'),b'a'..=b'f'=>Some(self-b'a'+10),b'A'..=b'F'=>Some(self-b'A'+10),_=>None}}}
pub fn hex_format(rgba:u32)->String{if rgba as u8==255{format!("#{:02X}{:02X}{:02X}",rgba>>24,(rgba>>16)&255,(rgba>>8)&255)}else{format!("#{:08X}",rgba)}}
#[derive(Debug,Clone,PartialEq)] pub struct ColorField{pub edit:TextEdit,pub rgba:u32,pub original_rgba:u32}
impl ColorField{pub fn new(label:&str,rgba:u32)->Option<Self>{Some(Self{edit:TextEdit::new(label,&hex_format(rgba))?,rgba,original_rgba:rgba})}pub fn set_draft(&mut self,s:&str)->bool{if s.len()>MAX_EDIT_BYTES||!self.edit.editing&&!self.edit.begin(){return false}self.edit.value=match bounded_copy(s,MAX_EDIT_BYTES){Some(v)=>v,None=>return false};self.edit.cursor=s.len();self.edit.anchor=s.len();match hex_parse(s){Some(v)=>{self.edit.invalid=false;self.rgba=v},None=>self.edit.invalid=true}true}pub fn key(&mut self,key:u32,mods:u32)->InputResult{if key==KEY_RETURN{let Some(v)=hex_parse(&self.edit.value)else{self.edit.invalid=true;return InputResult::Invalid};self.rgba=v;self.edit.invalid=false;if self.edit.commit(){self.original_rgba=v;return InputResult::Committed}return InputResult::Unhandled}if key==KEY_ESCAPE{if !self.edit.cancel(){return InputResult::Unhandled}self.rgba=self.original_rgba;return InputResult::Cancelled}let r=self.edit.key(key,mods);if r==InputResult::Changed{if let Some(v)=hex_parse(&self.edit.value){self.rgba=v;self.edit.invalid=false}else{self.edit.invalid=true}}r}}

#[derive(Debug,Clone,PartialEq)] pub struct Slider{pub semantic_label:String,pub value:f64,pub minimum:f64,pub maximum:f64,pub step:f64,pub dragging:bool}
impl Slider{pub fn new(label:&str,value:f64,min:f64,max:f64,step:f64)->Option<Self>{if !valid_label(label,true)||!finite_range(value,min,max)||!step.is_finite()||step<=0.0{return None}Some(Self{semantic_label:bounded_copy(label,MAX_LABEL_BYTES)?,value,minimum:min,maximum:max,step,dragging:false})}pub fn key(&mut self,key:u32,mods:u32)->InputResult{if !matches!(key,KEY_UP|KEY_DOWN|KEY_LEFT|KEY_RIGHT){return InputResult::Unhandled}let dir=if matches!(key,KEY_UP|KEY_RIGHT){1.0}else{-1.0};let factor=if mods&MOD_SHIFT!=0{10.0}else if mods&MOD_OPTION!=0{0.1}else{1.0};self.value=(self.value+dir*self.step*factor).clamp(self.minimum,self.maximum);InputResult::Changed}pub fn pointer(&mut self,e:PointerEvent,x:i32,track_x:i32,width:u32)->bool{if width==0{return false}if e==PointerEvent::Down{self.dragging=true}if matches!(e,PointerEvent::UpInside|PointerEvent::UpOutside|PointerEvent::Cancel)&&!self.dragging{return false}if matches!(e,PointerEvent::Enter|PointerEvent::Leave){return true}if e==PointerEvent::Move&&!self.dragging{return false}if e==PointerEvent::Cancel{self.dragging=false;return true}let position=((x as i64-track_x as i64)as f64).clamp(0.0,width as f64);let raw=self.minimum+position/width as f64*(self.maximum-self.minimum);self.value=(self.minimum+((raw-self.minimum)/self.step).round()*self.step).clamp(self.minimum,self.maximum);if matches!(e,PointerEvent::UpInside|PointerEvent::UpOutside){self.dragging=false}true}}

#[derive(Debug,Clone,PartialEq,Eq)] pub struct Dropdown{pub semantic_label:String,pub options:Vec<String>,pub selected_index:usize,pub active_index:usize,pub expanded:bool}
impl Dropdown{pub fn new(label:&str,options:&[&str],selected:usize)->Option<Self>{if !valid_label(label,true)||options.is_empty()||options.len()>MAX_OPTIONS||selected>=options.len()||options.iter().any(|s|!valid_label(s,true)){return None}let mut owned=Vec::new();owned.try_reserve_exact(options.len()).ok()?;for s in options{owned.push(bounded_copy(s,MAX_LABEL_BYTES)?)}Some(Self{semantic_label:bounded_copy(label,MAX_LABEL_BYTES)?,options:owned,selected_index:selected,active_index:selected,expanded:false})}pub fn key(&mut self,key:u32,_mods:u32)->InputResult{if !self.expanded{if !matches!(key,KEY_RETURN|KEY_SPACE|KEY_DOWN|KEY_UP){return InputResult::Unhandled}self.expanded=true;self.active_index=self.selected_index;return InputResult::Changed}match key{KEY_ESCAPE=>{self.expanded=false;self.active_index=self.selected_index;InputResult::Cancelled},KEY_RETURN|KEY_SPACE=>{self.selected_index=self.active_index;self.expanded=false;InputResult::Committed},KEY_UP|KEY_LEFT=>{self.active_index=if self.active_index==0{self.options.len()-1}else{self.active_index-1};InputResult::Changed},KEY_DOWN|KEY_RIGHT=>{self.active_index=(self.active_index+1)%self.options.len();InputResult::Changed},_=>InputResult::Unhandled}}}

