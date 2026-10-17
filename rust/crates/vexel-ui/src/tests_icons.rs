//! WO-RUST-UI-ICONS-01 acceptance tests: 16-unit icon grid, crispness, consistency, centring.
use crate::icons_draw::{GRID, ROUND_CAPS, ROUND_JOINS, STROKE_UNITS, snapped_width};
use crate::*;

fn mask(cache: &mut IconCache, icon: IconKind, scale: usize) -> Vec<u8> {
    cache.mask(icon, scale).unwrap().to_vec()
}
fn covered(m: &[u8]) -> usize {
    m.iter().filter(|&&c| c > 0).count()
}

#[test]
fn ac1_every_kind_defined_unique_snake_case_and_bounded() {
    assert!(icon_validate_all());
    let defs = icon_definitions();
    assert_eq!(defs.len(), IconKind::COUNT as usize - 1);
    assert!(defs.len() <= 70);
    for (i, d) in defs.iter().enumerate() {
        assert_eq!(d.icon as usize, i + 1, "{}", d.name);
        assert!(
            d.name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                && !d.name.starts_with('_')
                && !d.name.ends_with('_'),
            "{}",
            d.name
        );
        assert!(defs[..i].iter().all(|p| p.name != d.name));
        assert_eq!(icons::icon_from(i as u32 + 1), Some(d.icon));
    }
    assert!(icons::icon_from(0).is_none() && icons::icon_from(IconKind::COUNT as u32).is_none());
    // Appended kinds keep the original 53 ids stable.
    assert_eq!(IconKind::SIDEBAR_RIGHT as u8, 53);
    assert_eq!(IconKind::EXPORT as u8, 54);
    assert_eq!(IconKind::TIMELINE as u8, 66);
}

#[test]
fn ac2_masks_are_non_empty_and_scale_consistently() {
    let mut cache = IconCache::new();
    let mut bad = Vec::new();
    for d in icon_definitions() {
        let m1 = mask(&mut cache, d.icon, 1);
        let m2 = mask(&mut cache, d.icon, 2);
        assert_eq!(m1.len(), GRID * GRID);
        assert_eq!(m2.len(), 4 * GRID * GRID);
        let n1 = covered(&m1);
        assert!(n1 >= 8, "{} covers {n1} px", d.name);
        let area = m1.iter().map(|&c| c as f64 / 255.0).sum::<f64>() / (GRID * GRID) as f64;
        if !(0.04..=0.60).contains(&area) {
            bad.push(format!("{} area {area:.3}", d.name));
        }
        let ratio = covered(&m2) as f64 / n1 as f64;
        if !(3.0..=5.0).contains(&ratio) {
            bad.push(format!("{} {ratio:.2}", d.name));
        }
    }
    assert!(
        bad.is_empty(),
        "2x/1x covered ratio or area outside 4%..60%: {bad:?}"
    );
}

#[test]
fn ac3_geometry_stays_inside_keyline_and_is_centred() {
    // Exempt from the centre rule: none. Chevrons and play are authored with centred bounds too.
    let exempt: [IconKind; 0] = [];
    for d in icon_definitions() {
        let b = icon_path_validate(d.path).unwrap();
        assert!(
            b.left >= 1.0 && b.top >= 1.0 && b.right <= 15.0 && b.bottom <= 15.0,
            "{} {b:?}",
            d.name
        );
        assert!(b.right > b.left || b.bottom > b.top, "{} empty", d.name);
        if exempt.contains(&d.icon) {
            continue;
        }
        let (cx, cy) = ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0);
        assert!(
            (cx - 8.0).abs() <= 0.75 && (cy - 8.0).abs() <= 0.75,
            "{} centre ({cx},{cy})",
            d.name
        );
    }
}

fn partial_share(m: &[u8]) -> f64 {
    m.iter().filter(|&&c| c > 16 && c < 239).count() as f64 / covered(m) as f64
}

