//! Main-thread callback state. Native delegates retain only Weak<RouteState>.
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use super::{queue::EventQueue, types::*};
use crate::{NativeFault, guard_native_call};
use objc2::rc::{Retained, Weak as NativeWeak};
use objc2_app_kit::{NSView, NSWindow};

pub(crate) struct RouteState {
    pub id: WindowId,
    pub active: Cell<bool>,
    pub focused: Cell<bool>,
    pub queue: RefCell<EventQueue>,
    pub text: RefCell<Option<TextInputState>>,
    pub no_target: Cell<bool>,
    pub composition: RefCell<Option<Composition>>,
    pub native_fault: Cell<Option<NativeFault>>,
    native: RefCell<Option<(NativeWeak<NSWindow>, NativeWeak<NSView>)>>,
    pub tick_generation: Cell<u64>,
    last_timestamp: Cell<f64>,
}
impl RouteState {
    pub fn new(id: WindowId) -> Rc<Self> {
        Rc::new(Self {
            id,
            active: Cell::new(true),
            focused: Cell::new(false),
            queue: RefCell::new(EventQueue::default()),
            text: RefCell::new(None),
            no_target: Cell::new(false),
            composition: RefCell::new(None),
            native_fault: Cell::new(None),
            native: RefCell::new(None),
            tick_generation: Cell::new(0),
            last_timestamp: Cell::new(0.0),
        })
    }
    pub fn bind(&self, window: &Retained<NSWindow>, view: &Retained<NSView>) {
        let weak = (
            NativeWeak::from_retained(window),
            NativeWeak::from_retained(view),
        );
        *self.native.borrow_mut() = Some(weak);
    }
    pub fn native(&self) -> Option<(Retained<NSWindow>, Retained<NSView>)> {
        if !self.active.get() {
            return None;
        }
        // objc2's weak load/clone performs runtime retain bookkeeping, not a
        // message send. Retain/release exceptions are outside our unwind fence.
        let pair = self
            .native
            .borrow()
            .as_ref()
            .map(|(w, v)| (w.clone(), v.clone()))?;
        Some((pair.0.load()?, pair.1.load()?))
    }
    pub fn timestamp(&self, value: f64) -> Option<f64> {
        if !value.is_finite() || value < 0.0 || value > 1.0e12 {
            self.fail(QueueFaultReason::InputOverflow);
            return None;
        }
        let value = value.max(self.last_timestamp.get());
        self.last_timestamp.set(value);
        Some(value)
    }

    pub fn emit(&self, event: WindowEvent, control: bool) {
        if !self.active.get() {
            return;
        }
        // Enqueue failure is represented by fault/reset latches, or by a reset
        // already disabling ingress. The callback never discards an unreported loss.
        let accepted = self.queue.borrow_mut().push(event, control);
        if !accepted && self.queue.borrow().disabled() {
            self.text.borrow_mut().take();
            self.composition.borrow_mut().take();
            self.no_target.set(false);
            self.queue.borrow_mut().set_external_payload(0);
        }
        super::ticks::wake();
    }
    pub fn fault(&self, reason: QueueFaultReason) {
        if self.active.get() {
            self.queue.borrow_mut().fault(reason);
            super::ticks::wake();
        }
    }
    pub fn fail(&self, reason: QueueFaultReason) {
        if self.active.get() {
            self.queue.borrow_mut().fail(reason);
            self.text.borrow_mut().take();
            self.composition.borrow_mut().take();
            self.no_target.set(false);
            self.queue.borrow_mut().set_external_payload(0);
            super::ticks::wake();
        }
    }
    pub fn publish(&self, state: TextInputState, composition: Option<Composition>) -> bool {
        let bytes = state.text.len()
            + state.marked_attributes.len() * std::mem::size_of::<TextAttributeSpan>()
            + composition.as_ref().map_or(0, |c| {
                c.text.len() + c.attributes.len() * std::mem::size_of::<TextAttributeSpan>()
            });
        if !self.queue.borrow_mut().set_external_payload(bytes) {
            return false;
        }
        *self.text.borrow_mut() = Some(state);
        *self.composition.borrow_mut() = composition;
        self.no_target.set(false);
        true
    }
    pub fn declare_no_text_target(&self) {
        self.text.borrow_mut().take();
        self.composition.borrow_mut().take();
        self.no_target.set(true);
        self.queue.borrow_mut().set_external_payload(0);
    }
    pub fn snapshot(&self) -> Option<TextInputState> {
        if !self.active.get() || !self.focused.get() {
            return None;
        }
        let snapshot = self.text.borrow().clone();
        if snapshot.as_ref().is_none_or(|s| s.source_window != self.id) {
            if !self.no_target.get() {
                self.fault(QueueFaultReason::TextInputStateMissing);
            }
            None
        } else {
            snapshot
        }
    }
    pub fn invalidate(&self) {
        self.active.set(false);
        self.tick_generation
            .set(self.tick_generation.get().saturating_add(1));
        self.focused.set(false);
        self.text.borrow_mut().take();
        self.composition.borrow_mut().take();
        self.queue.borrow_mut().clear();
        self.native.borrow_mut().take();
    }
    pub fn callback<T>(&self, fallback: T, f: impl FnOnce() -> T) -> T {
        if !self.active.get() {
            return fallback;
        }
        match guard_native_call(f) {
            Ok(value) => value,
            Err(fault) => {
                self.native_fault.set(Some(fault));
                self.fail(QueueFaultReason::NativeCallbackFailure);
                fallback
            }
        }
    }
}
pub(crate) fn callback<T>(
    weak: &Weak<RouteState>,
    fallback: T,
    f: impl FnOnce(&Rc<RouteState>) -> T,
) -> T {
    // The outer fence also covers route upgrades and callback setup. No Rust
    // unwind may leave an Objective-C IMP, including failure reporting itself.
    match guard_native_call(|| {
        if let Some(route) = weak.upgrade() {
            route.callback(fallback, || f(&route))
        } else {
            fallback
        }
    }) {
        Ok(value) => value,
        Err(_) => std::process::abort(),
    }
}
