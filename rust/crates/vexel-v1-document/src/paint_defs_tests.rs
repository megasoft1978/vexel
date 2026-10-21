use super::*;

const HEAD: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"100\" viewBox=\"0 0 200 100\"";

fn text(doc: &V1Document) -> String {
    String::from_utf8(doc.source().unwrap()).unwrap()
}
fn with(body: &str) -> V1Document {
    V1Document::from_source(format!("{HEAD}>{body}</svg>").as_bytes()).unwrap()
}
fn stop(offset: f64, color: &str, opacity: f64) -> GradientStop {
    GradientStop {
        offset,
        color: color.to_owned(),
        opacity,
    }
}
fn two() -> Vec<GradientStop> {
    vec![stop(0.0, "#ff0000", 1.0), stop(1.0, "#0000ff", 0.5)]
}
fn linear(doc: &mut V1Document) -> Result<String, V1DocumentError> {
    doc.add_linear_gradient(
        0.0,
        0.0,
        1.0,
        0.0,
        GradientUnits::ObjectBoundingBox,
        Spread::Pad,
        &two(),
    )
}

fn assert_atomic<T: std::fmt::Debug>(
    doc: &mut V1Document,
    expected: V1DocumentError,
    op: impl FnOnce(&mut V1Document) -> Result<T, V1DocumentError>,
) {
    let before = doc.source().unwrap();
    let (count, cursor) = (doc.history_count(), doc.history_cursor());
    assert_eq!(op(doc).unwrap_err(), expected);
    assert_eq!(doc.source().unwrap(), before);
    assert_eq!((doc.history_count(), doc.history_cursor()), (count, cursor));
}

/// One edit, one history entry; undo/redo restore exact bytes.
fn one_entry<T>(
    doc: &mut V1Document,
    op: impl FnOnce(&mut V1Document) -> Result<T, V1DocumentError>,
) -> (T, String) {
    let before = doc.source().unwrap();
    let count = doc.history_count();
    let out = op(doc).unwrap();
    assert_eq!(doc.history_count(), count + 1);
    let after = doc.source().unwrap();
    assert_ne!(after, before);
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
    (out, String::from_utf8(after).unwrap())
}

#[test]
fn linear_gradient_creates_defs_in_an_empty_document() {
    let mut doc = V1Document::new(200, 100).unwrap();
    let (id, got) = one_entry(&mut doc, linear);
    assert_eq!(id, "vexel-gradient-1");
    assert_eq!(
        got,
        format!(
            "{HEAD}><defs><linearGradient id=\"vexel-gradient-1\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\" gradientUnits=\"objectBoundingBox\" spreadMethod=\"pad\"><stop id=\"vexel-gradient-1-stop-0\" offset=\"0\" stop-color=\"#ff0000\" stop-opacity=\"1\"/><stop id=\"vexel-gradient-1-stop-1\" offset=\"1\" stop-color=\"#0000ff\" stop-opacity=\"0.5\"/></linearGradient></defs></svg>"
        )
    );
}

#[test]
fn defs_is_inserted_first_and_unrelated_bytes_are_kept() {
    let body = "\n  <rect id=\"r\" x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#00ff00\"/>\n";
    let mut doc = with(body);
    let (_, got) = one_entry(&mut doc, linear);
    assert!(got.starts_with(&format!("{HEAD}><defs><linearGradient")));
    assert!(got.ends_with(&format!("</defs>{body}</svg>")));
}

#[test]
fn existing_top_level_defs_is_reused() {
    let mut doc =
        with("<defs><clipPath id=\"keep\"/></defs><rect id=\"r\" width=\"5\" height=\"5\"/>");
    let (_, got) = one_entry(&mut doc, linear);
    assert_eq!(got.matches("<defs").count(), 1);
    assert!(got.contains("<clipPath id=\"keep\"/><linearGradient id=\"vexel-gradient-1\""));
    // Empty `<defs/>` is also reused.
    let mut doc = with("<defs/>");
    let (_, got) = one_entry(&mut doc, linear);
    assert_eq!(got.matches("<defs").count(), 1);
    assert!(got.contains("<defs><linearGradient"));
}

