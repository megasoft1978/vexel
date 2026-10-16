mod support;
use support::*;
use std::cell::RefCell;
use vexel_base::WorkControl;
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_svg_media::*;
use vexel_scene::TextCache;

// Fixture-only explicit common-style adapter. It executes solid paint, alpha
// rounding, stroke, real clip and real group publication; not a native painter.
#[derive(Default)]
struct Publisher { calls: RefCell<Vec<&'static str>>, fail: Option<&'static str> }
fn error(kind: ErrorKind, offset: usize) -> Diagnostic { Diagnostic { kind, offset } }
fn opacity(c: &PublicationContext<'_, '_>, p: Property) -> Result<f64> {
    let Some(v) = c.style.value(p) else { return Ok(1.); };
    let percent = v.bytes.last() == Some(&b'%');
    let end = v.span.offset + v.span.length - usize::from(percent);
    let n = vexel_svg_values::parse_number(c.element.source(), Span { start: v.span.offset, end })
        .map_err(|_| error(ErrorKind::Number, v.span.offset))? / if percent {100.} else {1.};
    if !(0. ..=1.).contains(&n) { return Err(error(ErrorKind::Number, v.span.offset)); }
    Ok(n)
}
fn color(c: &PublicationContext<'_, '_>, p: Property, default: [u8;4]) -> Result<[u8;4]> {
    let Some(mut v) = c.style.value(p) else { return Ok(default); };
    if v.bytes.eq_ignore_ascii_case(b"none") { return Ok([0;4]); }
    if v.bytes.eq_ignore_ascii_case(b"currentColor") {
        let Some(current) = c.style.value(Property::Color) else { return Ok([0,0,0,255]); };
        v = current;
    }
    vexel_svg_color::parse_color(c.element.source(), Span::from_offset_length(v.span.offset, v.span.length).unwrap())
        .ok_or_else(|| error(ErrorKind::Unsupported, v.span.offset))
}
impl Publisher {
    fn step(&self, stage: &'static str, c: &PublicationContext<'_, '_>) -> Result<()> {
        self.calls.borrow_mut().push(stage);
        if self.fail == Some(stage) { return Err(error(ErrorKind::Unsupported, c.source_map.element.start)); }
        Ok(())
    }
}
impl StylePublisher for Publisher {
    fn begin(&self, c: &PublicationContext<'_, '_>, _: &mut SceneBuilder,
        _: &DraftToken, _: &mut dyn WorkControl) -> Result<()> {
        self.step("begin", c)?;
        opacity(c, Property::Opacity)?;
        Ok(())
    }
    fn prepare_geometry(&self, c: &PublicationContext<'_, '_>, _: &mut SceneBuilder,
        _: &DraftToken, _: &mut dyn WorkControl) -> Result<()> {
        self.step("prepare", c)?;
        color(c, Property::Fill, [0,0,0,255])?;
        if let Some(v) = c.style.value(Property::FillRule) {
            if v.bytes != b"evenodd" && v.bytes != b"nonzero" {
                return Err(error(ErrorKind::Unsupported, v.span.offset));
            }
        }
        Ok(())
    }
    fn prepare_admitted_geometry(&self, c: &PublicationContext<'_, '_>, _: &mut SceneBuilder,
        _: &DraftToken, _: &mut dyn WorkControl) -> Result<()> {
        opacity(c, Property::FillOpacity)?;
        color(c, Property::Stroke, [0;4])?;
        opacity(c, Property::StrokeOpacity)?;
        Ok(())
    }
    fn publish(&self, c: &PublicationContext<'_, '_>, g: CustomGeometry,
        b: &mut SceneBuilder, d: &DraftToken, control: &mut dyn WorkControl) -> Result<ShapeId> {
        self.step("publish", c)?;
        assert_eq!(g.source_map, c.source_map);
        assert!(!g.path.edges.is_empty());
        let mut rgba = color(c, Property::Fill, [0,0,0,255])?;
        rgba[3] = (f64::from(rgba[3]) * opacity(c, Property::FillOpacity)?).round() as u8;
        if c.kind == MediaKind::Image { rgba[3] = 0; assert!(g.image.is_some()); }
        let mut shape = Shape::new(g.source_map);
        shape.image = g.image;
        shape.evenodd = c.style.keyword_equals(Property::FillRule, b"evenodd");
        shape.stroke.rgba = color(c, Property::Stroke, [0;4])?;
        shape.stroke.rgba[3] = (f64::from(shape.stroke.rgba[3]) * opacity(c, Property::StrokeOpacity)?).round() as u8;
        shape.stroke.width = 2.;
        if c.kind == MediaKind::Image && c.style.value(Property::ClipPath).is_some() {
            let clip = Clip { geometry: ClipGeometry { edges: g.path.edges.clone(),
                shapes: vec![ClipChild { edges: TableRange { first: 0, count: g.path.edges.len() },
                    evenodd: false, filled: true }] }, parent: None, owner_node: g.source_map.transform_node,
                owner_before_suffix: false, object_bounding_box: false,
                bounds: [0.,0.,100.,100.], user_inverse: vexel_geometry::affine_identity() };
            shape.clip = Some(b.append_clip(c.scene, clip, control)?);
        }
        let colors = vec![rgba; g.path.edges.len()];
        let id = b.append_admitted_shape(d, &g.admission, ShapeInput { shape, path: g.path, edge_colors: colors }, control)?;
        let shape = *b.scene(c.scene)?.shape(id).unwrap();
        b.finalize_shape(d, id, shape, control)?;
        Ok(id)
    }
    fn finish(&self, c: &PublicationContext<'_, '_>, first: usize, b: &mut SceneBuilder,
        d: &DraftToken, control: &mut dyn WorkControl) -> Result<()> {
        if c.kind == MediaKind::Text && c.style.value(Property::ClipPath).is_some()
            && first < b.scene(c.scene)?.shapes.len() {
            let scene=b.scene(c.scene)?;
            let start=scene.shapes[first].edges.first;
            let edges=scene.edges[start..].to_vec();
            let count=edges.len();
            let clip=b.append_clip(c.scene,Clip {geometry:ClipGeometry {edges,
                shapes:vec![ClipChild{edges:TableRange{first:0,count},evenodd:false,filled:true}]},
                parent:None,owner_node:c.source_map.transform_node,owner_before_suffix:false,
                object_bounding_box:false,bounds:[0.,0.,100.,100.],
                user_inverse:vexel_geometry::affine_identity()},control)?;
            let end=b.scene(c.scene)?.shapes.len();
            for index in first..end {
                let scene=b.scene(c.scene)?;
                let id=scene.shape_id(index).unwrap();
                let mut shape=*scene.shape(id).unwrap();shape.clip=Some(clip);
                b.finalize_shape(d, id,shape,control)?;
            }
        }
        // Real group append is deliberately before an injected finish failure.
        let count = b.scene(c.scene)?.shapes.len() - first;
        if c.style.value(Property::Opacity).is_some() {
            b.append_group(c.scene, OpacityGroup { shapes: TableRange { first, count }, parent: None,
                opacity: opacity(c, Property::Opacity)?, filter: None, mask: None,
                blend_mode: 0, isolation: false }, control)?;
        }
        self.step("finish", c)
    }
}

