//! Pure ownership/queue proofs plus real Foundation input and callback fencing.
//! No NSApplication/NSWindow/NSView instance is created by these tests.
use super::{
    app::{AppPhase, AppState, WindowSlot},
    ime_native as ime,
    queue::EventQueue,
    route::{RouteState, callback},
    types::*,
    window::Window,
};
use objc2::{AllocAnyThread, ClassType, runtime::AnyObject, sel};
use objc2_foundation::{NSMutableAttributedString, NSNumber, NSRange, NSString};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

fn id() -> WindowId {
    WindowId {
        slot: 1,
        generation: 3,
    }
}
fn state(text: &str) -> TextInputState {
    TextInputState {
        source_window: id(),
        text: text.into(),
        selection_utf8: 0..0,
        marked_utf8: None,
        marked_attributes: vec![],
        candidate_rect_points: Rect {
            origin: Point { x: 12.0, y: 24.0 },
            size: Size {
                width: 1.0,
                height: 16.0,
            },
        },
    }
}
fn editable(text: &str) -> Rc<RouteState> {
    let route = RouteState::new(id());
    route.focused.set(true);
    assert!(route.publish(state(text), None));
    route
}
fn key(up: bool) -> WindowEvent {
    if up {
        WindowEvent::KeyUp {
            code: 42,
            modifiers: Modifiers::default(),
            timestamp_seconds: 1.0,
        }
    } else {
        WindowEvent::KeyDown {
            code: 42,
            modifiers: Modifiers::default(),
            timestamp_seconds: 1.0,
        }
    }
}
fn motion(x: f64, kind: PointerKind) -> WindowEvent {
    WindowEvent::Pointer(PointerEvent {
        kind,
        scale: 2.0,
        logical: (x as i32, 0),
        at: Point { x, y: 0.0 },
        button: (kind != PointerKind::Move).then_some(0),
        click_count: 1,
        modifiers: Modifiers::default(),
        timestamp_seconds: x.abs(),
    })
}

#[test]
fn all_protocol_selectors_are_registered_with_real_appkit_superclasses() {
    let class = super::callbacks::NativeView::class();
    assert_eq!(class.superclass().unwrap().name().to_bytes(), b"NSView");
    for selector in [
        sel!(keyDown:),
        sel!(keyUp:),
        sel!(mouseDown:),
        sel!(mouseUp:),
        sel!(scrollWheel:),
        sel!(magnifyWithEvent:),
        sel!(insertText:replacementRange:),
        sel!(setMarkedText:selectedRange:replacementRange:),
        sel!(unmarkText),
        sel!(hasMarkedText),
        sel!(selectedRange),
        sel!(markedRange),
        sel!(validAttributesForMarkedText),
        sel!(attributedSubstringForProposedRange:actualRange:),
        sel!(firstRectForCharacterRange:actualRange:),
        sel!(characterIndexForPoint:),
        sel!(doCommandBySelector:),
    ] {
        assert!(
            class.instance_method(selector).is_some(),
            "missing {selector:?}"
        );
    }
    let delegate = super::callbacks::NativeDelegate::class();
    for selector in [
        sel!(windowShouldClose:),
        sel!(windowDidBecomeKey:),
        sel!(windowDidResignKey:),
        sel!(windowDidResize:),
        sel!(windowDidChangeBackingProperties:),
    ] {
        assert!(delegate.instance_method(selector).is_some());
    }
    assert!(
        super::callbacks::TickTarget::class()
            .instance_method(sel!(tick:))
            .is_some()
    );
}

#[test]
fn callback_panic_and_foundation_exception_return_fallback_and_sequence_reset() {
    for foreign in [false, true] {
        let route = editable("abc");
        let weak = Rc::downgrade(&route);
        let answer = callback(&weak, ime::NOT_FOUND, |_| {
            if foreign {
                let string = NSString::from_str("abc");
                let _ = string.substringFromIndex(usize::MAX);
            } else {
                panic!("callback proof");
            }
            0usize
        });
        assert_eq!(answer, ime::NOT_FOUND);
        assert!(route.native_fault.get().is_some());
        let events = route.queue.borrow_mut().drain(4, false).events;
        assert!(matches!(
            events[0],
            WindowEvent::QueueFault(QueueFault {
                reason: QueueFaultReason::NativeCallbackFailure,
                ..
            })
        ));
        assert!(matches!(
            events[1],
            WindowEvent::InputReset {
                all_keys_up: true,
                all_buttons_up: true,
                ..
            }
        ));
        assert!(route.text.borrow().is_none());
    }
}