#[test]
fn gradient_ids_are_unique_and_increasing() {
    let mut doc = V1Document::new(200, 100).unwrap();
    assert_eq!(linear(&mut doc).unwrap(), "vexel-gradient-1");
    assert_eq!(linear(&mut doc).unwrap(), "vexel-gradient-2");
    doc.undo().unwrap();
    assert_eq!(linear(&mut doc).unwrap(), "vexel-gradient-2");
    let mut doc = with("<defs><linearGradient id=\"vexel-gradient-7\"/></defs>");
    assert_eq!(linear(&mut doc).unwrap(), "vexel-gradient-8");
    let mut doc = with("<rect id=\"vexel-gradient-1\" width=\"4\" height=\"4\"/>");
    assert_eq!(linear(&mut doc).unwrap(), "vexel-gradient-2");
}

#[test]
fn radial_gradient_writes_exact_source_and_round_trips() {
    let mut doc = V1Document::new(200, 100).unwrap();
    let (id, got) = one_entry(&mut doc, |d| {
        d.add_radial_gradient(
            0.5,
            0.5,
            0.5,
            0.25,
            0.75,
            GradientUnits::UserSpace,
            Spread::Reflect,
            &two(),
        )
    });
    assert_eq!(id, "vexel-gradient-1");
    assert!(got.contains(
        "<radialGradient id=\"vexel-gradient-1\" cx=\"0.5\" cy=\"0.5\" r=\"0.5\" fx=\"0.25\" fy=\"0.75\" gradientUnits=\"userSpaceOnUse\" spreadMethod=\"reflect\">"
    ));
    assert!(got.contains("</radialGradient></defs></svg>"));
    V1Document::from_source(got.as_bytes()).unwrap();
}

#[test]
fn gradient_validation_errors_are_atomic() {
    let mut doc = with("<rect id=\"r\" width=\"5\" height=\"5\"/>");
    let add = |stops: Vec<GradientStop>| {
        move |d: &mut V1Document| {
            d.add_linear_gradient(
                0.0,
                0.0,
                1.0,
                0.0,
                GradientUnits::UserSpace,
                Spread::Pad,
                &stops,
            )
        }
    };
    let bad = V1DocumentError::InvalidGeometry;
    assert_atomic(&mut doc, bad, add(vec![stop(0.0, "#000000", 1.0)]));
    assert_atomic(&mut doc, bad, add(Vec::new()));
    let many = (0..17).map(|_| stop(0.5, "#000000", 1.0)).collect();
    assert_atomic(&mut doc, bad, add(many));
    assert_atomic(
        &mut doc,
        bad,
        add(vec![stop(0.6, "#000000", 1.0), stop(0.4, "#000000", 1.0)]),
    );
    assert_atomic(
        &mut doc,
        bad,
        add(vec![stop(-0.1, "#000000", 1.0), stop(1.0, "#000000", 1.0)]),
    );
    assert_atomic(
        &mut doc,
        bad,
        add(vec![stop(0.0, "#000000", 1.5), stop(1.0, "#000000", 1.0)]),
    );
    assert_atomic(
        &mut doc,
        bad,
        add(vec![
            stop(f64::NAN, "#000000", 1.0),
            stop(1.0, "#000000", 1.0),
        ]),
    );
    assert_atomic(
        &mut doc,
        V1DocumentError::InvalidColor,
        add(vec![stop(0.0, "red", 1.0), stop(1.0, "#000000", 1.0)]),
    );
    assert_atomic(&mut doc, bad, |d| {
        d.add_linear_gradient(
            f64::INFINITY,
            0.0,
            1.0,
            0.0,
            GradientUnits::UserSpace,
            Spread::Pad,
            &two(),
        )
    });
    assert_atomic(&mut doc, bad, |d| {
        d.add_linear_gradient(
            2e6,
            0.0,
            1.0,
            0.0,
            GradientUnits::UserSpace,
            Spread::Pad,
            &two(),
        )
    });
    for r in [0.0, -1.0, f64::NAN] {
        assert_atomic(&mut doc, bad, |d| {
            d.add_radial_gradient(
                0.5,
                0.5,
                r,
                0.5,
                0.5,
                GradientUnits::UserSpace,
                Spread::Pad,
                &two(),
            )
        });
    }
    assert!(!doc.is_dirty());
}

