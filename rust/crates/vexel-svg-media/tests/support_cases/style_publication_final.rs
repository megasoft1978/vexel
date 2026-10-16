use super::*;

struct BadPublication {
    wrong_source: Option<SourceMap>,
}
impl StylePublisher for BadPublication {
    fn begin(
        &self,
        _: &PublicationContext<'_, '_>,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        Ok(())
    }
    fn prepare_geometry(
        &self,
        _: &PublicationContext<'_, '_>,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        Ok(())
    }
    fn prepare_admitted_geometry(
        &self,
        _: &PublicationContext<'_, '_>,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        Ok(())
    }
    fn publish(
        &self,
        _c: &PublicationContext<'_, '_>,
        g: &CustomGeometry,
        _b: &mut SceneBuilder,
        _d: &DraftToken,
        _control: &mut dyn WorkControl,
    ) -> Result<PublishedStyle> {
        if let Some(source) = self.wrong_source {
            let colors = vec![[255, 0, 0, 255]; g.path.edges.len()];
            let mut shape = Shape::new(source);
            shape.image = g.image;
            Ok(PublishedStyle {
                shape,
                edge_colors: colors,
            })
        } else {
            Ok(PublishedStyle {
                shape: Shape::new(g.source_map),
                edge_colors: vec![],
            })
        }
    }
    fn finish(
        &self,
        _: &PublicationContext<'_, '_>,
        _: usize,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        Ok(())
    }
}
#[test]
fn production_rejects_empty_success_prefix_handle_and_changed_host_without_caller_cleanup() {
    for wrong_source in [false, true] {
        let source=br##"<svg><path d="M0 0L1 0L1 1Z"/><image width="2" height="3" href="data:image/png;base64,AQID"/><use href="#image"/></svg>"##;
        let mut f = Fixture::new(source, Limits::default());
        let i = f.index(b"image");
        let map = f.map(f.index(b"use"));
        let wrong_map = f.map(i);
        let element = view(&f.lex, i);
        let scene = f.builder.root_id();
        let draft = f.builder.begin_draft(scene).unwrap();
        let prefix = f
            .builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
        let prefix_shape = *f.builder.scene(scene).unwrap().shape(prefix).unwrap();
        let p = BadPublication {
            wrong_source: wrong_source.then_some(wrong_map),
        };
        let mut provider = Provider::default();
        let style = CssPaint::initial();
        let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
        let e = publish_image_request(
            ImageDecodeRequest {
                element,
                source_map: map,
                style: &style,
                transform: vexel_geometry::affine_identity(),
                provider: &mut provider,
                scene,
                ledger: &mut ledger,
            },
            &p,
            &mut f.builder,
            &draft,
            &mut Run,
        )
        .unwrap_err();
        assert_eq!(e, error(ErrorKind::InvalidArgument, map.element.start));
        let s = f.builder.scene(scene).unwrap();
        assert_eq!(s.shapes.len(), 1);
        assert_eq!(s.shape(prefix), Some(&prefix_shape));
        assert!(s.images.is_empty());
        f.builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
        f.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

#[test]
fn production_delegate_receives_canonical_geometry_and_independent_use_host_annotation() {
    let source=br##"<svg><image id="image" x="2" y="3" width="8" height="4" style="fill:red;opacity:.5" href="data:image/png;base64,AQID"/><use href="#image"/></svg>"##;
    let mut f = Fixture::new(source, Limits::default());
    let i = f.index(b"image");
    let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
    let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
    let element = view(&f.lex, i);
    let mut map = f.map(f.index(b"use"));
    map.transform.e = 42.;
    let mut transform = vexel_geometry::affine_identity();
    transform.e = 7.;
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let mut provider = Provider::default();
    let p = Publisher::default();
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    let id = publish_image_request(
        ImageDecodeRequest {
            element,
            source_map: map,
            style: &style,
            transform,
            provider: &mut provider,
            scene,
            ledger: &mut ledger,
        },
        &p,
        &mut f.builder,
        &draft,
        &mut Run,
    )
    .unwrap()
    .unwrap();
    let s = f.builder.scene(scene).unwrap();
    assert_eq!(s.shape(id).unwrap().source, map);
    assert_eq!(s.edges[0].a.x, 9.);
    assert_eq!(s.edges[0].a.y, 3.);
    assert_eq!(s.edge_colors, [[255, 0, 0, 0]; 4]);
    assert_eq!(s.groups[0].opacity, 0.5);
    f.builder.commit_draft(&draft, &mut Run).unwrap();
}

#[test]
fn production_text_provider_unsupported_precedes_custom_fill_and_restores_admissions() {
    let source = br#"<svg><text style="fill:bogus">A</text></svg>"#;
    let mut f = Fixture::new(source, Limits::default());
    let i = f.index(b"text");
    let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
    let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
    let element = view(&f.lex, i);
    let map = f.map(i);
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let p = Publisher::default();
    let mut provider = Provider {
        fail_text: Some(MediaError::Unsupported),
        ..Default::default()
    };
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    let e = publish_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &style,
            transform: map.transform,
            state: &mut state,
            preserve_space: false,
            provider: &mut provider,
            cache: &mut cache,
            scene,
            ledger: &mut ledger,
        },
        &p,
        &mut f.builder,
        &draft,
        &mut Run,
    )
    .unwrap_err();
    let offset = source.iter().position(|&v| v == b'A').unwrap();
    assert_eq!(e, error(ErrorKind::Unsupported, offset));
    assert_eq!(&*p.calls.borrow(), &["begin"]);
    assert_eq!(provider.seen.len(), 1);
    assert_eq!((state.text_bytes, state.glyph_count), (0, 0));
    assert!(cache.entries.is_empty());
    assert_eq!(f.builder.document_text_commands(), 0);
    f.builder.commit_draft(&draft, &mut Run).unwrap();
}

