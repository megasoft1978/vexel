use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::{Rc, Weak};

use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::{MainThread, guard_native_call};
use super::geometry;
use super::queue::EventQueue;
use super::types::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppPhase { Running, ShuttingDown, Stopped }
struct WindowRecord { native: Retained<NSWindow>, content: Retained<NSView>, render: Retained<NSView>, queue: EventQueue, text_input: Option<TextInputState>, ticks: super::ticks::TickState }
struct WindowSlot { generation: u32, retired: bool, record: Option<WindowRecord> }
struct AppState { phase: AppPhase, slots: Vec<WindowSlot>, free: Vec<u32> }

pub struct NativeApp { state: Rc<RefCell<AppState>>, main_thread: MainThread, application: Retained<NSApplication> }
pub struct Window { owner: Weak<RefCell<AppState>>, id: WindowId }

impl NativeApp {
    pub fn start(main_thread: MainThread) -> Result<Self, WindowError> {
        let app = guard_native_call(|| {
            let native = NSApplication::sharedApplication(main_thread.marker());
            let _ = native.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            native.finishLaunching();
            native
        }).map_err(WindowError::Native)?;
        Ok(Self { state: Rc::new(RefCell::new(AppState { phase: AppPhase::Running, slots: Vec::new(), free: Vec::new() })), main_thread, application: app })
    }

    pub fn create_window(&mut self, config: WindowConfig) -> Result<Window, WindowError> {
        if !config.valid() { return Err(WindowError::InvalidConfig); }
        let title = NSString::from_str(&config.title);
        let mut style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
        if config.resizable { style |= NSWindowStyleMask::Resizable; }
        let marker = self.main_thread.marker();
        let (native, content) = guard_native_call(|| unsafe {
            let window = NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(marker),
                NSRect::new(NSPoint::new(80.0, 80.0), NSSize::new(config.width_points as f64, config.height_points as f64)),
                style, NSBackingStoreType::Buffered, false,
            );
            window.setTitle(&title);
            window.setContentMaxSize(NSSize::new(4096.0, 4096.0));
            window.setReleasedWhenClosed(false);
            window.contentView().map(|content| (window, content))
        }).map_err(WindowError::Native)?.ok_or(WindowError::InvalidConfig)?;
        let mut state = self.state.borrow_mut();
        if state.phase != AppPhase::Running { return Err(WindowError::ExpiredOwner); }
        let index = if let Some(index) = state.free.pop() { index } else {
            let index = u32::try_from(state.slots.len()).map_err(|_| WindowError::InvalidConfig)?;
            state.slots.push(WindowSlot { generation: 1, retired: false, record: None }); index
        };
        let slot = &mut state.slots[index as usize];
        let id = WindowId { slot: index, generation: slot.generation };
        let render = content.clone();
        let scale = guard_native_call(|| {
            let backing=native.convertRectToBacking(NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(1.0,1.0)));
            backing.size.width
        }).map_err(WindowError::Native)?;
        let pixels=geometry::pixels(Size{width:1.0,height:1.0},scale).ok_or(WindowError::InvalidConfig)?;
        let mut queue = EventQueue::default();
        queue.push(WindowEvent::Open, true);
        let pts = Size { width: config.width_points as f64, height: config.height_points as f64 };
        let size_pixels=geometry::pixels(pts,scale).ok_or(WindowError::InvalidConfig)?;
        let _=pixels;
        queue.push(WindowEvent::Resize { points: pts, pixels:size_pixels }, true);
        queue.push(WindowEvent::BackingScaleChanged { scale, pixels:size_pixels }, true);
        slot.record = Some(WindowRecord { native: native.clone(), content, render, queue, text_input: None, ticks: super::ticks::TickState::default() });
        drop(state);
        if !config.initially_hidden { let _ = guard_native_call(|| native.makeKeyAndOrderFront(None)).map_err(WindowError::Native)?; }
        Ok(Window { owner: Rc::downgrade(&self.state), id })
    }

    pub fn poll_events(&mut self, window: WindowId, max: NonZeroUsize) -> Result<EventBatch, WindowError> {
        let mut state = self.state.borrow_mut(); let record = record_mut(&mut state, window)?;
        Ok(record.queue.drain(validate_poll_limit(max).get(), false))
    }

    pub fn destroy_window(&mut self, id: WindowId) -> Result<(), WindowError> {
        let native = {
            let mut state = self.state.borrow_mut();
            let slot = slot_mut(&mut state, id)?;
            if let Some(mut record) = slot.record.take() {
                record.ticks.set(false);
                record.text_input = None;
                slot.generation = super::teardown::next_generation(slot.generation).unwrap_or_else(|| { slot.retired = true; 0 });
                if !slot.retired { state.free.push(id.slot); }
                Some(record.native)
            } else { return Err(WindowError::ExpiredWindow); }
        };
        if let Some(native) = native { guard_native_call(|| native.close()).map_err(WindowError::Native)?; }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), WindowError> {
        let windows = {
            let mut state = self.state.borrow_mut();
            if state.phase == AppPhase::Stopped { return Ok(()); }
            state.phase = AppPhase::ShuttingDown;
            let mut windows = Vec::new();
            for slot in &mut state.slots {
                if let Some(mut record) = slot.record.take() {
                    slot.generation = super::teardown::next_generation(slot.generation).unwrap_or_else(|| { slot.retired = true; 0 });
                    record.ticks.set(false); record.text_input = None; windows.push(record.native);
                }
            }
            state.free.clear(); windows
        };
        for window in windows { guard_native_call(|| window.close()).map_err(WindowError::Native)?; }
        let mut state = self.state.borrow_mut(); state.phase = AppPhase::Stopped;
        let _ = &self.application;
        Ok(())
    }
}

