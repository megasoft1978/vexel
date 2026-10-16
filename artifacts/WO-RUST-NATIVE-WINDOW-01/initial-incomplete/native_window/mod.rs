mod app;
mod geometry;
mod ime;
mod queue;
mod teardown;
mod ticks;
mod types;

pub use app::{NativeApp, Window};
pub use types::{Composition, Cursor, EventBatch, Modifiers, Point, PointerEvent, QueueFault, QueueFaultReason, Rect, ScrollEvent, Size, SizeU32, TextAttributeKind, TextAttributeSpan, TextInputState, WindowConfig, WindowError, WindowEvent, WindowId};

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    #[test]
    fn geometry_checks_scale_rounding_and_overflow() {
        assert_eq!(geometry::pixels(Size { width: 10.25, height: 6.25 }, 2.0), Some(SizeU32 { width: 21, height: 13 }));
        assert_eq!(geometry::pixels(Size { width: 10.0, height: 6.0 }, 1.0), Some(SizeU32 { width: 10, height: 6 }));
        assert_eq!(geometry::pixels(Size { width: f64::MAX, height: 1.0 }, 3.0), None);
        assert_eq!(geometry::pixels(Size { width: 10.0, height: 6.0 }, 1.5), None);
    }

    #[test]
    fn point_origin_keeps_negative_display_coordinates() {
        let value=geometry::checked_screen_origin(
            Rect { origin:Point{x:-1920.0,y:0.0},size:Size{width:1920.0,height:1080.0} },
            Rect { origin:Point{x:-1920.0,y:0.0},size:Size{width:1920.0,height:1080.0} },
            Rect { origin:Point{x:-1800.0,y:300.0},size:Size{width:500.0,height:400.0} });
        assert_eq!(value,Some(Point{x:-1800.0,y:380.0}));
        assert_eq!(geometry::top_left(Point{x:1.0,y:20.0},50.0),Some(Point{x:1.0,y:30.0}));
    }

    #[test]
    fn utf16_ranges_round_trip_and_reject_surrogate_interiors() {
        assert_eq!(ime::utf8_to_utf16("a🦀z",1..5),Some(1..3));
        assert_eq!(ime::utf16_to_utf8("a🦀z",1..3),Some(1..5));
        assert_eq!(ime::utf16_to_utf8("a🦀z",2..3),None);
        assert_eq!(ime::utf16_to_utf8("a🦀z",0..99),None);
    }

    #[test]
    fn queue_coalesces_only_adjacent_motion_and_batches_are_bounded() {
        let mut q=queue::EventQueue::default();
        q.push(WindowEvent::Pointer(PointerEvent{at:Point{x:1.0,y:1.0},button:None,click_count:0,modifiers:Modifiers::default(),timestamp_seconds:1.0}),false);
        q.push(WindowEvent::Pointer(PointerEvent{at:Point{x:2.0,y:1.0},button:None,click_count:0,modifiers:Modifiers::default(),timestamp_seconds:2.0}),false);
        let batch=q.drain(NonZeroUsize::new(1).unwrap().get(),false);
        assert_eq!(batch.events.len(),1);
        assert!(matches!(&batch.events[0],WindowEvent::Pointer(p) if p.at.x==2.0));
        assert!(!batch.pending);
    }

    #[test]
    fn modifiers_are_mapped_to_ui_order() {
        let mapped=Modifiers::from_native_bits((1<<17)|(1<<20)|(1<<19)|(1<<18));
        assert_eq!(mapped.0,15);
    }

    #[test]
    fn title_and_text_input_snapshots_enforce_ownership_and_bounds() {
        let id=WindowId{slot:2,generation:7};
        let cfg=WindowConfig{title:"window".into(),width_points:1,height_points:4096,resizable:true,initially_hidden:true};
        assert!(cfg.valid());
        let bad=WindowConfig{title:"bad\0title".into(),..cfg.clone()};
        assert!(!bad.valid());
        let state=TextInputState{source_window:id,text:"a🦀".into(),selection_utf8:1..5,marked_utf8:Some(1..5),marked_attributes:vec![],candidate_rect_points:Rect{origin:Point{x:1.0,y:2.0},size:Size{width:3.0,height:4.0}}};
        assert!(types::validate_text_state(id,&state));
        assert!(!types::validate_text_state(WindowId{generation:8,..id},&state));
        let invalid=TextInputState{selection_utf8:2..5,..state};
        assert!(!types::validate_text_state(id,&invalid));
    }

    #[test]
    fn overflow_reset_and_close_are_sticky_and_ordered() {
        let mut q=queue::EventQueue::default();
        for _ in 0..240 { q.push(WindowEvent::KeyDown{code:1,modifiers:Modifiers::default()},false); }
        for _ in 0..17 { q.push(WindowEvent::Focus(true),true); }
        q.request_close();
        let first=q.drain(NonZeroUsize::new(256).unwrap().get(),false);
        assert!(first.pending);
        let batch=q.drain(NonZeroUsize::new(4).unwrap().get(),false);
        let reset=batch.events.iter().position(|e| matches!(e,WindowEvent::InputReset{..})).unwrap();
        let close=batch.events.iter().position(|e| matches!(e,WindowEvent::CloseRequested)).unwrap();
        assert_eq!(first.events.iter().filter(|e| matches!(e,WindowEvent::Focus(_))).count(),16);
        assert!(close>reset);
        let seq=match &batch.events[reset] { WindowEvent::InputReset{sequence,..}=>*sequence,_=>unreachable!() };
        assert!(q.acknowledge_reset(seq));
    }
}
