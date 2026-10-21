use super::*;
#[test]
fn raster_bounds_mapping_and_pixel_change() {
    let p = rasterize(&[Shape::Rect {
        x: 10.,
        y: 20.,
        width: 30.,
        height: 40.,
        rgb: 0x123456,
    }])
    .unwrap();
    assert_eq!(
        &p[(25 * 512 + 20) * 4..(25 * 512 + 20) * 4 + 4],
        &[0x12, 0x34, 0x56, 255]
    );
    assert_eq!(&p[..4], &[255, 255, 255, 255]);
    let e = rasterize(&[Shape::Ellipse {
        cx: 100.,
        cy: 100.,
        rx: 20.,
        ry: 10.,
        rgb: 0xff0000,
    }])
    .unwrap();
    assert_eq!(
        &e[(100 * 512 + 100) * 4..(100 * 512 + 100) * 4 + 3],
        &[255, 0, 0]
    );
}
#[test]
fn invalid_operation_does_not_change_source_or_history() {
    let s = PreviewState::new().unwrap();
    let source = s.source().unwrap();
    assert!(
        s.stage(&Operation::Add(Shape::Rect {
            x: 510.,
            y: 1.,
            width: 5.,
            height: 2.,
            rgb: 0
        }))
        .is_err()
    );
    assert_eq!(s.source().unwrap(), source);
    assert!(!s.is_dirty());
    let edited = s
        .stage(&Operation::Add(Shape::Rect {
            x: 2.,
            y: 2.,
            width: 2.,
            height: 2.,
            rgb: 0,
        }))
        .unwrap()
        .0;
    let (restored, _) = edited.stage(&Operation::Undo).unwrap();
    assert_eq!(restored.source().unwrap(), source);
    assert!(!restored.is_dirty());
}
#[test]
fn undo_redo_restore_exact_source_and_branch_order() {
    let blank = PreviewState::new().unwrap();
    let first = Operation::Add(Shape::Rect {
        x: 1.,
        y: 1.,
        width: 2.,
        height: 2.,
        rgb: 1,
    });
    let (one, _) = blank.stage(&first).unwrap();
    let source = one.source().unwrap();
    let (two, _) = one
        .stage(&Operation::Add(Shape::Ellipse {
            cx: 8.,
            cy: 8.,
            rx: 2.,
            ry: 3.,
            rgb: 2,
        }))
        .unwrap();
    let (back, _) = two.stage(&Operation::Undo).unwrap();
    assert_eq!(back.shapes().len(), 1);
    assert_eq!(back.source().unwrap(), source);
    let (redo, _) = back.stage(&Operation::Redo).unwrap();
    assert_eq!(redo.source().unwrap(), two.source().unwrap());
    let (branched, _) = back.stage(&first).unwrap();
    assert!(branched.stage(&Operation::Redo).is_err());
}

#[test]
fn branching_after_undo_never_reuses_an_element_id() {
    let blank = PreviewState::new().unwrap();
    let first = Operation::Add(Shape::Rect {
        x: 1.,
        y: 1.,
        width: 2.,
        height: 2.,
        rgb: 1,
    });
    let (one, _) = blank.stage(&first).unwrap();
    assert_eq!(one.element_ids()[0], "vexel-object-1");
    let (two, _) = one.stage(&first).unwrap();
    assert_eq!(two.element_ids()[1], "vexel-object-2");
    let (back, _) = two.stage(&Operation::Undo).unwrap();
    assert_eq!(back.element_ids()[0], "vexel-object-1");
    let (branch, _) = back.stage(&first).unwrap();
    assert_eq!(branch.element_ids()[1], "vexel-object-3");
    let source = branch.source().unwrap();
    assert!(
        source
            .windows(b"vexel-object-3".len())
            .any(|w| w == b"vexel-object-3")
    );
    assert!(
        !source
            .windows(b"vexel-object-2".len())
            .any(|w| w == b"vexel-object-2")
    );
}
#[test]
fn rejects_invalid_shapes_and_caps_scene() {
    for s in [
        Shape::Rect {
            x: f64::NAN,
            y: 0.,
            width: 1.,
            height: 1.,
            rgb: 0,
        },
        Shape::Rect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 1.,
            rgb: 0,
        },
        Shape::Ellipse {
            cx: 511.,
            cy: 2.,
            rx: 2.,
            ry: 1.,
            rgb: 0,
        },
    ] {
        assert!(validate_shape(s).is_err())
    }
    let mut state = PreviewState::new().unwrap();
    for _ in 0..SCENE_LIMIT {
        state = state
            .stage(&Operation::Add(Shape::Rect {
                x: 1.,
                y: 1.,
                width: 1.,
                height: 1.,
                rgb: 0,
            }))
            .unwrap()
            .0;
    }
    assert!(
        state
            .stage(&Operation::Add(Shape::Rect {
                x: 2.,
                y: 2.,
                width: 1.,
                height: 1.,
                rgb: 0
            }))
            .is_err()
    );
}

