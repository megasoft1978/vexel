//! Text parity against C `svgworkbench` PPM goldens (read-only inputs, macOS only).
//!
//! Pipeline under test: SVG -> real Decoder + engine text decode -> CoreText
//! layout (`vexel-platform-apple`) -> `from_scene_shape` -> `render_shapes`.
//! Goldens depend on installed fonts and the macOS version recorded in
//! `rust/fixtures/render-text/manifest.json`; if a recorded font file is
//! missing the parity tests print an explicit SKIP with the reason (never a pass
//! claim). Tolerances are copied unchanged from `vexel-render/tests/parity.rs`.
#![cfg(target_os = "macos")]
use std::sync::Arc;
use vexel_raster::Tile;
use vexel_render::{
    FlatEdge, PreserveAspect, SceneFeatures, ViewBox, ViewMap, from_scene_shape, render_shapes,
};
use vexel_scene::*;
use vexel_svg_decode::TraversalDecodeEvent;
use vexel_svg_engine::{SvgEngineSession, TextLayoutProvider};

/// Thresholds of the work order. They are fixed here and must not be raised.
mod tolerance {
    pub const MAX_CHANNEL_DELTA: i32 = 1;
    pub const MAX_DIFFERING_FRACTION: f64 = 0.005;
    pub const MAX_ANY_DELTA: i32 = 2;
    /// Pixels farther than this from any shape edge must be exactly equal.
    pub const DILATION_PIXELS: f64 = 1.0;
}

const NEVER: &dyn Fn() -> bool = &|| false;

fn empty_tile(width: u32, height: u32) -> Tile {
    Tile::from_owned_rgba(width, height, vec![0; width as usize * height as usize * 4])
        .expect("tile")
}

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/render-text");

struct Unlimited;
impl WorkControl for Unlimited {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

/// Engine provider over the platform CoreText layout.
struct CoreText;
impl TextLayoutProvider for CoreText {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        use vexel_platform_apple as p;
        let style = match request.style {
            TextStyle::Normal => p::TextStyle::Normal,
            TextStyle::Italic => p::TextStyle::Italic,
            TextStyle::Oblique => p::TextStyle::Oblique,
        };
        let layout = p::text_layout(&p::TextRequest {
            text: request.text,
            font_families: request.font_families,
            size_points: request.size_points,
            weight: request.weight,
            style,
            letter_spacing_points: request.letter_spacing_points,
        })
        .map_err(|e| match e {
            p::TextLayoutError::Limit => MediaError::Limit,
            p::TextLayoutError::OutOfMemory => MediaError::OutOfMemory,
            p::TextLayoutError::InvalidUtf8 => MediaError::InvalidUtf8,
            p::TextLayoutError::InvalidArgument => MediaError::InvalidArgument,
            _ => MediaError::Unsupported,
        })?;
        Ok(TextLayout {
            glyphs: layout
                .glyphs
                .iter()
                .map(|g| TextGlyph {
                    glyph_id: Some(g.glyph_id),
                    x: g.x,
                    y: g.y,
                    advance: g.advance,
                    commands: TableRange {
                        first: g.command_start,
                        count: g.command_count,
                    },
                })
                .collect(),
            commands: layout
                .commands
                .iter()
                .map(|c| TextCommand {
                    verb: match c.verb {
                        p::TextVerb::Move => TextVerb::Move,
                        p::TextVerb::Line => TextVerb::Line,
                        p::TextVerb::Cubic => TextVerb::Cubic,
                        p::TextVerb::Close => TextVerb::Close,
                    },
                    values: c.values,
                })
                .collect(),
            advance_width: layout.advance_width,
            ascent: layout.ascent,
            descent: layout.descent,
            leading: layout.leading,
        })
    }
}

/// Present only so the decoder reaches the engine's own provider.
struct Gate;
impl MediaProvider for Gate {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        Err(MediaError::Unsupported)
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}

fn decode(svg: &[u8]) -> SceneGraph {
    let mut control = Unlimited;
    let owner = Arc::new(svg.to_vec());
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control).expect("parse");
    let mut source = DecodeSource::new(&document, &mut control).expect("source");
    let job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536).expect("ledger"))
        .ok()
        .expect("job");
    let mut gate = Gate;
    let mut session = SvgEngineSession::from_source(
        &document,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: Some(&mut gate),
        },
        &mut control,
    )
    .expect("session");
    session.set_text_provider(Box::new(CoreText));
    loop {
        match session.step(64, &job, &mut control).expect("step") {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::Complete => break,
            TraversalDecodeEvent::NeedPattern(_) => panic!("pattern"),
        }
    }
    match session.finish(&mut control).expect("finish") {
        DecodedScene::Drawable(graph) => graph,
        DecodedScene::Hidden(_) => panic!("hidden"),
    }
}

