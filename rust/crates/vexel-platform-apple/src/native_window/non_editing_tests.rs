use super::{ime_native as ime, route::RouteState, types::*};
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

fn id() -> WindowId {
    WindowId {
        slot: 1,
        generation: 3,
    }
}

#[test]
fn declared_non_editing_focus_survives_key_and_ime_callbacks_without_fault() {
    let route = RouteState::new(id());
    route.focused.set(true);
    route.declare_no_text_target();
    route.emit(
        WindowEvent::KeyDown {
            code: 42,
            modifiers: Modifiers::default(),
            timestamp_seconds: 1.0,
        },
        false,
    );
    assert_eq!(ime::selected(&route, false), ime::missing());
    let inserted = NSString::from_str("d");
    let object: &AnyObject = &inserted;
    ime::insert(&route, object, ime::missing());
    let events = route.queue.borrow_mut().drain(8, false).events;
    assert!(matches!(events.first(), Some(WindowEvent::KeyDown { .. })));
    assert!(!events.iter().any(|event| matches!(
        event,
        WindowEvent::QueueFault(QueueFault {
            reason: QueueFaultReason::TextInputStateMissing,
            ..
        })
    )));
}