#[test]
fn request_mapping_rejects_non_finite_color_and_extra_attributes() {
    fn parse(attributes: &str) -> Result<Operation, PreviewArgumentError> {
        let json = format!(
            "{{\"documentId\":\"{DOCUMENT_ID}\",\"parentId\":\"root\",\"template\":\"rect\",\"attributes\":{attributes}}}"
        );
        let doc = JsonDocument::parse_owned(json.as_bytes()).unwrap();
        parse_operation(&doc, doc.root(), "element_add")
    }
    assert!(parse(r##"{"x":"1e999","y":"1","width":"2","height":"3","fill":"#AABBCC"}"##).is_err());
    assert!(parse(r#"{"x":"1","y":"1","width":"2","height":"3","fill":"red"}"#).is_err());
    assert!(
        parse(r##"{"x":"1","y":"1","width":"2","height":"3","fill":"#AABBCC","opacity":"0.5"}"##)
            .is_err()
    );
}

#[test]
fn element_add_rejects_unknown_attribute_with_actionable_error() {
    let json = format!(
        r##"{{"documentId":"{DOCUMENT_ID}","parentId":"root","template":"rect","attributes":{{"x":"1","y":"2","width":"3","height":"4","fill":"#AABBCC","rx":"2"}}}}"##
    );
    let doc = JsonDocument::parse_owned(json.as_bytes()).unwrap();
    let error = parse_operation(&doc, doc.root(), "element_add").unwrap_err();
    assert_eq!(
        error,
        PreviewArgumentError::Actionable(
            "element_add rect does not accept attribute \"rx\"; allowed: x, y, width, height, fill"
                .to_owned()
        )
    );
}

#[test]
fn rejected_attribute_and_parent_echo_is_sanitized_and_error_json_is_valid() {
    let json = format!(
        r##"{{"documentId":"{DOCUMENT_ID}","parentId":"root","template":"rect","attributes":{{"bad\n\"key\u0001":"1"}}}}"##
    );
    let doc = JsonDocument::parse_owned(json.as_bytes()).unwrap();
    let PreviewArgumentError::Actionable(text) =
        parse_operation(&doc, doc.root(), "element_add").unwrap_err()
    else {
        panic!("expected an actionable error");
    };
    assert!(text.contains("bad??key?"), "{text}");
    assert!(!text.chars().any(|c| c.is_control()), "{text}");
    // An over-long parent id is refused without being echoed.
    let long_parent = "p".repeat(200);
    let json = format!(
        r##"{{"documentId":"{DOCUMENT_ID}","parentId":"{long_parent}","template":"rect","attributes":{{"x":"1","y":"2","width":"3","height":"4","fill":"#AABBCC"}}}}"##
    );
    let doc = JsonDocument::parse_owned(json.as_bytes()).unwrap();
    let PreviewArgumentError::Actionable(text) =
        parse_operation(&doc, doc.root(), "element_add").unwrap_err()
    else {
        panic!("expected an actionable error");
    };
    assert!(
        text.contains("parentId") && !text.contains(&"p".repeat(33)),
        "{text}"
    );
}

fn one_rect() -> PreviewState {
    PreviewState::new()
        .unwrap()
        .stage(&Operation::Add(Shape::Rect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
            rgb: 0xff0000,
        }))
        .unwrap()
        .0
}

#[test]
fn generated_ids_survive_rebuild_and_animation_round_trip() {
    let state = one_rect();
    let source = state.source().unwrap();
    assert!(
        source
            .windows(b"id=\"vexel-object-1\"".len())
            .any(|w| w == b"id=\"vexel-object-1\"")
    );
    let (animated, _) = state.stage(&Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: br#"<animate attributeName="x" values="10;30" dur="1s" calcMode="linear" fill="freeze"/>"#.to_vec(),
        }).unwrap();
    assert!(
        animated
            .source()
            .unwrap()
            .windows(b"<animate".len())
            .any(|w| w == b"<animate")
    );
    let (back, _) = animated.stage(&Operation::Undo).unwrap();
    assert_eq!(back.source().unwrap(), source);
    let (forward, _) = back.stage(&Operation::Redo).unwrap();
    assert_eq!(forward.source().unwrap(), animated.source().unwrap());
}

#[test]
fn owned_animation_parser_evaluator_moves_the_raster_at_distinct_times() {
    let (state, _) = one_rect().stage(&Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: br#"<animate attributeName="x" values="10;30" dur="1s" calcMode="linear" fill="freeze"/>"#.to_vec(),
        }).unwrap();
    let early = rasterize_at(&state, 0.25).unwrap();
    let late = rasterize_at(&state, 0.75).unwrap();
    assert_ne!(early, late);
    assert_eq!(
        &early[(20 * 512 + 15) * 4..(20 * 512 + 15) * 4 + 3],
        &[255, 0, 0]
    );
    assert_eq!(
        &late[(20 * 512 + 35) * 4..(20 * 512 + 35) * 4 + 3],
        &[255, 0, 0]
    );
}

#[test]
fn bounded_values_and_opacity_composite_over_white() {
    let state = one_rect();
    let invalid = Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: br#"<animate attributeName="opacity" values="0;1;0;1;0;1;0;1;0;1;0;1;0;1;0;1;0" dur="1s"/>"#.to_vec(),
        };
    assert!(state.stage(&invalid).is_err());
    let (state, _) = state.stage(&Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: br#"<animate attributeName="opacity" values="0.5;0.5" dur="1s" calcMode="linear" fill="freeze"/>"#.to_vec(),
        }).unwrap();
    let raster = rasterize_at(&state, 0.5).unwrap();
    assert_eq!(
        &raster[(20 * 512 + 20) * 4..(20 * 512 + 20) * 4 + 3],
        &[255, 128, 128]
    );
    assert!(
        state
            .stage(&Operation::AnimationAdd {
                target_id: "missing".into(),
                xml: br#"<animate attributeName="x" values="1;2" dur="1s"/>"#.to_vec()
            })
            .is_err()
    );
}