#[test]
fn reentrant_callback_uses_owned_snapshot_without_refcell_borrow() {
    let route = editable("a🦀z");
    let weak = Rc::downgrade(&route);
    callback(&weak, (), |r| {
        let owned = r.snapshot().unwrap();
        callback(&weak, (), |nested| {
            nested.emit(key(false), false);
            nested.text.borrow_mut().as_mut().unwrap().selection_utf8 = 1..5;
        });
        assert_eq!(owned.selection_utf8, 0..0);
    });
    assert_eq!(route.snapshot().unwrap().selection_utf8, 1..5);
    assert_eq!(route.queue.borrow_mut().drain(1, false).events.len(), 1);
}

#[test]
fn marked_text_projects_utf16_selection_and_commits_separately() {
    let route = editable("ab🦀cd");
    let weak = Rc::downgrade(&route);
    let text = NSString::from_str("😀é");
    let object: &AnyObject = &text;
    callback(&weak, (), |r| {
        ime::marked(r, object, NSRange::new(2, 1), NSRange::new(2, 2))
    });
    let projected = route.snapshot().unwrap();
    assert_eq!(projected.text, "ab😀écd");
    assert_eq!(projected.selection_utf8, 6..8);
    assert_eq!(projected.marked_utf8, Some(2..8));
    assert_eq!(ime::selected(&route, false), NSRange::new(4, 1));
    assert_eq!(ime::selected(&route, true), NSRange::new(2, 3));
    let events = route.queue.borrow_mut().drain(4, false).events;
    assert!(
        matches!(&events[0],WindowEvent::Composition(c) if c.selection_utf8== (4..6) && c.replacement_utf8==Some(2..6))
    );
    let commit = NSString::from_str("漢");
    let object: &AnyObject = &commit;
    callback(&weak, (), |r| ime::insert(r, object, ime::missing()));
    assert_eq!(route.snapshot().unwrap().text, "ab漢cd");
    assert_eq!(ime::selected(&route, true), ime::missing());
    assert!(
        matches!(&route.queue.borrow_mut().drain(4,false).events[0],WindowEvent::TextCommitted(s) if s=="漢")
    );
}

#[test]
fn invalid_utf16_ranges_never_clamp_or_replace_the_valid_snapshot() {
    let route = editable("a🦀z");
    let weak = Rc::downgrade(&route);
    let text = NSString::from_str("😀");
    let object: &AnyObject = &text;
    callback(&weak, (), |r| {
        ime::marked(r, object, NSRange::new(1, 0), NSRange::new(0, 0))
    });
    assert_eq!(route.snapshot().unwrap().text, "a🦀z");
    assert!(ime::substring(&route, NSRange::new(2, 1)).is_none());
    let events = route.queue.borrow_mut().drain(8, false).events;
    assert!(events.iter().any(|e| matches!(
        e,
        WindowEvent::QueueFault(QueueFault {
            reason: QueueFaultReason::InvalidTextRange,
            ..
        })
    )));
    assert!(ime::requested("x", NSRange::new(usize::MAX, 2)).is_none());
}

#[test]
fn marked_attributes_are_owned_advertised_and_returned_in_substrings() {
    let route = editable("ab");
    let weak = Rc::downgrade(&route);
    let string = NSString::from_str("😀é");
    let attributed =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &string);
    let keys = ime::attributes();
    let key = keys.objectAtIndex(0);
    let number = NSNumber::numberWithLongLong(2);
    // SAFETY: scalar-aligned full UTF16 range, legal advertised NSNumber style value.
    unsafe {
        attributed.addAttribute_value_range(&key, &number, NSRange::new(0, 3));
    }
    let object: &AnyObject = &attributed;
    callback(&weak, (), |r| {
        ime::marked(r, object, NSRange::new(3, 0), NSRange::new(1, 0))
    });
    let snapshot = route.snapshot().unwrap();
    assert_eq!(snapshot.marked_attributes.len(), 1);
    assert_eq!(snapshot.marked_attributes[0].value, 2);
    let (substring, actual) = ime::substring(&route, NSRange::new(1, 3)).unwrap();
    assert_eq!(substring.string().to_string(), "😀é");
    assert_eq!(actual, NSRange::new(1, 3));
    let mut effective = ime::missing();
    let attributes = unsafe { substring.attributesAtIndex_effectiveRange(0, &mut effective) };
    assert_eq!(
        attributes
            .objectForKey(&key)
            .unwrap()
            .downcast::<NSNumber>()
            .unwrap()
            .longLongValue(),
        2
    );
    callback(&weak, (), ime_unmark);
    assert_eq!(ime::selected(&route, true), ime::missing());
    assert!(
        route
            .queue
            .borrow_mut()
            .drain(8, false)
            .events
            .iter()
            .any(|e| matches!(e, WindowEvent::CompositionEnded))
    );
}
fn ime_unmark(route: &Rc<RouteState>) {
    ime::unmark(route)
}

