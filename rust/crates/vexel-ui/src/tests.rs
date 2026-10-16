use crate::*;

fn child(id:u32,parent:u32,width:u32,height:u32,color:u32)->NodeSpec{
    let mut s=NodeSpec::new(id,parent);s.axis=Axis::Horizontal;s.preferred_width=width;s.preferred_height=height;s.color=color;s
}

fn linear_channel(color:u32,shift:u32)->f64{
    let channel=((color>>shift)&255)as f64/255.0;
    if channel<=0.04045{channel/12.92}else{((channel+0.055)/1.055).powf(2.4)}
}
fn contrast(first:u32,second:u32)->f64{
    let luminance=|color|0.2126*linear_channel(color,24)+0.7152*linear_channel(color,16)+0.0722*linear_channel(color,8);
    let (a,b)=(luminance(first),luminance(second));(a.max(b)+0.05)/(a.min(b)+0.05)
}

#[test]
fn tokens_type_scale_and_metrics_match_core_defaults(){
    let dark=theme_tokens(Theme::Dark,false);let light=theme_tokens(Theme::Light,false);
    assert_eq!(dark.window,0x171a20ff);assert_eq!(dark.panel,0x20242bff);assert_eq!(dark.accent,0x83b4ffff);
    assert_eq!(dark.panel_alt,dark.panel_raised);assert_eq!(dark.canvas,dark.canvas_well);assert_eq!(dark.text,dark.text_primary);
    let light_high=theme_tokens(Theme::Light,true);let dark_high=theme_tokens(Theme::Dark,true);
    assert_eq!(light.window,0xf2f4f7ff);assert_eq!(light.panel_raised,0xedf0f4ff);assert_eq!(light.canvas_well,0xe7ebf0ff);
    assert_eq!(dark.panel_raised,0x292e37ff);assert_eq!(dark.canvas_well,0x11151bff);assert_eq!(light.accent,0x185fc5ff);
    assert_eq!(dark_high.focus_ring,0xbedaffff);
    assert_eq!(contrast(light.text_primary,light.panel)>=14.0,true);assert_eq!(contrast(light.text_secondary,light.panel_raised)>=5.5,true);
    assert_eq!(contrast(dark.text_primary,dark.panel)>=14.0,true);assert_eq!(contrast(dark.text_secondary,dark.panel_raised)>=5.5,true);
    assert!(contrast(light_high.text_secondary,light_high.panel_raised)>contrast(light.text_secondary,light.panel_raised));
    assert!(contrast(dark_high.text_secondary,dark_high.panel_raised)>contrast(dark.text_secondary,dark.panel_raised));
    assert!(contrast(light_high.control_edge,light_high.panel)>contrast(light.control_edge,light.panel));
    assert!(contrast(dark_high.control_edge,dark_high.panel)>contrast(dark.control_edge,dark.panel));
    assert_eq!(type_scale().geometry_pt,12);assert!(type_scale().geometry_monospaced&&type_scale().title_semibold);
    assert_eq!(design_metrics().toolbar_height,40);assert_eq!(design_metrics().inspector_width,320);assert_eq!(design_metrics().control_radius,6);
}

#[test]
fn accent_choices_keep_contrast_on_both_surfaces_in_all_modes(){
    let choices=[0xff3b30ff,0xff9500ff,0xffcc00ff,0x34c759ff,0x00c7beff,0x007affff,0xaf52deff,0x8e8e93ff];
    for accent in choices{for theme in [Theme::Light,Theme::Dark]{for high_contrast in [false,true]{
        let tokens=theme_tokens_accented(theme,high_contrast,accent);let minimum=if high_contrast{4.5}else{3.0};
        assert_eq!(tokens.accent,tokens.selection_edge,"accent {accent:08x}, {theme:?}, high={high_contrast}");
        assert_eq!(tokens.accent,tokens.focus_ring,"accent {accent:08x}, {theme:?}, high={high_contrast}");
        assert!(contrast(tokens.accent,tokens.panel)>=minimum,"panel contrast for {accent:08x}, {theme:?}, high={high_contrast}");
        assert!(contrast(tokens.accent,tokens.panel_raised)>=minimum,"raised contrast for {accent:08x}, {theme:?}, high={high_contrast}");
        assert_ne!(tokens.selection_fill,tokens.panel,"selection fill for {accent:08x}, {theme:?}, high={high_contrast}");
        assert_eq!(tokens.selection_fill&255,255,"opaque selection fill for {accent:08x}, {theme:?}, high={high_contrast}");
    }}}
    for theme in [Theme::Light,Theme::Dark]{for high_contrast in [false,true]{
        let base=theme_tokens(theme,high_contrast);
        assert_eq!(theme_tokens_accented(theme,high_contrast,0).accent,base.accent);
        assert_eq!(theme_tokens_accented(theme,high_contrast,0x12345600).accent,base.accent);
    }}
}