fn image(source: &[u8], publisher: &Publisher, provider: &mut Provider) -> (Result<Option<ShapeId>>, Fixture) {
    let mut f = Fixture::new(source, Limits::default());
    let index = f.index(b"image");
    let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
    let style = shared.compute_style(ElementId(index), None, &mut Run).unwrap();
    let element = view(&f.lex, index);
    let map = f.map(index);
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    let result = publish_image_request(ImageDecodeRequest { element, source_map: map, style: &style,
        transform: vexel_geometry::affine_identity(), provider, scene, ledger: &mut ledger },
        publisher, &mut f.builder, &draft, &mut Run);
    f.builder.commit_draft(&draft, &mut Run).unwrap();
    (result, f)
}

#[test]
fn empty_payload_is_unsupported_at_href_start_and_position_cap_is_number_at_excess_item() {
    for uri in [b"data:image/png;base64,".as_slice(), b"data:image/jpeg;base64,"] {
        let mut source = b"prefix ".to_vec(); source.extend_from_slice(uri);
        let e = parse_data_uri(&source, vexel_xml::XmlSpan { offset: 7, length: uri.len() }).unwrap_err();
        assert_eq!((e.kind,e.offset), (ErrorKind::Unsupported,7));
    }
    let mut source = b"pad ".to_vec();
    for _ in 0..MAX_POSITION_VALUES { source.extend_from_slice(b"1 "); }
    source.extend_from_slice(b"2");
    let e = parse_position_list(&source, vexel_xml::XmlSpan { offset: 4, length: source.len()-4 }, 100.).unwrap_err();
    assert_eq!((e.kind,e.offset), (ErrorKind::Number, 4 + MAX_POSITION_VALUES*2));
}

#[test]
fn image_hidden_and_display_none_skip_geometry_callbacks_and_provider_even_with_bad_media_and_style() {
    for style in ["visibility:hidden;fill:bogus", "visibility:collapse", "display:none;visibility:bogus"] {
        let source = format!("<svg><image style=\"{style}\" href=\"bad\"/></svg>");
        let p = Publisher::default(); let mut provider = Provider::default();
        let (result, f) = image(source.as_bytes(), &p, &mut provider);
        assert_eq!(result.unwrap(), None);
        assert_eq!(provider.image_calls,0);
        assert_eq!(&*p.calls.borrow(), if style.starts_with("display") {&[][..]}else{&["begin","finish"][..]});
        assert!(f.builder.scene(f.builder.root_id()).unwrap().shapes.is_empty());
    }
}

#[test]
fn image_style_errors_precede_media_provider_and_keep_exact_property_offsets() {
    for (style, bad, kind) in [("visibility:bogus", "bogus", ErrorKind::Unsupported),
        ("display:bogus", "bogus", ErrorKind::Unsupported),
        ("opacity:bad;fill:bogus", "bad", ErrorKind::Number),
        ("opacity:bad;visibility:bogus", "bad", ErrorKind::Number),
        ("fill:bogus;fill-opacity:bad", "bogus", ErrorKind::Unsupported),
        ("fill-rule:bad;fill-opacity:bogus", "bad", ErrorKind::Unsupported),
        ("fill-opacity:bad;stroke:bogus", "bad", ErrorKind::Number)] {
        let source = format!("<svg><image style=\"{style}\" href=\"bad\"/></svg>");
        let p = Publisher::default(); let mut provider = Provider::default();
        let (r,f) = image(source.as_bytes(), &p, &mut provider);
        let e = r.unwrap_err();
        assert_eq!((e.kind,e.offset),(kind,source.find(bad).unwrap()));
        assert_eq!(provider.image_calls,0);
        assert!(f.builder.scene(f.builder.root_id()).unwrap().images.is_empty());
    }
}