#[test]
fn missing_focus_snapshot_faults_but_explicit_no_target_does_not() {
    let route = RouteState::new(id());
    assert!(route.snapshot().is_none());
    assert!(!route.queue.borrow().has_pending());
    route.focused.set(true);
    assert!(route.snapshot().is_none());
    assert!(matches!(
        route.queue.borrow_mut().drain(1, false).events[0],
        WindowEvent::QueueFault(QueueFault {
            reason: QueueFaultReason::TextInputStateMissing,
            ..
        })
    ));
    route.no_target.set(true);
    assert_eq!(ime::selected(&route, false), ime::missing());
    assert!(!route.queue.borrow().has_pending());
    assert_eq!(
        ime::character_index(&route, objc2_foundation::NSPoint::new(f64::NAN, 0.0)),
        ime::NOT_FOUND
    );
}

#[test]
fn full_input_reserve_preserves_releases_or_orders_reset_after_them() {
    let mut q = EventQueue::default();
    for _ in 0..240 {
        assert!(q.push(key(false), false));
    }
    for _ in 0..16 {
        assert!(q.push(key(true), false));
    }
    assert!(!q.push(key(true), false));
    q.request_close();
    q.request_close();
    let accepted = q.drain(256, false);
    assert!(accepted.pending);
    assert_eq!(
        accepted
            .events
            .iter()
            .filter(|e| matches!(e, WindowEvent::KeyUp { .. }))
            .count(),
        16
    );
    let sticky = q.drain(3, false);
    assert!(matches!(sticky.events[0], WindowEvent::QueueFault(_)));
    let sequence = match sticky.events[1] {
        WindowEvent::InputReset { sequence, .. } => sequence,
        _ => panic!("reset missing"),
    };
    assert!(matches!(sticky.events[2], WindowEvent::CloseRequested));
    assert!(!sticky.pending);
    assert!(!q.acknowledge_reset(sequence.wrapping_sub(1)));
    assert!(!q.push(key(false), false));
    assert!(q.push(WindowEvent::Focus(false), true));
    assert!(q.acknowledge_reset(sequence));
    assert!(!q.acknowledge_reset(sequence));
    assert!(q.push(key(false), false));
}

#[test]
fn alternating_controls_cannot_exceed_sixteen_reserved_records() {
    let mut q = EventQueue::default();
    // Fill ordinary occupancy with non-coalescing key transitions.
    for _ in 0..240 {
        assert!(q.push(key(false), false));
    }
    // Alternate control kinds so every item is a distinct tail and cannot coalesce.
    for index in 0..16 {
        let event = if index % 2 == 0 {
            WindowEvent::Focus(index % 4 == 0)
        } else {
            WindowEvent::AppearanceChanged {
                dark: index % 4 == 1,
            }
        };
        assert!(q.push(event, true), "reserved control {index} rejected");
    }
    assert!(!q.push(WindowEvent::Focus(false), true));
    assert!(q.disabled());

    let batch = q.drain(256, false);
    assert!(batch.pending);
    assert_eq!(batch.events.len(), 240 + 16);
    assert!(
        batch.events[..240]
            .iter()
            .all(|event| matches!(event, WindowEvent::KeyDown { .. }))
    );
    assert!(batch.events[240..].iter().all(|event| matches!(
        event,
        WindowEvent::Focus(_) | WindowEvent::AppearanceChanged { .. }
    )));

    let fault = q.drain(1, false);
    assert!(matches!(fault.events[0], WindowEvent::QueueFault(_)));
    assert!(fault.pending);
    let reset = q.drain(1, false);
    let sequence = match reset.events[0] {
        WindowEvent::InputReset { sequence, .. } => sequence,
        _ => panic!("overflow reset missing"),
    };
    assert!(!reset.pending);
    assert!(!q.acknowledge_reset(sequence.wrapping_add(1)));
    assert!(q.disabled());
    assert!(q.acknowledge_reset(sequence));
    assert!(!q.disabled());
}