#[test]
fn set_gradient_stop_edits_one_stop_in_one_entry() {
    let mut doc = V1Document::new(200, 100).unwrap();
    let id = linear(&mut doc).unwrap();
    let before = text(&doc);
    let ((), got) = one_entry(&mut doc, |d| {
        d.set_gradient_stop(&id, 1, Some(0.75), Some("#00ff00"), Some(0.25))
    });
    assert!(got.contains(
        "<stop id=\"vexel-gradient-1-stop-1\" offset=\"0.75\" stop-color=\"#00ff00\" stop-opacity=\"0.25\"/>"
    ));
    assert!(got.contains("<stop id=\"vexel-gradient-1-stop-0\" offset=\"0\" stop-color=\"#ff0000\" stop-opacity=\"1\"/>"));
    assert_ne!(before, got);
    // Partial edit touches only the named attribute.
    let ((), got) = one_entry(&mut doc, |d| {
        d.set_gradient_stop(&id, 0, None, None, Some(0.0))
    });
    assert!(got.contains("stop-color=\"#ff0000\" stop-opacity=\"0\"/>"));
}

#[test]
fn set_gradient_stop_adds_missing_attributes_and_ignores_empty_edit() {
    let mut doc = with(
        "<defs><linearGradient id=\"g\"><stop offset=\"0\"/><stop offset=\"1\"/></linearGradient></defs>",
    );
    let ((), got) = one_entry(&mut doc, |d| {
        d.set_gradient_stop("g", 0, None, Some("#123456"), Some(0.5))
    });
    assert!(got.contains("<stop offset=\"0\" stop-color=\"#123456\" stop-opacity=\"0.5\"/>"));
    let count = doc.history_count();
    doc.set_gradient_stop("g", 1, None, None, None).unwrap();
    assert_eq!(doc.history_count(), count);
}

#[test]
fn set_gradient_stop_errors_are_atomic() {
    let mut doc = V1Document::new(200, 100).unwrap();
    let three = vec![
        stop(0.0, "#000000", 1.0),
        stop(0.5, "#777777", 1.0),
        stop(1.0, "#ffffff", 1.0),
    ];
    let id = doc
        .add_linear_gradient(
            0.0,
            0.0,
            1.0,
            0.0,
            GradientUnits::UserSpace,
            Spread::Pad,
            &three,
        )
        .unwrap();
    let bad = V1DocumentError::InvalidGeometry;
    assert_atomic(&mut doc, bad, |d| {
        d.set_gradient_stop(&id, 0, Some(0.6), None, None)
    });
    assert_atomic(&mut doc, bad, |d| {
        d.set_gradient_stop(&id, 2, Some(0.4), None, None)
    });
    assert_atomic(&mut doc, bad, |d| {
        d.set_gradient_stop(&id, 1, Some(-0.1), None, None)
    });
    assert_atomic(&mut doc, bad, |d| {
        d.set_gradient_stop(&id, 0, None, None, Some(2.0))
    });
    assert_atomic(&mut doc, bad, |d| {
        d.set_gradient_stop(&id, 2, Some(f64::NAN), None, None)
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidColor, |d| {
        d.set_gradient_stop(&id, 0, None, Some("#12"), None)
    });
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_gradient_stop(&id, 3, Some(1.0), None, None)
    });
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_gradient_stop("nope", 0, Some(0.0), None, None)
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidStableId, |d| {
        d.set_gradient_stop("bad id", 0, Some(0.0), None, None)
    });
}