#[test]
fn original_layout_cases_fixed_flexible_overflow_and_trailing_fixed(){
    let mut t=Tree::new(Theme::Dark).unwrap();t.nodes[0].spec.axis=Axis::Horizontal;t.nodes[0].spec.gap=4;
    t.add(child(2,ROOT_ID,100,20,t.tokens.accent)).unwrap();t.add(child(3,ROOT_ID,0,20,t.tokens.panel_alt)).unwrap();
    assert_eq!(t.add(child(4,99,20,20,1)),Err(TreeError::MissingParent));assert_eq!(t.add(child(2,ROOT_ID,1,1,1)),Err(TreeError::DuplicateId));
    assert!(t.layout(0,200).is_err());t.layout(320,200).unwrap();assert_eq!(t.find(2).unwrap().bounds,Rect{x:0,y:0,width:100,height:20});
    assert_eq!(t.find(3).unwrap().bounds,Rect{x:104,y:0,width:216,height:20});assert_eq!(t.add(child(4,ROOT_ID,1,1,1)),Err(TreeError::Capacity));
    let mut over=Tree::new(Theme::Dark).unwrap();over.nodes[0].spec.axis=Axis::Horizontal;over.nodes[0].spec.gap=4;
    over.add(child(2,ROOT_ID,100,20,0)).unwrap();over.add(child(3,ROOT_ID,80,20,0)).unwrap();over.layout(120,20).unwrap();
    assert_eq!(over.find(2).unwrap().bounds.width,100);assert_eq!(over.find(3).unwrap().bounds,Rect{x:104,y:0,width:16,height:20});
    let mut tail=Tree::new(Theme::Dark).unwrap();tail.nodes[0].spec.axis=Axis::Horizontal;tail.nodes[0].spec.gap=4;
    tail.add(child(2,ROOT_ID,40,20,0)).unwrap();tail.add(child(3,ROOT_ID,0,20,0)).unwrap();tail.add(child(4,ROOT_ID,40,20,0)).unwrap();tail.layout(320,20).unwrap();
    assert_eq!(tail.find(3).unwrap().bounds.width,232);assert_eq!(tail.find(4).unwrap().bounds,Rect{x:280,y:0,width:40,height:20});
}

#[test]
fn depth_limit_layout_failure_is_atomic_and_retryable(){
    let mut t=Tree::new(Theme::Dark).unwrap();let mut parent=ROOT_ID;
    for id in 2..=26{let mut n=NodeSpec::new(id,parent);n.axis=Axis::Overlay;t.add(n).unwrap();parent=id;}
    assert_eq!(t.layout(100,100),Err(TreeError::Layout));assert!(!t.laid_out);
    assert!(t.nodes.iter().all(|n|n.bounds==Rect::default()));
    let mut shallow=Tree::new(Theme::Dark).unwrap();parent=ROOT_ID;for id in 2..=25{shallow.add(NodeSpec::new(id,parent)).unwrap();parent=id;}shallow.layout(10,10).unwrap();
}

#[test]
fn add_caps_duplicate_and_failure_leave_tree_unchanged(){
    let mut t=Tree::new(Theme::Dark).unwrap();let before=t.clone();assert_eq!(t.add(child(2,77,0,0,0)),Err(TreeError::MissingParent));assert_eq!(t,before);
    let mut long=NodeSpec::new(2,ROOT_ID);long.label="é".repeat(40);t.add(long).unwrap();let node=t.find(2).unwrap();assert_eq!(node.label_preview.as_bytes().len(),62);assert_eq!(node.label_preview,"é".repeat(31));
    let snap=t.semantics().unwrap();assert_eq!(snap.nodes[1].label,"é".repeat(40));
    let mut total=Tree::new(Theme::Dark).unwrap();let mut item=NodeSpec::new(2,ROOT_ID);item.label="x".repeat(MAX_TEXT_TOTAL_BYTES-6);total.add(item).unwrap();
    let before=total.clone();let mut rejected=NodeSpec::new(3,ROOT_ID);rejected.value="y".to_owned();assert_eq!(total.add(rejected),Err(TreeError::TextLimit));assert_eq!(total,before);
    let mut full=Tree::new(Theme::Dark).unwrap();for id in 2..MAX_NODES as u32+1{full.add(NodeSpec::new(id,ROOT_ID)).unwrap()}assert_eq!(full.nodes.len(),MAX_NODES);assert_eq!(full.add(NodeSpec::new(129,ROOT_ID)),Err(TreeError::Capacity));
}

#[test]
fn focus_commands_checkboxes_and_disabled_nodes(){
    let mut t=Tree::new(Theme::Dark).unwrap();let mut a=NodeSpec::new(2,ROOT_ID);a.role=Role::Button;a.command_id=11;t.add(a).unwrap();
    let mut disabled=NodeSpec::new(3,ROOT_ID);disabled.role=Role::Button;disabled.state=1;t.add(disabled).unwrap();
    let mut text_field=NodeSpec::new(4,ROOT_ID);text_field.role=Role::TextField;t.add(text_field).unwrap();
    let mut slider=NodeSpec::new(5,ROOT_ID);slider.role=Role::Slider;t.add(slider).unwrap();
    let mut row=NodeSpec::new(6,ROOT_ID);row.role=Role::Row;t.add(row).unwrap();
    let mut check=NodeSpec::new(7,ROOT_ID);check.role=Role::Checkbox;check.command_id=12;t.add(check).unwrap();
    assert!(t.focus_next(false));assert_eq!(t.focused_id,2);
    for expected in [4,5,6,7,2]{assert!(t.focus_next(false));assert_eq!(t.focused_id,expected,"forward traversal skipped or reordered focusable roles")}
    assert!(t.focus_next(true));assert_eq!(t.focused_id,7);assert!(t.focus_next(true));assert_eq!(t.focused_id,6);
    assert!(t.focus_next(false));assert_eq!(t.focused_id,7);assert_eq!(t.dispatch_key(None,0x110001,0),Some(12));assert_eq!(t.find(7).unwrap().spec.state&2,2);
    for expected in [2,4,5,6,7]{assert!(t.focus_next(false));assert_eq!(t.focused_id,expected,"forward wrap across row/checkbox/button roles")}
    assert!(t.focus_next(false));assert_eq!(t.focused_id,2);assert!(t.focus_next(true));assert_eq!(t.focused_id,7);
    assert!(t.focus_next(false));assert_eq!(t.focused_id,2);assert_eq!(t.dispatch_key(None,0x110002,0),Some(11));
    let mut r=CommandRegistry::new();r.register(20,'s' as u32,2,true).unwrap();assert_eq!(r.register(20,'x' as u32,0,true),Err(TreeError::DuplicateId));
    assert_eq!(r.register(21,'s' as u32,2,true),Err(TreeError::DuplicateId));assert_eq!(r.register(22,'q' as u32,0x8000_0000,true),Err(TreeError::InvalidInput));
    assert_eq!(t.dispatch_key(Some(&r),'s' as u32,2),Some(20));assert_eq!(t.dispatch_key(Some(&r),'s' as u32,0),None);
    let mut capped=CommandRegistry::new();for id in 1..=MAX_COMMANDS as u32{capped.register(id,1000+id,0,true).unwrap()}assert_eq!(capped.register(99,9999,0,true),Err(TreeError::Capacity));
}

