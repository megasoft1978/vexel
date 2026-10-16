use crate::json::{array_spans, decode_string, object_fields, Field, Reader};
use crate::{access, Result};
use vexel_base::Span;

pub const MAX_TRACKS: usize = 256;
pub const MAX_ITEMS: usize = 1_000_000;
pub const MAX_ITEMS_PER_TRACK: usize = 100_000;
pub const MAX_STRING_BYTES: usize = 4096;
pub const MAX_SOURCE_BYTES: usize = vexel_project::MAX_TIMELINE_BYTES;
const MAX_SCENE_IDS: usize = 1_000_000;
const MAX_ASSETS: usize = vexel_project::MAX_ASSET_COUNT;
const MAX_ASSET_NAME_BYTES: usize = vexel_project::MAX_ASSET_NAME_BYTES;
const MAX_MEDIA_TYPE_BYTES: usize = vexel_project::MAX_MEDIA_TYPE_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error { InvalidArgument, MalformedJson, UnsupportedVersion, InvalidSchema, InvalidReference, Duplicate, Limit, InvalidValue }
impl Error {
    pub const fn name(self) -> &'static str { match self {
        Self::InvalidArgument => "invalid_argument", Self::MalformedJson => "malformed_json",
        Self::UnsupportedVersion => "unsupported_version", Self::InvalidSchema => "invalid_schema",
        Self::InvalidReference => "invalid_reference", Self::Duplicate => "duplicate",
        Self::Limit => "limit", Self::InvalidValue => "invalid_value",
    }}
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeFailure { pub error: Error, pub offset: usize, pub canceled: bool }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode { Once, Repeat, PingPong }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRef { pub name: Vec<u8>, pub media_type: Vec<u8> }
/// Owns the bounded, sorted scene index and asset table used for reference checks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimelineContext { pub scene_ids: Vec<Vec<u8>>, pub assets: Vec<AssetRef> }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackView<'a> {
    pub id: &'a [u8], pub kind: &'a [u8], pub target: &'a [u8], pub name: Option<&'a [u8]>,
    pub visible: bool, pub payload_json: &'a [u8], pub item_count: usize,
    pub property: Option<&'a [u8]>, pub value_type: Option<&'a [u8]>, pub interpolation: Option<&'a [u8]>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemView<'a> { pub tick: u32, pub asset: Option<&'a [u8]>, pub value_json: Option<&'a [u8]>, pub source: Span }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimelineIterator { pub track_index: usize, pub next_index: usize, pub source_cursor: usize, pub initialized: bool }
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrackMeta { pub span: Span, pub items: Span, pub drawing: bool, pub item_count: usize }
#[derive(Debug, Clone)]
pub struct TimelineView<'a> {
    pub(crate) source: &'a [u8], pub(crate) tracks: Vec<TrackMeta>, tracks_span: Span,
    ticks_per_second: u32, duration_ticks: u32, fps_numerator: u32,
    fps_denominator: u32, loop_mode: LoopMode, onion_before: u32,
    onion_after: u32, onion_opacity: f64, empty: bool,
}

