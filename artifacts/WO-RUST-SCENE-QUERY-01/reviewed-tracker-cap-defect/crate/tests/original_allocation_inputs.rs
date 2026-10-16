//! Faithful safe construction equivalent for the assigned frozen C fixture.
fn build_allocation_source(padding: usize, uses: usize) -> Option<Vec<u8>> {
    const PREFIX:&[u8]=b"<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"paint\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"clip\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"8\" height=\"8\"/></clipPath><g id=\"target\"><rect width=\"8\" height=\"8\" fill=\"url(#paint)\"/></g></defs><!--";
    const MIDDLE: &[u8] = b"-->";
    const USE: &[u8] = b"<use href=\"#target\"/>";
    const SUFFIX: &[u8] =
        b"<rect width=\"8\" height=\"8\" fill=\"url(#paint)\" clip-path=\"url(#clip)\"/></svg>";
    if uses == 0 {
        return None;
    }
    let len = PREFIX
        .len()
        .checked_add(padding)?
        .checked_add(MIDDLE.len())?
        .checked_add(uses.checked_mul(USE.len())?)?
        .checked_add(SUFFIX.len())?;
    let mut out = Vec::new();
    out.try_reserve_exact(len).ok()?;
    out.extend_from_slice(PREFIX);
    out.resize(out.len().checked_add(padding)?, b'p');
    out.extend_from_slice(MIDDLE);
    for _ in 0..uses {
        out.extend_from_slice(USE);
    }
    out.extend_from_slice(SUFFIX);
    (out.len() == len).then_some(out)
}

#[test]
fn assigned_original_source_builder_preserves_bytes_and_checked_lengths() {
    let small = build_allocation_source(0, 1).unwrap();
    let padded = build_allocation_source(65536, 1).unwrap();
    let many = build_allocation_source(65536 - 31 * 21, 32).unwrap();
    assert_eq!(padded.len(), many.len());
    assert!(small.starts_with(b"<svg width=\"8\" height=\"8\">"));
    assert!(small.ends_with(b"clip-path=\"url(#clip)\"/></svg>"));
    assert!(padded.windows(8).any(|w| w == b"pppppppp"));
    assert_eq!(
        many.windows(b"<use href=\"#target\"/>".len())
            .filter(|w| *w == b"<use href=\"#target\"/>")
            .count(),
        32
    );
    assert!(build_allocation_source(0, 0).is_none());
    assert!(build_allocation_source(usize::MAX, 1).is_none());
    assert!(build_allocation_source(0, usize::MAX).is_none());
}