#[test]
fn semantics_are_owned_full_text_and_preserve_defaults_frames(){
    let mut t=Tree::accessible(Theme::Light,true).unwrap();let mut s=NodeSpec::new(2,ROOT_ID);s.role=Role::TextField;s.label="日本語の長いラベル🙂".repeat(8);s.value="編集済み value".to_owned();s.state=1<<3;s.command_id=77;t.add(s).unwrap();t.layout(300,200).unwrap();
    let snapshot=t.semantics().unwrap();let n=&snapshot.nodes[1];assert_eq!(n.label,"日本語の長いラベル🙂".repeat(8));assert_eq!(n.value,"編集済み value");assert!(n.label.len()>n.label_preview.len());assert_eq!(n.frame,Rect{x:0,y:0,width:300,height:200});assert_eq!(snapshot.text(n,true),Some("編集済み value"));
    let second=t.semantics().unwrap();assert_eq!(snapshot,second);let mut changed=t.clone();changed.find_mut(2).unwrap().spec.label.clear();assert_eq!(snapshot.nodes[1].label,"日本語の長いラベル🙂".repeat(8));
}

#[test]
fn basic_render_and_focus_ring_use_owned_rgba_tile(){
    let mut t=Tree::new(Theme::Dark).unwrap();let mut n=NodeSpec::new(2,ROOT_ID);n.axis=Axis::Overlay;n.color=0xff0000ff;n.role=Role::Button;t.add(n).unwrap();t.layout(4,4).unwrap();let plain=t.render().unwrap();assert_eq!(&plain.rgba()[0..4],&[255,0,0,255]);
    t.find_mut(2).unwrap().spec.color=0x00ff0080;let opaque=t.render().unwrap();assert_eq!(&opaque.rgba()[0..4],&[0,255,0,255]);
    t.find_mut(2).unwrap().spec.color=0xff0000ff;t.find_mut(2).unwrap().spec.state|=4;let focused=t.render().unwrap();assert_eq!(&focused.rgba()[0..4],&[(t.tokens.focus_ring>>24)as u8,(t.tokens.focus_ring>>16)as u8,(t.tokens.focus_ring>>8)as u8,255]);assert_ne!(plain,focused);
}

#[test]
fn icon_catalog_strict_paths_masks_and_frozen_hash_relationships(){
    assert_eq!(icon_path_count(),53);assert!(icon_validate_all());
    assert_eq!(icon_path_at(0).unwrap().name,"select");assert_eq!(icon_path_at(52).unwrap().name,"sidebar_right");assert!(icon_path_at(53).is_none());
    assert_eq!(icon_path_validate("M0 12 A12 12 0 0 1 24 12").unwrap(),IconBounds{left:0.0,top:0.0,right:24.0,bottom:12.0});
    for bad in ["M0 0 L30 30","M0 0 H10","M0 0 m1 1","M0 12 A12 12 0 2 1 24 12","M1 1 A24 24 0 1 1 2 1"] {
        assert_eq!(icon_path_validate(bad),Err(IconError::InvalidPath),"{bad}");
    }
    // Match strtod's ERANGE rejection for both zero-underflow and subnormal
    // numbers, while preserving exact zero (the C oracle accepts 0e-9999).
    for underflow in ["M0 0 L1e-9999 1", "M0 0 L1e-308 1", "M0 0 L2e-308 1"] {
        assert_eq!(icon_path_validate(underflow),Err(IconError::InvalidPath),"{underflow}");
    }
    assert!(icon_path_validate("M0 0 L0e-9999 1").is_ok());
    assert_eq!(icon_path_validate("M0 0"),Err(IconError::InvalidPath));
    assert_eq!(icon_path_validate(&format!("M0 0{}", " ".repeat(2045))),Err(IconError::InvalidPath));
    let mut dense=String::from("M1 1 ");for _ in 0..385{dense.push_str("L1 1 ");}
    assert_eq!(icon_path_validate(&dense),Err(IconError::TooComplex));
    assert!(icon_path_validate("M0 0 C1 1 2 2 3 3").is_ok());
    let mut cache=IconCache::new();let dark_1=cache.asset_hash(Theme::Dark,1);let light_1=cache.asset_hash(Theme::Light,1);let dark_2=cache.asset_hash(Theme::Dark,2);let dark_3=cache.asset_hash(Theme::Dark,3);
    // Frozen ui_icon_asset_hash outputs from the owner C implementation.
    assert_eq!(dark_1,0xdd0c_edd4_ec72_3d06);assert_eq!(light_1,0xdd0c_edd4_ec72_3d06);
    assert_eq!(dark_2,0x2d4e_975f_7b57_75c5);assert_eq!(dark_3,0xbe12_0e3a_5aa4_1d42);
    assert_eq!(cache.asset_hash(Theme::Light,2),0x2d4e_975f_7b57_75c5);
    assert_eq!(cache.asset_hash(Theme::Light,3),0xbe12_0e3a_5aa4_1d42);
    assert_ne!(dark_1,0);assert_eq!(dark_1,light_1);assert_ne!(dark_2,0);assert_ne!(dark_1,dark_2);
    let mut hashes=[0u64;15];let kinds=[IconKind::PEN,IconKind::NODE_EDIT,IconKind::PENCIL,IconKind::STROKE,IconKind::UNGROUP,IconKind::MORE,IconKind::CIRCLE,IconKind::ELLIPSE,IconKind::DOCUMENT,IconKind::STOP,IconKind::PATH,IconKind::POLYLINE,IconKind::IMAGE,IconKind::SIDEBAR_LEFT,IconKind::SIDEBAR_RIGHT];
    for (i,icon) in kinds.into_iter().enumerate(){let mut rgba=vec![0;48*48*4];cache.paint(icon,Theme::Light,2.0,Rect{x:0,y:0,width:48,height:48},0x1f2732ff,&mut rgba,48,48,48*4).unwrap();let mut h=1_469_598_103_934_665_603u64;let mut visible=0;for at in (3..rgba.len()).step_by(4){if rgba[at]!=0{visible+=1;h=(h^((at/4)as u64)).wrapping_mul(1_099_511_628_211)}}assert!(visible>=16);hashes[i]=h;assert!(hashes[..i].iter().all(|prior|*prior!=h));}
    assert_ne!(cache.asset_hash(Theme::Dark,3),0);
    for icon in [IconKind::SELECT,IconKind::PAN,IconKind::ZOOM_IN]{for scale in 1..=4{let mut rgba=vec![0;72*72*4];cache.paint_legacy(icon,Theme::Dark,scale as f64,Rect{x:0,y:0,width:72,height:72},0xffffffff,&mut rgba,72,72,72*4).unwrap();assert!(rgba.chunks_exact(4).filter(|p|p[3]!=0).count()>=16,"legacy {icon:?} at {scale}x");}}
    assert_eq!(cache.paint_legacy(IconKind::PEN,Theme::Dark,1.0,Rect{x:0,y:0,width:24,height:24},0xffffffff,&mut vec![0;24*24*4],24,24,24*4),Err(IconError::InvalidIcon));
    let mut small=vec![0;4*4*4];assert_eq!(cache.paint(IconKind::PEN,Theme::Dark,0.49,Rect{x:0,y:0,width:4,height:4},0xffffffff,&mut small,4,4,16),Err(IconError::InvalidScale));
    cache.clear();assert_eq!(cache.asset_hash(Theme::Dark,0),0);
}