pub fn decode<'a>(source: &'a [u8], context: &TimelineContext) -> Result<TimelineView<'a>> {
    decode_cancellable(source, context, None::<fn() -> bool>)
}
pub fn decode_cancellable<'a, F>(source: &'a [u8], context: &TimelineContext, cancel: Option<F>) -> Result<TimelineView<'a>>
where F: FnMut() -> bool {
    validate_context(context)?;
    if source.is_empty() { return Ok(TimelineView { source, tracks: Vec::new(), tracks_span: Span {start:0,end:0},
        ticks_per_second:0,duration_ticks:0,fps_numerator:0,fps_denominator:0,loop_mode:LoopMode::Once,
        onion_before:0,onion_after:0,onion_opacity:0.0,empty:true }); }
    if source.len() > MAX_SOURCE_BYTES { return Err(DecodeFailure { error:Error::Limit,offset:0,canceled:false }); }
    let mut callback = cancel;
    let mut poll_cancel = || callback.as_mut().is_some_and(|check| check());
    let mut reader = Reader::new(source, Some(&mut poll_cancel));
    reader.scan(0)?;
    let root_end = reader.cursor; reader.space()?;
    if reader.cursor != source.len() { return Err(reader.failure(Error::MalformedJson, reader.cursor)); }
    drop(reader);
    let root = object_fields(source, Span {start:0,end:root_end},Some(&mut poll_cancel))?;
    let schema = required(&root,b"schema").ok_or(DecodeFailure{error:Error::UnsupportedVersion,offset:0,canceled:false})?;
    let schema_value = string(source,schema.value).map_err(|_| DecodeFailure{error:Error::InvalidSchema,offset:schema.value.start,canceled:false})?;
    if schema_value != b"vexel.animation" { return Err(DecodeFailure{error:Error::UnsupportedVersion,offset:schema.value.start,canceled:false}); }
    let version_field = required(&root,b"version").ok_or(DecodeFailure{error:Error::InvalidSchema,offset:schema.value.start,canceled:false})?;
    let version = uint(source,version_field.value)?;
    if version != 1 { return Err(DecodeFailure{error:Error::UnsupportedVersion,offset:version_field.value.start,canceled:false}); }
    require_fields(&root,&[b"schema",b"version",b"ticks_per_second",b"duration_ticks",b"fps",b"loop",b"onion",b"tracks"],0)?;
    let tps_span = required(&root,b"ticks_per_second").ok_or_else(|| at(Error::InvalidSchema,0))?.value;
    let duration_span = required(&root,b"duration_ticks").ok_or_else(|| at(Error::InvalidSchema,0))?.value;
    let tps=uint(source,tps_span)?; let duration=uint(source,duration_span)?;
    if tps==0 || tps>240 { return Err(at(Error::InvalidValue,tps_span.start)); }
    if duration==0 || duration>14_400_000 { return Err(at(Error::InvalidValue,duration_span.start)); }
    let fps_span=required(&root,b"fps").ok_or_else(|| at(Error::InvalidSchema,0))?.value;
    let fps=object_fields(source,fps_span,Some(&mut poll_cancel))?; require_fields(&fps,&[b"numerator",b"denominator"],fps_span.start).map_err(|_| at(Error::InvalidValue,fps_span.start))?;
    let num_field=required(&fps,b"numerator").ok_or_else(||at(Error::InvalidValue,fps_span.start))?;
    let den_field=required(&fps,b"denominator").ok_or_else(||at(Error::InvalidValue,fps_span.start))?;
    let numerator=uint(source,num_field.value)?; let denominator=uint(source,den_field.value)?;
    if numerator==0 || denominator==0 || (u64::from(tps)*u64::from(denominator))%u64::from(numerator)!=0 { return Err(at(Error::InvalidValue,fps_span.start)); }
    let loop_span=required(&root,b"loop").ok_or_else(||at(Error::InvalidSchema,0))?.value;
    let loop_text=string(source,loop_span).map_err(|_|at(Error::InvalidValue,loop_span.start))?;
    let loop_mode=match loop_text.as_slice(){b"once"=>LoopMode::Once,b"loop"=>LoopMode::Repeat,b"ping_pong"=>LoopMode::PingPong,_=>return Err(at(Error::InvalidValue,loop_span.start))};
    let onion_span=required(&root,b"onion").ok_or_else(||at(Error::InvalidSchema,0))?.value;
    let onion=object_fields(source,onion_span,Some(&mut poll_cancel))?; require_fields(&onion,&[b"before",b"after",b"opacity"],onion_span.start).map_err(|_|at(Error::InvalidValue,onion_span.start))?;
    let before_span=required(&onion,b"before").ok_or_else(||at(Error::InvalidValue,onion_span.start))?.value;
    let after_span=required(&onion,b"after").ok_or_else(||at(Error::InvalidValue,onion_span.start))?.value;
    let opacity_span=required(&onion,b"opacity").ok_or_else(||at(Error::InvalidValue,onion_span.start))?.value;
    let before=uint(source,before_span).map_err(|_|at(Error::InvalidValue,onion_span.start))?;
    let after=uint(source,after_span).map_err(|_|at(Error::InvalidValue,onion_span.start))?;
    let opacity=number(source,opacity_span).map_err(|_|at(Error::InvalidValue,opacity_span.start))?;
    if before>8 || after>8 || !(0.0..=1.0).contains(&opacity) { return Err(at(Error::InvalidValue,opacity_span.start)); }
    let tracks_field=required(&root,b"tracks").ok_or_else(||at(Error::InvalidSchema,0))?;
    let track_spans=array_spans(source,tracks_field.value,MAX_TRACKS,Some(&mut poll_cancel))?;
    let mut metas=Vec::new(); metas.try_reserve_exact(track_spans.len()).map_err(|_|at(Error::Limit,tracks_field.value.start))?;
    let mut ids: Vec<Vec<u8>>=Vec::new(); let mut total_items=0usize;
    for span in &track_spans {
        let fields=object_fields(source,*span,Some(&mut poll_cancel))?;
        let kind_field=required(&fields,b"kind").ok_or_else(||at(Error::InvalidSchema,span.start))?;
        let kind=string(source,kind_field.value).map_err(|_|at(Error::InvalidSchema,span.start))?;
        let drawing=kind==b"drawing";
        if !drawing && kind!=b"property" { return Err(at(Error::InvalidValue,kind_field.value.start)); }
        let id_field=required(&fields,b"id").ok_or_else(||at(Error::InvalidSchema,span.start))?;
        let target_field=required(&fields,b"target").ok_or_else(||at(Error::InvalidSchema,span.start))?;
        let visible_field=required(&fields,b"visible").ok_or_else(||at(Error::InvalidSchema,span.start))?;
        let id=string(source,id_field.value).map_err(|_|at(Error::InvalidSchema,span.start))?;
        let target=string(source,target_field.value).map_err(|_|at(Error::InvalidSchema,span.start))?;
        let visible=boolean(source,visible_field.value).map_err(|_|at(Error::InvalidSchema,span.start))?;
        let _ = visible;
        if !valid_track_id(&id) || target.is_empty() || target.len()>256 { return Err(at(Error::InvalidSchema,span.start)); }
        if ids.iter().any(|prior|prior==&id) { return Err(at(Error::Duplicate,id_field.value.start)); }
        if ids.len()==MAX_TRACKS { return Err(at(Error::Limit,span.start)); }
        ids.try_reserve(1).map_err(|_|at(Error::Limit,span.start))?; ids.push(id);
        if context.scene_ids.binary_search(&target).is_err() { return Err(at(Error::InvalidReference,target_field.value.start)); }
        let name=field(&fields,b"name"); if let Some(n)=name { string(source,n.value)?; }
        let items_field=required(&fields,if drawing {b"cels"} else {b"keys"}).ok_or_else(||at(Error::InvalidSchema,span.start))?;
        if drawing {
            require_track_fields(&fields,name.is_some(),true,*span)?;
        } else {
            require_track_fields(&fields,name.is_some(),false,*span)?;
            validate_property(source,&fields,*span)?;
        }
        let item_spans=array_spans(source,items_field.value,MAX_ITEMS_PER_TRACK,Some(&mut poll_cancel))?;
        if item_spans.is_empty() { return Err(at(Error::InvalidValue,items_field.value.start)); }
        let type_val=field(&fields,b"value_type").map(|f|string(source,f.value)).transpose()?.unwrap_or_default();
        let mut previous=0u32;
        for (index,item_span) in item_spans.iter().enumerate() {
            let item_fields=object_fields(source,*item_span,Some(&mut poll_cancel))?;
            if drawing {
                validate_item_fields(&item_fields,&[b"tick",b"asset",b"label"],&[b"tick",b"asset"],*item_span)?;
            } else {
                require_fields(&item_fields,&[b"tick",b"value"],item_span.start)?;
            }
            let tick_field=required(&item_fields,b"tick").ok_or_else(||at(Error::InvalidSchema,item_span.start))?;
            let tick=uint(source,tick_field.value)?;
            if tick>=duration { return Err(at(Error::InvalidValue,tick_field.value.start)); }
            if drawing {
                let asset_field=required(&item_fields,b"asset").ok_or_else(||at(Error::InvalidSchema,item_span.start))?;
                let asset_name=string(source,asset_field.value)?;
                if let Some(label)=field(&item_fields,b"label") { string(source,label.value)?; }
                if (index==0 && tick!=0)||(index>0&&tick<=previous) { return Err(at(Error::Duplicate,item_span.start)); }
                previous=tick;
                if !context.assets.iter().any(|a|a.name==asset_name&&a.media_type==b"image/svg+xml") { return Err(at(Error::InvalidReference,item_span.start)); }
            } else {
                let value=required(&item_fields,b"value").ok_or_else(||at(Error::InvalidSchema,item_span.start))?.value;
                validate_typed_value(source,value,&type_val,&mut poll_cancel)?;
                if (index==0 && tick!=0)||(index>0&&tick<=previous) { return Err(at(Error::Duplicate,item_span.start)); }
                previous=tick;
            }
        }
        total_items=total_items.checked_add(item_spans.len()).ok_or_else(||at(Error::Limit,items_field.value.start))?;
        if total_items>MAX_ITEMS { return Err(at(Error::Limit,items_field.value.start)); }
        metas.push(TrackMeta {span:*span,items:items_field.value,drawing,item_count:item_spans.len()});
    }
    Ok(TimelineView {source,tracks:metas,tracks_span:tracks_field.value,ticks_per_second:tps,duration_ticks:duration,
        fps_numerator:numerator,fps_denominator:denominator,loop_mode,onion_before:before,onion_after:after,onion_opacity:opacity,empty:false})
}

