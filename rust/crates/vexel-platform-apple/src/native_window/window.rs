use super::{
    app::{AppState, record, slot_mut},
    route::RouteState,
    types::*,
};
use crate::{MainThread, guard_native_call};
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSVisualEffectView, NSWindow};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo,
};
use objc2_foundation::{NSOperatingSystemVersion, NSProcessInfo, NSSize, NSString};
use objc2_quartz_core::{CALayer, kCAGravityResize};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A weak, generation checked handle. It never owns native objects or an app.
/// ```compile_fail
/// use vexel_platform_apple::Window;
/// fn require_send<T: Send>() {} require_send::<Window>();
/// ```
/// ```compile_fail
/// use vexel_platform_apple::Window;
/// fn require_sync<T: Sync>() {} require_sync::<Window>();
/// ```
pub struct Window {
    pub(crate) owner: Weak<RefCell<AppState>>,
    pub(crate) id: WindowId,
}
pub(crate) struct Handles {
    pub window: Retained<NSWindow>,
    pub view: Retained<NSView>,
    pub material: Retained<NSVisualEffectView>,
    pub route: Rc<RouteState>,
    pub mt: MainThread,
}
impl Window {
    pub fn id(&self) -> WindowId {
        self.id
    }
    fn handles(&self) -> Result<Handles, WindowError> {
        let owner = self.owner.upgrade().ok_or(WindowError::ExpiredOwner)?;
        let state = owner.borrow();
        let r = record(&state, self.id)?;
        let handles = Handles {
            window: r.native.clone(),
            view: r.view.clone().into_super(),
            material: r.material.clone(),
            route: r.route.clone(),
            mt: state.main_thread.ok_or(WindowError::ExpiredOwner)?,
        };
        // Native retain bookkeeping is not an objc message. No app borrow remains
        // when the caller performs ANY AppKit/NSString/NSProcessInfo operation.
        drop(state);
        if let Some(f) = handles.route.native_fault.get() {
            return Err(WindowError::Native(f));
        }
        Ok(handles)
    }
    pub fn set_title(&mut self, title: String) -> Result<(), WindowError> {
        let h = self.handles()?;
        if title.is_empty() || title.len() > 1024 || title.contains('\0') {
            return Err(WindowError::InvalidConfig);
        }
        guard_native_call(|| h.window.setTitle(&NSString::from_str(&title)))
            .map_err(WindowError::Native)
    }
    /// Synchronously prepares and publishes one owned RGBA8 frame.
    ///
    /// Input is top-left, tightly packed, straight-alpha sRGB RGBA8. The
    /// presenter converts it to premultiplied RGBA8 for CoreGraphics in place
    /// and hands the buffer to the image's data provider without a copy; the
    /// provider frees it when the layer drops the image. The first present
    /// configures a complete candidate layer off-hierarchy before one guarded
    /// add. Later presents reuse that single layer and only swap its contents
    /// (implicit animations disabled), so no layer is allocated per frame. A
    /// failed preparation leaves the current layer untouched; native exception
    /// atomicity is not established by this API's unit tests.
    pub fn present_rgba(&mut self, frame: super::frame::RgbaFrame) -> Result<(), WindowError> {
        let (width, height, scale) = (frame.width(), frame.height(), frame.backing_scale());
        let pixels = frame.into_premultiplied_pixels();
        let image = guard_native_call(|| prepare_rgba_image(width, height, pixels))
            .map_err(WindowError::Native)?
            .ok_or(WindowError::PresentationUnavailable)?;
        let h = self.handles()?;
        let parent = guard_native_call(|| h.view.layer())
            .map_err(WindowError::Native)?
            .ok_or(WindowError::PresentationUnavailable)?;
        let owner = self.owner.upgrade().ok_or(WindowError::ExpiredOwner)?;
        let previous = {
            let state = owner.borrow();
            record(&state, self.id)?.presentation_layer.clone()
        };
        let bounds = guard_native_call(|| parent.bounds()).map_err(WindowError::Native)?;
        // SAFETY: CGImage is the exact Core Graphics object type required by
        // CALayer.contents, and the retained image outlives every use below.
        let object: &objc2::runtime::AnyObject =
            unsafe { &*(image.as_ref() as *const CGImage as *const objc2::runtime::AnyObject) };
        if let Some(layer) = previous.as_deref() {
            guard_native_call(|| {
                without_implicit_animations(|| {
                    if layer.frame() != bounds {
                        layer.setFrame(bounds);
                    }
                    if layer.contentsScale() != scale {
                        layer.setContentsScale(scale);
                    }
                    // SAFETY: `object` points to the valid retained CGImage.
                    unsafe { layer.setContents(Some(object)) };
                })
            })
            .map_err(WindowError::Native)?;
            return Ok(());
        }
        let candidate = guard_native_call(|| {
            let layer = CALayer::new();
            layer.setFrame(bounds);
            layer.setContentsScale(scale);
            // SAFETY: Apple's exported gravity constant has process lifetime
            // and is the documented CALayer resize-gravity value.
            layer.setContentsGravity(unsafe { kCAGravityResize });
            // SAFETY: `object` points to the valid retained CGImage above.
            unsafe {
                layer.setContents(Some(object));
            }
            layer
        })
        .map_err(WindowError::Native)?;
        let published = super::frame::publish_candidate(candidate, |candidate| {
            guard_native_call(|| parent.addSublayer(candidate))
        })
        .map_err(WindowError::Native)?;
        // Native calls are complete. Commit the retained presenter only after
        // the single hierarchy operation returned successfully.
        let mut state = owner.borrow_mut();
        let record = slot_mut(&mut state, self.id)?
            .record
            .as_mut()
            .ok_or(WindowError::ExpiredOwner)?;
        record.presentation_layer = Some(published);
        Ok(())
    }
    pub fn set_visible(&mut self, visible: bool) -> Result<(), WindowError> {
        let h = self.handles()?;
        guard_native_call(|| {
            if visible {
                h.window.makeKeyAndOrderFront(None);
                if !h.window.makeFirstResponder(Some(&h.view)) {
                    return Err(WindowError::InvalidConfig);
                }
            } else {
                h.window.orderOut(None)
            }
            Ok(())
        })
        .map_err(WindowError::Native)?
    }
    pub fn resize(&mut self, size: Size) -> Result<(), WindowError> {
        let h = self.handles()?;
        if !size.width.is_finite()
            || !size.height.is_finite()
            || !(1.0..=4096.0).contains(&size.width)
            || !(1.0..=4096.0).contains(&size.height)
        {
            return Err(WindowError::InvalidConfig);
        }
        guard_native_call(|| {
            h.window
                .setContentSize(NSSize::new(size.width, size.height))
        })
        .map_err(WindowError::Native)
    }
    /// Returns the content view point size, checked integral backing scale, and pixel size.
    pub fn surface_metrics(&self) -> Result<(Size, SizeU32, f64), WindowError> {
        let h = self.handles()?;
        guard_native_call(|| {
            let bounds = h.view.bounds();
            let points = Size {
                width: bounds.size.width,
                height: bounds.size.height,
            };
            let scale = super::geometry::scale(&h.window).ok_or(WindowError::InvalidConfig)?;
            let pixels =
                super::geometry::pixels(points, scale).ok_or(WindowError::InvalidConfig)?;
            if pixels.width > super::frame::MAX_FRAME_AXIS
                || pixels.height > super::frame::MAX_FRAME_AXIS
                || (pixels.width as usize)
                    .checked_mul(pixels.height as usize)
                    .and_then(|n| n.checked_mul(4))
                    .is_none_or(|n| n > 64 * 1024 * 1024)
            {
                return Err(WindowError::InvalidConfig);
            }
            Ok((points, pixels, scale))
        })
        .map_err(WindowError::Native)?
    }
    pub fn content_screen_origin(&self) -> Result<Point, WindowError> {
        let h = self.handles()?;
        guard_native_call(|| super::geometry::screen_origin(&h.window, &h.view))
            .map_err(WindowError::Native)?
    }
    pub fn set_cursor(&mut self, cursor: Cursor) -> Result<(), WindowError> {
        let h = self.handles()?;
        guard_native_call(|| super::cursor::select(h.mt, cursor).set()).map_err(WindowError::Native)
    }
    pub fn is_dark_appearance(&self) -> Result<bool, WindowError> {
        let h = self.handles()?;
        guard_native_call(|| super::geometry::dark(&h.window)).map_err(WindowError::Native)
    }
    pub fn set_material_enabled(&mut self, enabled: bool) -> Result<bool, WindowError> {
        let h = self.handles()?;
        guard_native_call(|| {
            let allowed = enabled
                && NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(
                    NSOperatingSystemVersion {
                        majorVersion: 26,
                        minorVersion: 0,
                        patchVersion: 0,
                    },
                );
            h.material.setHidden(!allowed);
            allowed
        })
        .map_err(WindowError::Native)
    }
    pub fn set_text_input_state(&mut self, state: TextInputState) -> Result<(), WindowError> {
        let h = self.handles()?;
        if !validate_text_state(self.id, &state) {
            return Err(WindowError::InvalidTextInputState);
        }
        if !h.route.publish(state, None) {
            h.route.fault(QueueFaultReason::InputOverflow);
            return Err(WindowError::QueueFault);
        }
        // Invalidating candidate coordinates can synchronously query the client.
        guard_native_call(|| {
            if let Some(context) = h.view.inputContext() {
                context.invalidateCharacterCoordinates();
            }
        })
        .map_err(WindowError::Native)
    }
    pub fn clear_text_input_state(&mut self) -> Result<(), WindowError> {
        let h = self.handles()?;
        h.route.declare_no_text_target();
        guard_native_call(|| {
            if let Some(context) = h.view.inputContext() {
                context.discardMarkedText();
                context.invalidateCharacterCoordinates();
            }
        })
        .map_err(WindowError::Native)
    }
    pub fn acknowledge_input_reset(&mut self, sequence: u64) -> Result<(), WindowError> {
        let h = self.handles()?;
        if h.route.queue.borrow_mut().acknowledge_reset(sequence) {
            Ok(())
        } else {
            Err(WindowError::QueueFault)
        }
    }
    pub fn request_ticks(&mut self, enabled: bool) -> Result<(), WindowError> {
        let h = self.handles()?;
        let owner = self.owner.upgrade().ok_or(WindowError::ExpiredOwner)?;
        let previous = {
            let mut state = owner.borrow_mut();
            let r = slot_mut(&mut state, self.id)?.record.as_mut().unwrap();
            if enabled
                && r.timer
                    .as_ref()
                    .is_some_and(|t| t.1 == h.route.tick_generation.get())
            {
                return Ok(());
            }
            r.timer.take()
        };
        let generation = h.route.tick_generation.get().checked_add(1).unwrap_or(0);
        h.route.tick_generation.set(generation);
        if let Some(timer) = previous {
            if let Err(fault) = super::ticks::cancel(&timer.0) {
                let mut state = owner.borrow_mut();
                if let Ok(slot) = slot_mut(&mut state, self.id) {
                    slot.record.as_mut().unwrap().timer = Some(timer);
                }
                return Err(WindowError::Native(fault));
            }
        }
        if !enabled {
            return Ok(());
        }
        if generation == 0 {
            return Err(WindowError::QueueFault);
        }
        let timer = match guard_native_call(|| {
            let target = super::callbacks::TickTarget::new(h.mt, &h.route, generation);
            super::ticks::install(&target)
        })
        .and_then(|r| r)
        {
            Ok(timer) => timer,
            Err(fault) => {
                h.route
                    .tick_generation
                    .set(generation.checked_add(1).unwrap_or(0));
                return Err(WindowError::Native(fault));
            }
        };
        // A timer installation may reenter; validate owner and generation again.
        let mut state = owner.borrow_mut();
        match slot_mut(&mut state, self.id) {
            Ok(slot) => slot.record.as_mut().unwrap().timer = Some((timer, generation)),
            Err(error) => {
                drop(state);
                let _ = super::ticks::cancel(&timer);
                return Err(error);
            }
        }
        drop(state);
        super::ticks::wake();
        Ok(())
    }
    /// Internal presenter attachment seam. The crate-local presenter may attach
    /// a real layer to this retained NSView after handle validation; no pointer,
    /// fake presenter or rendering promise is exposed by the public API.
    #[allow(dead_code)]
    pub(crate) fn surface_view(&self) -> Result<Retained<NSView>, WindowError> {
        Ok(self.handles()?.view)
    }
}