#[test]
fn barriers_and_button_transitions_are_never_coalesced() {
    let mut q = EventQueue::default();
    for e in [
        motion(1.0, PointerKind::Move),
        key(false),
        motion(2.0, PointerKind::Move),
        motion(3.0, PointerKind::Down),
        motion(4.0, PointerKind::Up),
        WindowEvent::TextCommitted("x".into()),
        WindowEvent::AnimationTick {
            monotonic_seconds: 1.0,
        },
    ] {
        assert!(q.push(e, false));
    }
    assert_eq!(q.drain(20, false).events.len(), 7);
    assert!(q.drain(1, true).pending); // deferred native work is independently pending
}

#[test]
fn payload_accounting_includes_native_snapshot_and_pending_composition() {
    let route = editable(&"x".repeat(64 * 1024));
    for _ in 0..15 {
        route.emit(WindowEvent::TextCommitted("x".repeat(64 * 1024)), false);
    }
    assert!(!route.queue.borrow().disabled());
    route.emit(WindowEvent::TextCommitted("x".repeat(64 * 1024)), false);
    assert!(route.queue.borrow().disabled());
    assert!(route.text.borrow().is_none());
    let events = route.queue.borrow_mut().drain(256, false).events;
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, WindowEvent::TextCommitted(_)))
            .count(),
        15
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, WindowEvent::InputReset { .. }))
    );
}

#[test]
fn lossy_motion_reports_drop_without_losing_non_droppable_input() {
    let mut q = EventQueue::default();
    for _ in 0..240 {
        assert!(q.push(key(false), false));
    }
    assert!(!q.push(motion(1.0, PointerKind::Move), false));
    assert!(!q.disabled());
    assert!(q.push(key(true), false));
    let events = q.drain(256, false).events;
    assert!(events.iter().any(|e| matches!(
        e,
        WindowEvent::QueueFault(QueueFault {
            coalesced_or_dropped: 1,
            ..
        })
    )));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, WindowEvent::KeyUp { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, WindowEvent::InputReset { .. }))
    );
}

#[test]
fn timer_generation_and_weak_teardown_ignore_late_callbacks() {
    let route = editable("x");
    let weak = Rc::downgrade(&route);
    route.tick_generation.set(1);
    super::callbacks::emit_tick(&route, 1, 10.0);
    route.tick_generation.set(2);
    super::callbacks::emit_tick(&route, 1, 11.0);
    super::callbacks::emit_tick(&route, 2, 12.0);
    assert_eq!(route.queue.borrow_mut().drain(8, false).events.len(), 2);
    route.invalidate();
    route.invalidate();
    callback(&weak, (), |r| r.emit(key(true), false));
    super::callbacks::emit_tick(&route, 2, 13.0);
    assert!(!route.queue.borrow().has_pending());
    drop(route);
    assert!(weak.upgrade().is_none());
    assert_eq!(callback(&weak, 42, |_| 0), 42);
    assert_eq!(super::teardown::next_generation(u32::MAX), None);
    assert_eq!(super::teardown::next_generation(1), Some(2));
}

#[test]
fn timestamps_are_monotonic_and_nonfinite_input_resets_ingress() {
    let route = editable("x");
    assert_eq!(route.timestamp(100.0), Some(100.0));
    assert_eq!(route.timestamp(99.0), Some(100.0));
    assert_eq!(route.timestamp(f64::NAN), None);
    assert!(route.queue.borrow().disabled());
}