#[test]
fn image_actual_style_publishes_colors_stroke_clip_opacity_and_host_data() {
    let source = br##"<svg><image width="8" height="4" style="color:red;fill:currentColor;fill-rule:evenodd;fill-opacity:50%;stroke:blue;stroke-opacity:.5;opacity:.4;clip-path:url(#c)" href="data:image/png;base64,AQID"/></svg>"##;
    let p = Publisher::default(); let mut provider = Provider::default();
    let (r,f) = image(source, &p, &mut provider); let id=r.unwrap().unwrap();
    let s=f.builder.scene(f.builder.root_id()).unwrap();
    let shape=s.shape(id).unwrap();
    assert!(shape.evenodd); assert_eq!(shape.stroke.rgba,[0,0,255,128]);
    assert_eq!(shape.stroke.width,2.); assert!(shape.clip.is_some());
    assert_eq!(s.edge_colors,[[255,0,0,0];4]);
    assert_eq!(s.images.len(),1); assert_eq!(s.clips.len(),1);
    assert_eq!(s.groups[0].opacity,0.4); assert_eq!(s.groups[0].shapes,TableRange{first:0,count:1});
    assert_eq!(&*p.calls.borrow(), &["begin","prepare","publish","finish"]);
    assert_eq!(provider.image_calls,1);
}

