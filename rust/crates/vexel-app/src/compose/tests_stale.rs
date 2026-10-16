//! Stale worker results never replace a newer frame.
use super::tests::fixture;
use super::*;

#[test]
fn stale_worker_events_never_replace_a_newer_frame() {
    let svg = fixture("compose-v1.svg");
    let mut view = DocumentView::new();
    view.set_name("doc");
    view.show(&svg, 1, (160, 120));
    let old_generation = view.generation;
    view.show(&svg, 2, (80, 60));
    assert!(view.generation > old_generation);
    let event = |generation, w, h| WorkerEvent {
        generation,
        result: compose_document(&svg, w, h, &|| false),
    };
    // The newer frame arrives first, then a late result for the older request.
    assert!(view.accept(event(view.generation, 80, 60)));
    assert_eq!(view.composed().unwrap().frame.width, 80);
    assert!(!view.accept(event(old_generation, 160, 120)));
    assert_eq!(view.composed().unwrap().frame.width, 80);
    // An older error does not clobber the frame either.
    assert!(!view.accept(WorkerEvent {
        generation: old_generation,
        result: Err(ComposeError::Hidden),
    }));
    assert!(view.composed().is_some());
}
