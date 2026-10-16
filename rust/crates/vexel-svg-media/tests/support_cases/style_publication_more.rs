use super::*;

#[test]
fn both_production_routes_restore_real_prefix_and_all_admissions_on_each_callback_failure() {
    for is_text in [false, true] {
        for stage in ["begin", "prepare", "publish", "finish"] {
            let source=br##"<svg><path d="M0 0L1 0L1 1Z"/><image width="8" height="4" style="opacity:.5;clip-path:url(#c)" href="data:image/png;base64,AQID"/><text style="opacity:.5;clip-path:url(#fixture)">A<![CDATA[B]]></text></svg>"##;
            let mut f = Fixture::new(source, Limits::default());
            let i = f.index(if is_text { b"text" } else { b"image" });
            let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
            let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
            let element = view(&f.lex, i);
            let map = f.map(i);
            let scene = f.builder.root_id();
            let draft = f.builder.begin_draft(scene).unwrap();
            let prefix = f
                .builder
                .append_shape(scene, f.prefix_input(), &mut Run)
                .unwrap();
            let old_shape = *f.builder.scene(scene).unwrap().shape(prefix).unwrap();
            let old_edges = f.builder.scene(scene).unwrap().edges.to_vec();
            let mut state = TextState::default();
            state.cursor_x = 11.;
            let before = state;
            let mut cache = TextCache::default();
            let mut provider = Provider::default();
            let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
            let p = Publisher {
                fail: Some(stage),
                ..Default::default()
            };
            let result = if is_text {
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
                .map(|_| ())
            } else {
                publish_image_request(
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
                .map(|_| ())
            };
            assert_eq!(
                result.unwrap_err(),
                error(ErrorKind::Unsupported, map.element.start)
            );
            let s = f.builder.scene(scene).unwrap();
            assert_eq!(s.shapes.len(), 1);
            assert_eq!(s.shape(prefix), Some(&old_shape));
            assert_eq!(s.edges, old_edges);
            assert!(s.images.is_empty());
            assert!(s.clips.is_empty());
            assert!(s.groups.is_empty());
            assert_eq!(
                (state.cursor_x, state.text_bytes, state.glyph_count),
                (before.cursor_x, before.text_bytes, before.glyph_count)
            );
            assert!(cache.entries.is_empty());
            assert_eq!((cache.commands, cache.text_bytes), (0, 0));
            assert_eq!(f.builder.document_text_commands(), 0);
            // No caller rollback: the same real draft accepts another actual shape.
            f.builder
                .append_shape(scene, f.prefix_input(), &mut Run)
                .unwrap();
            f.builder.commit_draft(&draft, &mut Run).unwrap();
        }
    }
}

struct Interrupt {
    at: usize,
    calls: usize,
    stop: vexel_base::Stop,
}
impl WorkControl for Interrupt {
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
        self.calls += 1;
        if self.calls == self.at {
            Err(self.stop)
        } else {
            Ok(())
        }
    }
    fn charge(
        &mut self,
        _: vexel_base::WorkKind,
        _: u64,
    ) -> std::result::Result<(), vexel_base::Stop> {
        self.checkpoint()
    }
}
#[test]
fn styled_routes_cancel_at_every_control_boundary_including_real_clip_group_and_commit() {
    for is_text in [false, true] {
        let mut boundaries = 0;
        for stop in [
            None,
            Some(vexel_base::Stop::Canceled),
            Some(vexel_base::Stop::Limit),
            Some(vexel_base::Stop::Overflow),
        ] {
            // First run counts successful boundaries, then interrupt every one.
            let points = if stop.is_none() {
                vec![usize::MAX]
            } else {
                (1..=boundaries).collect()
            };
            let stop = stop.unwrap_or(vexel_base::Stop::Canceled);
            for at in points {
                let source=br##"<svg><path d="M0 0L1 0L1 1Z"/><image width="8" height="4" style="opacity:.5;clip-path:url(#c)" href="data:image/png;base64,AQID"/><text style="opacity:.5;clip-path:url(#fixture)">A<![CDATA[B]]></text></svg>"##;
                let mut f = Fixture::new(source, Limits::default());
                let i = f.index(if is_text { b"text" } else { b"image" });
                let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
                let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
                let element = view(&f.lex, i);
                let map = f.map(i);
                let scene = f.builder.root_id();
                let draft = f.builder.begin_draft(scene).unwrap();
                let prefix = f
                    .builder
                    .append_shape(scene, f.prefix_input(), &mut Run)
                    .unwrap();
                let prefix_shape = *f.builder.scene(scene).unwrap().shape(prefix).unwrap();
                let mut state = TextState::default();
                let mut cache = TextCache::default();
                let mut provider = Provider::default();
                let p = Publisher::default();
                let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
                let mut control = Interrupt { at, calls: 0, stop };
                let result = if is_text {
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
                        &mut control,
                    )
                    .map(|_| ())
                } else {
                    publish_image_request(
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
                        &mut control,
                    )
                    .map(|_| ())
                };
                if at == usize::MAX {
                    result.unwrap();
                    boundaries = control.calls;
                    continue;
                }
                assert_eq!(
                    result.unwrap_err().kind,
                    match stop {
                        vexel_base::Stop::Canceled => ErrorKind::Canceled,
                        vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
                        vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
                    }
                );
                let s = f.builder.scene(scene).unwrap();
                assert_eq!(s.shapes.len(), 1);
                assert_eq!(s.shape(prefix), Some(&prefix_shape));
                assert_eq!(s.edges.len(), prefix_shape.edges.count);
                assert!(s.images.is_empty());
                assert!(s.clips.is_empty());
                assert!(s.groups.is_empty());
                assert_eq!(
                    (state.text_bytes, state.glyph_count, state.cursor_x),
                    (0, 0, 0.)
                );
                assert_eq!(
                    (cache.entries.len(), cache.text_bytes, cache.commands),
                    (0, 0, 0)
                );
                assert_eq!(f.builder.document_text_commands(), 0);
                f.builder
                    .append_shape(scene, f.prefix_input(), &mut Run)
                    .unwrap();
                f.builder.commit_draft(&draft, &mut Run).unwrap();
            }
        }
        assert!(boundaries > 20);
    }
}

