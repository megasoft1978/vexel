use vexel_timeline::{classify, decode, decode_cancellable, string_copy, AssetRef, Error, LoopMode, TimelineContext, TimelineIterator,
    MAX_ITEMS_PER_TRACK, MAX_SOURCE_BYTES};
use std::fmt::Write as _;

fn context() -> TimelineContext {
    TimelineContext { scene_ids: vec![b"layer1".to_vec(), b"shape1".to_vec()], assets: vec![
        AssetRef { name: b"cel0".to_vec(), media_type: b"image/svg+xml".to_vec() },
        AssetRef { name: b"pixel.png".to_vec(), media_type: b"image/png".to_vec() },
    ] }
}
fn base(tracks:&str)->String { format!(r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":240,"fps":{{"numerator":24,"denominator":1}},"loop":"loop","onion":{{"before":1,"after":2,"opacity":0.35}},"tracks":{tracks}}}"#) }

#[test]
fn borrowed_view_accessors_and_json_spans_match_c_contract() {
    let source=base(r#"[{"id":"walk","kind":"drawing","target":"layer1","name":"walk","visible":true,"cels":[{"tick":0,"asset":"cel0","label":"key pose"}]}]"#);
    let view=decode(source.as_bytes(),&context()).unwrap();
    assert_eq!(view.loop_mode(),LoopMode::Repeat); assert_eq!(view.ticks_per_second(),24);
    assert_eq!(view.onion(),(1,2,0.35));
    let track=view.track_at(0).unwrap(); assert_eq!(track.id,b"walk"); assert_eq!(track.name,Some(&b"walk"[..]));
    let item=view.item_at(0,0).unwrap(); assert_eq!(item.tick,0); assert_eq!(item.asset,Some(&b"cel0"[..]));
    assert_eq!(&source.as_bytes()[item.source.start..item.source.end],br#"{"tick":0,"asset":"cel0","label":"key pose"}"#);
    assert_eq!(view.item_at(0,1),None);
}

#[test]
fn property_track_iteration_and_raw_value_spans_are_preserved() {
    let source=base(r#"[{"id":"opacity","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":1},{"tick":12,"value":0.25}]}]"#);
    let view=decode(source.as_bytes(),&context()).unwrap(); let track=view.track_at(0).unwrap();
    assert_eq!(track.property,Some(&b"opacity"[..])); assert_eq!(track.item_count,2);
    let mut it=TimelineIterator::default();
    let first=view.item_next(0,&mut it).unwrap(); let second=view.item_next(0,&mut it).unwrap();
    assert_eq!(first.value_json,Some(&b"1"[..])); assert_eq!(second.tick,12); assert_eq!(second.value_json,Some(&b"0.25"[..]));
    assert_eq!(view.item_next(0,&mut it),None); assert_eq!(it.next_index,2);
}

#[test]
fn iterator_is_bound_to_one_track_until_reset() {
    let source=base(r#"[{"id":"a","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":1}]},{"id":"b","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":0}]}]"#);
    let view=decode(source.as_bytes(),&context()).unwrap(); let mut it=TimelineIterator::default();
    assert_eq!(view.item_next(0,&mut it).unwrap().tick,0);
    assert_eq!(view.item_next(1,&mut it),None);
    it=TimelineIterator::default(); assert_eq!(view.item_next(1,&mut it).unwrap().value_json,Some(&b"0"[..]));
}

#[test]
fn escaped_keys_surrogates_and_duplicate_decoded_fields() {
    let escaped=base(r#"[{"id":"wa\u006bk","kind":"draw\u0069ng","target":"lay\u0065r1","visible":true,"cels":[{"tick":0,"asset":"ce\u006c0"}]}]"#);
    let view=decode(escaped.as_bytes(),&context()).unwrap(); assert_eq!(view.track_at(0).unwrap().id,b"wa\\u006bk");
    assert!(classify(escaped.as_bytes()).unwrap()); assert!(!classify(b"{\"schema\":\"future\",\"version\":1}").unwrap());
    let dup=base(r#"[]"#).replace("\"numerator\":24", "\"numerator\":24,\"num\\u0065rator\":24");
    let duplicate=decode(dup.as_bytes(),&context()).unwrap_err();
    assert_eq!(duplicate.error,Error::Duplicate);
    assert_eq!(duplicate.offset,dup.find("\"num\\u0065rator\"").unwrap());
    let malformed_surrogate=br#""\uD800\u0041""#;
    let surrogate=decode(malformed_surrogate,&context()).unwrap_err();
    assert_eq!(surrogate.error,Error::MalformedJson);
    assert_eq!(surrogate.offset,malformed_surrogate.len()-1);
    assert_eq!(decode(br#""\uD83D\uDE00""#,&context()).unwrap_err().error,Error::InvalidSchema);
    let mut output=[0u8;8]; let count=string_copy(br#"a\u0062\uD83D\uDE00"#,&mut output).unwrap();
    assert_eq!(&output[..count],"ab😀".as_bytes());
    assert_eq!(string_copy(b"\\u0061",&mut [0u8;0]),None);
    assert_eq!(string_copy(br#"\uD800"#,&mut output),None);
    let oversized_schema=format!("{{\"schema\":\"{}\"}}","x".repeat(4097));
    assert_eq!(decode(oversized_schema.as_bytes(),&context()).unwrap_err().error,Error::Limit);
}

#[test]
fn string_copy_enforces_decoded_byte_limit_with_large_output() {
    let valid=vec![b'a';4096]; let too_long=vec![b'a';4097];
    let mut valid_output=vec![0;8192]; let mut long_output=vec![0;8192];
    assert_eq!(string_copy(&valid,&mut valid_output),Some(4096));
    assert_eq!(string_copy(&too_long,&mut long_output),None);
}

#[test]
fn exact_error_categories_and_offsets() {
    let json=base("[]").replace("\"ticks_per_second\":24","\"ticks_per_second\":0");
    let result=decode(json.as_bytes(),&context()).unwrap_err();
    assert_eq!(result.error,Error::InvalidValue);
    assert_eq!(result.offset,json.find("\"ticks_per_second\":0").unwrap()+"\"ticks_per_second\":".len());
    assert_eq!(decode(b"{\"schema\":",&context()).unwrap_err().error,Error::MalformedJson);
    assert_eq!(decode(b"{\"schema\":\"future.timeline\",\"version\":2}",&context()).unwrap_err().error,Error::UnsupportedVersion);
    assert_eq!(decode(b"{}",&context()).unwrap_err().error,Error::UnsupportedVersion);
    assert_eq!(decode(base("[]").replace("\"tracks\":[]","\"tracks\":[],\"extra\":true").as_bytes(),&context()).unwrap_err().error,Error::InvalidSchema);
}

#[test]
fn schema_property_value_reference_and_tick_rules() {
    let invalid=base(r#"[{"id":"bad","kind":"property","target":"shape1","visible":true,"property":"nope","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":1}]}]"#);
    assert_eq!(decode(invalid.as_bytes(),&context()).unwrap_err().error,Error::InvalidValue);
    let missing=base(r#"[{"id":"a","kind":"drawing","target":"layer1","visible":true,"cels":[{"tick":0,"asset":"absent"}]}]"#);
    assert_eq!(decode(missing.as_bytes(),&context()).unwrap_err().error,Error::InvalidReference);
    let wrong_media=base(r#"[{"id":"a","kind":"drawing","target":"layer1","visible":true,"cels":[{"tick":0,"asset":"pixel.png"}]}]"#);
    assert_eq!(decode(wrong_media.as_bytes(),&context()).unwrap_err().error,Error::InvalidReference);
    let duplicate_track=base(r#"[{"id":"a","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":0}]},{"id":"a","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":1}]}]"#);
    assert_eq!(decode(duplicate_track.as_bytes(),&context()).unwrap_err().error,Error::Duplicate);
    let duplicate=base(r#"[{"id":"a","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":0},{"tick":0,"value":1}]}]"#);
    assert_eq!(decode(duplicate.as_bytes(),&context()).unwrap_err().error,Error::Duplicate);
    let rgba=base(r#"[{"id":"color","kind":"property","target":"shape1","visible":true,"property":"fill","value_type":"rgba","interpolation":"step","keys":[{"tick":0,"value":[1.1,0,0,1]}]}]"#);
    assert_eq!(decode(rgba.as_bytes(),&context()).unwrap_err().error,Error::InvalidValue);
    let supported=base(r#"[{"id":"color","kind":"property","target":"shape1","visible":true,"property":"fill","value_type":"rgba","interpolation":"step","keys":[{"tick":0,"value":[1,0.5,0,1]}]},{"id":"move","kind":"property","target":"shape1","visible":true,"property":"transform","value_type":"transform2d","interpolation":"linear","keys":[{"tick":0,"value":[1,0,0,1,2,3]}]},{"id":"visible","kind":"property","target":"shape1","visible":true,"property":"visibility","value_type":"boolean","interpolation":"step","keys":[{"tick":0,"value":true}]}]"#);
    assert_eq!(decode(supported.as_bytes(),&context()).unwrap().track_count(),3);
    let bad_boolean=base(r#"[{"id":"visible","kind":"property","target":"shape1","visible":true,"property":"visibility","value_type":"boolean","interpolation":"linear","keys":[{"tick":0,"value":true}]}]"#);
    let failure=decode(bad_boolean.as_bytes(),&context()).unwrap_err();
    assert_eq!(failure.error,Error::InvalidValue);
    assert_eq!(failure.offset,bad_boolean.find("\"linear\"").unwrap());
}

#[test]
fn bounded_context_and_empty_source_cleanup() {
    let empty=decode(b"",&context()).unwrap(); assert!(empty.is_empty()); assert_eq!(empty.track_count(),0);
    let duplicate=TimelineContext{scene_ids:vec![b"a".to_vec(),b"a".to_vec()],assets:vec![]};
    assert_eq!(decode(b"",&duplicate).unwrap_err().error,Error::Duplicate);
    let unsorted=TimelineContext{scene_ids:vec![b"z".to_vec(),b"a".to_vec()],assets:vec![]};
    assert_eq!(decode(b"",&unsorted).unwrap_err().error,Error::InvalidArgument);
    let invalid_assets=TimelineContext{scene_ids:vec![],assets:vec![AssetRef{name:b"../cel".to_vec(),media_type:b"image/svg+xml".to_vec()}]};
    assert_eq!(decode(b"",&invalid_assets).unwrap_err().error,Error::InvalidArgument);
}

#[test]
fn cancellation_stops_long_source_scan_and_reports_owned_cleanup_state() {
    let mut source=base("[]").into_bytes(); source.extend(std::iter::repeat_n(b' ',5000));
    let mut calls=0; let result=decode_cancellable(&source,&context(),Some(||{calls+=1;calls==1}));
    let failure=result.unwrap_err(); assert!(failure.canceled); assert_eq!(failure.error,Error::InvalidArgument); assert_eq!(calls,1);
    // Returned failures own no partial view and the caller can immediately reuse the bytes.
    assert_eq!(decode(base("[]").as_bytes(),&context()).unwrap().track_count(),0);
}

#[test]
fn cancellation_is_polled_during_later_validation_rescans() {
    let mut source=String::from(r#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":14400000,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":1},"tracks":[{"id":"a","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":["#);
    for tick in 0..6000 {
        if tick>0 {source.push(',');}
        write!(&mut source,"{{\"tick\":{tick},\"value\":1}}").unwrap();
    }
    source.push_str("]}]} ");
    let source_bytes=source.as_bytes();
    let full_pass_poll_bound=source_bytes.len().div_ceil(4096);
    let cancel_on=2*full_pass_poll_bound+1;
    let mut polls=0usize;
    let failure=decode_cancellable(source_bytes,&context(),Some(||{polls+=1;polls==cancel_on})).unwrap_err();
    assert!(failure.canceled); assert_eq!(failure.error,Error::InvalidArgument);
    assert_eq!(polls,cancel_on); assert_eq!(failure.offset,131072);
}

#[test]
fn per_track_limit_and_large_sequential_iteration_are_bounded() {
    let prefix=r#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":14400000,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":1},"tracks":[{"id":"a","kind":"property","target":"shape1","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":["#;
    let mut source=String::with_capacity(prefix.len()+MAX_ITEMS_PER_TRACK*22);
    source.push_str(prefix);
    for tick in 0..MAX_ITEMS_PER_TRACK { if tick!=0 {source.push(',');} source.push_str(&format!("{{\"tick\":{tick},\"value\":1}}")); }
    source.push_str("]}]}");
    let view=decode(source.as_bytes(),&context()).unwrap(); assert_eq!(view.track_at(0).unwrap().item_count,MAX_ITEMS_PER_TRACK);
    let mut it=TimelineIterator::default(); let mut count=0;
    while let Some(item)=view.item_next(0,&mut it) { assert_eq!(item.tick,count as u32); count+=1; }
    assert_eq!(count,MAX_ITEMS_PER_TRACK);
    let over=source.replace("]}]}",",{\"tick\":100000,\"value\":1}]}]}");
    assert_eq!(decode(over.as_bytes(),&context()).unwrap_err().error,Error::Limit);
}

#[test]
fn track_count_limit_is_exact() {
    fn tracks(count:usize)->String {
        let mut out=String::new();
        for index in 0..count {
            if index>0 {out.push(',');}
            write!(&mut out,"{{\"id\":\"t{index}\",\"kind\":\"property\",\"target\":\"shape1\",\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\",\"interpolation\":\"linear\",\"keys\":[{{\"tick\":0,\"value\":1}}]}}").unwrap();
        }
        out
    }
    let valid=base(&format!("[{}]",tracks(256)));
    assert_eq!(decode(valid.as_bytes(),&context()).unwrap().track_count(),256);
    let over=base(&format!("[{}]",tracks(257)));
    assert_eq!(decode(over.as_bytes(),&context()).unwrap_err().error,Error::Limit);
}

#[test]
fn total_item_limit_covers_items_across_tracks() {
    let mut source=String::from(r#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":14400000,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":1},"tracks":["#);
    for track in 0..11 {
        if track>0 {source.push(',');}
        write!(&mut source,"{{\"id\":\"t{track}\",\"kind\":\"property\",\"target\":\"shape1\",\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\",\"interpolation\":\"linear\",\"keys\":[").unwrap();
        for tick in 0..MAX_ITEMS_PER_TRACK {
            if tick>0 {source.push(',');}
            write!(&mut source,"{{\"tick\":{tick},\"value\":1}}").unwrap();
        }
        source.push_str("]}");
    }
    source.push_str("]}");
    let failure=decode(source.as_bytes(),&context()).unwrap_err();
    assert_eq!(failure.error,Error::Limit);
}

#[test]
fn source_limit_is_checked_before_parse() {
    // Keep physical allocation compact while exercising the exact boundary check.
    let over=vec![b' ';MAX_SOURCE_BYTES+1];
    let failure=decode(&over,&context()).unwrap_err(); assert_eq!(failure.error,Error::Limit); assert_eq!(failure.offset,0);
}