#[test]
fn conflicting_plain_and_xlink_href_uses_original_xlink_span_before_provider() {
    let source=br#"<svg xmlns:xlink="http://www.w3.org/1999/xlink"><image width="2" height="3" href="data:image/png;base64,AQID" xlink:href="wrong"/></svg>"#;
    let p = Publisher::default();
    let mut provider = Provider::default();
    let (r, _) = image(source, &p, &mut provider);
    let offset = source.windows(5).position(|b| b == b"wrong").unwrap();
    assert_eq!(r.unwrap_err(), error(ErrorKind::Unsupported, offset));
    assert_eq!(provider.image_calls, 0);
}

#[test]
fn production_shape_admission_precedes_image_provider_opacity_stroke_and_geometry() {
    for (attrs, expected, bad) in [
        (
            "width=\"bad\" style=\"fill-opacity:bad;stroke:bogus\"",
            ErrorKind::Limit(LimitKind::Shapes),
            "image",
        ),
        (
            "style=\"fill-rule:bad;fill-opacity:bogus\"",
            ErrorKind::Unsupported,
            "bad",
        ),
        (
            "style=\"fill:bogus;fill-rule:bad\"",
            ErrorKind::Unsupported,
            "bogus",
        ),
    ] {
        let source = format!("<svg><path d=\"M0 0L1 0L1 1Z\"/><image {attrs} href=\"bad\"/></svg>");
        let mut f = Fixture::new(
            source.as_bytes(),
            Limits {
                shapes: 1,
                ..Default::default()
            },
        );
        let i = f.index(b"image");
        let map = f.map(i);
        let element = view(&f.lex, i);
        let mut decode = DecodeSource::new(&f.lex, &mut Run).unwrap();
        let style = decode.compute_style(ElementId(i), None, &mut Run).unwrap();
        let scene = f.builder.root_id();
        let draft = f.builder.begin_draft(scene).unwrap();
        let prefix = f
            .builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
        let before = *f.builder.scene(scene).unwrap().shape(prefix).unwrap();
        let mut provider = Provider::default();
        let p = Publisher::default();
        let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
        let e = publish_image_request(
            ImageDecodeRequest {
                element,
                source_map: map,
                style: &style,
                transform: map.transform,
                provider: &mut provider,
                scene,
                ledger: &mut ledger,
            },
            &p,
            &mut f.builder,
            &draft,
            &mut Run,
        )
        .unwrap_err();
        assert_eq!((e.kind, e.offset), (expected, source.find(bad).unwrap()));
        assert_eq!(provider.image_calls, 0);
        assert_eq!(f.builder.scene(scene).unwrap().shapes.len(), 1);
        assert!(f.builder.scene(scene).unwrap().images.is_empty());
        f.builder
            .finalize_shape(&draft, prefix, before, &mut Run)
            .unwrap();
        let token = f.builder.begin_request(&draft, &mut Run).unwrap();
        f.builder.commit_request(&draft, token, &mut Run).unwrap();
        f.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

struct PanicPublisher {
    inner: Publisher,
    stage: &'static str,
}
impl StylePublisher for PanicPublisher {
    fn begin(
        &self,
        c: &PublicationContext<'_, '_>,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<()> {
        self.inner.begin(c, b, d, w)
    }
    fn prepare_geometry(
        &self,
        c: &PublicationContext<'_, '_>,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<()> {
        self.inner.prepare_geometry(c, b, d, w)
    }
    fn prepare_admitted_geometry(
        &self,
        c: &PublicationContext<'_, '_>,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<()> {
        assert_ne!(self.stage, "admitted", "fixture panic with live admission");
        self.inner.prepare_admitted_geometry(c, b, d, w)
    }
    fn publish(
        &self,
        c: &PublicationContext<'_, '_>,
        g: &CustomGeometry,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<PublishedStyle> {
        assert_ne!(
            self.stage, "publish",
            "fixture panic before lease consumption"
        );
        self.inner.publish(c, g, b, d, w)
    }
    fn finish(
        &self,
        c: &PublicationContext<'_, '_>,
        first: usize,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<()> {
        assert_ne!(
            self.stage, "finish",
            "fixture panic after lease consumption"
        );
        self.inner.finish(c, first, b, d, w)
    }
}

#[test]
fn image_panics_before_and_after_consumption_restore_prefix_and_allow_successful_retry() {
    for stage in ["admitted", "publish", "finish"] {
        let source=br#"<svg><path d="M0 0L1 0L1 1Z"/><image width="4" height="4" href="data:image/png;base64,AQID"/></svg>"#;
        let mut f = Fixture::new(source, Limits::default());
        let i = f.index(b"image");
        let element = view(&f.lex, i);
        let mut map = f.map(i);
        // Host annotation is independent of the image's canonical geometry transform.
        map.transform.e = 17.;
        map.transform.f = 23.;
        let scene = f.builder.root_id();
        let draft = f.builder.begin_draft(scene).unwrap();
        let prefix = f
            .builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
        let mut provider = Provider::default();
        let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
        let style = CssPaint::initial();
        let p = PanicPublisher {
            inner: Publisher::default(),
            stage,
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            publish_image_request(
                ImageDecodeRequest {
                    element,
                    source_map: map,
                    style: &style,
                    transform: vexel_geometry::affine_identity(),
                    provider: &mut provider,
                    scene,
                    ledger: &mut ledger,
                },
                &p,
                &mut f.builder,
                &draft,
                &mut Run,
            )
        }));
        assert!(outcome.is_err());
        let s = f.builder.scene(scene).unwrap();
        assert_eq!(s.shapes.len(), 1);
        assert!(s.shape(prefix).is_some());
        assert!(s.images.is_empty());
        let id = publish_image_request(
            ImageDecodeRequest {
                element,
                source_map: map,
                style: &style,
                transform: vexel_geometry::affine_identity(),
                provider: &mut provider,
                scene,
                ledger: &mut ledger,
            },
            &Publisher::default(),
            &mut f.builder,
            &draft,
            &mut Run,
        )
        .unwrap()
        .unwrap();
        let s = f.builder.scene(scene).unwrap();
        assert_eq!(s.shape(id).unwrap().source, map);
        assert_eq!(s.edges[s.shape(id).unwrap().edges.first].a.x, 0.);
        assert!(
            s.shape_geometry_source(id).is_none(),
            "image has no canonical path d source"
        );
        assert_eq!(
            s.path_numbers.len(),
            6,
            "only the prefix has authored path numbers"
        );
        f.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

#[test]
fn text_panics_restore_state_cache_budget_and_admissions_before_retry() {
    for stage in ["admitted", "publish", "finish"] {
        let source = br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A<![CDATA[B]]></text></svg>"#;
        let mut f = Fixture::new(source, Limits::default());
        let i = f.index(b"text");
        let element = view(&f.lex, i);
        let map = f.map(i);
        let scene = f.builder.root_id();
        let draft = f.builder.begin_draft(scene).unwrap();
        let prefix = f
            .builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
        let mut provider = Provider::default();
        let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
        let mut state = TextState::default();
        state.cursor_x = 9.;
        let mut cache = TextCache::default();
        let style = CssPaint::initial();
        let p = PanicPublisher {
            inner: Publisher::default(),
            stage,
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            publish_text_request(
                TextDecodeRequest {
                    element,
                    source_map: map,
                    end: element.end,
                    style: &style,
                    transform: map.transform,
                    state: &mut state,
                    preserve_space: false,
                    provider: &mut provider,
                    cache: &mut cache,
                    scene,
                    ledger: &mut ledger,
                },
                &p,
                &mut f.builder,
                &draft,
                &mut Run,
            )
        }));
        assert!(outcome.is_err());
        assert_eq!(state.cursor_x, 9.);
        assert_eq!((state.text_bytes, state.glyph_count), (0, 0));
        assert!(!state.has_position);
        assert!(!state.pending_space);
        assert_eq!(state.cursor_y, 0.);
        assert!(cache.entries.is_empty());
        assert_eq!((cache.commands, cache.text_bytes), (0, 0));
        assert_eq!(f.builder.document_text_commands(), 0);
        assert_eq!(f.builder.scene(scene).unwrap().shapes.len(), 1);
        assert!(f.builder.scene(scene).unwrap().shape(prefix).is_some());
        publish_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: map.transform,
                state: &mut state,
                preserve_space: false,
                provider: &mut provider,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &Publisher::default(),
            &mut f.builder,
            &draft,
            &mut Run,
        )
        .unwrap();
        let s = f.builder.scene(scene).unwrap();
        assert_eq!(s.shapes.len(), 3);
        assert!(s.shapes[1..].iter().all(|shape| shape.source == map));
        assert!(s.shapes[1..].iter().enumerate().all(|(i, _)| {
            s.shape_geometry_source(s.shape_id(i + 1).unwrap())
                .is_none()
        }));
        assert_eq!(
            s.path_numbers.len(),
            6,
            "only the prefix has authored path numbers"
        );
        f.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

#[test]
fn second_text_shape_admission_failure_rolls_back_first_run_and_allows_image_retry() {
    let source=br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A<![CDATA[B]]></text><image width="4" height="4" href="data:image/png;base64,AQID"/></svg>"#;
    let mut f = Fixture::new(
        source,
        Limits {
            shapes: 2,
            ..Default::default()
        },
    );
    let i = f.index(b"text");
    let element = view(&f.lex, i);
    let map = f.map(i);
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let prefix = f
        .builder
        .append_shape(scene, f.prefix_input(), &mut Run)
        .unwrap();
    let before = *f.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let mut provider = Provider::default();
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let p = Publisher::default();
    let e = publish_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &CssPaint::initial(),
            transform: map.transform,
            state: &mut state,
            preserve_space: false,
            provider: &mut provider,
            cache: &mut cache,
            scene,
            ledger: &mut ledger,
        },
        &p,
        &mut f.builder,
        &draft,
        &mut Run,
    )
    .unwrap_err();
    assert_eq!(
        (e.kind, e.offset),
        (
            ErrorKind::Limit(LimitKind::Shapes),
            source.windows(4).position(|v| v == b"text").unwrap()
        )
    );
    assert_eq!(
        provider.seen.len(),
        2,
        "text admission preserves original layout-before-custom ordering"
    );
    assert_eq!(
        &*p.calls.borrow(),
        &["begin", "prepare", "publish", "prepare"]
    );
    assert_eq!(f.builder.scene(scene).unwrap().shapes.len(), 1);
    assert_eq!(f.builder.scene(scene).unwrap().shape(prefix), Some(&before));
    assert_eq!(f.builder.document_text_commands(), 0);
    assert!(cache.entries.is_empty());
    assert_eq!(
        (state.text_bytes, state.glyph_count, state.cursor_x),
        (0, 0, 0.)
    );
    let i = f.index(b"image");
    let element = view(&f.lex, i);
    let map = f.map(i);
    let id = publish_image_request(
        ImageDecodeRequest {
            element,
            source_map: map,
            style: &CssPaint::initial(),
            transform: map.transform,
            provider: &mut provider,
            scene,
            ledger: &mut ledger,
        },
        &Publisher::default(),
        &mut f.builder,
        &draft,
        &mut Run,
    )
    .unwrap()
    .unwrap();
    assert_eq!(provider.image_calls, 1);
    assert!(f.builder.scene(scene).unwrap().shape(id).is_some());
    f.builder.commit_draft(&draft, &mut Run).unwrap();
}