fn check_expired(window: &mut Window, error: WindowError) {
    assert_eq!(window.set_title("".into()), Err(error));
    assert_eq!(window.set_visible(true), Err(error));
    assert_eq!(window.resize(Size::default()), Err(error));
    assert_eq!(window.content_screen_origin(), Err(error));
    assert_eq!(window.set_cursor(Cursor::Rotate), Err(error));
    assert_eq!(window.is_dark_appearance(), Err(error));
    assert_eq!(window.set_material_enabled(true), Err(error));
    assert_eq!(window.set_text_input_state(state("x")), Err(error));
    assert_eq!(window.clear_text_input_state(), Err(error));
    assert_eq!(window.acknowledge_input_reset(42), Err(error));
    assert_eq!(window.request_ticks(true), Err(error));
    assert_eq!(window.request_ticks(false), Err(error));
}
#[test]
fn surviving_window_operations_fail_without_native_ownership_or_resurrection() {
    let owner = Rc::new(RefCell::new(AppState {
        phase: AppPhase::Stopped,
        slots: vec![],
        free: vec![],
        main_thread: None,
    }));
    let weak = Rc::downgrade(&owner);
    let mut window = Window {
        owner: weak.clone(),
        id: id(),
    };
    assert_eq!(Rc::strong_count(&owner), 1);
    check_expired(&mut window, WindowError::ExpiredOwner);
    drop(owner);
    check_expired(&mut window, WindowError::ExpiredOwner);
    drop(window);
    assert!(weak.upgrade().is_none());
    let owner = Rc::new(RefCell::new(AppState {
        phase: AppPhase::Running,
        slots: vec![WindowSlot {
            generation: 4,
            retired: false,
            record: None,
        }],
        free: vec![],
        main_thread: None,
    }));
    let mut stale = Window {
        owner: Rc::downgrade(&owner),
        id: WindowId {
            slot: 0,
            generation: 3,
        },
    };
    check_expired(&mut stale, WindowError::ExpiredWindow);
    drop(stale);
    assert_eq!(Rc::strong_count(&owner), 1);
    let mut absent = Window {
        owner: Weak::new(),
        id: id(),
    };
    check_expired(&mut absent, WindowError::ExpiredOwner);
}

#[test]
fn geometry_three_scales_nonfinite_edges_and_exact_cursor_hotspot_recipe() {
    for scale in [1.0, 2.0, 3.0] {
        assert_eq!(
            super::geometry::pixels(
                Size {
                    width: 400.0,
                    height: 300.0
                },
                scale
            ),
            Some(SizeU32 {
                width: (400.0 * scale) as u32,
                height: (300.0 * scale) as u32
            })
        );
    }
    assert!(
        super::geometry::pixels(
            Size {
                width: f64::NAN,
                height: 1.0
            },
            1.0
        )
        .is_none()
    );
    assert!(
        super::geometry::pixels(
            Size {
                width: 1.0,
                height: 1.0
            },
            4.0
        )
        .is_none()
    );
    let descending = super::cursor::arrow_geometry(4.0, 4.0, 16.0, 16.0);
    let ascending = super::cursor::arrow_geometry(4.0, 20.0, 16.0, -16.0);
    assert_eq!(descending[1], (20.0, 20.0));
    assert_eq!(ascending[1], (20.0, 4.0));
    for endpoint in &descending[2..] {
        assert!(((endpoint.0 - 4.0).hypot(endpoint.1 - 4.0) - 4.0).abs() < 1.0e-12);
    }
}

#[test]
fn invalid_native_utf16_and_unadvertised_attributes_preserve_prior_snapshot() {
    let route = editable("saved");
    let weak = Rc::downgrade(&route);
    let units = [0xd800u16];
    // SAFETY: one readable UTF16 unit; NSString can represent an unpaired surrogate.
    let invalid = unsafe {
        NSString::initWithCharacters_length(
            NSString::alloc(),
            std::ptr::NonNull::from(&units[0]),
            1,
        )
    };
    let object: &AnyObject = &invalid;
    callback(&weak, (), |r| ime::insert(r, object, ime::missing()));
    assert_eq!(route.snapshot().unwrap().text, "saved");
    let attr = NSMutableAttributedString::initWithString(
        NSMutableAttributedString::alloc(),
        &NSString::from_str("x"),
    );
    let key = NSString::from_str("unadvertised-attribute");
    let number = NSNumber::numberWithLongLong(1);
    unsafe {
        attr.addAttribute_value_range(&key, &number, NSRange::new(0, 1));
    }
    let object: &AnyObject = &attr;
    callback(&weak, (), |r| {
        ime::marked(r, object, NSRange::new(0, 0), ime::missing())
    });
    assert_eq!(route.snapshot().unwrap().text, "saved");
    assert!(
        route
            .queue
            .borrow_mut()
            .drain(8, false)
            .events
            .iter()
            .any(|e| matches!(
                e,
                WindowEvent::QueueFault(QueueFault {
                    reason: QueueFaultReason::InvalidTextRange,
                    ..
                })
            ))
    );
}

