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
