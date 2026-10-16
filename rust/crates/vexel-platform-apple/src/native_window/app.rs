pub use super::window::Window;
use super::{
    callbacks::{NativeDelegate, NativeView},
    route::RouteState,
    types::*,
};
use crate::{MainThread, guard_native_call};
use objc2::{MainThreadOnly, rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSAutoresizingMaskOptions, NSBackingStoreType,
    NSEventMask, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    NSWindowTabbingMode,
};
use objc2_foundation::{
    NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSRunLoop, NSSize, NSString, NSTimer,
};
use objc2_quartz_core::CALayer;
use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppPhase {
    Running,
    ShuttingDown,
    Stopped,
}
pub(crate) struct WindowRecord {
    pub native: Retained<NSWindow>,
    pub _content: Retained<NSView>,
    pub view: Retained<NativeView>,
    pub material: Retained<NSVisualEffectView>,
    pub presentation_layer: Option<Retained<CALayer>>,
    pub _delegate: Retained<NativeDelegate>,
    pub route: Rc<RouteState>,
    pub timer: Option<(Retained<NSTimer>, u64)>,
}
pub(crate) struct WindowSlot {
    pub generation: u32,
    pub retired: bool,
    pub record: Option<WindowRecord>,
}
pub(crate) struct AppState {
    pub phase: AppPhase,
    pub slots: Vec<WindowSlot>,
    pub free: Vec<u32>,
    pub main_thread: Option<MainThread>,
}
thread_local! {static ACTIVE:RefCell<Weak<RefCell<AppState>>>=RefCell::new(Weak::new());}