#[test]
fn config_snapshot_attribute_and_candidate_limits_are_checked_before_replacement() {
    let good = WindowConfig {
        title: "x".repeat(1024),
        width_points: 4096,
        height_points: 1,
        resizable: true,
        initially_hidden: true,
    };
    assert!(good.valid());
    for config in [
        WindowConfig {
            title: "x".repeat(1025),
            ..good.clone()
        },
        WindowConfig {
            width_points: 0,
            ..good.clone()
        },
        WindowConfig {
            height_points: 4097,
            ..good.clone()
        },
        WindowConfig {
            title: String::new(),
            ..good
        },
    ] {
        assert!(!config.valid());
    }
    let mut snapshot = state("🦀");
    snapshot.marked_utf8 = Some(0..4);
    snapshot.marked_attributes = vec![TextAttributeSpan {
        range_utf8: 0..4,
        kind: TextAttributeKind::Underline,
        value: 2,
    }];
    assert!(validate_text_state(id(), &snapshot));
    snapshot.marked_attributes[0].range_utf8 = 1..4;
    assert!(!validate_text_state(id(), &snapshot));
    snapshot.marked_attributes[0].range_utf8 = 0..4;
    snapshot.marked_attributes[0].value = -1;
    assert!(!validate_text_state(id(), &snapshot));
    snapshot.marked_attributes.clear();
    snapshot.candidate_rect_points.origin.x = f64::INFINITY;
    assert!(!validate_text_state(id(), &snapshot));
    assert!(!validate_text_state(id(), &state(&"x".repeat(65537))));
}

#[test]
fn quartz_missing_display_and_candidate_inverse_geometry_are_explicit() {
    let screen = Rect {
        origin: Point {
            x: -1920.0,
            y: 100.0,
        },
        size: Size {
            width: 1920.0,
            height: 1080.0,
        },
    };
    let content = Rect {
        origin: Point {
            x: -1800.0,
            y: 400.0,
        },
        size: Size {
            width: 500.0,
            height: 400.0,
        },
    };
    assert!(super::geometry::display_origin(None, screen, content).is_none());
    assert_eq!(
        super::geometry::display_origin(
            Some(Rect {
                origin: Point {
                    x: -1920.0,
                    y: -1080.0
                },
                size: screen.size
            }),
            screen,
            content
        ),
        Some(Point {
            x: -1800.0,
            y: -700.0
        })
    );
    let candidate = Rect {
        origin: Point { x: 10.0, y: 20.0 },
        size: Size {
            width: 3.0,
            height: 16.0,
        },
    };
    let local = super::geometry::candidate_local(candidate, 300.0);
    assert_eq!(local.origin.y, 264.0);
    assert_eq!(
        super::geometry::top_left(
            Point {
                x: local.origin.x,
                y: local.origin.y + local.size.height
            },
            300.0
        ),
        Some(candidate.origin)
    );
    assert!(super::geometry::display_origin(Some(Rect::default()), screen, content).is_none());
}

#[test]
fn adjacent_geometry_replaces_but_scale_change_survives_a_barrier_with_same_pixels() {
    let mut q = EventQueue::default();
    let pixels = SizeU32 {
        width: 100,
        height: 100,
    };
    q.push(
        WindowEvent::Resize {
            points: Size {
                width: 10.0,
                height: 10.0,
            },
            pixels,
        },
        false,
    );
    q.push(
        WindowEvent::Resize {
            points: Size {
                width: 20.0,
                height: 20.0,
            },
            pixels,
        },
        false,
    );
    q.push(
        WindowEvent::BackingScaleChanged { scale: 2.0, pixels },
        false,
    );
    q.push(key(false), false);
    q.push(
        WindowEvent::BackingScaleChanged { scale: 3.0, pixels },
        false,
    );
    let events = q.drain(8, false).events;
    assert_eq!(events.len(), 4);
    assert!(matches!(
        events[1],
        WindowEvent::BackingScaleChanged { scale: 2.0, .. }
    ));
    assert!(matches!(
        events[3],
        WindowEvent::BackingScaleChanged { scale: 3.0, .. }
    ));
}