pub fn classify(source:&[u8]) -> Result<bool> { classify_cancellable(source,None::<fn()->bool>) }
pub fn classify_cancellable<F>(source:&[u8],cancel:Option<F>) -> Result<bool>
where F:FnMut()->bool {
    if source.len()>MAX_SOURCE_BYTES { return Err(at(Error::Limit,0)); }
    let mut callback=cancel;
    let mut poll_cancel=||callback.as_mut().is_some_and(|f|f());
    let mut reader=Reader::new(source,Some(&mut poll_cancel)); reader.scan(0)?; let end=reader.cursor; reader.space()?;
    if reader.cursor!=source.len() { return Err(reader.failure(Error::MalformedJson,reader.cursor)); }
    drop(reader);
    let root=object_fields(source,Span{start:0,end},Some(&mut poll_cancel))?;
    let Some(s)=field(&root,b"schema") else{return Ok(false)};
    let value=string(source,s.value)?; if value!=b"vexel.animation" {return Ok(false)}
    let Some(v)=field(&root,b"version") else{return Ok(false)};
    Ok(uint(source,v.value).is_ok_and(|n|n==1))
}

fn validate_context(context:&TimelineContext)->Result<()> {
    if context.scene_ids.len()>MAX_SCENE_IDS||context.assets.len()>MAX_ASSETS{return Err(at(Error::Limit,0));}
    for (i,id) in context.scene_ids.iter().enumerate(){
        if id.is_empty()||id.len()>256{return Err(at(Error::InvalidArgument,0));}
        if i>0 {match context.scene_ids[i-1].cmp(id){std::cmp::Ordering::Equal=>return Err(at(Error::Duplicate,0)),std::cmp::Ordering::Greater=>return Err(at(Error::InvalidArgument,0)),_=>()}}
    }
    for (i,a) in context.assets.iter().enumerate(){
        if !valid_asset_name(&a.name)||!valid_media_type(&a.media_type){return Err(at(Error::InvalidArgument,0));}
        if context.assets[..i].iter().any(|prior|prior.name==a.name){return Err(at(Error::Duplicate,0));}
    }
    Ok(())
}
fn valid_asset_name(s:&[u8])->bool { !s.is_empty()&&s.len()<=MAX_ASSET_NAME_BYTES&&s!=b"."&&s!=b".."&&s.iter().all(|b|b.is_ascii_alphanumeric()||matches!(*b,b'.'|b'_'|b'-')) }
fn valid_media_type(s:&[u8])->bool { s.len()>=3&&s.len()<=MAX_MEDIA_TYPE_BYTES&&s.iter().filter(|b|**b==b'/').count()==1&&s.iter().all(|b|(0x21..=0x7e).contains(b))&&s[0]!=b'/'&&s[s.len()-1]!=b'/' }
fn valid_track_id(s:&[u8])->bool { !s.is_empty()&&s.len()<=128&&s.iter().all(|b|b.is_ascii_alphanumeric()||matches!(*b,b'_'|b'-')) }
fn validate_property(source:&[u8],fields:&[Field],span:Span)->Result<()> {
    let prop_f=required(fields,b"property").ok_or_else(||at(Error::InvalidValue,span.start))?;
    let type_f=required(fields,b"value_type").ok_or_else(||at(Error::InvalidValue,span.start))?;
    let interp_f=required(fields,b"interpolation").ok_or_else(||at(Error::InvalidValue,span.start))?;
    let prop=string(source,prop_f.value).map_err(|_|at(Error::InvalidValue,span.start))?;
    let kind=string(source,type_f.value).map_err(|_|at(Error::InvalidValue,span.start))?;
    let interpolation=string(source,interp_f.value).map_err(|_|at(Error::InvalidValue,span.start))?;
    if prop.is_empty() || ![b"step".as_slice(),b"linear",b"cubic"].contains(&interpolation.as_slice()) ||
       ![b"scalar".as_slice(),b"transform2d",b"rgba",b"boolean"].contains(&kind.as_slice()) {
        return Err(at(Error::InvalidValue,span.start));
    }
    let scalar=[b"x".as_slice(),b"y",b"width",b"height",b"cx",b"cy",b"r",b"rx",b"ry",b"opacity",b"fill-opacity",b"stroke-opacity",b"stroke-width",b"stroke-dashoffset"].contains(&prop.as_slice())&&kind==b"scalar";
    let match_ok=scalar||([b"fill".as_slice(),b"stroke"].contains(&prop.as_slice())&&kind==b"rgba")||(prop==b"transform"&&kind==b"transform2d")||(prop==b"visibility"&&kind==b"boolean");
    if !match_ok {return Err(at(Error::InvalidValue,prop_f.value.start));}
    if kind==b"boolean"&&interpolation!=b"step" {return Err(at(Error::InvalidValue,interp_f.value.start));}
    Ok(())
}
fn validate_typed_value(source:&[u8],span:Span,kind:&[u8],cancel:&mut dyn FnMut()->bool)->Result<()> {
    if kind==b"boolean" {boolean(source,span).map(|_|())?;return Ok(())}
    if kind==b"scalar" {number(source,span).map(|_|())?;return Ok(())}
    if kind==b"transform2d"||kind==b"rgba" {
        let spans=array_spans(source,span,6,Some(cancel)).map_err(|error|if error.canceled{error}else{at(Error::InvalidValue,span.start)})?;
        let expected=if kind==b"rgba"{4}else{6}; if spans.len()!=expected{return Err(at(Error::InvalidValue,span.start));}
        for component in spans {let n=number(source,component).map_err(|_|at(Error::InvalidValue,component.start))?; if kind==b"rgba"&&!(0.0..=1.0).contains(&n){return Err(at(Error::InvalidValue,component.start));}}
        return Ok(())
    }
    Err(at(Error::InvalidValue,span.start))
}
fn validate_item_fields(fields:&[Field],with_label:&[&[u8]],without_label:&[&[u8]],span:Span)->Result<()> {
    let allowed=if field(fields,b"label").is_some(){with_label}else{without_label};require_fields(fields,allowed,span.start)
}
fn require_track_fields(fields:&[Field],name:bool,drawing:bool,span:Span)->Result<()> {
    if drawing {if name {require_fields(fields,&[b"id",b"kind",b"target",b"name",b"visible",b"cels"],span.start)}else{require_fields(fields,&[b"id",b"kind",b"target",b"visible",b"cels"],span.start)}}
    else if name {require_fields(fields,&[b"id",b"kind",b"target",b"name",b"visible",b"property",b"value_type",b"interpolation",b"keys"],span.start)}
    else {require_fields(fields,&[b"id",b"kind",b"target",b"visible",b"property",b"value_type",b"interpolation",b"keys"],span.start)}
}
fn required<'a>(fields:&'a[Field],key:&[u8])->Option<&'a Field>{field(fields,key)}
pub(crate) fn field<'a>(fields:&'a[Field],key:&[u8])->Option<&'a Field>{fields.iter().find(|f|f.key==key)}
fn require_fields(fields:&[Field],names:&[&[u8]],offset:usize)->Result<()> {
    if fields.len()!=names.len()||fields.iter().any(|f|!names.contains(&f.key.as_slice())){return Err(at(Error::InvalidSchema,offset));} Ok(())
}
pub(crate) fn string(source:&[u8],span:Span)->Result<Vec<u8>> {
    let bytes=source.get(span.start..span.end).ok_or_else(||at(Error::InvalidValue,span.start))?;
    if bytes.len()<2||bytes[0]!=b'"'||bytes[bytes.len()-1]!=b'"'{return Err(at(Error::MalformedJson,span.start));}
    decode_string(&bytes[1..bytes.len()-1]).ok_or_else(||at(Error::Limit,span.start))
}
pub(crate) fn uint(source:&[u8],span:Span)->Result<u32>{
    let bytes=source.get(span.start..span.end).ok_or_else(||at(Error::InvalidValue,span.start))?;
    if bytes.is_empty()||!bytes.iter().all(u8::is_ascii_digit){return Err(at(Error::InvalidValue,span.start));}
    let mut value=0u32;for b in bytes {value=value.checked_mul(10).and_then(|v|v.checked_add(u32::from(*b-b'0'))).ok_or_else(||at(Error::Limit,span.start))?;}Ok(value)
}
pub(crate) fn boolean(source:&[u8],span:Span)->Result<bool>{match source.get(span.start..span.end){Some(b"true")=>Ok(true),Some(b"false")=>Ok(false),_=>Err(at(Error::InvalidValue,span.start))}}
fn number(source:&[u8],span:Span)->Result<f64>{let raw=source.get(span.start..span.end).ok_or_else(||at(Error::InvalidValue,span.start))?;let s=std::str::from_utf8(raw).map_err(|_|at(Error::InvalidValue,span.start))?;let n=s.parse::<f64>().map_err(|_|at(Error::InvalidValue,span.start))?;if n.is_finite(){Ok(n)}else{Err(at(Error::InvalidValue,span.start))}}
fn at(error:Error,offset:usize)->DecodeFailure{DecodeFailure{error,offset,canceled:false}}

// Accessors share the exact source spans captured by this decoder.
impl<'a> TimelineView<'a> {
    pub fn track_at(&self,index:usize)->Option<TrackView<'a>>{access::track_at(self,index)}
    pub fn item_at(&self,track:usize,index:usize)->Option<ItemView<'a>>{access::item_at(self,track,index)}
    pub fn item_next(&self,track:usize,it:&mut TimelineIterator)->Option<ItemView<'a>>{access::item_next(self,track,it)}
    pub fn track_count(&self)->usize{self.tracks.len()}
    pub fn source(&self)->&'a [u8]{self.source}
    pub fn tracks_span(&self)->Span{self.tracks_span}
    pub fn ticks_per_second(&self)->u32{self.ticks_per_second}
    pub fn duration_ticks(&self)->u32{self.duration_ticks}
    pub fn fps(&self)->(u32,u32){(self.fps_numerator,self.fps_denominator)}
    pub fn loop_mode(&self)->LoopMode{self.loop_mode}
    pub fn onion(&self)->(u32,u32,f64){(self.onion_before,self.onion_after,self.onion_opacity)}
    pub fn is_empty(&self)->bool{self.empty}
}