#[test]
fn legacy_pan_mask_matches_frozen_c_reference_at_scales_one_through_four(){
    // Occupancy hashes cover every mask pixel and were derived from the frozen
    // C pan segments, Bresenham loop, and square stroke expansion.
    let expected = [
        0x7323_620c_6cdd_19ff,
        0x07e8_8963_e179_c015,
        0xdf29_7786_b8b2_6697,
        0x0408_259a_f00c_95a1,
    ];
    let mut cache = IconCache::new();
    for (scale, expected_hash) in (1usize..=4).zip(expected) {
        let axis = 24 * scale;
        let mut rgba = vec![0; axis * axis * 4];
        cache.paint_legacy(
            IconKind::PAN,
            Theme::Dark,
            scale as f64,
            Rect { x: 0, y: 0, width: axis as u32, height: axis as u32 },
            0xffff_ffff,
            &mut rgba,
            axis as u32,
            axis as u32,
            axis * 4,
        ).unwrap();
        let mut hash = 1_469_598_103_934_665_603u64;
        for pixel in (3..rgba.len()).step_by(4) {
            if rgba[pixel] != 0 {
                hash = (hash ^ (pixel / 4) as u64).wrapping_mul(1_099_511_628_211);
            }
        }
        assert_eq!(hash, expected_hash, "frozen C pan mask at {scale}x");
        for (x, y) in [(12, 3), (9, 17), (7, 9), (17, 9)] {
            let at = (y * scale * axis + x * scale) * 4;
            assert_eq!(&rgba[at..at + 4], &[255, 255, 255, 255], "C endpoint ({x}, {y}) at {scale}x");
        }
    }
}

#[test]
fn scalar_draw_and_tree_paint_validate_before_writing(){
    let mut pixels=vec![0;8*8*4];assert!(scalar_text_draw("A🙂",Rect{x:1,y:1,width:7,height:7},13,0xffffffff,&mut pixels,8,8,32));
    assert_eq!(&pixels[(1*8+2)*4..(1*8+2)*4+4],&[255,255,255,255]);assert_eq!(&pixels[(1*8+1)*4..(1*8+1)*4+4],&[0,0,0,0]);
    assert!(!scalar_text_draw("x",Rect{x:0,y:0,width:9,height:9},13,0xffffffff,&mut pixels,8,8,32));
    let mut alpha=[0u8;4];crate::draw::blend(&mut alpha,0x33669980,255);assert_eq!(alpha,[0x33,0x66,0x99,0x80]);
    let mut tree=Tree::accessible(Theme::Dark,false).unwrap();let mut spec=NodeSpec::new(2,ROOT_ID);spec.role=Role::Button;spec.color=0xff000080;spec.state=1<<5;tree.add(spec).unwrap();tree.layout(4,4).unwrap();
    let mut short=vec![0x5a;63];assert_eq!(tree.paint_into(&mut short,16),Err(PaintError::InvalidBuffer));assert!(short.iter().all(|b|*b==0x5a));
    let mut rgba=vec![0;4*4*4];tree.paint_into(&mut rgba,16).unwrap();assert_eq!(&rgba[(1*4+1)*4..(1*4+1)*4+4],&[38,60,88,255]);
    let mut background=vec![0;4*4*4];tree.paint_backgrounds_into(&mut background,16).unwrap();assert_eq!(background,rgba);
    tree.find_mut(2).unwrap().spec.state=1<<2;let mut focused=vec![0;4*4*4];tree.paint_into(&mut focused,16).unwrap();
    let ring=tree.tokens.focus_ring;assert_eq!(&focused[4..8],&[(ring>>24)as u8,(ring>>16)as u8,(ring>>8)as u8,255]);
}

