//! Audited Objective-C IMP boundary: all callbacks enter route::callback.
//! AppKit objects are main-thread-only; ivars contain weak Rust routes only.
use super::{
    ime_native as ime, input,
    route::{RouteState, callback},
    types::*,
};
use crate::MainThread;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, Sel};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSTextInputClient, NSView, NSWindow, NSWindowDelegate};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSNotification, NSPoint, NSRange,
    NSRangePointer, NSRect, NSTimer,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub(crate) struct RouteIvars {
    route: RefCell<Weak<RouteState>>,
}
pub(crate) struct TickIvars {
    route: Weak<RouteState>,
    generation: u64,
}

define_class!(
    // SAFETY: designated initWithFrame: is called after Rust ivars initialization;
    // no custom destructor, all NSView/NSResponder callbacks keep AppKit ABI.
    #[unsafe(super(NSView))]
    #[thread_kind=MainThreadOnly]
    #[ivars=RouteIvars]
    pub(crate) struct NativeView;
    unsafe impl NSObjectProtocol for NativeView {}
    impl NativeView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first(&self)->bool {callback(&self.weak_route(),false,|_|true)}
        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {callback(&self.weak_route(),(),|r| {
            // SAFETY: NSView superclass implements the exact no-argument void method.
            unsafe {let _:()=msg_send![super(self),viewDidChangeEffectiveAppearance];}
            if let Some((w,_))=r.native(){r.emit(WindowEvent::AppearanceChanged{dark:super::geometry::dark(&w)},true);}
        });}
        #[unsafe(method(keyDown:))]
        fn key_down(&self,event:&NSEvent) {callback(&self.weak_route(),(),|r| { input::key(r,event,true); self.interpretKeyEvents(&NSArray::from_slice(&[event])); });}
        #[unsafe(method(keyUp:))]
        fn key_up(&self,event:&NSEvent) {callback(&self.weak_route(),(),|r|input::key(r,event,false));}
        #[unsafe(method(mouseMoved:))]
        fn moved(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Move));}
        #[unsafe(method(mouseDragged:))]
        fn dragged(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Move));}
        #[unsafe(method(rightMouseDragged:))]
        fn right_dragged(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Move));}
        #[unsafe(method(otherMouseDragged:))]
        fn other_dragged(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Move));}
        #[unsafe(method(mouseDown:))]
        fn down(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Down));}
        #[unsafe(method(mouseUp:))]
        fn up(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Up));}
        #[unsafe(method(rightMouseDown:))]
        fn right_down(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Down));}
        #[unsafe(method(rightMouseUp:))]
        fn right_up(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Up));}
        #[unsafe(method(otherMouseDown:))]
        fn other_down(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Down));}
        #[unsafe(method(otherMouseUp:))]
        fn other_up(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::pointer(r,self,event,PointerKind::Up));}
        #[unsafe(method(scrollWheel:))]
        fn scroll(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::scroll(r,self,event));}
        #[unsafe(method(magnifyWithEvent:))]
        fn magnify(&self,event:&NSEvent){callback(&self.weak_route(),(),|r|input::magnify(r,self,event));}
    }
    // SAFETY: required NSTextInputClient selectors below have exact generated ABI;
    // borrowed native arguments are converted synchronously to bounded owned values.
    unsafe impl NSTextInputClient for NativeView {
        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self,string:&AnyObject,replacement:NSRange){callback(&self.weak_route(),(),|r|ime::insert(r,string,replacement));}
        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn marked_text(&self,string:&AnyObject,selected:NSRange,replacement:NSRange){callback(&self.weak_route(),(),|r|ime::marked(r,string,selected,replacement));}
        #[unsafe(method(unmarkText))]
        fn unmark(&self){callback(&self.weak_route(),(),|r|ime::unmark(r));}
        #[unsafe(method(hasMarkedText))]
        fn has_marked(&self)->bool{callback(&self.weak_route(),false,|r|ime::selected(r,true).length>0)}
        #[unsafe(method(markedRange))]
        fn marked_range(&self)->NSRange{callback(&self.weak_route(),ime::missing(),|r|ime::selected(r,true))}
        #[unsafe(method(selectedRange))]
        fn selected_range(&self)->NSRange{callback(&self.weak_route(),ime::missing(),|r|ime::selected(r,false))}
        #[unsafe(method_id(validAttributesForMarkedText))]
        fn valid_attributes(&self)->Retained<NSArray<NSAttributedStringKey>> {
            // An NSArray is required by this ABI. Failure creating even the empty
            // fallback terminates; no unguarded message exception may leave an IMP.
            let fallback=crate::guard_native_call(NSArray::new).unwrap_or_else(|_|std::process::abort());
            callback(&self.weak_route(),fallback,|_|ime::attributes())
        }
        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        fn substring(&self,range:NSRange,actual:NSRangePointer)->Option<Retained<NSAttributedString>> {
            // Initialize the ABI output even when the weak route has expired.
            let _=crate::guard_native_call(||unsafe{if !actual.is_null(){actual.write(ime::missing());}});
            callback(&self.weak_route(),None,|r| {
                // SAFETY: NSTextInputClient gives a writable NSRange* or null.
                unsafe {if !actual.is_null(){actual.write(ime::missing());}}
                let (value,range)=ime::substring(r,range)?;
                unsafe {if !actual.is_null(){actual.write(range);}} Some(value)
            })
        }
        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect(&self,range:NSRange,actual:NSRangePointer)->NSRect {
            let _=crate::guard_native_call(||unsafe{if !actual.is_null(){actual.write(ime::missing());}});
            callback(&self.weak_route(),ime::zero_rect(),|r| {
                // SAFETY: writable protocol output pointer or null, stack value only.
                unsafe {if !actual.is_null(){actual.write(ime::missing());}}
                let Some((rect,range))=ime::first_rect(r,range) else{return ime::zero_rect()};
                unsafe {if !actual.is_null(){actual.write(range);}} rect
            })
        }
        #[unsafe(method(characterIndexForPoint:))]
        fn character_index(&self,point:NSPoint)->usize{callback(&self.weak_route(),ime::NOT_FOUND,|r|ime::character_index(r,point))}
        #[unsafe(method(doCommandBySelector:))]
        fn command(&self,selector:Sel){callback(&self.weak_route(),(),|r|{let name=selector.name();if name.to_bytes().len()<=1024{r.emit(WindowEvent::TextCommand(match name.to_str(){Ok(name)=>name.to_owned(),Err(_)=>{r.fault(QueueFaultReason::InvalidTextRange);return;}}),false)}else{r.fail(QueueFaultReason::InputOverflow)}});}
    }
);
impl NativeView {
    fn weak_route(&self) -> Weak<RouteState> {
        // Fence callback setup too; a corrupt route borrow cannot unwind from an IMP.
        crate::guard_native_call(|| self.ivars().route.borrow().clone())
            .unwrap_or_else(|_| std::process::abort())
    }
    pub(crate) fn clear_route(&self) {
        *self.ivars().route.borrow_mut() = Weak::new();
    }
    pub(crate) fn new(mt: MainThread, route: &Rc<RouteState>, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mt.marker()).set_ivars(RouteIvars {
            route: RefCell::new(Rc::downgrade(route)),
        });
        // SAFETY: initialized ivars, designated superclass initializer, retained +1 return.
        unsafe { msg_send![super(this),initWithFrame:frame] }
    }
}

