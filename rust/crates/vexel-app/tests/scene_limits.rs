//! WO-RUST-SCENE-LIMITS-01 boundaries through the real compose pipeline:
//! reference ids and cumulative `<use>` reference bytes, N accepted and N+1
//! refused with the same typed error.
use vexel_app::compose::{ComposeError, compose_svg};

/// Mirrors `vexel_svg_reference::MAX_REFERENCE_IDS` (pinned in that crate's tests).
const MAX_REFERENCE_IDS: usize = 262_144;

fn ids_document(count: usize) -> String {
    let mut svg = String::from("<svg width='8' height='8' xmlns='http://www.w3.org/2000/svg'>");
    for n in 0..count {
        svg.push_str(&format!("<g id='i{n}'/>"));
    }
    svg.push_str("<rect width='4' height='4'/></svg>");
    svg
}

#[test]
fn reference_id_cap_is_262144_and_n_plus_one_is_refused() {
    assert!(compose_svg(ids_document(MAX_REFERENCE_IDS).as_bytes(), 8, 8).is_ok());
    assert_eq!(
        compose_svg(ids_document(MAX_REFERENCE_IDS + 1).as_bytes(), 8, 8),
        Err(ComposeError::LimitDecode)
    );
}

#[test]
fn corpus_measured_id_counts_compose() {
    // chuaattractormodified: 100,088 id attributes; ideogram chromosome: 144,217.
    for count in [100_088usize, 144_217] {
        assert!(
            compose_svg(ids_document(count).as_bytes(), 8, 8).is_ok(),
            "{count}"
        );
    }
}

fn use_document(target_bytes: usize, uses: usize) -> String {
    let base = "<g id='x'><!----><rect width='1' height='1'/></g>".len();
    format!(
        "<svg width='8' height='8' xmlns='http://www.w3.org/2000/svg'><defs><g id='x'><!--{}--><rect width='1' height='1'/></g></defs>{}</svg>",
        "p".repeat(target_bytes - base),
        "<use href='#x'/>".repeat(uses)
    )
}

#[test]
fn use_reference_budget_is_16_mib_and_the_next_reference_is_refused() {
    // 64 KiB targets: 256 references fill the 16 MiB budget exactly.
    assert!(compose_svg(use_document(64 * 1024, 256).as_bytes(), 8, 8).is_ok());
    assert_eq!(
        compose_svg(use_document(64 * 1024, 257).as_bytes(), 8, 8),
        Err(ComposeError::LimitDecode)
    );
}

#[test]
fn generated_use_t1_shape_composes() {
    // 17,536 uses of a 91-byte target (1.6 MB cumulative); the former 1 MiB budget
    // refused this at about 11,500 uses.
    assert!(compose_svg(use_document(91, 17_536).as_bytes(), 8, 8).is_ok());
}