#[test]
fn fill_and_stroke_refs_set_url_and_round_trip() {
    let mut doc =
        with("<rect id=\"r\" x=\"0\" y=\"0\" width=\"9\" height=\"9\" fill=\"#00ff00\"/>");
    let id = linear(&mut doc).unwrap();
    let ((), got) = one_entry(&mut doc, |d| d.set_fill_ref("r", &id));
    assert!(got.contains("fill=\"url(#vexel-gradient-1)\"/>"));
    let ((), got) = one_entry(&mut doc, |d| d.set_stroke_ref("r", &id));
    assert!(got.contains("fill=\"url(#vexel-gradient-1)\" stroke=\"url(#vexel-gradient-1)\"/>"));
}

#[test]
fn ref_errors_are_atomic() {
    let mut doc = with(
        "<rect id=\"r\" width=\"9\" height=\"9\"/><g id=\"grp\"/><text id=\"t\">x</text><defs><clipPath id=\"c\"/></defs>",
    );
    let id = linear(&mut doc).unwrap();
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_fill_ref("r", "missing")
    });
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_fill_ref("missing", &id)
    });
    assert_atomic(&mut doc, V1DocumentError::Unsupported, |d| {
        d.set_fill_ref("grp", &id)
    });
    assert_atomic(&mut doc, V1DocumentError::Unsupported, |d| {
        d.set_stroke_ref("t", &id)
    });
    // A clipPath is not a gradient.
    assert_atomic(&mut doc, V1DocumentError::Unsupported, |d| {
        d.set_fill_ref("r", "c")
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidStableId, |d| {
        d.set_fill_ref("r", "x y")
    });
}

#[test]
fn gradients_outside_defs_are_not_references() {
    let mut doc = with("<rect id=\"r\" width=\"9\" height=\"9\"/><linearGradient id=\"loose\"/>");
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_fill_ref("r", "loose")
    });
}

#[test]
fn set_fill_and_stroke_replace_url_paint() {
    let mut doc = with("<rect id=\"r\" width=\"9\" height=\"9\"/>");
    let id = linear(&mut doc).unwrap();
    doc.set_fill_ref("r", &id).unwrap();
    doc.set_stroke_ref("r", &id).unwrap();
    doc.set_fill("r", "#112233").unwrap();
    doc.set_stroke("r", "#445566", 2.0).unwrap();
    let got = text(&doc);
    assert!(got.contains("fill=\"#112233\" stroke=\"#445566\" stroke-width=\"2e0\""));
    assert!(!got.contains("url(#"));
}

#[test]
fn clip_rect_and_ellipse_write_exact_source() {
    let mut doc = V1Document::new(200, 100).unwrap();
    let (id, got) = one_entry(&mut doc, |d| d.add_clip_rect(1.0, 2.0, 30.0, 40.0));
    assert_eq!(id, "vexel-clip-1");
    assert!(got.contains(
        "<defs><clipPath id=\"vexel-clip-1\"><rect x=\"1\" y=\"2\" width=\"30\" height=\"40\"/></clipPath></defs>"
    ));
    let (id, got) = one_entry(&mut doc, |d| d.add_clip_ellipse(10.0, 20.0, 5.0, 6.0));
    assert_eq!(id, "vexel-clip-2");
    assert!(got.contains(
        "<clipPath id=\"vexel-clip-2\"><ellipse cx=\"10\" cy=\"20\" rx=\"5\" ry=\"6\"/></clipPath></defs>"
    ));
}

