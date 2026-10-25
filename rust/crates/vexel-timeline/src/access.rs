use crate::json::{object_fields, Reader};
use crate::schema::{boolean, field, uint, ItemView, TimelineIterator, TimelineView, TrackMeta, TrackView};
use vexel_base::Span;

pub(crate) fn track_at<'a>(view:&TimelineView<'a>,index:usize)->Option<TrackView<'a>> {
    let meta=*view.tracks.get(index)?; let fields=object_fields(view.source,meta.span,None).ok()?;
    let id=raw_string(view.source,field(&fields,b"id")?.value)?;
    let kind=raw_string(view.source,field(&fields,b"kind")?.value)?;
    let target=raw_string(view.source,field(&fields,b"target")?.value)?;
    let visible=boolean(view.source,field(&fields,b"visible")?.value).ok()?;
    let name=field(&fields,b"name").and_then(|f|raw_string(view.source,f.value));
    let (property,value_type,interpolation)=if meta.drawing {(None,None,None)} else {
        (Some(raw_string(view.source,field(&fields,b"property")?.value)?),
         Some(raw_string(view.source,field(&fields,b"value_type")?.value)?),
         Some(raw_string(view.source,field(&fields,b"interpolation")?.value)?))
    };
    Some(TrackView {id,kind,target,name,visible,payload_json:view.source.get(meta.span.start..meta.span.end)?,item_count:meta.item_count,property,value_type,interpolation})
}

pub(crate) fn item_at<'a>(view:&TimelineView<'a>,track:usize,index:usize)->Option<ItemView<'a>> {
    let meta=*view.tracks.get(track)?; let mut reader=Reader::new(view.source,None); reader.cursor=meta.items.start;
    reader.space().ok()?; if reader.source.get(reader.cursor)!=Some(&b'['){return None;} reader.cursor+=1; reader.space().ok()?;
    for current in 0..=index {
        let start=reader.cursor; if reader.source.get(start)==Some(&b']'){return None;}
        reader.scan(1).ok()?; let span=Span{start,end:reader.cursor};
        if current==index{return make_item(view.source,meta,span)}
        reader.space().ok()?; if reader.source.get(reader.cursor)!=Some(&b','){return None;} reader.cursor+=1; reader.space().ok()?;
    }
    None
}

pub(crate) fn item_next<'a>(view:&TimelineView<'a>,track:usize,it:&mut TimelineIterator)->Option<ItemView<'a>> {
    let meta=*view.tracks.get(track)?;
    if it.initialized&&it.track_index!=track{return None;}
    if !it.initialized {
        it.track_index=track; it.next_index=0; it.source_cursor=meta.items.start; it.initialized=true;
        let mut reader=Reader::new(view.source,None); reader.cursor=it.source_cursor; reader.space().ok()?;
        if reader.source.get(reader.cursor)!=Some(&b'['){return None;} reader.cursor+=1; reader.space().ok()?; it.source_cursor=reader.cursor;
    }
    if it.source_cursor==usize::MAX||it.source_cursor<meta.items.start||it.source_cursor>meta.items.end{return None;}
    let mut reader=Reader::new(view.source,None); reader.cursor=it.source_cursor; reader.space().ok()?;
    if reader.source.get(reader.cursor)==Some(&b']'){it.source_cursor=usize::MAX;return None;}
    let start=reader.cursor; reader.scan(1).ok()?; let span=Span{start,end:reader.cursor}; reader.space().ok()?;
    match reader.source.get(reader.cursor) {Some(b',')=>{reader.cursor+=1;reader.space().ok()?;it.source_cursor=reader.cursor;},Some(b']')=>{it.source_cursor=usize::MAX;},_=>return None}
    it.next_index=it.next_index.checked_add(1)?;
    make_item(view.source,meta,span)
}

fn make_item<'a>(source:&'a[u8],meta:TrackMeta,span:Span)->Option<ItemView<'a>> {
    let fields=object_fields(source,span,None).ok()?;
    let tick=uint(source,field(&fields,b"tick")?.value).ok()?;
    let (asset,value_json)=if meta.drawing {
        (Some(raw_string(source,field(&fields,b"asset")?.value)?),None)
    }else{
        let value=field(&fields,b"value")?.value;
        (None,Some(source.get(value.start..value.end)?))
    };
    let ease=match field(&fields,b"ease") {Some(f)=>Some(crate::easing::parse(source,f.value).ok()?),None=>None};
    Some(ItemView{tick,asset,value_json,source:span,ease})
}
fn raw_string(source:&[u8],span:Span)->Option<&[u8]>{
    let raw=source.get(span.start..span.end)?;
    if raw.len()<2||raw.first()!=Some(&b'"')||raw.last()!=Some(&b'"'){return None;}
    raw.get(1..raw.len()-1)
}

pub(crate) fn track_noise(view:&TimelineView<'_>,index:usize)->Option<crate::noise::Noise> {
    let meta=*view.tracks.get(index)?; if meta.drawing {return None;}
    // Cheap pre-check: tracks without the member skip the field parse on every frame.
    if !view.source.get(meta.span.start..meta.span.end)?.windows(7).any(|w|w==b"\"noise\"") {return None;}
    let fields=object_fields(view.source,meta.span,None).ok()?;
    crate::noise::parse(view.source,field(&fields,b"noise")?.value).ok()
}
