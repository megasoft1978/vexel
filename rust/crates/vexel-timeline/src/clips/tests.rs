use super::*;

fn all(_: Role) -> bool {
    true
}

fn params(mode: ClipMode, loops: u32) -> ClipParams {
    ClipParams { start: 4, duration: 96, loops, mode, ..ClipParams::default() }
}

#[test]
fn every_clip_counts_keys_and_stays_in_bounds() {
    for clip in Clip::ALL {
        let p = params(ClipMode::Loop, 2);
        let tracks = generate(clip, &p, &all).unwrap();
        assert!(!tracks.is_empty(), "{}", clip.name());
        let expected = (cycles(&p) * clip.samples()) as usize + 1;
        for track in &tracks {
            assert_eq!(track.keys.len(), expected, "{} {}", clip.name(), track.role.name());
            assert_eq!(track.keys[0].tick, 4);
            assert_eq!(track.keys.last().unwrap().tick, 100);
            assert!(track.keys.windows(2).all(|w| w[0].tick < w[1].tick));
            for key in &track.keys {
                assert!(key.pose.rotate.abs() <= 150.0, "{} rotate", clip.name());
                assert!(key.pose.translate.iter().all(|v| v.abs() <= 40.0));
                assert!(key.pose.scale.iter().all(|v| (0.5..=1.5).contains(v)));
            }
        }
    }
}

#[test]
fn loops_end_where_they_start() {
    for clip in Clip::ALL {
        for mode in [ClipMode::Loop, ClipMode::PingPong] {
            for track in generate(clip, &params(mode, 3), &all).unwrap() {
                assert_eq!(
                    track.keys.first().unwrap().pose,
                    track.keys.last().unwrap().pose,
                    "{} {:?} {}",
                    clip.name(),
                    mode,
                    track.role.name()
                );
            }
        }
    }
}

#[test]
fn pingpong_is_symmetric_and_doubles_keys() {
    let loop_keys = generate(Clip::Walk, &params(ClipMode::Loop, 1), &all).unwrap();
    let ping = generate(Clip::Walk, &params(ClipMode::PingPong, 1), &all).unwrap();
    assert_eq!(ping[0].keys.len(), (loop_keys[0].keys.len() - 1) * 2 + 1);
    let keys = &ping[0].keys;
    let n = keys.len();
    for i in 0..n {
        assert_eq!(keys[i].pose, keys[n - 1 - i].pose);
    }
}

#[test]
fn deterministic_and_amplitude_scales() {
    let p = params(ClipMode::Loop, 1);
    assert_eq!(generate(Clip::Run, &p, &all), generate(Clip::Run, &p, &all));
    let big = generate(Clip::Walk, &ClipParams { amplitude: 2.0, ..p }, &all).unwrap();
    let small = generate(Clip::Walk, &p, &all).unwrap();
    let peak = |t: &[RoleKeys]| t[0].keys.iter().map(|k| k.pose.rotate.abs()).fold(0.0, f64::max);
    assert!(peak(&big) > peak(&small) * 1.9);
    let flat = generate(Clip::Walk, &ClipParams { amplitude: 0.0, ..p }, &all).unwrap();
    assert!(flat.iter().all(|t| t.keys.iter().all(|k| k.pose.rotate == 0.0)));
}

#[test]
fn speed_multiplies_cycles_and_short_durations_dedupe() {
    let p = ClipParams { speed: 2.0, ..params(ClipMode::Loop, 2) };
    assert_eq!(cycles(&p), 4);
    let tight = ClipParams { duration: 5, ..ClipParams::default() };
    let tracks = generate(Clip::Walk, &tight, &all).unwrap();
    assert!(tracks[0].keys.windows(2).all(|w| w[0].tick < w[1].tick));
    assert_eq!(tracks[0].keys.last().unwrap().tick, 5);
}

#[test]
fn rejects_bad_params_and_missing_roles() {
    let base = ClipParams::default();
    for bad in [
        ClipParams { duration: 0, ..base },
        ClipParams { duration: MAX_DURATION + 1, ..base },
        ClipParams { loops: 0, ..base },
        ClipParams { loops: 65, ..base },
        ClipParams { speed: f64::NAN, ..base },
        ClipParams { speed: 0.1, ..base },
        ClipParams { amplitude: -1.0, ..base },
        ClipParams { amplitude: 9.0, ..base },
        ClipParams { start: u32::MAX, ..base },
    ] {
        assert_eq!(generate(Clip::Walk, &bad, &all), Err(ClipError::BadParams));
    }
    let no_legs = |r: Role| !matches!(r, Role::LegL);
    assert_eq!(generate(Clip::Walk, &base, &no_legs), Err(ClipError::MissingRole(Role::LegL)));
    let only_head = |r: Role| r == Role::Head;
    let tracks = generate(Clip::TalkBody, &base, &only_head).unwrap();
    assert_eq!(tracks.len(), 1);
}

#[test]
fn names_round_trip() {
    for clip in Clip::ALL {
        assert_eq!(Clip::parse(clip.name()), Some(clip));
    }
    assert_eq!(Clip::parse("blink-body-bob"), Some(Clip::BlinkBob));
    assert_eq!(Clip::parse("dance"), None);
}

#[test]
fn tick_clash_keeps_loop_closed() {
    let tight = ClipParams { start: 4, duration: 3, ..params(ClipMode::Loop, 1) };
    for clip in Clip::ALL {
        for track in generate(clip, &tight, &all).unwrap() {
            assert!(track.keys.windows(2).all(|w| w[0].tick < w[1].tick));
            assert_eq!(track.keys.last().unwrap().tick, 7);
            assert_eq!(track.keys.first().unwrap().pose, track.keys.last().unwrap().pose);
        }
    }
}