define_class!(
    // SAFETY: NSObject initialization, no custom destructor; delegate is main-thread-only.
    #[unsafe(super(NSObject))]
    #[thread_kind=MainThreadOnly]
    #[ivars=RouteIvars]
    pub(crate) struct NativeDelegate;
    unsafe impl NSObjectProtocol for NativeDelegate {}
    unsafe impl NSWindowDelegate for NativeDelegate {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _sender: &NSWindow) -> bool {
            callback(&self.weak_route(), false, |r| {
                r.queue.borrow_mut().request_close();
                super::ticks::wake();
                false
            })
        }
        #[unsafe(method(windowDidBecomeKey:))]
        fn focused(&self, _note: &NSNotification) {
            callback(&self.weak_route(), (), |r| {
                r.focused.set(true);
                r.declare_no_text_target();
                r.emit(WindowEvent::Focus(true), true)
            });
        }
        #[unsafe(method(windowDidResignKey:))]
        fn blurred(&self, _note: &NSNotification) {
            callback(&self.weak_route(), (), |r| {
                r.focused.set(false);
                r.composition.borrow_mut().take();
                r.text.borrow_mut().take();
                r.no_target.set(false);
                r.queue.borrow_mut().set_external_payload(0);
                r.emit(WindowEvent::Focus(false), true)
            });
        }
        #[unsafe(method(windowDidResize:))]
        fn resized(&self, _note: &NSNotification) {
            callback(&self.weak_route(), (), |r| size(r, false));
        }
        #[unsafe(method(windowDidChangeBackingProperties:))]
        fn backing(&self, _note: &NSNotification) {
            callback(&self.weak_route(), (), |r| size(r, true));
        }
    }
);
impl NativeDelegate {
    fn weak_route(&self) -> Weak<RouteState> {
        // Fence callback setup too; a corrupt route borrow cannot unwind from an IMP.
        crate::guard_native_call(|| self.ivars().route.borrow().clone())
            .unwrap_or_else(|_| std::process::abort())
    }
    pub(crate) fn clear_route(&self) {
        *self.ivars().route.borrow_mut() = Weak::new();
    }
    pub(crate) fn new(mt: MainThread, route: &Rc<RouteState>) -> Retained<Self> {
        let this = Self::alloc(mt.marker()).set_ivars(RouteIvars {
            route: RefCell::new(Rc::downgrade(route)),
        });
        unsafe { msg_send![super(this), init] }
    }
}
pub(crate) fn size(route: &RouteState, backing: bool) {
    let Some((window, view)) = route.native() else {
        return;
    };
    let b = view.bounds();
    let points = Size {
        width: b.size.width,
        height: b.size.height,
    };
    let Some(scale) = super::geometry::scale(&window) else {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    };
    let Some(pixels) = super::geometry::pixels(points, scale) else {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    };
    route.emit(
        if backing {
            WindowEvent::BackingScaleChanged { scale, pixels }
        } else {
            WindowEvent::Resize { points, pixels }
        },
        false,
    );
}