struct Rendered {
    tile: Tile,
    edges: Vec<FlatEdge>,
}

fn render(graph: &SceneGraph) -> Rendered {
    let view = graph.root();
    SceneFeatures {
        clips: view.clips.len(),
        opacity_groups: view.groups.len(),
        masks: view.masks.len(),
        filters: view.filters.len(),
        ..Default::default()
    }
    .require_supported()
    .expect("supported scene");
    struct Owned {
        edges: Vec<FlatEdge>,
        ranges: Vec<std::ops::Range<usize>>,
        closed: Vec<bool>,
    }
    let mut owned = Vec::new();
    for shape in view.shapes {
        let edges: Vec<FlatEdge> = shape
            .edges
            .slice(view.edges)
            .unwrap()
            .iter()
            .copied()
            .map(FlatEdge::from)
            .collect();
        let (mut ranges, mut closed) = (Vec::new(), Vec::new());
        for sub in shape.subpaths.slice(view.path_subpaths).unwrap() {
            let start = sub.first_edge - shape.edges.first;
            ranges.push(start..start + sub.edge_count);
            closed.push(sub.closed);
        }
        owned.push(Owned {
            edges,
            ranges,
            closed,
        });
    }
    let mut shapes = Vec::new();
    for (shape, geometry) in view.shapes.iter().zip(&owned) {
        let fill = view
            .edge_colors
            .get(shape.edges.first)
            .copied()
            .unwrap_or([0; 4]);
        shapes.push(
            from_scene_shape(
                shape,
                fill,
                view.fill_paints,
                view.stroke_paints,
                &geometry.edges,
                &geometry.ranges,
                &geometry.closed,
            )
            .expect("shape"),
        );
    }
    let vp = view.viewport;
    let aspect = vp.aspect.set.then_some(PreserveAspect {
        align_x: vp.aspect.align_x,
        align_y: vp.aspect.align_y,
        none: vp.aspect.none,
        slice: vp.aspect.slice,
    });
    let map = ViewMap::from_viewbox(
        ViewBox {
            x: vp.view_x,
            y: vp.view_y,
            width: vp.view_width,
            height: vp.view_height,
        },
        vp.width,
        vp.height,
        aspect,
    )
    .unwrap();
    let mut tile = empty_tile(vp.width, vp.height);
    render_shapes(&shapes, map, &mut tile, NEVER).expect("render");
    Rendered {
        tile,
        edges: owned.into_iter().flat_map(|o| o.edges).collect(),
    }
}

/// C `write_tile_ppm` colour conversion.
fn over_white(tile: &Tile) -> Vec<u8> {
    tile.rgba()
        .chunks_exact(4)
        .flat_map(|p| {
            let a = u32::from(p[3]);
            (0..3).map(move |c| (u32::from(p[c]) + 255 - a).min(255) as u8)
        })
        .collect()
}

fn load_ppm(id: &str) -> (u32, u32, Vec<u8>) {
    let bytes = std::fs::read(format!("{DIR}/{id}.ppm")).expect("golden");
    let mut parts = bytes.splitn(4, |b| *b == b'\n');
    assert_eq!(parts.next().unwrap(), b"P6");
    let dims = String::from_utf8(parts.next().unwrap().to_vec()).unwrap();
    let (w, h) = dims.split_once(' ').unwrap();
    assert_eq!(parts.next().unwrap(), b"255");
    (
        w.parse().unwrap(),
        h.parse().unwrap(),
        parts.next().unwrap().to_vec(),
    )
}

#[derive(Debug)]
struct Report {
    total: usize,
    max_delta: i32,
    differing: usize,
    over_two: usize,
    outside_dilation: usize,
}
impl Report {
    fn passes(&self) -> bool {
        self.max_delta <= tolerance::MAX_CHANNEL_DELTA
            && self.over_two == 0
            && self.outside_dilation == 0
            && (self.differing as f64) <= tolerance::MAX_DIFFERING_FRACTION * self.total as f64
    }
}

fn near_edge(edges: &[FlatEdge], x: f64, y: f64) -> bool {
    let (cx, cy) = (x + 0.5, y + 0.5);
    edges.iter().any(|e| {
        let (dx, dy) = (e.x1 - e.x0, e.y1 - e.y0);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((cx - e.x0) * dx + (cy - e.y0) * dy) / len2).clamp(0.0, 1.0)
        };
        (cx - (e.x0 + t * dx)).hypot(cy - (e.y0 + t * dy)) <= tolerance::DILATION_PIXELS + 0.75
    })
}