fn outer_edge_is_solid(m: &[u8], axis: usize) -> bool {
    let ink = |x: usize, y: usize| m[y * axis + x] > 0;
    let (mut l, mut t, mut r, mut b) = (axis, axis, 0, 0);
    for y in 0..axis {
        for x in 0..axis {
            if ink(x, y) {
                l = l.min(x);
                r = r.max(x);
                t = t.min(y);
                b = b.max(y);
            }
        }
    }
    (l..=r).any(|x| m[t * axis + x] >= 200 || m[b * axis + x] >= 200)
        || (t..=b).any(|y| m[y * axis + l] >= 200 || m[y * axis + r] >= 200)
}

#[test]
fn ac4_axis_aligned_icons_are_crisp() {
    use IconKind as K;
    let set = [
        K::RECTANGLE,
        K::PLUS,
        K::MINUS,
        K::ALIGN_LEFT,
        K::ALIGN_CENTER,
        K::ALIGN_RIGHT,
        K::ALIGN_TOP,
        K::ALIGN_MIDDLE,
        K::ALIGN_BOTTOM,
        K::GRID,
        K::GROUP,
        K::UNGROUP,
        K::DUPLICATE,
        K::PAUSE,
        K::FIT,
        K::TEXT,
        K::SIDEBAR_LEFT,
        K::SIDEBAR_RIGHT,
        K::DOCUMENT,
        K::SAVE,
    ];
    let mut cache = IconCache::new();
    for icon in set {
        for (scale, limit) in [(1usize, 0.45), (2, 0.35)] {
            let m = mask(&mut cache, icon, scale);
            let share = partial_share(&m);
            assert!(share <= limit, "{icon:?} {scale}x partial share {share:.2}");
            assert!(
                outer_edge_is_solid(&m, GRID * scale),
                "{icon:?} {scale}x outer edge"
            );
        }
    }
}

#[test]
fn ac5_stroke_width_and_joins_are_global_constants() {
    assert_eq!(STROKE_UNITS, 1.5);
    assert!(ROUND_CAPS && ROUND_JOINS);
    // Snapped device width follows the single constant: 1.5 * scale, rounded up to an even pixel count.
    assert_eq!(
        (snapped_width(1), snapped_width(2), snapped_width(3)),
        (2.0, 4.0, 6.0)
    );
    assert_eq!(GRID, 16);
}

#[test]
fn ac6_centred_helper_centres_tool_icons_in_a_32_target() {
    let tokens = theme_tokens(Theme::Dark, false);
    for scale in [1u32, 2] {
        let (target, pad) = (32 * scale, 8 * scale);
        let side = target + 2 * pad;
        for icon in [
            IconKind::SELECT,
            IconKind::PEN,
            IconKind::RECTANGLE,
            IconKind::ELLIPSE,
            IconKind::LINE,
        ] {
            let mut rgba = vec![0u8; (side * side * 4) as usize];
            let capacity = rgba.len();
            let mut c = WidgetCanvas::new(
                &mut rgba,
                capacity,
                side,
                side,
                (side * 4) as usize,
                scale,
                Theme::Dark,
                false,
                None,
            )
            .unwrap();
            assert!(icon_centered(
                &mut c,
                Rect {
                    x: 8,
                    y: 8,
                    width: 32,
                    height: 32
                },
                icon,
                ControlState::Rest,
                &tokens
            ));
            let (mut l, mut t, mut r, mut b) = (side, side, 0, 0);
            for y in 0..side {
                for x in 0..side {
                    if rgba[((y * side + x) * 4 + 3) as usize] != 0 {
                        l = l.min(x);
                        r = r.max(x);
                        t = t.min(y);
                        b = b.max(y);
                    }
                }
            }
            assert!(
                l >= pad && t >= pad && r < pad + target && b < pad + target,
                "{icon:?} {scale}x outside target"
            );
            let (cx, cy) = ((l + r + 1) as f64 / 2.0, (t + b + 1) as f64 / 2.0);
            let mid = (pad + target / 2) as f64;
            assert!(
                (cx - mid).abs() <= 1.0 && (cy - mid).abs() <= 1.0,
                "{icon:?} {scale}x centre ({cx},{cy}) vs {mid}"
            );
        }
    }
    let mut small = vec![0u8; 8 * 8 * 4];
    let mut c = WidgetCanvas::new(&mut small, 256, 8, 8, 32, 1, Theme::Dark, false, None).unwrap();
    assert!(!icon_centered(
        &mut c,
        Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 8
        },
        IconKind::PEN,
        ControlState::Rest,
        &tokens
    ));
}