define_class!(
    // SAFETY: NSTimer target uses the tick:(NSTimer*) void ABI on the main run loop.
    #[unsafe(super(NSObject))]
    #[thread_kind=MainThreadOnly]
    #[ivars=TickIvars]
    pub(crate) struct TickTarget;
    unsafe impl NSObjectProtocol for TickTarget {}
    impl TickTarget {
        #[unsafe(method(tick:))]
        fn tick(&self,_timer:&NSTimer){callback(&self.ivars().route,(),|r| {if r.tick_generation.get()==self.ivars().generation{emit_tick(r,self.ivars().generation,objc2_foundation::NSProcessInfo::processInfo().systemUptime())}});}
    }
);
impl TickTarget {
    pub(crate) fn new(mt: MainThread, route: &Rc<RouteState>, generation: u64) -> Retained<Self> {
        let this = Self::alloc(mt.marker()).set_ivars(TickIvars {
            route: Rc::downgrade(route),
            generation,
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub(crate) fn emit_tick(route: &RouteState, generation: u64, seconds: f64) {
    if route.active.get() && route.tick_generation.get() == generation {
        if let Some(seconds) = route.timestamp(seconds) {
            route.emit(
                WindowEvent::AnimationTick {
                    monotonic_seconds: seconds,
                },
                false,
            );
        }
    }
}
