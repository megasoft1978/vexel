use super::super::SessionError;
use super::super::history::PayloadSnapshot;
use super::super::svg_asset::OwnedProjectAsset;
use super::super::timeline_edit::{ProjectTimelineCandidate, ProjectTimelineError};
use super::SessionPayload;
use vexel_project::{Asset, BundleInput, decode, encode};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 240,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Once,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.5,
    }
}

fn payload(timeline: &[u8]) -> SessionPayload {
    let asset = Asset {
        name: b"opaque.bin",
        media_type: b"application/octet-stream",
        data: b"\x00opaque\xff",
    };
    let bytes = encode(&BundleInput {
        manifest_json: br#"{ "format" : "vexel-project", "x":1 }"#,
        scene_svg: b"<svg/>",
        timeline_json: timeline,
        assets: &[asset],
    })
    .unwrap();
    let view = decode(&bytes).unwrap();
    SessionPayload::try_from_view_with(view, &mut || true).unwrap()
}

#[test]
fn candidate_round_trip_keeps_manifest_and_opaque_assets_exact() {
    let source_timeline = br#"{ "schema" : "vexel.animation", "version": 2, "future":true }"#;
    let source = payload(source_timeline);
    let original_identity = source.identity();
    let candidate = source
        .timeline_candidate(Vec::new(), settings(), EditLimits::default())
        .unwrap();
    assert_eq!(candidate.timeline_bytes(), source_timeline);

    let replaced = source.with_timeline_candidate(&candidate).unwrap();
    let bundle = replaced
        .try_with_bundle_input(b"<svg/>", &mut || true, |input| {
            encode(input).map_err(|_| ())
        })
        .unwrap();
    let view = decode(&bundle).unwrap();
    assert_eq!(
        view.manifest_json,
        br#"{ "format" : "vexel-project", "x":1 }"#
    );
    assert_eq!(view.timeline_json, source_timeline);
    let asset = view.asset_at(0).unwrap();
    assert_eq!(asset.name, b"opaque.bin");
    assert_eq!(asset.media_type, b"application/octet-stream");
    assert_eq!(asset.data, b"\x00opaque\xff");
    assert_eq!(source.identity(), original_identity);
}

#[test]
fn candidate_initialization_is_read_only_and_replacement_has_new_identity() {
    let source = payload(b"{}");
    let clone = source.clone();
    let source_identity = source.identity();
    assert_eq!(source_identity, clone.identity());
    let candidate = source
        .timeline_candidate(Vec::new(), settings(), EditLimits::default())
        .unwrap();
    assert_eq!(source.identity(), source_identity);

    let replaced = source.with_timeline_candidate(&candidate).unwrap();
    assert_ne!(replaced.identity(), source_identity);
    assert_eq!(source.retained_bytes(), clone.retained_bytes());
}