#[test]
fn animation_duration_and_track_count_limits_are_atomic() {
    let state = one_rect();
    for duration in ["99ms", "10.001s"] {
        let bad = Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: format!("<animate attributeName=\"x\" values=\"10;20\" dur=\"{duration}\"/>")
                .into_bytes(),
        };
        assert!(state.stage(&bad).is_err());
    }
    let (mut state, _) = state
        .stage(&Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: br#"<animate attributeName="x" values="10;20" dur="100ms" calcMode="linear"/>"#
                .to_vec(),
        })
        .unwrap();
    for _ in 1..ANIMATION_LIMIT {
        state = state
            .stage(&Operation::AnimationAdd {
                target_id: "vexel-object-1".to_owned(),
                xml:
                    br#"<animate attributeName="y" values="10;20" dur="100ms" calcMode="linear"/>"#
                        .to_vec(),
            })
            .unwrap()
            .0;
    }
    let source = state.source().unwrap();
    assert!(
        state
            .stage(&Operation::AnimationAdd {
                target_id: "vexel-object-1".to_owned(),
                xml:
                    br#"<animate attributeName="x" values="10;20" dur="100ms" calcMode="linear"/>"#
                        .to_vec(),
            })
            .is_err()
    );
    assert_eq!(state.source().unwrap(), source);
    let listed = state.read_result(&Operation::AnimationList).unwrap();
    assert!(listed.contains("\"targetId\":\"vexel-object-1\""));
    assert_eq!(listed.matches("\"attributeName\"").count(), ANIMATION_LIMIT);
}

