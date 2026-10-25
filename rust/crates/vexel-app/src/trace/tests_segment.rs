use super::inpaint::inpaint;
use super::segment::{Spec, rasterize, segment};
use super::{Rgba, TraceError};

fn disc_image(w: usize, h: usize) -> Vec<u8> {
    // Blue speckled background, red disc (r=20 at 40,40) with a darker red eye.
    let mut px = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let n = ((x * 7 + y * 13) % 9) as u8;
            let r2 = (x as f64 - 40.0).powi(2) + (y as f64 - 40.0).powi(2);
            let eye = (x as f64 - 46.0).powi(2) + (y as f64 - 36.0).powi(2) < 9.0;
            let c = if eye {
                [90, 10, 10]
            } else if r2 <= 400.0 {
                [210 + n, 40 + n, 40]
            } else {
                [30, 60 + n, 190 + n]
            };
            px[i..i + 3].copy_from_slice(&c);
            px[i + 3] = 255;
        }
    }
    px
}

fn spec(seeds: &[(usize, usize)]) -> Spec {
    Spec {
        seeds: seeds.to_vec(),
        include: vec![],
        exclude: vec![],
        tolerance: 40.0,
        fill_holes: true,
    }
}

#[test]
fn seeded_region_matches_the_disc_and_fills_the_eye() {
    let px = disc_image(80, 80);
    let img = Rgba::new(80, 80, &px).unwrap();
    let mask = segment(&img, &spec(&[(40, 40)])).unwrap();
    let s = mask.stats();
    let truth = std::f64::consts::PI * 400.0;
    assert!(
        (s.area as f64 - truth).abs() / truth < 0.04,
        "area {}",
        s.area
    );
    assert_eq!(s.bbox[0] <= 21 && s.bbox[2] >= 59, true, "{:?}", s.bbox);
    assert!(mask.bits[36 * 80 + 46], "eye hole is filled");
    assert!(!mask.bits[2 * 80 + 2]);
    assert_eq!(mask, segment(&img, &spec(&[(40, 40)])).unwrap());
}

#[test]
fn exclude_and_include_polygons_constrain_the_mask() {
    let px = disc_image(80, 80);
    let img = Rgba::new(80, 80, &px).unwrap();
    let mut s = spec(&[(30, 40)]);
    s.exclude = vec![vec![(41.0, 0.0), (80.0, 0.0), (80.0, 80.0), (41.0, 80.0)]];
    let right_cut = segment(&img, &s).unwrap();
    assert!(right_cut.bits[40 * 80 + 38] && !right_cut.bits[40 * 80 + 50]);
    let mut s = spec(&[(30, 40)]);
    s.include = vec![vec![(0.0, 0.0), (35.0, 0.0), (35.0, 80.0), (0.0, 80.0)]];
    let left_only = segment(&img, &s).unwrap();
    assert!(left_only.bits[40 * 80 + 34] && !left_only.bits[40 * 80 + 36]);
}

#[test]
fn bad_regions_are_refused() {
    let px = disc_image(20, 20);
    let img = Rgba::new(20, 20, &px).unwrap();
    assert_eq!(
        segment(&img, &spec(&[])).unwrap_err(),
        TraceError::BadRegion
    );
    assert_eq!(
        segment(&img, &spec(&[(20, 0)])).unwrap_err(),
        TraceError::BadRegion
    );
    let mut s = spec(&[(1, 1)]);
    s.exclude = vec![vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)]];
    assert_eq!(segment(&img, &s).unwrap_err(), TraceError::BadRegion);
    let mut s = spec(&[(1, 1)]);
    s.include = vec![vec![(0.0, f64::NAN), (5.0, 0.0), (5.0, 5.0)]];
    assert_eq!(segment(&img, &s).unwrap_err(), TraceError::BadRegion);
    assert_eq!(
        rasterize(
            &[vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]],
            8,
            8
        )
        .iter()
        .filter(|&&b| b)
        .count(),
        16
    );
}

#[test]
fn inpaint_rebuilds_a_gradient_hole() {
    let (w, h) = (64, 40);
    let mut px = vec![255u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            px[i] = (x * 4) as u8;
            px[i + 1] = (y * 6) as u8;
            px[i + 2] = 100;
        }
    }
    let mut mask = vec![false; w * h];
    for y in 12..28 {
        for x in 24..40 {
            mask[y * w + x] = true;
        }
    }
    let mut holed = px.clone();
    for (i, m) in mask.iter().enumerate() {
        if *m {
            holed[i * 4..i * 4 + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
    }
    let out = inpaint(&Rgba::new(w, h, &holed).unwrap(), &mask).unwrap();
    let (mut err, mut n) = (0u64, 0u64);
    for (i, m) in mask.iter().enumerate() {
        if *m {
            for c in 0..3 {
                err += u64::from(out[i * 4 + c].abs_diff(px[i * 4 + c]));
                n += 1;
            }
        } else {
            assert_eq!(out[i * 4..i * 4 + 4], holed[i * 4..i * 4 + 4]);
        }
    }
    assert!(
        (err as f64 / n as f64) < 12.0,
        "mean err {}",
        err as f64 / n as f64
    );
    assert_eq!(
        out,
        inpaint(&Rgba::new(w, h, &holed).unwrap(), &mask).unwrap()
    );
    assert_eq!(
        inpaint(&Rgba::new(w, h, &holed).unwrap(), &vec![true; w * h]).unwrap_err(),
        TraceError::BadRegion
    );
}

#[test]
fn inpaint_rejects_oversized_or_nearly_full_masks() {
    // Over 4 million masked pixels.
    let (w, h) = (2100, 2100);
    let px = vec![200u8; w * h * 4];
    let img = Rgba::new(w, h, &px).unwrap();
    let mut mask = vec![true; w * h];
    mask[0] = false;
    assert_eq!(inpaint(&img, &mask), Err(TraceError::MaskTooLarge));
    // Under 1% of the image left known.
    let (w, h) = (100, 100);
    let px = vec![200u8; w * h * 4];
    let img = Rgba::new(w, h, &px).unwrap();
    let mut mask = vec![true; w * h];
    mask[0] = false;
    assert_eq!(inpaint(&img, &mask), Err(TraceError::MaskTooLarge));
    // A modest hole still works.
    mask = vec![false; w * h];
    mask[5050] = true;
    assert!(inpaint(&img, &mask).is_ok());
}