#[test]
fn unsupported_timeline_stays_opaque_and_source_bytes_remain_unchanged() {
    let source = payload(br#"{ "schema":"vexel.animation", "version":2 }"#);
    let bytes_before = source.timeline_bytes().to_vec();
    let candidate = source
        .timeline_candidate(Vec::new(), settings(), EditLimits::default())
        .unwrap();
    assert!(!candidate.is_editable());
    assert_eq!(candidate.timeline_bytes(), bytes_before);
    assert_eq!(source.timeline_bytes(), bytes_before);
    assert_eq!(
        candidate.set_loop(LoopMode::Repeat),
        Err(ProjectTimelineError::UnsupportedTimeline)
    );
}

#[test]
fn invalid_timeline_candidate_fails_without_changing_source() {
    let source = payload(b"not-json");
    let identity = source.identity();
    let bytes_before = source.timeline_bytes().to_vec();
    assert!(matches!(
        source.timeline_candidate(Vec::new(), settings(), EditLimits::default()),
        Err(ProjectTimelineError::InvalidTimeline(_))
    ));
    assert_eq!(source.identity(), identity);
    assert_eq!(source.timeline_bytes(), bytes_before);
}

fn asset(name: &[u8], data: &[u8]) -> OwnedProjectAsset {
    OwnedProjectAsset {
        name: name.to_vec(),
        media_type: b"application/octet-stream".to_vec(),
        source: data.to_vec(),
    }
}

fn candidate(assets: Vec<OwnedProjectAsset>) -> ProjectTimelineCandidate {
    ProjectTimelineCandidate::open(
        br#"{ "schema":"vexel.animation", "version":2, "future":true }"#,
        Vec::new(),
        assets,
        settings(),
        EditLimits::default(),
    )
    .unwrap()
}

fn bundle_bytes(payload: &SessionPayload) -> Vec<u8> {
    payload.with_bundle_input(b"<svg/>", |input| encode(input).unwrap())
}

fn assert_preserved(source: &SessionPayload, identity: usize, retained: usize, bytes: &[u8]) {
    assert_eq!(source.identity(), identity);
    assert_eq!(source.retained_bytes(), retained);
    // Encoding compares the exact manifest, timeline, asset names, media and data.
    assert_eq!(bundle_bytes(source), bytes);
}

#[test]
fn every_replacement_allocation_failure_preserves_the_source() {
    let source = payload(b"{}");
    let identity = source.identity();
    let retained = source.retained_bytes();
    let before = bundle_bytes(&source);
    let candidate = candidate(vec![
        asset(b"a.bin", b"\x00first\xff"),
        asset(b"b.bin", b"second"),
    ]);
    // Validation vector, timeline, owned vector, three copies per asset,
    // and manifest. All contain data so each reservation reaches the hook.
    let allocations = 10;
    for denied in 0..allocations {
        let mut calls = 0;
        let result = source.with_timeline_candidate_with(&candidate, &mut || {
            let allowed = calls != denied;
            calls += 1;
            allowed
        });
        assert!(
            matches!(result, Err(SessionError::OutOfMemory)),
            "reservation {denied}"
        );
        assert_eq!(calls, denied + 1);
        assert_preserved(&source, identity, retained, &before);
    }
    let mut calls = 0;
    let replaced = source
        .with_timeline_candidate_with(&candidate, &mut || {
            calls += 1;
            true
        })
        .unwrap();
    assert_eq!(calls, allocations);
    assert_ne!(replaced.identity(), identity);
    assert_preserved(&source, identity, retained, &before);
}

#[test]
fn invalid_asset_names_preserve_the_source() {
    let source = payload(b"{}");
    let identity = source.identity();
    let retained = source.retained_bytes();
    let before = bundle_bytes(&source);
    for name in [
        Vec::new(),
        b".".to_vec(),
        b"..".to_vec(),
        b"../x".to_vec(),
        b"bad/name".to_vec(),
        b"bad name".to_vec(),
        vec![0xff],
        vec![b'a'; vexel_project::MAX_ASSET_NAME_BYTES + 1],
    ] {
        let candidate = candidate(vec![asset(&name, b"data")]);
        assert!(matches!(
            source.with_timeline_candidate(&candidate),
            Err(SessionError::TooLarge)
        ));
        assert_preserved(&source, identity, retained, &before);
    }
}

#[test]
fn duplicate_asset_names_preserve_the_source() {
    let source = payload(b"{}");
    let identity = source.identity();
    let retained = source.retained_bytes();
    let before = bundle_bytes(&source);
    let candidate = candidate(vec![
        asset(b"same.bin", b"first"),
        asset(b"same.bin", b"second"),
    ]);
    assert!(matches!(
        source.with_timeline_candidate(&candidate),
        Err(SessionError::TooLarge)
    ));
    assert_preserved(&source, identity, retained, &before);
}

#[test]
fn excessive_asset_count_fails_before_allocation_and_preserves_the_source() {
    let source = payload(b"{}");
    let identity = source.identity();
    let retained = source.retained_bytes();
    let before = bundle_bytes(&source);
    let candidate = candidate(
        (0..=vexel_project::MAX_ASSET_COUNT)
            .map(|index| asset(format!("asset-{index}.bin").as_bytes(), b"data"))
            .collect(),
    );
    let mut calls = 0;
    assert!(matches!(
        source.with_timeline_candidate_with(&candidate, &mut || {
            calls += 1;
            true
        }),
        Err(SessionError::TooLarge)
    ));
    assert_eq!(calls, 0);
    assert_preserved(&source, identity, retained, &before);
}

#[test]
fn retained_byte_totals_include_each_opaque_component_exactly_once() {
    let source = payload(b"{}");
    let manifest = br#"{ "format" : "vexel-project", "x":1 }"#;
    let media = b"application/octet-stream";
    assert_eq!(
        source.retained_bytes(),
        manifest.len() + 2 + b"opaque.bin".len() + media.len() + b"\x00opaque\xff".len()
    );
    assert_eq!(source.clone().retained_bytes(), source.retained_bytes());
    let candidate = candidate(vec![
        asset(b"a.bin", b"\x00\xff"),
        asset(b"longer.bin", b""),
    ]);
    let replaced = source.with_timeline_candidate(&candidate).unwrap();
    assert_eq!(
        replaced.retained_bytes(),
        manifest.len()
            + candidate.timeline_bytes().len()
            + b"a.bin".len()
            + b"longer.bin".len()
            + 2 * media.len()
            + 2
    );
    assert_ne!(replaced.identity(), source.identity());
}