#[test]
fn clip_validation_errors_are_atomic() {
    let mut doc = V1Document::new(200, 100).unwrap();
    let bad = V1DocumentError::InvalidGeometry;
    for (w, h) in [
        (0.0, 5.0),
        (5.0, -1.0),
        (f64::NAN, 5.0),
        (5.0, f64::INFINITY),
    ] {
        assert_atomic(&mut doc, bad, |d| d.add_clip_rect(0.0, 0.0, w, h));
    }
    assert_atomic(&mut doc, bad, |d| d.add_clip_rect(f64::NAN, 0.0, 5.0, 5.0));
    assert_atomic(&mut doc, bad, |d| d.add_clip_rect(0.0, 9e6, 5.0, 5.0));
    assert_atomic(&mut doc, bad, |d| d.add_clip_ellipse(0.0, 0.0, 0.0, 5.0));
    assert_atomic(&mut doc, bad, |d| d.add_clip_ellipse(0.0, 0.0, 5.0, -5.0));
    assert_atomic(&mut doc, bad, |d| {
        d.add_clip_ellipse(f64::NAN, 0.0, 5.0, 5.0)
    });
}

#[test]
fn set_clip_sets_and_removes_clip_path() {
    let mut doc = with(
        "<rect id=\"r\" x=\"0\" y=\"0\" width=\"9\" height=\"9\" fill=\"#00ff00\"/><g id=\"grp\"/>",
    );
    let clip = doc.add_clip_rect(0.0, 0.0, 4.0, 4.0).unwrap();
    let ((), got) = one_entry(&mut doc, |d| d.set_clip("r", Some(&clip)));
    assert!(got.contains("fill=\"#00ff00\" clip-path=\"url(#vexel-clip-1)\"/>"));
    // Replace with another clip.
    let other = doc.add_clip_ellipse(2.0, 2.0, 2.0, 2.0).unwrap();
    let ((), got) = one_entry(&mut doc, |d| d.set_clip("r", Some(&other)));
    assert!(got.contains("clip-path=\"url(#vexel-clip-2)\""));
    assert!(!got.contains("clip-path=\"url(#vexel-clip-1)\""));
    // Remove restores the original tag bytes.
    let ((), got) = one_entry(&mut doc, |d| d.set_clip("r", None));
    assert!(
        got.contains("<rect id=\"r\" x=\"0\" y=\"0\" width=\"9\" height=\"9\" fill=\"#00ff00\"/>")
    );
    // Groups can be clipped.
    let ((), got) = one_entry(&mut doc, |d| d.set_clip("grp", Some(&clip)));
    assert!(got.contains("<g id=\"grp\" clip-path=\"url(#vexel-clip-1)\"/>"));
}

#[test]
fn set_clip_removal_of_absent_attribute_is_a_noop_and_errors_are_atomic() {
    let mut doc = with("<rect id=\"r\" width=\"9\" height=\"9\"/><text id=\"t\">x</text>");
    let clip = doc.add_clip_rect(0.0, 0.0, 4.0, 4.0).unwrap();
    let count = doc.history_count();
    doc.set_clip("r", None).unwrap();
    assert_eq!(doc.history_count(), count);
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_clip("r", Some("missing"))
    });
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_clip("missing", Some(&clip))
    });
    assert_atomic(&mut doc, V1DocumentError::Unsupported, |d| {
        d.set_clip("t", Some(&clip))
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidStableId, |d| {
        d.set_clip("r", Some("a b"))
    });
    // A gradient is not a clip path.
    let gradient = linear(&mut doc).unwrap();
    assert_atomic(&mut doc, V1DocumentError::Unsupported, |d| {
        d.set_clip("r", Some(&gradient))
    });
}

#[test]
fn prefixed_root_and_attributeless_root_get_defs() {
    let mut doc = V1Document::from_source(
        b"<svg width=\"10\" height=\"10\" xmlns=\"http://www.w3.org/2000/svg\" />",
    )
    .unwrap();
    let (_, got) = one_entry(&mut doc, linear);
    assert!(got.starts_with(
        "<svg width=\"10\" height=\"10\" xmlns=\"http://www.w3.org/2000/svg\" ><defs>"
    ));
    assert!(got.ends_with("</defs></svg>"));
}