#[test]
fn ac7_deterministic_and_cache_is_bounded() {
    let (mut a, mut b) = (IconCache::new(), IconCache::new());
    assert_eq!(a.slots(), IconKind::COUNT as usize * 4);
    for scale in 1..=3 {
        for d in icon_definitions() {
            assert_eq!(
                mask(&mut a, d.icon, scale),
                mask(&mut b, d.icon, scale),
                "{} {scale}x",
                d.name
            );
        }
    }
    assert_eq!(a.slots(), IconKind::COUNT as usize * 4);
    assert_eq!(a.used(), icon_definitions().len() * 3);
    assert_eq!(a.asset_hash(Theme::Dark, 2), b.asset_hash(Theme::Light, 2));
    a.clear();
    assert_eq!(a.used(), 0);
}

// ---- WO-RUST-UI-ICONS-02: redrawn artwork properties ----

/// Pre-change 1x covered pixel counts (ICONS-01 artwork), recorded in result.md.
const PEN_PX_BEFORE: usize = 88;
const NODE_EDIT_PX_BEFORE: usize = 82;

const IDS_1_53: [(u8, &str); 53] = [
    (1, "select"),
    (2, "pan"),
    (3, "zoom_in"),
    (4, "zoom_out"),
    (5, "rectangle"),
    (6, "ellipse"),
    (7, "line"),
    (8, "pen"),
    (9, "node_edit"),
    (10, "pencil"),
    (11, "text"),
    (12, "eyedropper"),
    (13, "gradient"),
    (14, "fill"),
    (15, "stroke"),
    (16, "layers"),
    (17, "eye"),
    (18, "eye_off"),
    (19, "lock"),
    (20, "unlock"),
    (21, "plus"),
    (22, "minus"),
    (23, "close"),
    (24, "chevron_right"),
    (25, "chevron_down"),
    (26, "more"),
    (27, "search"),
    (28, "save"),
    (29, "undo"),
    (30, "redo"),
    (31, "duplicate"),
    (32, "trash"),
    (33, "group"),
    (34, "ungroup"),
    (35, "fit"),
    (36, "actual_size"),
    (37, "ruler"),
    (38, "grid"),
    (39, "snap"),
    (40, "align_left"),
    (41, "align_center"),
    (42, "align_right"),
    (43, "align_top"),
    (44, "align_middle"),
    (45, "align_bottom"),
    (46, "circle"),
    (47, "document"),
    (48, "stop"),
    (49, "path"),
    (50, "polyline"),
    (51, "image"),
    (52, "sidebar_left"),
    (53, "sidebar_right"),
];

#[test]
fn icons02_ac2_ids_1_to_53_are_unchanged() {
    let defs = icon_definitions();
    for (id, name) in IDS_1_53 {
        let d = &defs[id as usize - 1];
        assert_eq!((d.icon as u8, d.name), (id, name));
        assert_eq!(icons::icon_from(id as u32), Some(d.icon));
    }
    assert_eq!(IconKind::PAUSE as u8, 56);
    assert_eq!(IconKind::TIMELINE as u8, 66);
}

/// 4-connected components of pixels with coverage >= `min`: (pixel list, l, t, r, b).
type Comp = (Vec<(usize, usize)>, usize, usize, usize, usize);