fn compare(width: u32, edges: &[FlatEdge], actual: &[u8], golden: &[u8]) -> Report {
    let mut r = Report {
        total: golden.len() / 3,
        max_delta: 0,
        differing: 0,
        over_two: 0,
        outside_dilation: 0,
    };
    for index in 0..r.total {
        let delta = (0..3)
            .map(|c| (i32::from(actual[index * 3 + c]) - i32::from(golden[index * 3 + c])).abs())
            .max()
            .unwrap();
        r.max_delta = r.max_delta.max(delta);
        if delta > 0 {
            r.differing += 1;
            if delta > tolerance::MAX_ANY_DELTA {
                r.over_two += 1;
            }
            let (x, y) = (
                (index % width as usize) as f64,
                (index / width as usize) as f64,
            );
            if !near_edge(edges, x, y) {
                r.outside_dilation += 1;
            }
        }
    }
    r
}

fn manifest() -> String {
    std::fs::read_to_string(format!("{DIR}/manifest.json")).expect("manifest")
}

/// Explicit skip reason when the recorded fonts are not installed.
fn font_precondition() -> Option<String> {
    let manifest = manifest();
    for line in manifest
        .lines()
        .filter(|l| l.trim_start().starts_with("\"file\":"))
    {
        let path = line.split('"').nth(3).unwrap_or_default();
        if !std::path::Path::new(path).exists() {
            return Some(format!("recorded font file {path} is not installed"));
        }
    }
    None
}

fn run(id: &str) {
    if let Some(reason) = font_precondition() {
        println!("SKIP {id}: {reason}");
        return;
    }
    let svg = std::fs::read(format!("{DIR}/{id}.svg")).expect("svg");
    let graph = decode(&svg);
    let rendered = render(&graph);
    let (w, h, golden) = load_ppm(id);
    assert_eq!(
        (w, h),
        (rendered.tile.width(), rendered.tile.height()),
        "{id}: size"
    );
    let report = compare(w, &rendered.edges, &over_white(&rendered.tile), &golden);
    println!(
        "PARITY {id}: pixels={} max_delta={} differing={} over_two={} outside_dilation={}",
        report.total, report.max_delta, report.differing, report.over_two, report.outside_dilation
    );
    assert!(report.passes(), "{id}: {report:?}");
}

macro_rules! parity {
    ($name:ident, $id:expr) => {
        #[test]
        fn $name() {
            run($id);
        }
    };
}
parity!(helvetica_line, "helvetica-line");
parity!(anchor_middle, "anchor-middle");
parity!(anchor_end, "anchor-end");
parity!(glyph_x_dx, "glyph-x-dx");
parity!(glyph_dx_only, "glyph-dx-only");
parity!(letter_spacing, "letter-spacing");
parity!(word_spacing, "word-spacing");
parity!(tspan_colour, "tspan-colour");
parity!(bold, "bold");
parity!(italic, "italic");
parity!(times, "times");
parity!(menlo, "menlo");
parity!(transform, "transform");
parity!(corpus_text_basic, "corpus-text-basic");
parity!(corpus_text_spans, "corpus-text-spans");
parity!(corpus_text_positioning, "corpus-text-positioning");

// ---- SHA-256 (std only) for the manifest hash test ----
fn sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[4 * i..4 * i + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

fn field(block: &str, name: &str) -> String {
    let line = block
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("\"{name}\":")))
        .unwrap();
    line.split('"').nth(3).unwrap().to_string()
}

#[test]
fn manifest_hashes_match_every_fixture_file() {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let manifest = manifest();
    let mut seen = 0;
    for block in manifest.split("\"fixture_id\":").skip(1) {
        let id = block.split('"').nth(1).unwrap().to_string();
        let block = format!("\"fixture_id\":{block}");
        let svg = std::fs::read(format!("{DIR}/{id}.svg")).unwrap();
        let ppm = std::fs::read(format!("{DIR}/{id}.ppm")).unwrap();
        assert_eq!(
            sha256(&svg),
            field(&block, "source_sha256"),
            "{id}: source hash"
        );
        assert_eq!(sha256(&ppm), field(&block, "ppm_sha256"), "{id}: ppm hash");
        seen += 1;
    }
    assert_eq!(seen, 16);
    for key in [
        "binary_sha256",
        "legacy_tag_ref_value",
        "product_version",
        "build_version",
        "cmake_version",
    ] {
        assert!(manifest.contains(&format!("\"{key}\"")), "{key}");
    }
}
