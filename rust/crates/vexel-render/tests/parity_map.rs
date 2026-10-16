//! AC-7: parity-map.json covers every C function in the four reference files.
//! The map is plain text; entries are checked by substring so the crate keeps
//! its no-new-dependency rule (no JSON parser).

const MAP: &str = include_str!("../../../../artifacts/WO-RUST-V1-RENDER-01/parity-map.json");

/// Every function defined in shape_render.c, shape_render_fill.c,
/// shape_render_stroke.c and shape_render_stroke_grid.c.
const C_FUNCTIONS: &[&str] = &[
    // shape_render.c
    "checked_add_size",
    "checked_mul_size",
    "compute_stroke_work",
    "sample_is_in_view",
    "pixel_is_fully_in_view",
    "composite_source_over",
    "resolve_shape_sample",
    "composite_shape_row",
    "active_count_for",
    "active_edges_for",
    "prepare_local_stroke_edges",
    "raster_shape_render_flat",
    "raster_shape_render_with_stats",
    "raster_shape_render_with_stats_scalar_oracle_alpha",
    "raster_shape_render",
    // shape_render_fill.c
    "raster_shape_render_scanline_fill_shape",
    // shape_render_stroke.c
    "segment_hit_caps",
    "segment_hit",
    "in_triangle",
    "join_hit",
    "edge_box_contains",
    "dash_cache_contains",
    "stroke_range_contains",
    "raster_shape_stroke_contains",
    "raster_shape_stroke_sample_work",
    // shape_render_stroke_grid.c
    "add_size",
    "mul_size",
    "grid_bytes_fit",
    "segment_margin",
    "join_margin",
    "grid_axis_cell",
    "bbox_cells",
    "segment_bbox",
    "join_bbox",
    "add_bbox_references",
    "fill_bbox_references",
    "abandon_grid",
    "build_shape_grid",
    "raster_shape_stroke_grid_build",
    "raster_shape_stroke_grid_lookup",
];

fn entry(name: &str) -> &'static str {
    let key = format!("\"c_function\": \"{name}\"");
    let start = MAP.find(&key).unwrap_or_else(|| panic!("parity-map.json has no entry for {name}"));
    let rest = &MAP[start..];
    &rest[..rest.find('}').expect("entry end")]
}

#[test]
fn every_c_function_is_mapped_to_a_symbol_and_test_or_a_named_deferral() {
    for name in C_FUNCTIONS {
        let text = entry(name);
        let mapped = text.contains("\"rust_symbol\"") && text.contains("\"test\"");
        let deferred = text.contains("\"deferred\"");
        assert!(mapped || deferred, "{name}: neither mapped nor deferred");
    }
}

/// Function definitions found by a column-0 heuristic in a C source file.
fn defined_in(file: &str) -> Vec<String> {
    let path = format!("{}/../../../src/raster/{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut names = Vec::new();
    for line in text.lines() {
        let starts_lower = line.chars().next().is_some_and(|c| c.is_ascii_lowercase());
        let skipped = ["typedef", "struct", "return", "if", "for", "while", "else"]
            .iter()
            .any(|p| line.starts_with(p));
        if !starts_lower || skipped || line.trim_end().ends_with(';') {
            continue;
        }
        let Some(open) = line.find('(') else { continue };
        if line[..open].contains('=') {
            continue;
        }
        let name = line[..open]
            .trim_end()
            .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .next()
            .unwrap_or("");
        names.push(name.to_string());
    }
    names
}

#[test]
fn parity_map_lists_exactly_the_functions_defined_in_the_c_files() {
    let mut found: Vec<String> = [
        "shape_render.c",
        "shape_render_fill.c",
        "shape_render_stroke.c",
        "shape_render_stroke_grid.c",
    ]
    .iter()
    .flat_map(|f| defined_in(f))
    .collect();
    found.sort();
    found.dedup();
    let mut listed: Vec<String> = C_FUNCTIONS.iter().map(|s| s.to_string()).collect();
    listed.sort();
    listed.dedup();
    assert_eq!(found, listed, "C function inventory differs from the parity-map test list");
}

#[test]
fn mapped_symbols_and_tests_exist_in_the_crate() {
    let sources: String = ["lib", "adapter", "bounds", "composite", "fill", "presentation", "render", "stroke", "validate", "view"]
        .iter()
        .map(|m| std::fs::read_to_string(format!("{}/src/{m}.rs", env!("CARGO_MANIFEST_DIR"))).unwrap())
        .collect();
    let tests: String = ["fill", "stroke", "parity", "errors", "offscreen", "parity_map"]
        .iter()
        .map(|m| std::fs::read_to_string(format!("{}/tests/{m}.rs", env!("CARGO_MANIFEST_DIR"))).unwrap())
        .collect();
    for name in C_FUNCTIONS {
        let text = entry(name);
        let field = |key: &str| {
            let at = text.find(&format!("\"{key}\": \""))?;
            let rest = &text[at + key.len() + 5..];
            Some(rest[..rest.find('"')?].to_string())
        };
        if let Some(symbol) = field("rust_symbol") {
            assert!(sources.contains(&format!("fn {symbol}")) || sources.contains(&format!("struct {symbol}")), "{name}: symbol {symbol}");
        }
        if let Some(test) = field("test") {
            assert!(tests.contains(&format!("fn {test}")) || tests.contains(&format!("({test},")), "{name}: test {test}");
        }
        if let Some(kind) = field("deferred") {
            assert!(!kind.is_empty());
        }
    }
}