/// Owns all registered native records on Apple's process main thread.
/// ```compile_fail
/// use vexel_platform_apple::NativeApp;
/// fn require_send<T: Send>() {} require_send::<NativeApp>();
/// ```
/// ```compile_fail
/// use vexel_platform_apple::NativeApp;
/// fn require_sync<T: Sync>() {} require_sync::<NativeApp>();
/// ```
pub struct NativeApp {
    pub(crate) state: Rc<RefCell<AppState>>,
    main_thread: MainThread,
    application: Retained<NSApplication>,
}
impl NativeApp {
    pub fn start(main_thread: MainThread) -> Result<Self, WindowError> {
        if ACTIVE.with(|a| {
            a.borrow()
                .upgrade()
                .is_some_and(|s| s.borrow().phase != AppPhase::Stopped)
        }) {
            return Err(WindowError::InvalidConfig);
        }
        let application =
            guard_native_call(|| NSApplication::sharedApplication(main_thread.marker()))
                .map_err(WindowError::Native)?;
        let startup = guard_native_call(|| {
            if !application.setActivationPolicy(NSApplicationActivationPolicy::Regular) {
                return Err(WindowError::InvalidConfig);
            }
            application.finishLaunching();
            let _ = NSRunLoop::mainRunLoop();
            Ok(())
        })
        .map_err(WindowError::Native)
        .and_then(|r| r);
        if let Err(error) = startup {
            let _ = guard_native_call(|| application.stop(None));
            return Err(error);
        }
        let state = Rc::new(RefCell::new(AppState {
            phase: AppPhase::Running,
            slots: vec![],
            free: vec![],
            main_thread: Some(main_thread),
        }));
        ACTIVE.with(|a| *a.borrow_mut() = Rc::downgrade(&state));
        Ok(Self {
            state,
            main_thread,
            application,
        })
    }
    #[allow(deprecated)] // Baseline activation API, available on macOS13.
    pub fn create_window(&mut self, config: WindowConfig) -> Result<Window, WindowError> {
        if self.state.borrow().phase != AppPhase::Running {
            return Err(WindowError::ExpiredOwner);
        }
        if !config.valid() {
            return Err(WindowError::InvalidConfig);
        }
        let id = {
            let mut state = self.state.borrow_mut();
            let slot = if let Some(slot) = state.free.pop() {
                slot
            } else {
                let slot =
                    u32::try_from(state.slots.len()).map_err(|_| WindowError::InvalidConfig)?;
                state
                    .slots
                    .try_reserve(1)
                    .map_err(|_| WindowError::QueueFault)?;
                let additional = state.slots.len() + 1 - state.free.len();
                state
                    .free
                    .try_reserve(additional)
                    .map_err(|_| WindowError::QueueFault)?;
                state.slots.push(WindowSlot {
                    generation: 1,
                    retired: false,
                    record: None,
                });
                slot
            };
            WindowId {
                slot,
                generation: state.slots[slot as usize].generation,
            }
        };
        let route = RouteState::new(id);
        let result = guard_native_call(|| create_record(self.main_thread, &config, route.clone()))
            .map_err(WindowError::Native)
            .and_then(|r| r);
        let created = match result {
            Ok(record) => record,
            Err(error) => {
                route.invalidate();
                self.release_slot(id);
                return Err(error);
            }
        };
        // Native calls, including convertRectToBacking, completed before this borrow.
        let mut created = Some(created);
        {
            let mut state = self.state.borrow_mut();
            if state.phase == AppPhase::Running
                && state.slots[id.slot as usize].generation == id.generation
            {
                state.slots[id.slot as usize].record = created.take();
            }
        }
        if let Some(created) = created {
            let _ = super::teardown::destroy(created);
            return Err(WindowError::ExpiredOwner);
        }
        let window = Window {
            owner: Rc::downgrade(&self.state),
            id,
        };
        if !config.initially_hidden {
            let (native, view) = {
                let state = self.state.borrow();
                let record = record(&state, id)?;
                (record.native.clone(), record.view.clone())
            };
            let shown = guard_native_call(|| {
                native.makeKeyAndOrderFront(None);
                if !native.makeFirstResponder(Some(&view)) {
                    return Err(WindowError::InvalidConfig);
                }
                self.application.activateIgnoringOtherApps(true);
                Ok(())
            })
            .map_err(WindowError::Native)
            .and_then(|r| r);
            if let Err(error) = shown {
                let _ = self.destroy_window(id);
                return Err(error);
            }
        }
        Ok(window)
    }
    fn release_slot(&mut self, id: WindowId) {
        let mut state = self.state.borrow_mut();
        let slot = &mut state.slots[id.slot as usize];
        slot.generation = super::teardown::next_generation(slot.generation).unwrap_or_else(|| {
            slot.retired = true;
            0
        });
        if !slot.retired {
            state.free.push(id.slot);
        }
    }
    pub fn poll_events(
        &mut self,
        id: WindowId,
        max: NonZeroUsize,
    ) -> Result<EventBatch, WindowError> {
        let route = {
            let state = self.state.borrow();
            record(&state, id)?.route.clone()
        };
        if let Some(fault) = route.native_fault.get() {
            if route.queue.borrow().has_pending() {
                return Ok(route.queue.borrow_mut().drain(max.get(), true));
            }
            return Err(WindowError::Native(fault));
        }
        let pending = guard_native_call(|| {
            objc2::rc::autoreleasepool(|_| {
                // All callbacks run after releasing both app and queue RefCell borrows.
                let past = NSDate::distantPast();
                // SAFETY: baseline process-lifetime Foundation mode NSString.
                let mode = unsafe { NSDefaultRunLoopMode };
                NSRunLoop::mainRunLoop().runMode_beforeDate(mode, &NSDate::date());
                for _ in 0..super::queue::EventQueue::native_budget() {
                    let Some(event) = self
                        .application
                        .nextEventMatchingMask_untilDate_inMode_dequeue(
                            NSEventMask::Any,
                            Some(&past),
                            mode,
                            true,
                        )
                    else {
                        break;
                    };
                    self.application.sendEvent(&event);
                    if route.native_fault.get().is_some() {
                        break;
                    }
                }
                self.application
                    .nextEventMatchingMask_untilDate_inMode_dequeue(
                        NSEventMask::Any,
                        Some(&past),
                        mode,
                        false,
                    )
                    .is_some()
            })
        })
        .map_err(|fault| {
            route.native_fault.set(Some(fault));
            route.fail(QueueFaultReason::NativeCallbackFailure);
            WindowError::Native(fault)
        })?;
        Ok(route.queue.borrow_mut().drain(max.get(), pending))
    }
    pub fn destroy_window(&mut self, id: WindowId) -> Result<(), WindowError> {
        let record = {
            let mut state = self.state.borrow_mut();
            let slot = slot_mut(&mut state, id)?;
            let record = slot.record.take().ok_or(WindowError::ExpiredWindow)?;
            record.route.invalidate();
            slot.generation =
                super::teardown::next_generation(slot.generation).unwrap_or_else(|| {
                    slot.retired = true;
                    0
                });
            if !slot.retired {
                state.free.push(id.slot);
            }
            record
        };
        super::teardown::destroy(record).map_err(WindowError::Native)
    }
    pub fn shutdown(&mut self) -> Result<(), WindowError> {
        let count = {
            let mut state = self.state.borrow_mut();
            if state.phase == AppPhase::Stopped {
                return Ok(());
            }
            state.phase = AppPhase::ShuttingDown;
            // Invalidate every route and generation before stopping any callback source.
            for slot in &mut state.slots {
                if let Some(record) = &slot.record {
                    record.route.invalidate();
                    slot.generation = super::teardown::next_generation(slot.generation)
                        .unwrap_or_else(|| {
                            slot.retired = true;
                            0
                        });
                }
            }
            state.free.clear();
            state.slots.len()
        };
        let mut fault = None;
        // No allocation and no app borrow across teardown calls, even in Drop.
        for index in 0..count {
            let record = self.state.borrow_mut().slots[index].record.take();
            if let Some(record) = record {
                if let Err(f) = super::teardown::destroy(record) {
                    fault.get_or_insert(f);
                }
            }
        }
        if let Err(f) = guard_native_call(|| self.application.stop(None)) {
            fault.get_or_insert(f);
        }
        {
            let mut state = self.state.borrow_mut();
            state.phase = AppPhase::Stopped;
            state.main_thread = None;
        }
        fault.map_or(Ok(()), |f| Err(WindowError::Native(f)))
    }
}
impl Drop for NativeApp {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
pub(crate) fn slot_mut(state: &mut AppState, id: WindowId) -> Result<&mut WindowSlot, WindowError> {
    if state.phase != AppPhase::Running {
        return Err(WindowError::ExpiredOwner);
    }
    let slot = state
        .slots
        .get_mut(id.slot as usize)
        .ok_or(WindowError::ExpiredWindow)?;
    if id.generation == 0 || slot.generation != id.generation || slot.record.is_none() {
        return Err(WindowError::ExpiredWindow);
    }
    Ok(slot)
}
pub(crate) fn record(state: &AppState, id: WindowId) -> Result<&WindowRecord, WindowError> {
    if state.phase != AppPhase::Running {
        return Err(WindowError::ExpiredOwner);
    }
    let slot = state
        .slots
        .get(id.slot as usize)
        .ok_or(WindowError::ExpiredWindow)?;
    if id.generation == 0 || slot.generation != id.generation {
        return Err(WindowError::ExpiredWindow);
    }
    slot.record.as_ref().ok_or(WindowError::ExpiredWindow)
}
fn create_record(
    mt: MainThread,
    config: &WindowConfig,
    route: Rc<RouteState>,
) -> Result<WindowRecord, WindowError> {
    let frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(config.width_points as f64, config.height_points as f64),
    );
    let mut style =
        NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
    if config.resizable {
        style |= NSWindowStyleMask::Resizable;
    }
    // SAFETY: real checked MainThread marker, baseline NSWindow initializer, bounded dimensions.
    let native = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mt.marker()),
            NSRect::new(NSPoint::new(80.0, 80.0), frame.size),
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    // SAFETY: this registry exclusively owns the +1 NSWindow; close must not release that owner.
    unsafe {
        native.setReleasedWhenClosed(false);
    }
    // A local cleanup owner closes a partial NSWindow on any construction failure.
    struct Partial<'a> {
        window: &'a NSWindow,
        route: &'a RouteState,
        armed: bool,
    }
    impl Drop for Partial<'_> {
        fn drop(&mut self) {
            if self.armed {
                self.route.invalidate();
                let _ = guard_native_call(|| {
                    self.window.setDelegate(None);
                    self.window.close();
                });
            }
        }
    }
    let mut cleanup = Partial {
        window: &native,
        route: &route,
        armed: true,
    };
    native.setCollectionBehavior(
        native.collectionBehavior() | NSWindowCollectionBehavior::FullScreenPrimary,
    );
    native.setTabbingIdentifier(&NSString::from_str("com.vexel.editor.document"));
    native.setTabbingMode(NSWindowTabbingMode::Automatic);
    let content = NSView::initWithFrame(NSView::alloc(mt.marker()), frame);
    content.setWantsLayer(true);
    if let Some(layer) = content.layer() {
        layer.setBackgroundColor(None);
    }
    let view = NativeView::new(mt, &route, frame);
    view.setWantsLayer(true);
    let material = NSVisualEffectView::initWithFrame(
        NSVisualEffectView::alloc(mt.marker()),
        NSRect::new(
            NSPoint::new(0.0, config.height_points as f64 - 40.0),
            NSSize::new(config.width_points as f64, 40.0),
        ),
    );
    material.setMaterial(NSVisualEffectMaterial::Titlebar);
    material.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    material.setState(NSVisualEffectState::FollowsWindowActiveState);
    material.setHidden(true);
    material.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMinYMargin,
    );
    view.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    content.addSubview(&material);
    content.addSubview(&view);
    native.setContentView(Some(&content));
    native.setContentMinSize(NSSize::new(1.0, 1.0));
    native.setContentMaxSize(NSSize::new(4096.0, 4096.0));
    native.setAcceptsMouseMovedEvents(true);
    native.setTitle(&NSString::from_str(&config.title));
    let delegate = NativeDelegate::new(mt, &route);
    native.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    let native_view = view.clone().into_super();
    route.bind(&native, &native_view);
    if super::geometry::scale(&native).is_none() {
        return Err(WindowError::InvalidConfig);
    }
    route.emit(WindowEvent::Open, true);
    super::callbacks::size(&route, false);
    super::callbacks::size(&route, true);
    route.emit(
        WindowEvent::AppearanceChanged {
            dark: super::geometry::dark(&native),
        },
        true,
    );
    if route.queue.borrow().disabled() {
        return Err(WindowError::QueueFault);
    }
    cleanup.armed = false;
    drop(cleanup);
    Ok(WindowRecord {
        native,
        _content: content,
        view,
        material,
        presentation_layer: None,
        _delegate: delegate,
        route,
        timer: None,
    })
}