impl Drop for NativeApp { fn drop(&mut self) { let _ = self.shutdown(); } }
fn slot(state: &AppState, id: WindowId) -> Result<&WindowSlot, WindowError> {
    if state.phase != AppPhase::Running { return Err(WindowError::ExpiredOwner); }
    let slot = state.slots.get(id.slot as usize).ok_or(WindowError::ExpiredWindow)?;
    if slot.generation != id.generation || slot.record.is_none() { return Err(WindowError::ExpiredWindow); }
    Ok(slot)
}
fn slot_mut(state: &mut AppState, id: WindowId) -> Result<&mut WindowSlot, WindowError> {
    if state.phase != AppPhase::Running { return Err(WindowError::ExpiredOwner); }
    let slot = state.slots.get_mut(id.slot as usize).ok_or(WindowError::ExpiredWindow)?;
    if slot.generation != id.generation || slot.record.is_none() { return Err(WindowError::ExpiredWindow); }
    Ok(slot)
}
fn record(state: &AppState, id: WindowId) -> Result<&WindowRecord, WindowError> { slot(state,id)?.record.as_ref().ok_or(WindowError::ExpiredWindow) }
fn record_mut(state: &mut AppState, id: WindowId) -> Result<&mut WindowRecord, WindowError> { slot_mut(state,id)?.record.as_mut().ok_or(WindowError::ExpiredWindow) }
impl Window {
    pub fn id(&self) -> WindowId { self.id }
    fn native_handles(&self) -> Result<(Retained<NSWindow>, Retained<NSView>),WindowError> {
        let owner=self.owner.upgrade().ok_or(WindowError::ExpiredOwner)?; let state=owner.borrow();
        let r=record(&state,self.id)?; Ok((r.native.clone(),r.content.clone()))
    }
    fn with_record<T>(&self, f: impl FnOnce(&mut WindowRecord) -> Result<T,WindowError>) -> Result<T,WindowError> {
        let owner=self.owner.upgrade().ok_or(WindowError::ExpiredOwner)?;
        let mut state=owner.borrow_mut(); if state.phase != AppPhase::Running { return Err(WindowError::ExpiredOwner); }
        f(record_mut(&mut state,self.id)?)
    }
    pub fn set_title(&mut self,title:String)->Result<(),WindowError>{ if title.is_empty()||title.len()>1024||title.contains('\0'){return Err(WindowError::InvalidConfig)} let (n,_)=self.native_handles()?; let value=NSString::from_str(&title); guard_native_call(||n.setTitle(&value)).map_err(WindowError::Native) }
    pub fn set_visible(&mut self,visible:bool)->Result<(),WindowError>{ let (n,_)=self.native_handles()?; guard_native_call(||if visible {n.makeKeyAndOrderFront(None)} else {n.orderOut(None)}).map_err(WindowError::Native) }
    pub fn resize(&mut self,size:Size)->Result<(),WindowError>{ if !size.width.is_finite()||!size.height.is_finite()||!(1.0..=4096.0).contains(&size.width)||!(1.0..=4096.0).contains(&size.height){return Err(WindowError::InvalidConfig)} let (n,_)=self.native_handles()?; guard_native_call(||n.setContentSize(NSSize::new(size.width,size.height))).map_err(WindowError::Native) }
    pub fn content_screen_origin(&self)->Result<Point,WindowError>{
        use objc2_foundation::{NSPoint, NSRect, NSSize};
        let (native,content)=self.native_handles()?;
        guard_native_call(|| { let bounds=content.bounds(); let rect=native.convertRectToScreen(NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(bounds.size.width,bounds.size.height))); Point{x:rect.origin.x,y:rect.origin.y} }).map_err(WindowError::Native)
    }
    pub fn set_cursor(&mut self,cursor:Cursor)->Result<(),WindowError>{
        use objc2_app_kit::NSCursor;
        let _handles=self.native_handles()?;
        let native=match cursor {
            Cursor::Arrow=>NSCursor::arrowCursor(), Cursor::OpenHand=>NSCursor::openHandCursor(), Cursor::ClosedHand=>NSCursor::closedHandCursor(),
            Cursor::ResizeNorthSouth=>NSCursor::resizeUpDownCursor(), Cursor::ResizeEastWest=>NSCursor::resizeLeftRightCursor(),
            Cursor::ResizeNorthwestSoutheast|Cursor::ResizeNortheastSouthwest|Cursor::Rotate=>return Err(WindowError::InvalidConfig),
        };
        guard_native_call(|| native.set()).map_err(WindowError::Native)
    }
    pub fn is_dark_appearance(&self)->Result<bool,WindowError>{
        use objc2_app_kit::{NSAppearanceCustomization, NSAppearanceNameDarkAqua};
        let (n,_)=self.native_handles()?;
        guard_native_call(|| { let name=n.effectiveAppearance().name();
            // SAFETY: AppKit's process-lifetime static appearance-name object is valid here.
            name.isEqualToString(unsafe { NSAppearanceNameDarkAqua }) }).map_err(WindowError::Native)
    }
    /// The pinned AppKit feature set omits `NSVisualEffectView`; see this order's
    /// result artifact. Returning a fabricated success or unsupported answer would
    /// hide that prerequisite, so the operation is rejected explicitly.
    pub fn set_material_enabled(&mut self,_enabled:bool)->Result<bool,WindowError>{ let _handles=self.native_handles()?; Err(WindowError::InvalidConfig) }
    pub fn set_text_input_state(&mut self,state:TextInputState)->Result<(),WindowError>{ if !validate_text_state(self.id,&state){return Err(WindowError::InvalidTextInputState)} self.with_record(|r|{r.text_input=Some(state);Ok(())}) }
    pub fn clear_text_input_state(&mut self)->Result<(),WindowError>{self.with_record(|r|{r.text_input=None;Ok(())})}
    pub fn acknowledge_input_reset(&mut self,sequence:u64)->Result<(),WindowError>{self.with_record(|r|if r.queue.acknowledge_reset(sequence){Ok(())}else{Err(WindowError::QueueFault)})}
    pub fn request_ticks(&mut self,enabled:bool)->Result<(),WindowError>{self.with_record(|r|{r.ticks.set(enabled);Ok(())})}
    pub(crate) fn surface_view(&self)->Result<Retained<NSView>,WindowError>{ let owner=self.owner.upgrade().ok_or(WindowError::ExpiredOwner)?; let state=owner.borrow(); Ok(record(&state,self.id)?.render.clone()) }
}
impl Drop for Window { fn drop(&mut self) {} }