#[test]
fn playback_stage_updates_state_and_seek_is_bounded() {
    let (state, _) = one_rect()
        .stage(&Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: br#"<animate attributeName="x" values="10;20" dur="2s" calcMode="linear"/>"#
                .to_vec(),
        })
        .unwrap();
    let (playing, _) = state.stage(&Operation::Control(Control::Play)).unwrap();
    assert_eq!(playing.playback(), (true, 0.0, 2.0));
    let (seeked, _) = playing
        .stage(&Operation::Control(Control::Seek(1.25)))
        .unwrap();
    assert_eq!(seeked.playback(), (true, 1.25, 2.0));
    let (paused, _) = seeked.stage(&Operation::Control(Control::Pause)).unwrap();
    assert_eq!(paused.playback(), (false, 1.25, 2.0));
    assert!(
        paused
            .stage(&Operation::Control(Control::Seek(10.01)))
            .is_err()
    );
}

#[test]
fn animation_tool_mapping_uses_milliseconds_and_bounded_fragments() {
    let json = br#"{"documentId":"preview-document","action":"seek","timeMs":1250}"#;
    let doc = JsonDocument::parse_owned(json).unwrap();
    assert_eq!(
        parse_operation(&doc, doc.root(), "animation_control").unwrap(),
        Operation::Control(Control::Seek(1.25))
    );
    let bad_seek = br#"{"documentId":"preview-document","action":"seek","timeMs":10001}"#;
    let doc = JsonDocument::parse_owned(bad_seek).unwrap();
    assert!(parse_operation(&doc, doc.root(), "animation_control").is_err());
    let xml = br#"{"documentId":"preview-document","targetId":"vexel-object-1","xml":"<animate attributeName=\"x\" values=\"10;20\" dur=\"1s\"/>"}"#;
    let doc = JsonDocument::parse_owned(xml).unwrap();
    assert!(matches!(
        parse_operation(&doc, doc.root(), "animation_add"),
        Ok(Operation::AnimationAdd { .. })
    ));
}

#[test]
fn export_animation_mapping_is_gif_only_with_bounded_fps_and_duration() {
    let ok = br#"{"documentId":"preview-document","path":"/tmp/x.gif","format":"gif","fps":10,"duration":1.5}"#;
    let doc = JsonDocument::parse_owned(ok).unwrap();
    assert_eq!(
        parse_operation(&doc, doc.root(), "export_animation").unwrap(),
        Operation::Export {
            path: "/tmp/x.gif".to_owned(),
            fps: 10,
            duration: 1.5
        }
    );
    for bad in [
        r#"{"documentId":"preview-document","path":"/tmp/x.gif","format":"frames","fps":10,"duration":1}"#,
        r#"{"documentId":"preview-document","path":"/tmp/x.gif","format":"gif","fps":31,"duration":1}"#,
        r#"{"documentId":"preview-document","path":"/tmp/x.gif","format":"gif","fps":2.5,"duration":1}"#,
        r#"{"documentId":"preview-document","path":"/tmp/x.gif","format":"gif","fps":10,"duration":10.5}"#,
        r#"{"documentId":"preview-document","path":"","format":"gif","fps":10,"duration":1}"#,
        r#"{"documentId":"preview-document","path":"/tmp/x.gif","format":"gif","fps":"10","duration":1}"#,
        r#"{"documentId":"other","path":"/tmp/x.gif","format":"gif","fps":10,"duration":1}"#,
    ] {
        let doc = JsonDocument::parse_owned(bad.as_bytes()).unwrap();
        assert!(
            parse_operation(&doc, doc.root(), "export_animation").is_err(),
            "{bad}"
        );
    }
}

#[test]
fn incremental_source_length_matches_the_built_document() {
    let mut state = PreviewState::new().unwrap();
    for n in 0..40u32 {
        let operation = if n % 3 == 0 {
            Operation::Add(Shape::Ellipse {
                cx: 100.5 + f64::from(n),
                cy: 50.25,
                rx: 7.0,
                ry: 3.125,
                rgb: 0x00_ff_00 + n,
            })
        } else {
            Operation::Add(Shape::Rect {
                x: f64::from(n) * 10.0,
                y: 1.0 / 3.0,
                width: 12.0,
                height: 9.0,
                rgb: 0xab_cd_ef,
            })
        };
        state = state.stage(&operation).unwrap().0;
        assert_eq!(state.source_len, state.source().unwrap().len(), "after {n}");
    }
    let state = state.stage(&Operation::Undo).unwrap().0;
    assert_eq!(state.source_len, state.source().unwrap().len());
}