/// Runs `f` inside a CATransaction with implicit actions disabled, so a
/// contents swap on a live layer shows immediately instead of cross-fading.
fn without_implicit_animations(f: impl FnOnce()) {
    let class = objc2::runtime::AnyClass::get(c"CATransaction");
    // SAFETY: `+begin`, `+setDisableActions:` and `+commit` are the documented
    // CATransaction class methods; the class exists wherever QuartzCore is
    // linked, and a missing class degrades to running `f` unwrapped.
    unsafe {
        if let Some(class) = class {
            let _: () = objc2::msg_send![class, begin];
            let _: () = objc2::msg_send![class, setDisableActions: true];
        }
        f();
        if let Some(class) = class {
            let _: () = objc2::msg_send![class, commit];
        }
    }
}

unsafe extern "C-unwind" fn release_pixels(
    info: *mut std::ffi::c_void,
    _data: std::ptr::NonNull<std::ffi::c_void>,
    _size: usize,
) {
    // SAFETY: `info` is the `Box<Vec<u8>>` leaked in `prepare_rgba_image`;
    // CoreGraphics calls this exactly once when the provider is released.
    drop(unsafe { Box::from_raw(info.cast::<Vec<u8>>()) });
}

/// Constructs a CGImage whose data provider takes ownership of the
/// premultiplied buffer (no copy) and frees it when the image is released.
pub(super) fn prepare_rgba_image(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
) -> Option<objc2_core_foundation::CFRetained<CGImage>> {
    // SAFETY: this extern static is Apple's process-lifetime sRGB color-space
    // name constant; its pointer remains valid for the synchronous call.
    let srgb_name = unsafe { objc2_core_graphics::kCGColorSpaceSRGB };
    let bytes_per_row = usize::try_from(width).ok()?.checked_mul(4)?;
    if pixels.len() != bytes_per_row.checked_mul(height as usize)? {
        return None;
    }
    let (data, len) = (pixels.as_ptr(), pixels.len());
    let info = Box::into_raw(Box::new(pixels));
    // SAFETY: `data` points into the boxed Vec, whose heap buffer never moves
    // and stays alive until `release_pixels` runs; `info` is that box.
    let provider =
        unsafe { CGDataProvider::with_data(info.cast(), data.cast(), len, Some(release_pixels)) };
    let Some(provider) = provider else {
        // SAFETY: creation failed, so CoreGraphics never took ownership.
        drop(unsafe { Box::from_raw(info) });
        return None;
    };
    let color_space = CGColorSpace::with_name(Some(srgb_name))?;
    // SAFETY: dimensions and row size were validated by RgbaFrame; provider
    // owns exactly height * bytes_per_row bytes. The decode table is null as
    // required for no custom decode mapping.
    unsafe {
        CGImage::new(
            width as usize,
            height as usize,
            8,
            32,
            bytes_per_row,
            Some(&color_space),
            CGBitmapInfo(CGImageAlphaInfo::PremultipliedLast.0),
            Some(&provider),
            std::ptr::null(),
            true,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
}