#[test]
fn clipped_semantic_outline_uses_canvas_edges(){
    let mut tree=Tree::accessible(Theme::Dark,true).unwrap();
    let mut spec=NodeSpec::new(2,ROOT_ID);spec.role=Role::Button;tree.add(spec).unwrap();
    tree.layout(4,4).unwrap();
    // Model a retained node crossing the canvas after layout. The C painter
    // clamps right/bottom first and treats those clipped bounds as outline edges.
    tree.find_mut(2).unwrap().bounds=Rect{x:1,y:1,width:6,height:6};
    let mut rgba=vec![0;4*4*4];tree.paint_backgrounds_into(&mut rgba,16).unwrap();
    let edge=tree.tokens.control_edge;
    assert_eq!(&rgba[(2*4+3)*4..(2*4+3)*4+4],&[(edge>>24)as u8,(edge>>16)as u8,(edge>>8)as u8,255]);
    let window=tree.tokens.window;
    assert_eq!(&rgba[(2*4+2)*4..(2*4+2)*4+4],&[(window>>24)as u8,(window>>16)as u8,(window>>8)as u8,255]);
}


#[test]
fn five_widget_add_wrappers_preserve_roles_and_owned_text(){
    let mut tree=Tree::new(Theme::Dark).unwrap();let base=WidgetSpec::new(2,ROOT_ID);add_button(&mut tree,&base).unwrap();
    let mut s=WidgetSpec::new(3,ROOT_ID);s.label="check".into();add_checkbox(&mut tree,&s).unwrap();
    let s=WidgetSpec::new(4,ROOT_ID);add_text_field(&mut tree,&s).unwrap();let s=WidgetSpec::new(5,ROOT_ID);add_slider(&mut tree,&s).unwrap();let s=WidgetSpec::new(6,ROOT_ID);add_row(&mut tree,&s).unwrap();
    assert_eq!(tree.find(2).unwrap().spec.role,Role::Button);assert_eq!(tree.find(3).unwrap().spec.role,Role::Checkbox);assert_eq!(tree.find(3).unwrap().spec.label,"check");
    assert_eq!(tree.find(4).unwrap().spec.role,Role::TextField);assert_eq!(tree.find(5).unwrap().spec.role,Role::Slider);assert_eq!(tree.find(6).unwrap().spec.role,Role::Row);
    let before=tree.clone();let mut oversized=WidgetSpec::new(7,ROOT_ID);oversized.label="x".repeat(MAX_TEXT_TOTAL_BYTES);
    assert_eq!(add_button(&mut tree,&oversized),Err(TreeError::TextLimit));assert_eq!(tree,before);
}

#[test]
fn widget_input_preserves_utf8_editing_and_transactional_rules(){
    let mut edit=TextEdit::new("Layer name","box").unwrap();
    assert!(edit.begin());assert!(edit.pointer(0,false));
    assert_eq!(edit.key(0x1f642,0),InputResult::Changed);assert_eq!(edit.value,"🙂box");assert_eq!(edit.value.len(),7);
    assert!(edit.commit());assert!(edit.begin());assert_eq!(edit.key(b'x' as u32,0),InputResult::Changed);
    assert_eq!(edit.value,"🙂xbox");assert_eq!(edit.key(KEY_ESCAPE,0),InputResult::Cancelled);assert_eq!(edit.value,"🙂box");
    assert!(!edit.pointer(1,false));
    let full="a".repeat(WIDGET_MAX_EDIT_BYTES);let mut capped=TextEdit::new("Full value",&full).unwrap();assert!(capped.begin());
    assert_eq!(capped.key(b'b' as u32,0),InputResult::Invalid);assert_eq!(capped.value.len(),WIDGET_MAX_EDIT_BYTES);
    assert!(TextEdit::new(&"L".repeat(WIDGET_MAX_LABEL_BYTES+1),"").is_none());
    let mut selected=TextEdit::new("Name","a🙂z").unwrap();assert!(selected.begin());assert!(selected.pointer(1,false));assert!(selected.pointer(5,true));
    assert_eq!(selected.key(b'X' as u32,0),InputResult::Changed);assert_eq!(selected.value,"aXz");
}

