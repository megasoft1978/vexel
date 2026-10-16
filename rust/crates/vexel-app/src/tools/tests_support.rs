//! Shared helpers for the headless tool tests.

use super::{CanvasMap, Key, Outcome, PointerInput, ScaleMap, ToolController, ToolError};
use vexel_v1_document::V1Document;

pub const HEAD: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"100\" viewBox=\"0 0 200 100\"";
pub const PRESERVE: &[u8] = include_bytes!("../../../../fixtures/v1/draw/preserve.svg");

pub type Pt = (f64, f64);
pub type Res = Result<Outcome, ToolError>;

pub fn blank() -> V1Document {
    V1Document::new(200, 100).unwrap()
}
pub fn wrap(body: &str) -> String {
    format!("{HEAD}>{body}</svg>")
}
pub fn with(body: &str) -> V1Document {
    V1Document::from_source(wrap(body).as_bytes()).unwrap()
}
pub fn src(doc: &V1Document) -> String {
    String::from_utf8(doc.source().unwrap()).unwrap()
}

/// Canvas = document * 2 + (10, 20): a non-identity scale and offset.
pub fn map() -> ScaleMap {
    ScaleMap::new((10.0, 20.0), 2.0, (200.0, 100.0)).unwrap()
}
pub fn canvas(p: Pt) -> Pt {
    (p.0 * 2.0 + 10.0, p.1 * 2.0 + 20.0)
}

pub fn down(c: &mut ToolController, d: &mut V1Document, m: &dyn CanvasMap, p: Pt) -> Res {
    let (x, y) = canvas(p);
    c.pointer(PointerInput::Down { x, y }, d, m)
}
pub fn mv(c: &mut ToolController, d: &mut V1Document, m: &dyn CanvasMap, p: Pt) -> Res {
    let (x, y) = canvas(p);
    c.pointer(PointerInput::Move { x, y }, d, m)
}
pub fn up(c: &mut ToolController, d: &mut V1Document, m: &dyn CanvasMap, p: Pt) -> Res {
    let (x, y) = canvas(p);
    c.pointer(PointerInput::Up { x, y }, d, m)
}
pub fn click(c: &mut ToolController, d: &mut V1Document, p: Pt) -> Res {
    down(c, d, &map(), p)?;
    up(c, d, &map(), p)
}
/// Down, an intermediate Move, a Move to `to`, then Up at `to`.
pub fn drag(c: &mut ToolController, d: &mut V1Document, from: Pt, to: Pt) -> Res {
    let m = map();
    down(c, d, &m, from)?;
    mv(c, d, &m, ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0))?;
    mv(c, d, &m, to)?;
    up(c, d, &m, to)
}
pub fn enter(c: &mut ToolController, d: &mut V1Document) -> Res {
    c.key(Key::Enter, d)
}

/// Runs `act`, then checks it was exactly one history entry whose undo restores
/// the exact prior bytes and whose redo restores the exact post-edit bytes, with
/// `is_dirty` following the saved state. Returns the post-edit source.
pub fn one_undoable_entry(doc: &mut V1Document, act: impl FnOnce(&mut V1Document)) -> String {
    doc.mark_saved();
    assert!(!doc.is_dirty());
    let before = doc.source().unwrap();
    let count = doc.history_count();
    act(doc);
    assert_eq!(
        doc.history_count(),
        count + 1,
        "one gesture is one history entry"
    );
    assert!(doc.is_dirty());
    let after = doc.source().unwrap();
    assert_ne!(after, before);
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    assert!(!doc.is_dirty());
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
    assert!(doc.is_dirty());
    String::from_utf8(after).unwrap()
}

/// Asserts nothing was committed: same bytes, history and dirty state.
pub fn assert_untouched(doc: &V1Document, before: &[u8], count: usize) {
    assert_eq!(doc.source().unwrap(), before);
    assert_eq!(doc.history_count(), count);
    assert!(!doc.is_dirty());
}

pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}