#[test]
fn display_none_text_skips_layout_and_bad_font_before_visibility() {
    for style in ["display:none;font-size:bad;visibility:hidden", "display:none;visibility:bad"] {
        let source = format!("<svg><text style=\"{style}\">A</text></svg>");
        let mut f=Fixture::new(source.as_bytes(),Limits::default()); let i=f.index(b"text");
        let mut shared=DecodeSource::new(&f.lex,&mut Run).unwrap();
        let style=shared.compute_style(ElementId(i),None,&mut Run).unwrap();
        let element=view(&f.lex,i); let map=f.map(i); let scene=f.builder.root_id();
        let draft=f.builder.begin_draft(scene).unwrap(); let mut state=TextState::default();
        let mut cache=TextCache::default(); let mut provider=Provider::default();
        let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
        let p=Publisher::default();
        assert!(!publish_text_request(TextDecodeRequest{element,source_map:map,end:element.end,
            style:&style,transform:map.transform,state:&mut state,preserve_space:false,
            provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
            &mut f.builder,&draft,&mut Run).unwrap());
        assert!(provider.seen.is_empty());
        assert_eq!(&*p.calls.borrow(), if style.keyword_equals(Property::Display,b"none") {&[][..]}else{&["begin","finish"][..]});
        assert_eq!(state.text_bytes,0); assert_eq!(f.builder.document_text_commands(),0);
        f.builder.commit_draft(&draft,&mut Run).unwrap();
    }
}

#[test]
fn text_actual_solid_style_rounds_alpha_and_callbacks_follow_layout_positions() {
    let source=br##"<svg><text style="fill:red;fill-opacity:.5;stroke:blue;stroke-opacity:25%;opacity:.6;fill-rule:evenodd;clip-path:url(#fixture)">AB</text></svg>"##;
    let mut f=Fixture::new(source,Limits::default()); let i=f.index(b"text");
    let mut shared=DecodeSource::new(&f.lex,&mut Run).unwrap();
    let style=shared.compute_style(ElementId(i),None,&mut Run).unwrap();
    let element=view(&f.lex,i); let map=f.map(i); let scene=f.builder.root_id();
    let draft=f.builder.begin_draft(scene).unwrap(); let mut state=TextState::default();
    let mut cache=TextCache::default(); let mut provider=Provider::default();
    let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
    let p=Publisher::default();
    publish_text_request(TextDecodeRequest{element,source_map:map,end:element.end,
        style:&style,transform:map.transform,state:&mut state,preserve_space:false,
        provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
        &mut f.builder,&draft,&mut Run).unwrap();
    let s=f.builder.scene(scene).unwrap();
    assert_eq!(s.shapes.len(),1); assert_eq!(s.path_subpaths.len(),2);
    assert!(s.shapes[0].clip.is_some());assert_eq!(s.clips.len(),1);
    assert!(s.shapes[0].evenodd); assert_eq!(s.shapes[0].stroke.rgba,[0,0,255,64]);
    assert!(s.edge_colors.iter().all(|&v|v==[255,0,0,128]));
    assert!((s.groups[0].opacity-0.6).abs()<1e-15); assert_eq!(s.shapes[0].source,map);
    assert_eq!(&*p.calls.borrow(), &["begin","prepare","publish","finish"]);
    assert_eq!(provider.seen.len(),1); assert_eq!(state.glyph_count,2);
    f.builder.commit_draft(&draft,&mut Run).unwrap();
}

#[test]
fn text_font_then_layout_then_position_errors_precede_custom_fill_errors() {
    for (attrs,expected,provider_calls) in [
        ("style=\"font-size:bad;fill:bogus\"",ErrorKind::Unsupported,0),
        ("style=\"font-size:bad;visibility:bogus\"",ErrorKind::Unsupported,0),
        ("x=\"bad\" style=\"fill:bogus\"",ErrorKind::Number,1),
        ("style=\"fill:bogus\"",ErrorKind::Unsupported,1)] {
        let source=format!("<svg><text {attrs}>A</text></svg>");
        let mut f=Fixture::new(source.as_bytes(),Limits::default()); let i=f.index(b"text");
        let mut shared=DecodeSource::new(&f.lex,&mut Run).unwrap();
        let style=shared.compute_style(ElementId(i),None,&mut Run).unwrap();
        let element=view(&f.lex,i);let map=f.map(i);let scene=f.builder.root_id();
        let draft=f.builder.begin_draft(scene).unwrap();let mut state=TextState::default();
        let mut cache=TextCache::default();let mut provider=Provider::default();
        let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
        let p=Publisher::default();
        let e=publish_text_request(TextDecodeRequest{element,source_map:map,end:element.end,
            style:&style,transform:map.transform,state:&mut state,preserve_space:false,
            provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
            &mut f.builder,&draft,&mut Run).unwrap_err();
        assert_eq!(e.kind,expected);
        assert_eq!(e.offset,source.find(if attrs.contains("bad") {"bad"}else{"bogus"}).unwrap());
        assert_eq!(provider.seen.len(),provider_calls);
        assert_eq!(state.text_bytes,0);assert!(cache.entries.is_empty());
        assert_eq!(f.builder.document_text_commands(),0);
        assert!(f.builder.scene(scene).unwrap().shapes.is_empty());
        f.builder.commit_draft(&draft,&mut Run).unwrap();
    }
}

#[test]
fn both_production_routes_restore_real_prefix_and_all_admissions_on_each_callback_failure() {
    for is_text in [false,true] { for stage in ["begin","prepare","publish","finish"] {
        let source=br##"<svg><path d="M0 0L1 0L1 1Z"/><image width="8" height="4" style="opacity:.5;clip-path:url(#c)" href="data:image/png;base64,AQID"/><text style="opacity:.5;clip-path:url(#fixture)">A<![CDATA[B]]></text></svg>"##;
        let mut f=Fixture::new(source,Limits::default());
        let i=f.index(if is_text{b"text"}else{b"image"});
        let mut shared=DecodeSource::new(&f.lex,&mut Run).unwrap();
        let style=shared.compute_style(ElementId(i),None,&mut Run).unwrap();
        let element=view(&f.lex,i);let map=f.map(i);let scene=f.builder.root_id();
        let draft=f.builder.begin_draft(scene).unwrap();
        let prefix=f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
        let old_shape=*f.builder.scene(scene).unwrap().shape(prefix).unwrap();
        let old_edges=f.builder.scene(scene).unwrap().edges.to_vec();
        let mut state=TextState::default(); state.cursor_x=11.;
        let before=state;
        let mut cache=TextCache::default();let mut provider=Provider::default();
        let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
        let p=Publisher{fail:Some(stage),..Default::default()};
        let result=if is_text {
            publish_text_request(TextDecodeRequest{element,source_map:map,end:element.end,
                style:&style,transform:map.transform,state:&mut state,preserve_space:false,
                provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
                &mut f.builder,&draft,&mut Run).map(|_|())
        }else{
            publish_image_request(ImageDecodeRequest{element,source_map:map,style:&style,
                transform:map.transform,provider:&mut provider,scene,ledger:&mut ledger},&p,
                &mut f.builder,&draft,&mut Run).map(|_|())
        };
        assert_eq!(result.unwrap_err(),error(ErrorKind::Unsupported,map.element.start));
        let s=f.builder.scene(scene).unwrap();
        assert_eq!(s.shapes.len(),1); assert_eq!(s.shape(prefix),Some(&old_shape));
        assert_eq!(s.edges,old_edges); assert!(s.images.is_empty());
        assert!(s.clips.is_empty());assert!(s.groups.is_empty());
        assert_eq!((state.cursor_x,state.text_bytes,state.glyph_count),
            (before.cursor_x,before.text_bytes,before.glyph_count));
        assert!(cache.entries.is_empty());assert_eq!((cache.commands,cache.text_bytes),(0,0));
        assert_eq!(f.builder.document_text_commands(),0);
        // No caller rollback: the same real draft accepts another actual shape.
        f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
        f.builder.commit_draft(&draft,&mut Run).unwrap();
    }}
}

struct Interrupt { at: usize, calls: usize, stop: vexel_base::Stop }
impl WorkControl for Interrupt {
    fn checkpoint(&mut self) -> std::result::Result<(),vexel_base::Stop> {
        self.calls+=1; if self.calls==self.at {Err(self.stop)}else{Ok(())}
    }
    fn charge(&mut self, _: vexel_base::WorkKind, _: u64) -> std::result::Result<(),vexel_base::Stop> {
        self.checkpoint()
    }
}
#[test]
fn styled_routes_cancel_at_every_control_boundary_including_real_clip_group_and_commit() {
    for is_text in [false,true] {
        let mut boundaries=0;
        for stop in [None,Some(vexel_base::Stop::Canceled),Some(vexel_base::Stop::Limit),Some(vexel_base::Stop::Overflow)] {
            // First run counts successful boundaries, then interrupt every one.
            let points = if stop.is_none() {vec![usize::MAX]}else{(1..=boundaries).collect()};
            let stop=stop.unwrap_or(vexel_base::Stop::Canceled);
            for at in points {
                let source=br##"<svg><path d="M0 0L1 0L1 1Z"/><image width="8" height="4" style="opacity:.5;clip-path:url(#c)" href="data:image/png;base64,AQID"/><text style="opacity:.5;clip-path:url(#fixture)">A<![CDATA[B]]></text></svg>"##;
                let mut f=Fixture::new(source,Limits::default());let i=f.index(if is_text{b"text"}else{b"image"});
                let mut shared=DecodeSource::new(&f.lex,&mut Run).unwrap();
                let style=shared.compute_style(ElementId(i),None,&mut Run).unwrap();
                let element=view(&f.lex,i);let map=f.map(i);let scene=f.builder.root_id();
                let draft=f.builder.begin_draft(scene).unwrap();
                let prefix=f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
                let prefix_shape=*f.builder.scene(scene).unwrap().shape(prefix).unwrap();
                let mut state=TextState::default(); let mut cache=TextCache::default();
                let mut provider=Provider::default();let p=Publisher::default();
                let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
                let mut control=Interrupt{at,calls:0,stop};
                let result=if is_text {
                    publish_text_request(TextDecodeRequest{element,source_map:map,end:element.end,
                        style:&style,transform:map.transform,state:&mut state,preserve_space:false,
                        provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
                        &mut f.builder,&draft,&mut control).map(|_|())
                }else{
                    publish_image_request(ImageDecodeRequest{element,source_map:map,style:&style,
                        transform:map.transform,provider:&mut provider,scene,ledger:&mut ledger},&p,
                        &mut f.builder,&draft,&mut control).map(|_|())
                };
                if at==usize::MAX {result.unwrap();boundaries=control.calls;continue;}
                assert_eq!(result.unwrap_err().kind,match stop{
                    vexel_base::Stop::Canceled=>ErrorKind::Canceled,
                    vexel_base::Stop::Limit=>ErrorKind::Limit(LimitKind::Work),
                    vexel_base::Stop::Overflow=>ErrorKind::Limit(LimitKind::Arithmetic)});
                let s=f.builder.scene(scene).unwrap();
                assert_eq!(s.shapes.len(),1);assert_eq!(s.shape(prefix),Some(&prefix_shape));
                assert_eq!(s.edges.len(),prefix_shape.edges.count);
                assert!(s.images.is_empty());assert!(s.clips.is_empty());assert!(s.groups.is_empty());
                assert_eq!((state.text_bytes,state.glyph_count,state.cursor_x),(0,0,0.));
                assert_eq!((cache.entries.len(),cache.text_bytes,cache.commands),(0,0,0));
                assert_eq!(f.builder.document_text_commands(),0);
                f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
                f.builder.commit_draft(&draft,&mut Run).unwrap();
            }
        }
        assert!(boundaries>20);
    }
}

#[test]
fn typed_diagnostics_use_real_href_and_overcap_position_offsets_and_restore_text_admissions() {
    let source=b"<svg><image width=\"2\" height=\"3\" href=\"data:image/png;base64,\"/></svg>";
    let p=Publisher::default(); let mut provider=Provider::default();
    let (r,_)=image(source,&p,&mut provider);
    let offset=source.windows(5).position(|b|b==b"data:").unwrap();
    assert_eq!(r.unwrap_err(),error(ErrorKind::Unsupported,offset));assert_eq!(provider.image_calls,0);
    let mut source=b"<svg><text x=\"".to_vec();let first=source.len();
    for _ in 0..MAX_POSITION_VALUES {source.extend_from_slice(b"1 ");}
    source.extend_from_slice(b"2\">A</text></svg>");
    let mut f=Fixture::new(&source,Limits::default());let i=f.index(b"text");
    let element=view(&f.lex,i);let map=f.map(i);let scene=f.builder.root_id();
    let draft=f.builder.begin_draft(scene).unwrap();let mut state=TextState::default();
    let mut cache=TextCache::default();let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
    let style=CssPaint::initial();
    let e=publish_text_request(TextDecodeRequest{element,source_map:map,end:element.end,
        style:&style,transform:map.transform,state:&mut state,preserve_space:false,
        provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
        &mut f.builder,&draft,&mut Run).unwrap_err();
    assert_eq!(e,error(ErrorKind::Number,first+2*MAX_POSITION_VALUES));
    assert_eq!(provider.seen.len(),1);assert_eq!(state.text_bytes,0);
    assert_eq!(f.builder.document_text_commands(),0);assert!(cache.entries.is_empty());
    assert!(f.builder.scene(scene).unwrap().shapes.is_empty());
    f.builder.commit_draft(&draft,&mut Run).unwrap();
}

struct BadPublication { prefix: ShapeId, wrong_source: Option<SourceMap> }
impl StylePublisher for BadPublication {
    fn begin(&self, _: &PublicationContext<'_, '_>, _: &mut SceneBuilder,
        _: &DraftToken, _: &mut dyn WorkControl) -> Result<()> {Ok(())}
    fn prepare_geometry(&self, _: &PublicationContext<'_, '_>, _: &mut SceneBuilder,
        _: &DraftToken, _: &mut dyn WorkControl) -> Result<()> {Ok(())}
    fn prepare_admitted_geometry(&self, _: &PublicationContext<'_, '_>, _: &mut SceneBuilder,
        _: &DraftToken, _: &mut dyn WorkControl) -> Result<()> { Ok(()) }
    fn publish(&self, _c: &PublicationContext<'_, '_>, g: CustomGeometry, b: &mut SceneBuilder,
        d: &DraftToken, control: &mut dyn WorkControl) -> Result<ShapeId> {
        if let Some(source)=self.wrong_source {
            let colors=vec![[255,0,0,255];g.path.edges.len()];
            let mut shape=Shape::new(source);shape.image=g.image;
            b.append_admitted_shape(d, &g.admission,ShapeInput{shape,path:g.path,edge_colors:colors},control)
        }else{Ok(self.prefix)}
    }
    fn finish(&self, _: &PublicationContext<'_, '_>, _: usize, _: &mut SceneBuilder,
        _: &DraftToken, _: &mut dyn WorkControl) -> Result<()> {Ok(())}
}
#[test]
fn production_rejects_empty_success_prefix_handle_and_changed_host_without_caller_cleanup() {
    for wrong_source in [false,true] {
        let source=br##"<svg><path d="M0 0L1 0L1 1Z"/><image width="2" height="3" href="data:image/png;base64,AQID"/><use href="#image"/></svg>"##;
        let mut f=Fixture::new(source,Limits::default());let i=f.index(b"image");
        let map=f.map(f.index(b"use"));let wrong_map=f.map(i);
        let element=view(&f.lex,i);let scene=f.builder.root_id();
        let draft=f.builder.begin_draft(scene).unwrap();
        let prefix=f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
        let prefix_shape=*f.builder.scene(scene).unwrap().shape(prefix).unwrap();
        let p=BadPublication{prefix,wrong_source:wrong_source.then_some(wrong_map)};
        let mut provider=Provider::default();let style=CssPaint::initial();
        let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
        let e=publish_image_request(ImageDecodeRequest{element,source_map:map,style:&style,
            transform:vexel_geometry::affine_identity(),provider:&mut provider,scene,ledger:&mut ledger},
            &p,&mut f.builder,&draft,&mut Run).unwrap_err();
        assert_eq!(e,error(ErrorKind::InvalidArgument,map.element.start));
        let s=f.builder.scene(scene).unwrap();assert_eq!(s.shapes.len(),1);
        assert_eq!(s.shape(prefix),Some(&prefix_shape));assert!(s.images.is_empty());
        f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
        f.builder.commit_draft(&draft,&mut Run).unwrap();
    }
}

#[test]
fn production_delegate_receives_canonical_geometry_and_independent_use_host_annotation() {
    let source=br##"<svg><image id="image" x="2" y="3" width="8" height="4" style="fill:red;opacity:.5" href="data:image/png;base64,AQID"/><use href="#image"/></svg>"##;
    let mut f=Fixture::new(source,Limits::default());let i=f.index(b"image");
    let mut shared=DecodeSource::new(&f.lex,&mut Run).unwrap();
    let style=shared.compute_style(ElementId(i),None,&mut Run).unwrap();
    let element=view(&f.lex,i);let mut map=f.map(f.index(b"use"));
    map.transform.e=42.;
    let mut transform=vexel_geometry::affine_identity();transform.e=7.;
    let scene=f.builder.root_id();let draft=f.builder.begin_draft(scene).unwrap();
    let mut provider=Provider::default();let p=Publisher::default();
    let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
    let id=publish_image_request(ImageDecodeRequest{element,source_map:map,style:&style,
        transform,provider:&mut provider,scene,ledger:&mut ledger},&p,
        &mut f.builder,&draft,&mut Run).unwrap().unwrap();
    let s=f.builder.scene(scene).unwrap();assert_eq!(s.shape(id).unwrap().source,map);
    assert_eq!(s.edges[0].a.x,9.);assert_eq!(s.edges[0].a.y,3.);
    assert_eq!(s.edge_colors,[[255,0,0,0];4]);assert_eq!(s.groups[0].opacity,0.5);
    f.builder.commit_draft(&draft,&mut Run).unwrap();
}

#[test]
fn production_text_provider_unsupported_precedes_custom_fill_and_restores_admissions() {
    let source=br#"<svg><text style="fill:bogus">A</text></svg>"#;
    let mut f=Fixture::new(source,Limits::default());let i=f.index(b"text");
    let mut shared=DecodeSource::new(&f.lex,&mut Run).unwrap();
    let style=shared.compute_style(ElementId(i),None,&mut Run).unwrap();
    let element=view(&f.lex,i);let map=f.map(i);let scene=f.builder.root_id();
    let draft=f.builder.begin_draft(scene).unwrap();let mut state=TextState::default();
    let mut cache=TextCache::default();let p=Publisher::default();
    let mut provider=Provider{fail_text:Some(MediaError::Unsupported),..Default::default()};
    let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
    let e=publish_text_request(TextDecodeRequest{element,source_map:map,end:element.end,
        style:&style,transform:map.transform,state:&mut state,preserve_space:false,
        provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
        &mut f.builder,&draft,&mut Run).unwrap_err();
    let offset=source.iter().position(|&v|v==b'A').unwrap();
    assert_eq!(e,error(ErrorKind::Unsupported,offset));
    assert_eq!(&*p.calls.borrow(), &["begin"]);assert_eq!(provider.seen.len(),1);
    assert_eq!((state.text_bytes,state.glyph_count),(0,0));assert!(cache.entries.is_empty());
    assert_eq!(f.builder.document_text_commands(),0);
    f.builder.commit_draft(&draft,&mut Run).unwrap();
}

#[test]
fn conflicting_plain_and_xlink_href_uses_original_xlink_span_before_provider() {
    let source=br#"<svg xmlns:xlink="http://www.w3.org/1999/xlink"><image width="2" height="3" href="data:image/png;base64,AQID" xlink:href="wrong"/></svg>"#;
    let p=Publisher::default();let mut provider=Provider::default();
    let (r,_)=image(source,&p,&mut provider);
    let offset=source.windows(5).position(|b|b==b"wrong").unwrap();
    assert_eq!(r.unwrap_err(),error(ErrorKind::Unsupported,offset));assert_eq!(provider.image_calls,0);
}

#[test]
fn production_shape_admission_precedes_image_provider_opacity_stroke_and_geometry() {
    for (attrs, expected, bad) in [
        ("width=\"bad\" style=\"fill-opacity:bad;stroke:bogus\"", ErrorKind::Limit(LimitKind::Shapes), "image"),
        ("style=\"fill-rule:bad;fill-opacity:bogus\"", ErrorKind::Unsupported, "bad"),
        ("style=\"fill:bogus;fill-rule:bad\"", ErrorKind::Unsupported, "bogus"),
    ] {
        let source=format!("<svg><path d=\"M0 0L1 0L1 1Z\"/><image {attrs} href=\"bad\"/></svg>");
        let mut f=Fixture::new(source.as_bytes(),Limits {shapes:1,..Default::default()});
        let i=f.index(b"image"); let map=f.map(i); let element=view(&f.lex,i);
        let mut decode=DecodeSource::new(&f.lex,&mut Run).unwrap();
        let style=decode.compute_style(ElementId(i),None,&mut Run).unwrap();
        let scene=f.builder.root_id();let draft=f.builder.begin_draft(scene).unwrap();
        let prefix=f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
        let before=*f.builder.scene(scene).unwrap().shape(prefix).unwrap();
        let mut provider=Provider::default();let p=Publisher::default();
        let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
        let e=publish_image_request(ImageDecodeRequest {element,source_map:map,style:&style,
            transform:map.transform,provider:&mut provider,scene,ledger:&mut ledger},&p,
            &mut f.builder,&draft,&mut Run).unwrap_err();
        assert_eq!((e.kind,e.offset),(expected,source.find(bad).unwrap()));
        assert_eq!(provider.image_calls,0);
        assert_eq!(f.builder.scene(scene).unwrap().shapes.len(),1);
        assert!(f.builder.scene(scene).unwrap().images.is_empty());
        f.builder.finalize_shape(&draft,prefix,before,&mut Run).unwrap();
        let token=f.builder.begin_request(&draft,&mut Run).unwrap();
        f.builder.commit_request(&draft,token,&mut Run).unwrap();
        f.builder.commit_draft(&draft,&mut Run).unwrap();
    }
}

struct PanicPublisher { inner: Publisher, stage: &'static str }
impl StylePublisher for PanicPublisher {
    fn begin(&self,c:&PublicationContext<'_,'_>,b:&mut SceneBuilder,d:&DraftToken,w:&mut dyn WorkControl)->Result<()> {
        self.inner.begin(c,b,d,w)
    }
    fn prepare_geometry(&self,c:&PublicationContext<'_,'_>,b:&mut SceneBuilder,d:&DraftToken,w:&mut dyn WorkControl)->Result<()> {
        self.inner.prepare_geometry(c,b,d,w)
    }
    fn prepare_admitted_geometry(&self,c:&PublicationContext<'_,'_>,b:&mut SceneBuilder,d:&DraftToken,w:&mut dyn WorkControl)->Result<()> {
        assert_ne!(self.stage,"admitted","fixture panic with live admission");
        self.inner.prepare_admitted_geometry(c,b,d,w)
    }
    fn publish(&self,c:&PublicationContext<'_,'_>,g:CustomGeometry,b:&mut SceneBuilder,d:&DraftToken,w:&mut dyn WorkControl)->Result<ShapeId> {
        assert_ne!(self.stage,"publish","fixture panic before lease consumption");
        self.inner.publish(c,g,b,d,w)
    }
    fn finish(&self,c:&PublicationContext<'_,'_>,first:usize,b:&mut SceneBuilder,d:&DraftToken,w:&mut dyn WorkControl)->Result<()> {
        assert_ne!(self.stage,"finish","fixture panic after lease consumption");
        self.inner.finish(c,first,b,d,w)
    }
}

#[test]
fn image_panics_before_and_after_consumption_restore_prefix_and_allow_successful_retry() {
    for stage in ["admitted","publish","finish"] {
        let source=br#"<svg><path d="M0 0L1 0L1 1Z"/><image width="4" height="4" href="data:image/png;base64,AQID"/></svg>"#;
        let mut f=Fixture::new(source,Limits::default());let i=f.index(b"image");
        let element=view(&f.lex,i);let mut map=f.map(i);
        // Host annotation is independent of the image's canonical geometry transform.
        map.transform.e=17.;map.transform.f=23.;
        let scene=f.builder.root_id();let draft=f.builder.begin_draft(scene).unwrap();
        let prefix=f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
        let mut provider=Provider::default();let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
        let style=CssPaint::initial();let p=PanicPublisher {inner:Publisher::default(),stage};
        let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            publish_image_request(ImageDecodeRequest {element,source_map:map,style:&style,
                transform:vexel_geometry::affine_identity(),provider:&mut provider,scene,ledger:&mut ledger},
                &p,&mut f.builder,&draft,&mut Run)
        }));
        assert!(outcome.is_err());
        let s=f.builder.scene(scene).unwrap();assert_eq!(s.shapes.len(),1);
        assert!(s.shape(prefix).is_some());assert!(s.images.is_empty());
        let id=publish_image_request(ImageDecodeRequest {element,source_map:map,style:&style,
            transform:vexel_geometry::affine_identity(),provider:&mut provider,scene,ledger:&mut ledger},
            &Publisher::default(),&mut f.builder,&draft,&mut Run).unwrap().unwrap();
        let s=f.builder.scene(scene).unwrap();assert_eq!(s.shape(id).unwrap().source,map);
        assert_eq!(s.edges[s.shape(id).unwrap().edges.first].a.x,0.);
        assert!(s.shape_geometry_source(id).is_none(),"image has no canonical path d source");
        assert_eq!(s.path_numbers.len(),6,"only the prefix has authored path numbers");
        f.builder.commit_draft(&draft,&mut Run).unwrap();
    }
}

#[test]
fn text_panics_restore_state_cache_budget_and_admissions_before_retry() {
    for stage in ["admitted","publish","finish"] {
        let source=br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A<![CDATA[B]]></text></svg>"#;
        let mut f=Fixture::new(source,Limits::default());let i=f.index(b"text");
        let element=view(&f.lex,i);let map=f.map(i);let scene=f.builder.root_id();
        let draft=f.builder.begin_draft(scene).unwrap();
        let prefix=f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
        let mut provider=Provider::default();let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
        let mut state=TextState::default();state.cursor_x=9.;
        let mut cache=TextCache::default();let style=CssPaint::initial();
        let p=PanicPublisher {inner:Publisher::default(),stage};
        let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            publish_text_request(TextDecodeRequest {element,source_map:map,end:element.end,style:&style,
                transform:map.transform,state:&mut state,preserve_space:false,provider:&mut provider,
                cache:&mut cache,scene,ledger:&mut ledger},&p,&mut f.builder,&draft,&mut Run)
        }));
        assert!(outcome.is_err());assert_eq!(state.cursor_x,9.);
        assert_eq!((state.text_bytes,state.glyph_count),(0,0));assert!(!state.has_position);
        assert!(!state.pending_space);assert_eq!(state.cursor_y,0.);
        assert!(cache.entries.is_empty());assert_eq!((cache.commands,cache.text_bytes),(0,0));
        assert_eq!(f.builder.document_text_commands(),0);
        assert_eq!(f.builder.scene(scene).unwrap().shapes.len(),1);
        assert!(f.builder.scene(scene).unwrap().shape(prefix).is_some());
        publish_text_request(TextDecodeRequest {element,source_map:map,end:element.end,style:&style,
            transform:map.transform,state:&mut state,preserve_space:false,provider:&mut provider,
            cache:&mut cache,scene,ledger:&mut ledger},&Publisher::default(),&mut f.builder,&draft,&mut Run).unwrap();
        let s=f.builder.scene(scene).unwrap();assert_eq!(s.shapes.len(),3);
        assert!(s.shapes[1..].iter().all(|shape|shape.source==map));
        assert!(s.shapes[1..].iter().enumerate().all(|(i,_)|
            s.shape_geometry_source(s.shape_id(i+1).unwrap()).is_none()));
        assert_eq!(s.path_numbers.len(),6,"only the prefix has authored path numbers");
        f.builder.commit_draft(&draft,&mut Run).unwrap();
    }
}

#[test]
fn second_text_shape_admission_failure_rolls_back_first_run_and_allows_image_retry() {
    let source=br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A<![CDATA[B]]></text><image width="4" height="4" href="data:image/png;base64,AQID"/></svg>"#;
    let mut f=Fixture::new(source,Limits {shapes:2,..Default::default()});
    let i=f.index(b"text");let element=view(&f.lex,i);let map=f.map(i);
    let scene=f.builder.root_id();let draft=f.builder.begin_draft(scene).unwrap();
    let prefix=f.builder.append_shape(scene,f.prefix_input(),&mut Run).unwrap();
    let before=*f.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let mut provider=Provider::default();let mut ledger=ChargeLedger::new(f.lex.source_owner(),1<<20,8).unwrap();
    let mut state=TextState::default();let mut cache=TextCache::default();let p=Publisher::default();
    let e=publish_text_request(TextDecodeRequest {element,source_map:map,end:element.end,
        style:&CssPaint::initial(),transform:map.transform,state:&mut state,preserve_space:false,
        provider:&mut provider,cache:&mut cache,scene,ledger:&mut ledger},&p,
        &mut f.builder,&draft,&mut Run).unwrap_err();
    assert_eq!((e.kind,e.offset),(ErrorKind::Limit(LimitKind::Shapes),source.windows(4).position(|v|v==b"text").unwrap()));
    assert_eq!(provider.seen.len(),2,"text admission preserves original layout-before-custom ordering");
    assert_eq!(&*p.calls.borrow(),&["begin","prepare","publish","prepare"]);
    assert_eq!(f.builder.scene(scene).unwrap().shapes.len(),1);
    assert_eq!(f.builder.scene(scene).unwrap().shape(prefix),Some(&before));
    assert_eq!(f.builder.document_text_commands(),0);assert!(cache.entries.is_empty());
    assert_eq!((state.text_bytes,state.glyph_count,state.cursor_x),(0,0,0.));
    let i=f.index(b"image");let element=view(&f.lex,i);let map=f.map(i);
    let id=publish_image_request(ImageDecodeRequest {element,source_map:map,
        style:&CssPaint::initial(),transform:map.transform,provider:&mut provider,scene,ledger:&mut ledger},
        &Publisher::default(),&mut f.builder,&draft,&mut Run).unwrap().unwrap();
    assert_eq!(provider.image_calls,1);assert!(f.builder.scene(scene).unwrap().shape(id).is_some());
    f.builder.commit_draft(&draft,&mut Run).unwrap();
}