#[test]
fn widget_number_color_slider_dropdown_and_pointer_keep_original_semantics(){
    assert_eq!(number_parse(" -12.5e+2 "),Some(-1250.0));assert_eq!(number_parse("1px"),None);assert_eq!(number_parse("1e309"),None);assert_eq!(number_parse("nan"),None);
    assert_eq!(number_parse("1e-400"),None);assert_eq!(number_parse("5e-324"),None);assert_eq!(number_parse("1e-309"),None);
    assert_eq!(number_parse("0"),Some(0.0));assert_eq!(number_parse("-0.0"),Some(-0.0));
    assert_eq!(number_parse("2.2250738585072014e-308"),Some(f64::MIN_POSITIVE));
    assert_eq!(number_format(13.1).as_deref(),Some("13.1"));assert_eq!(number_format(1.0e20).as_deref(),Some("1e+20"));
    let mut n=NumberField::new("Width",2.0,0.0,20.0,1.0,"px").unwrap();assert_eq!(n.key(KEY_UP,0),InputResult::Changed);assert_eq!(n.key(KEY_UP,MOD_OPTION),InputResult::Changed);assert_eq!(n.key(KEY_UP,MOD_SHIFT),InputResult::Changed);assert!((n.value-13.1).abs()<1e-9);assert_eq!(n.key(KEY_ESCAPE,0),InputResult::Cancelled);assert_eq!(n.value,2.0);
    assert!(NumberField::new("Width",1.0,0.0,2.0,1.0,"123456789").is_none());let too_many=vec!["Option";WIDGET_MAX_OPTIONS+1];assert!(Dropdown::new("Choice",&too_many,0).is_none());
    assert!(n.set_draft("5.5"));assert_eq!(n.key(KEY_RETURN,0),InputResult::Committed);assert_eq!(n.value,5.5);assert!(n.set_draft("1e"));assert_eq!(n.key(KEY_RETURN,0),InputResult::Invalid);assert_eq!(n.key(KEY_ESCAPE,0),InputResult::Cancelled);assert_eq!(n.value,5.5);
    assert_eq!(hex_parse("#4F7"),Some(0x44ff77ff));assert_eq!(hex_parse("#4f7cff80"),Some(0x4f7cff80));assert_eq!(hex_parse("#xyz"),None);assert_eq!(hex_format(0x4f7cff80),"#4F7CFF80");
    let mut color=ColorField::new("Fill",0x4f7cffff).unwrap();assert!(color.set_draft("#12"));assert_eq!(color.key(KEY_RETURN,0),InputResult::Invalid);assert_eq!(color.key(KEY_ESCAPE,0),InputResult::Cancelled);assert_eq!(color.rgba,0x4f7cffff);assert!(color.set_draft("#00FF80"));assert_eq!(color.key(KEY_RETURN,0),InputResult::Committed);assert_eq!(color.rgba,0x00ff80ff);
    let mut slider=Slider::new("Opacity",0.5,0.0,1.0,0.1).unwrap();assert_eq!(slider.key(KEY_RIGHT,MOD_SHIFT),InputResult::Changed);assert_eq!(slider.value,1.0);assert!(slider.pointer(PointerEvent::Down,51,0,100));assert!((slider.value-0.5).abs()<1e-9);assert!(slider.pointer(PointerEvent::UpInside,51,0,100));
    let mut dd=Dropdown::new("Blend mode",&["Normal","Multiply","Screen"],0).unwrap();assert_eq!(dd.key(KEY_DOWN,0),InputResult::Changed);assert_eq!(dd.key(KEY_DOWN,0),InputResult::Changed);assert_eq!(dd.active_index,1);assert_eq!(dd.key(KEY_RETURN,0),InputResult::Committed);assert_eq!(dd.selected_index,1);assert_eq!(dd.key(KEY_SPACE,0),InputResult::Changed);assert_eq!(dd.key(KEY_ESCAPE,0),InputResult::Cancelled);assert_eq!(dd.selected_index,1);
    let mut pointer=PointerState{enabled:true,..PointerState::default()};assert_eq!(pointer.visual_state(),ControlState::Rest);assert!(!pointer.transition(PointerEvent::Enter));assert_eq!(pointer.visual_state(),ControlState::Hover);assert!(!pointer.transition(PointerEvent::Down));assert_eq!(pointer.visual_state(),ControlState::Pressed);assert!(pointer.transition(PointerEvent::UpInside));pointer.enabled=false;assert!(!pointer.transition(PointerEvent::Enter));assert_eq!(pointer.visual_state(),ControlState::Disabled);
}

#[test]
fn widget_canvas_keeps_native_text_hook_focus_and_buffer_bounds(){
    let mut small=vec![0u8;64*64*4];let mut large=vec![0u8;2048*1024*4];
    let mut a=WidgetCanvas::new(&mut small,64*64*4,64,64,64*4,1,Theme::Light,false,None).unwrap();
    let mut b=WidgetCanvas::new(&mut large,2048*1024*4,2048,1024,2048*4,1,Theme::Light,false,None).unwrap();
    let mut icon=WidgetDrawSpec::new(WidgetKind::IconButton,Rect{x:12,y:12,width:28,height:28});icon.state=ControlState::Focus;icon.icon=IconKind::SELECT;assert!(widget_paint(&mut a,&icon));assert!(widget_paint(&mut b,&icon));
    for y in 0..64{assert_eq!(&a.rgba[y*64*4..(y+1)*64*4],&b.rgba[y*2048*4..y*2048*4+64*4]);}
    use std::{cell::RefCell,rc::Rc};let seen=Rc::new(RefCell::new(Vec::new()));let capture=seen.clone();let mut hook=move |s:&str,_:Rect,_:Theme,size:u32,color:u32,f:FontFamily,w:FontWeight,_:&mut[u8],_:u32,_:u32,_:usize|{capture.borrow_mut().push((s.to_owned(),size,f,w,color));true};
    let mut pixels=vec![0;300*400*4];let mut c=WidgetCanvas::new(&mut pixels,300*400*4,300,400,300*4,1,Theme::Light,false,Some(&mut hook)).unwrap();
    let labels=["Select","Pan","Zoom"];let mut seg=WidgetDrawSpec::new(WidgetKind::Segmented,Rect{x:64,y:256,width:300,height:36});seg.segment_index=1;seg.segment_count=3;seg.segment_labels=&labels;assert!(widget_paint(&mut c,&seg));assert_eq!(seen.borrow().iter().map(|x|x.0.as_str()).collect::<Vec<_>>(),["Select","Pan","Zoom"]);assert_eq!(seen.borrow()[1].2,FontFamily::System);assert_eq!(seen.borrow()[1].3,FontWeight::Semibold);assert!(seen.borrow().iter().all(|x|x.1==12));seen.borrow_mut().clear();
    let mut number=WidgetDrawSpec::new(WidgetKind::NumberField,Rect{x:64,y:304,width:180,height:30});number.value=Some("64");number.unit=Some("px");assert!(widget_paint(&mut c,&number));assert_eq!(seen.borrow().iter().map(|x|x.0.as_str()).collect::<Vec<_>>(),["64","px"]);assert_eq!(seen.borrow()[0].2,FontFamily::Monospaced);assert_eq!(seen.borrow()[0].1,12);assert_eq!(seen.borrow()[0].4,c.tokens.text_primary);assert_eq!(seen.borrow()[1].1,11);assert_eq!(seen.borrow()[1].4,c.tokens.text_secondary);seen.borrow_mut().clear();
    let mut label=WidgetDrawSpec::new(WidgetKind::Label,Rect{x:64,y:352,width:180,height:24});label.label=Some("No selection");label.text_color=0x526171ff;assert!(widget_paint(&mut c,&label));assert_eq!(seen.borrow()[0].4,0x526171ff);
    let mut narrow_pixels=vec![0u8;32*32*4];let mut narrow_canvas=WidgetCanvas::new(&mut narrow_pixels,32*32*4,32,32,32*4,1,Theme::Light,false,None).unwrap();let mut narrow=WidgetDrawSpec::new(WidgetKind::Segmented,Rect{x:4,y:4,width:2,height:20});narrow.segment_count=3;narrow.segment_index=1;assert!(widget_paint(&mut narrow_canvas,&narrow));
    let mut expected_pixels=vec![0u8;32*32*4];let mut expected=WidgetCanvas::new(&mut expected_pixels,32*32*4,32,32,32*4,1,Theme::Light,false,None).unwrap();let tokens=expected.tokens;expected.paint_round(narrow.bounds,6,tokens.panel_raised,tokens.separator,1);expected.paint_rect(Rect{x:4,y:10,width:1,height:8},tokens.separator);expected.paint_rect(Rect{x:4,y:10,width:1,height:8},tokens.separator);assert_eq!(narrow_canvas.rgba,expected.rgba,"a zero-width selected segment must paint no active pixels");
    assert!(WidgetCanvas::new(&mut vec![0;64*64*4],64*64*4-1,64,64,64*4,1,Theme::Light,false,None).is_none());assert_eq!(widget_paint_scratch_limit(),0);
    let frames=std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));let output=frames.clone();let mut edge_hook=move |_:&str,r:Rect,_:Theme,_:u32,_:u32,_:FontFamily,_:FontWeight,_:&mut[u8],_:u32,_:u32,_:usize|{output.borrow_mut().push(r);true};
    let mut edge_pixels=vec![0x73;32*32*4];let edge_before=edge_pixels.clone();let mut edge_canvas=WidgetCanvas::new(&mut edge_pixels,32*32*4,32,32,32*4,1,Theme::Light,false,Some(&mut edge_hook)).unwrap();let edge_rect=Rect{x:u32::MAX-64,y:0,width:16,height:16};let mut edge_label=WidgetDrawSpec::new(WidgetKind::Label,edge_rect);edge_label.label=Some("Edge");assert!(widget_paint(&mut edge_canvas,&edge_label));assert!(edge_canvas.paint_text("Edge",edge_rect,0xffffffff,FontFamily::System,FontWeight::Regular,13));assert_eq!(&*frames.borrow(),&[edge_rect,edge_rect]);assert_eq!(edge_canvas.rgba,edge_before,"valid off-canvas text still reaches the callback unchanged");
    edge_label.bounds.x=u32::MAX-16;edge_label.focus_visible=true;assert!(widget_paint(&mut edge_canvas,&edge_label));assert_eq!(frames.borrow()[2],edge_label.bounds,"C skips an overflowing focus ring without dropping valid text");edge_canvas.paint_rect(edge_rect,0xffffffff);edge_canvas.paint_round(edge_rect,4,0xffffffff,0,0);
    let overflow=Rect{x:u32::MAX-8,y:0,width:16,height:16};assert!(!edge_canvas.paint_text("Edge",overflow,0xffffffff,FontFamily::System,FontWeight::Regular,13));assert!(!edge_canvas.paint_icon(IconKind::LAYERS,overflow,ControlState::Rest));assert!(!edge_canvas.paint_icon_color(IconKind::LAYERS,overflow,0xffffffff));edge_canvas.paint_rect(overflow,0xffffffff);edge_canvas.paint_round(overflow,4,0xffffffff,0,0);assert_eq!(edge_canvas.rgba,edge_before);assert_eq!(frames.borrow().len(),3,"overflow must not invoke the text callback");
}

