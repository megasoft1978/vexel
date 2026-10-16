//! File menu: titles, Command key equivalents and enablement come from the
//! application; activation only enqueues `WindowEvent::MenuCommand(tag)` on the
//! window's existing bounded queue. No panel is shown from inside a callback.
use super::{
    route::{RouteState, callback},
    types::*,
};
use crate::{MainThread, guard_native_call};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::NSString;
use std::rc::{Rc, Weak};

/// Maximum number of File menu items.
pub const MAX_MENU_ITEMS: usize = 16;

/// One File menu entry. The Command key is always part of the shortcut.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuItemSpec<'a> {
    pub title: &'a str,
    pub key: &'a str,
    pub shift: bool,
    pub tag: u32,
}

pub(crate) struct MenuIvars {
    route: Weak<RouteState>,
}

define_class!(
    // SAFETY: NSObject subclass with one main-thread action; ivars hold a weak Rust route.
    #[unsafe(super(NSObject))]
    #[thread_kind=MainThreadOnly]
    #[ivars=MenuIvars]
    pub(crate) struct MenuTarget;
    unsafe impl NSObjectProtocol for MenuTarget {}
    impl MenuTarget {
        #[unsafe(method(fileAction:))]
        fn file_action(&self, sender: &NSMenuItem) {
            let route = crate::guard_native_call(|| self.ivars().route.clone())
                .unwrap_or_else(|_| std::process::abort());
            callback(&route, (), |r| match u32::try_from(sender.tag()) {
                Ok(tag) => r.emit(WindowEvent::MenuCommand(tag), false),
                Err(_) => r.fault(QueueFaultReason::InvalidTextRange),
            });
        }
    }
);

impl MenuTarget {
    fn new(mt: MainThread, route: &Rc<RouteState>) -> Retained<Self> {
        let this = Self::alloc(mt.marker()).set_ivars(MenuIvars {
            route: Rc::downgrade(route),
        });
        // SAFETY: ivars initialized, NSObject designated initializer, +1 return.
        unsafe { msg_send![super(this), init] }
    }
}

pub(crate) struct FileMenu {
    _target: Retained<MenuTarget>,
    items: Vec<Retained<NSMenuItem>>,
}

impl FileMenu {
    pub(crate) fn install(
        mt: MainThread,
        application: &NSApplication,
        route: &Rc<RouteState>,
        specs: &[MenuItemSpec<'_>],
    ) -> Result<Self, WindowError> {
        if specs.is_empty() || specs.len() > MAX_MENU_ITEMS {
            return Err(WindowError::InvalidConfig);
        }
        if specs.iter().any(|s| {
            s.title.is_empty()
                || s.title.len() > 128
                || s.title.contains('\0')
                || s.key.len() != 1
                || s.key.contains('\0')
        }) {
            return Err(WindowError::InvalidConfig);
        }
        guard_native_call(|| {
            let mtm = mt.marker();
            let target = MenuTarget::new(mt, route);
            let file = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("File"));
            file.setAutoenablesItems(false);
            let mut items = Vec::with_capacity(specs.len());
            for spec in specs {
                // SAFETY: fileAction: is defined on MenuTarget with one NSMenuItem argument.
                let item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(spec.title),
                        Some(sel!(fileAction:)),
                        &NSString::from_str(spec.key),
                    )
                };
                let mut mask = NSEventModifierFlags::Command;
                if spec.shift {
                    mask |= NSEventModifierFlags::Shift;
                }
                item.setKeyEquivalentModifierMask(mask);
                item.setTag(spec.tag as isize);
                // SAFETY: the target is retained by FileMenu for the menu's lifetime.
                unsafe { item.setTarget(Some(&target)) };
                file.addItem(&item);
                items.push(item);
            }
            let main = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(""));
            // The first top-level item is the application menu on macOS.
            let app_item = NSMenuItem::new(mtm);
            let app_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("Vexel"));
            app_item.setSubmenu(Some(&app_menu));
            main.addItem(&app_item);
            let file_item = NSMenuItem::new(mtm);
            file_item.setSubmenu(Some(&file));
            main.addItem(&file_item);
            application.setMainMenu(Some(&main));
            Self {
                _target: target,
                items,
            }
        })
        .map_err(WindowError::Native)
    }

    /// Enables or disables items in table order; extra flags are ignored.
    pub(crate) fn set_enabled(&self, enabled: &[bool]) -> Result<(), WindowError> {
        guard_native_call(|| {
            for (item, on) in self.items.iter().zip(enabled) {
                item.setEnabled(*on);
            }
        })
        .map_err(WindowError::Native)
    }
}