fn components(m: &[u8], axis: usize, min: u8) -> Vec<Comp> {
    let mut seen = vec![false; m.len()];
    let mut out = Vec::new();
    for y in 0..axis {
        for x in 0..axis {
            if seen[y * axis + x] || m[y * axis + x] < min {
                continue;
            }
            let (mut px, mut stack) = (Vec::new(), vec![(x, y)]);
            seen[y * axis + x] = true;
            let (mut l, mut t, mut r, mut b) = (x, y, x, y);
            while let Some((cx, cy)) = stack.pop() {
                px.push((cx, cy));
                (l, t, r, b) = (l.min(cx), t.min(cy), r.max(cx), b.max(cy));
                let nb = [
                    (cx.wrapping_sub(1), cy),
                    (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)),
                    (cx, cy + 1),
                ];
                for (nx, ny) in nb {
                    if nx < axis && ny < axis && !seen[ny * axis + nx] && m[ny * axis + nx] >= min {
                        seen[ny * axis + nx] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            out.push((px, l, t, r, b));
        }
    }
    out
}

fn ink_bounds(m: &[u8], axis: usize) -> (usize, usize, usize, usize) {
    let (mut l, mut t, mut r, mut b) = (axis, axis, 0, 0);
    for y in 0..axis {
        for x in 0..axis {
            if m[y * axis + x] > 0 {
                (l, t, r, b) = (l.min(x), t.min(y), r.max(x), b.max(y));
            }
        }
    }
    (l, t, r, b)
}

#[test]
fn icons02_ac3_pause_is_two_separated_bars() {
    let m = mask(&mut IconCache::new(), IconKind::PAUSE, 1);
    let mut comps = components(&m, GRID, 128);
    assert_eq!(comps.len(), 2);
    comps.sort_by_key(|c| c.1);
    let (a, b) = (&comps[0], &comps[1]);
    assert!(
        a.3 - a.1 + 1 >= 2 && b.3 - b.1 + 1 >= 2,
        "bars at least 2 px wide"
    );
    let mut shared = 0;
    for y in 0..GRID {
        let ra = a.0.iter().filter(|p| p.1 == y).map(|p| p.0).max();
        let lb = b.0.iter().filter(|p| p.1 == y).map(|p| p.0).min();
        if let (Some(ra), Some(lb)) = (ra, lb) {
            shared += 1;
            assert!(lb - ra - 1 >= 2, "row {y} gap {}", lb - ra - 1);
            assert!(
                (ra + 1..lb).all(|x| m[y * GRID + x] < 64),
                "row {y} gap ink"
            );
        }
    }
    assert!(shared >= 6);
}

#[test]
fn icons02_ac4_timeline_playhead_above_track() {
    let m = mask(&mut IconCache::new(), IconKind::TIMELINE, 1);
    let mut comps = components(&m, GRID, 128);
    assert_eq!(comps.len(), 2);
    comps.sort_by_key(|c| c.2);
    let (head, track) = (&comps[0], &comps[1]);
    assert!(head.4 < track.2, "playhead bbox above track bbox");
    assert!(track.3 - track.1 + 1 >= 10, "track width");
    assert!(track.4 - track.2 + 1 <= 3, "track height");
}

#[test]
fn icons02_ac5_ruler_is_wide_with_three_tick_runs() {
    let m = mask(&mut IconCache::new(), IconKind::RULER, 1);
    let (l, t, r, b) = ink_bounds(&m, GRID);
    let (w, h) = (r - l + 1, b - t + 1);
    assert!(w >= 2 * h || h >= 2 * w, "ink {w}x{h}");
    // A row with >= 3 separate ink runs that touch neither ink-bounds side (ticks, not frame).
    let ok = (t..=b).any(|y| {
        let mut runs = 0;
        let mut x = l;
        while x <= r {
            if m[y * GRID + x] >= 128 {
                let s = x;
                while x <= r && m[y * GRID + x] >= 128 {
                    x += 1;
                }
                if s > l && x - 1 < r {
                    runs += 1;
                }
            } else {
                x += 1;
            }
        }
        runs >= 3
    });
    assert!(ok, "no row crosses 3 tick runs");
}

#[test]
fn icons02_ac6_group_has_dashed_ring_and_two_inner_squares() {
    let m = mask(&mut IconCache::new(), IconKind::GROUP, 1);
    let (l, t, r, b) = ink_bounds(&m, GRID);
    // Perimeter walk one pixel inside the ink bounds, clockwise, circular gap count.
    let mut ring = Vec::new();
    for x in l + 1..r {
        ring.push((x, t + 1));
    }
    for y in t + 1..b {
        ring.push((r - 1, y));
    }
    for x in (l + 2..=r - 1).rev() {
        ring.push((x, b - 1));
    }
    for y in (t + 2..=b - 1).rev() {
        ring.push((l + 1, y));
    }
    let gap = |i: usize| {
        let (x, y) = ring[i % ring.len()];
        m[y * GRID + x] < 64
    };
    let gaps = (0..ring.len())
        .filter(|&i| gap(i) && !gap(i + ring.len() - 1))
        .count();
    assert!(gaps >= 4, "ring gaps {gaps}");
    let inner = components(&m, GRID, 128)
        .into_iter()
        .filter(|c| c.1 >= l + 3 && c.2 >= t + 3 && c.3 + 3 <= r && c.4 + 3 <= b)
        .count();
    assert!(inner >= 2, "inner components {inner}");
}

#[test]
fn icons02_ac7_gradient_eyedropper_actual_size_pen_node_edit() {
    let mut c = IconCache::new();
    let gradient =
        icon_path_validate(icon_definitions()[IconKind::GRADIENT as usize - 1].path).unwrap();
    assert!(gradient.right - gradient.left >= 12.0 && gradient.bottom - gradient.top >= 12.0);
    let m = mask(&mut c, IconKind::GRADIENT, 1);
    let inner = (3..13)
        .flat_map(|y| (3..13).map(move |x| (x, y)))
        .filter(|&(x, y)| m[y * GRID + x] >= 128)
        .count();
    assert!(inner >= 24, "ramp ink inside the frame: {inner} px");
    let m = mask(&mut c, IconKind::EYEDROPPER, 1);
    let area: f64 = m.iter().map(|&v| v as f64 / 255.0).sum::<f64>() / (GRID * GRID) as f64;
    assert!(area <= 0.30, "eyedropper area {area:.3}");

    let m = mask(&mut c, IconKind::ACTUAL_SIZE, 1);
    let comps = components(&m, GRID, 128);
    let corner = |c: &Comp| (c.1 <= 3 || c.3 >= 12) && (c.2 <= 3 || c.4 >= 12);
    let centre = |c: &Comp| c.1 >= 4 && c.3 <= 11 && c.2 >= 3 && c.4 <= 12;
    assert!(
        comps.iter().filter(|c| corner(c)).count() >= 4,
        "corner components"
    );
    assert!(
        comps.iter().filter(|c| centre(c)).count() >= 1,
        "centre component"
    );

    let pen = covered(&mask(&mut c, IconKind::PEN, 1));
    let node = covered(&mask(&mut c, IconKind::NODE_EDIT, 1));
    assert!(pen < PEN_PX_BEFORE, "pen {pen} >= {PEN_PX_BEFORE}");
    assert!(
        node < NODE_EDIT_PX_BEFORE,
        "node_edit {node} >= {NODE_EDIT_PX_BEFORE}"
    );
}

#[test]
fn icons02_ac8_masks_are_distinct_and_timeline_differs_from_align_left() {
    let mut c = IconCache::new();
    for scale in [1usize, 2] {
        let all: Vec<_> = icon_definitions()
            .iter()
            .map(|d| (d.name, mask(&mut c, d.icon, scale)))
            .collect();
        for (i, (na, ma)) in all.iter().enumerate() {
            // keyframe and keyframe_filled differ only in fill, which is part of the mask.
            for (nb, mb) in &all[..i] {
                assert_ne!(ma, mb, "{na} == {nb} at {scale}x");
            }
        }
    }
    let t = mask(&mut c, IconKind::TIMELINE, 1);
    let a = mask(&mut c, IconKind::ALIGN_LEFT, 1);
    assert!(t.iter().zip(&a).filter(|(x, y)| x != y).count() >= 12);
}