#[test]
fn widget_tree_row_keeps_unicode_ellipsis_and_callback_segments(){
    use std::{cell::RefCell,rc::Rc};let mut pixels=vec![0;256*48*4];let captured=Rc::new(RefCell::new(Vec::new()));let output=captured.clone();let mut hook=move |s:&str,_:Rect,_:Theme,_:u32,_:u32,_:FontFamily,_:FontWeight,_:&mut[u8],_:u32,_:u32,_:usize|{output.borrow_mut().push(s.to_owned());true};let mut c=WidgetCanvas::new(&mut pixels,256*48*4,256,48,256*4,1,Theme::Light,false,Some(&mut hook)).unwrap();
    let mut row=WidgetDrawSpec::new(WidgetKind::TreeRow,Rect{x:0,y:0,width:232,height:28});row.icon=IconKind::GRADIENT;row.label=Some("Linear Gradient #a-very-long-layer-id");row.show_row_actions=true;assert!(widget_paint(&mut c,&row));assert_eq!(&*captured.borrow(), &["Linear Gradient","#a-v…"]);captured.borrow_mut().clear();row.bounds.width=160;row.label=Some("ExtremelyLongElementTypeName #id");assert!(widget_paint(&mut c,&row));assert_eq!(&*captured.borrow(), &["Extre…eName"]);
    let wide=Rc::new(RefCell::new(Vec::new()));let output=wide.clone();let mut hook=move |s:&str,_:Rect,_:Theme,_:u32,_:u32,_:FontFamily,_:FontWeight,_:&mut[u8],_:u32,_:u32,_:usize|{output.borrow_mut().push(s.to_owned());true};let mut wide_pixels=vec![0;1024*48*4];let mut wide_canvas=WidgetCanvas::new(&mut wide_pixels,1024*48*4,1024,48,1024*4,1,Theme::Light,false,Some(&mut hook)).unwrap();let long="😀".repeat(40);row.bounds.width=1000;row.label=Some(&long);assert!(widget_paint(&mut wide_canvas,&row));assert!(!wide.borrow().is_empty());assert!(wide.borrow()[0].len()<64);
    let frames=Rc::new(RefCell::new(Vec::new()));let output=frames.clone();let mut hook=move |_:&str,r:Rect,_:Theme,_:u32,_:u32,_:FontFamily,_:FontWeight,_:&mut[u8],_:u32,_:u32,_:usize|{output.borrow_mut().push(r);true};let mut deep_pixels=vec![0;5000*32*4];let mut deep_canvas=WidgetCanvas::new(&mut deep_pixels,5000*32*4,5000,32,5000*4,1,Theme::Light,false,Some(&mut hook)).unwrap();let mut deep=WidgetDrawSpec::new(WidgetKind::TreeRow,Rect{x:0,y:0,width:5000,height:28});deep.indent=255;deep.label=Some("Layer");assert!(deep_canvas.paint_tree_row(&deep));assert_eq!(frames.borrow()[0].x,4+255*14+18+20,"C applies the full indent to content");assert_ne!(deep_canvas.rgba[82*4+3],0);assert_eq!(deep_canvas.rgba[96*4+3],0,"C draws at most six indent guides");
}