#[test]
fn typed_diagnostics_use_real_href_and_overcap_position_offsets_and_restore_text_admissions() {
    let source = b"<svg><image width=\"2\" height=\"3\" href=\"data:image/png;base64,\"/></svg>";
    let p = Publisher::default();
    let mut provider = Provider::default();
    let (r, _) = image(source, &p, &mut provider);
    let offset = source.windows(5).position(|b| b == b"data:").unwrap();
    assert_eq!(r.unwrap_err(), error(ErrorKind::Unsupported, offset));
    assert_eq!(provider.image_calls, 0);
    let mut source = b"<svg><text x=\"".to_vec();
    let first = source.len();
    for _ in 0..MAX_POSITION_VALUES {
        source.extend_from_slice(b"1 ");
    }
    source.extend_from_slice(b"2\">A</text></svg>");
    let mut f = Fixture::new(&source, Limits::default());
    let i = f.index(b"text");
    let element = view(&f.lex, i);
    let map = f.map(i);
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    let style = CssPaint::initial();
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
    assert_eq!(e, error(ErrorKind::Number, first + 2 * MAX_POSITION_VALUES));
    assert_eq!(provider.seen.len(), 1);
    assert_eq!(state.text_bytes, 0);
    assert_eq!(f.builder.document_text_commands(), 0);
    assert!(cache.entries.is_empty());
    assert!(f.builder.scene(scene).unwrap().shapes.is_empty());
    f.builder.commit_draft(&draft, &mut Run).unwrap();
}