#[test]
fn widget_painter_covers_every_named_kind_and_state_transition(){
    let mut pixels=vec![0;2048*1024*4];let mut hook=|_:&str,_:Rect,_:Theme,_:u32,_:u32,_:FontFamily,_:FontWeight,_:&mut[u8],_:u32,_:u32,_:usize|true;
    let mut canvas=WidgetCanvas::new(&mut pixels,2048*1024*4,2048,1024,2048*4,2,Theme::Dark,true,Some(&mut hook)).unwrap();assert!(canvas.set_text_scale(150));assert!(!canvas.set_text_scale(110));
    let kinds=[WidgetKind::Label,WidgetKind::IconButton,WidgetKind::ButtonPrimary,WidgetKind::ButtonSecondary,WidgetKind::ButtonGhost,WidgetKind::Segmented,WidgetKind::Toggle,WidgetKind::Checkbox,WidgetKind::TabBar,WidgetKind::Tooltip,WidgetKind::HudPill,WidgetKind::Divider,WidgetKind::PanelHeader,WidgetKind::SectionHeader,WidgetKind::TreeRow,WidgetKind::EmptyState,WidgetKind::InlineError,WidgetKind::Toast,WidgetKind::NumberField,WidgetKind::ScrubLabel,WidgetKind::Stepper,WidgetKind::ColorField,WidgetKind::Slider,WidgetKind::Dropdown,WidgetKind::TextField];
    for (i,kind) in kinds.into_iter().enumerate(){let mut s=WidgetDrawSpec::new(kind,Rect{x:8+(i as u32%8)*60,y:8+(i as u32/8)*72,width:180,height:36});s.icon=IconKind::SELECT;s.label=Some("Label");s.value=Some("Value");s.secondary=Some("Details");s.unit=Some("px");s.checked=true;s.expanded=true;s.visible=true;s.has_children=true;s.locked=true;s.show_row_actions=true;s.numeric_value=0.5;s.segment_count=3;s.segment_index=1;s.segment_labels=&["One","Two","Three"];s.tone=Tone::Warning;s.selection_start=1;s.selection_length=2;s.cursor_byte_offset=2;s.state=ControlState::Focus;s.focus_visible=true;s.has_transition=matches!(kind,WidgetKind::ButtonGhost|WidgetKind::TreeRow);s.transition_from_color=0x00000000;s.transition_to_color=0x224466ff;s.transition_progress_permille=500;assert!(widget_paint(&mut canvas,&s),"painter path {kind:?}");}
    let bad=WidgetDrawSpec::new(WidgetKind::Label,Rect{x:u32::MAX-1,y:0,width:1,height:1});assert!(!widget_paint(&mut canvas,&bad));let mut bad=WidgetDrawSpec::new(WidgetKind::Segmented,Rect{x:0,y:0,width:20,height:20});bad.segment_count=9;assert!(!widget_paint(&mut canvas,&bad));
    let mut tiny_pixels=vec![0x5au8;48*48*4];let before=tiny_pixels.clone();let mut tiny_canvas=WidgetCanvas::new(&mut tiny_pixels,48*48*4,48,48,48*4,1,Theme::Light,false,None).unwrap();let mut tiny=WidgetDrawSpec::new(WidgetKind::EmptyState,Rect{x:8,y:8,width:16,height:30});tiny.label=Some("Empty");assert!(!widget_paint(&mut tiny_canvas,&tiny));assert_eq!(tiny_canvas.rgba,before,"too-narrow empty state fails before writing pixels");
    let mut direct_empty_pixels=vec![0x69u8;48*48*4];let empty_before=direct_empty_pixels.clone();let mut direct_empty=WidgetCanvas::new(&mut direct_empty_pixels,48*48*4,48,48,48*4,1,Theme::Light,false,None).unwrap();let mut near_empty=WidgetDrawSpec::new(WidgetKind::EmptyState,Rect{x:8,y:u32::MAX-40,width:48,height:32});near_empty.secondary=Some("Details");assert!(!direct_empty.paint_empty_state(&near_empty));assert_eq!(direct_empty.rgba,empty_before,"direct empty-state overflow fails before writing pixels");
    let mut direct_row_pixels=vec![0x96u8;48*48*4];let row_before=direct_row_pixels.clone();let mut direct_row=WidgetCanvas::new(&mut direct_row_pixels,48*48*4,48,48,48*4,1,Theme::Light,false,None).unwrap();let mut near_row=WidgetDrawSpec::new(WidgetKind::TreeRow,Rect{x:u32::MAX-100,y:8,width:64,height:32});near_row.indent=6;near_row.has_children=true;near_row.label=Some("Layer");assert!(!direct_row.paint_tree_row(&near_row));assert_eq!(direct_row.rgba,row_before,"direct tree-row overflow fails before writing pixels");
    near_row.bounds=Rect{x:u32::MAX-1024,y:8,width:128,height:28};near_row.indent=255;assert!(!direct_row.paint_tree_row(&near_row));assert_eq!(direct_row.rgba,row_before,"full indent must be checked beyond the former fixed margin");
    for kind in kinds{let mut overflow=WidgetDrawSpec::new(kind,Rect{x:u32::MAX-8,y:8,width:16,height:16});overflow.label=Some("Overflow");assert!(!widget_paint(&mut direct_row,&overflow),"overflow geometry for {kind:?}");assert_eq!(direct_row.rgba,row_before,"no mutation for {kind:?}");}
}
