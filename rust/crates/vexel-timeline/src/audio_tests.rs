use crate::audio::{self, AudioKind, AudioTrack, Duck};
use crate::audio_codec::{self, AudioError, Pcm};
use crate::audio_mix::{self, frames_for_ticks, mixdown, MixError, OUT_RATE};
use crate::{decode, initialize_empty_timeline, EditLimits, EmptyTimelineSettings, Error, LoopMode, TimelineContext, TimelineEdit};

fn track(id: &str) -> AudioTrack {
    AudioTrack {
        id: id.to_owned(),
        kind: AudioKind::Sfx,
        src: format!("audio/{id}.wav"),
        start_tick: 0,
        trim_in_ms: 0,
        trim_out_ms: None,
        gain_db: 0.0,
        fade_in_ticks: 0,
        fade_out_ticks: 0,
        looped: false,
        duck: None,
    }
}

fn editor() -> (TimelineEdit, TimelineContext) {
    let context = TimelineContext { scene_ids: vec![b"a".to_vec()], assets: Vec::new() };
    let bytes = initialize_empty_timeline(
        EmptyTimelineSettings {
            ticks_per_second: 24,
            duration_ticks: 48,
            fps_numerator: 24,
            fps_denominator: 1,
            loop_mode: LoopMode::Repeat,
            onion_before: 0,
            onion_after: 0,
            onion_opacity: 0.25,
        },
        &context,
    )
    .unwrap();
    (TimelineEdit::new(&bytes, &context, EditLimits::default()).unwrap(), context)
}

#[test]
fn tracks_round_trip_undo_and_reject_bad_input() {
    let (mut edit, ctx) = editor();
    let before = edit.source().to_vec();
    let mut a = track("a1");
    a.trim_out_ms = Some(900);
    a.gain_db = -6.5;
    a.looped = true;
    let mut b = track("b1");
    b.kind = AudioKind::Voice;
    b.src = "we\"ird/\u{e9}.wav".to_owned();
    b.duck = Some(Duck { under: "music".to_owned(), amount_db: -12.0 });
    edit.set_audio_tracks(&ctx, &[a.clone(), b.clone()]).unwrap();
    let got = decode(edit.source(), &ctx).unwrap().audio_tracks();
    assert_eq!(got, vec![a.clone(), b]);
    edit.undo(&ctx).unwrap();
    assert_eq!(edit.source(), before.as_slice());
    assert!(decode(edit.source(), &ctx).unwrap().audio_tracks().is_empty());
    edit.redo(&ctx).unwrap();
    // Replacing an existing array also works.
    edit.set_audio_tracks(&ctx, &[a]).unwrap();
    assert_eq!(decode(edit.source(), &ctx).unwrap().audio_tracks().len(), 1);

    let bad = |f: &dyn Fn(&mut AudioTrack)| {
        let mut t = track("x");
        f(&mut t);
        let (mut e, c) = editor();
        e.set_audio_tracks(&c, &[t]).unwrap_err().error
    };
    assert_eq!(bad(&|t| t.src = "../x.wav".into()), Error::InvalidValue);
    assert_eq!(bad(&|t| t.src = "/abs.wav".into()), Error::InvalidValue);
    assert_eq!(bad(&|t| t.start_tick = 48), Error::InvalidValue);
    assert_eq!(bad(&|t| t.gain_db = 99.0), Error::InvalidValue);
    assert_eq!(bad(&|t| t.gain_db = f64::NAN), Error::InvalidValue);
    assert_eq!(bad(&|t| { t.trim_in_ms = 500; t.trim_out_ms = Some(500); }), Error::InvalidValue);
    assert_eq!(bad(&|t| t.id = "voice".into()), Error::InvalidSchema);
    assert_eq!(bad(&|t| t.duck = Some(Duck { under: "ghost".into(), amount_db: -6.0 })), Error::InvalidReference);
    assert_eq!(bad(&|t| t.duck = Some(Duck { under: "music".into(), amount_db: 3.0 })), Error::InvalidValue);
    let many: Vec<AudioTrack> = (0..65).map(|i| track(&format!("t{i}"))).collect();
    assert_eq!(edit.set_audio_tracks(&ctx, &many).unwrap_err().error, Error::Limit);
    let dup = [track("d"), track("d")];
    assert_eq!(edit.set_audio_tracks(&ctx, &dup).unwrap_err().error, Error::Duplicate);
    // 64 is fine and shrinking the timeline below a start tick is refused.
    let ok: Vec<AudioTrack> = (0..64).map(|i| track(&format!("t{i}"))).collect();
    edit.set_audio_tracks(&ctx, &ok).unwrap();
    let mut late = track("late");
    late.start_tick = 40;
    edit.set_audio_tracks(&ctx, &[late]).unwrap();
    assert!(edit.set_timing(&ctx, 24, None).is_err());
    assert_eq!(audio::to_json(&[]), b"[]");
}

fn wav16(rate: u32, ch: usize, samples: &[i16]) -> Vec<u8> {
    audio_codec::encode_wav16(rate, ch, samples)
}

#[test]
fn codec_reads_wav_aiff_caf_and_rejects_bad_files() {
    let pcm = audio_codec::decode(&wav16(44_100, 2, &[0, 16384, -16384, 32767])).unwrap();
    assert_eq!((pcm.rate, pcm.channels, pcm.frames()), (44_100, 2, 2));
    assert_eq!(pcm.samples[1], 0.5);
    // 24-bit mono WAV.
    let mut w = wav16(48_000, 1, &[0, 0]);
    w[34] = 24;
    w[32] = 3;
    w.truncate(44);
    w.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0xC0]);
    let len = (w.len() - 8) as u32;
    w[4..8].copy_from_slice(&len.to_le_bytes());
    w[40..44].copy_from_slice(&6u32.to_le_bytes());
    let p = audio_codec::decode(&w).unwrap();
    assert_eq!(p.samples, vec![0.5, -0.5]);
    // Float WAV.
    let mut f = wav16(48_000, 1, &[0, 0]);
    f[20] = 3;
    f[32] = 4;
    f[34] = 32;
    f.truncate(44);
    f.extend_from_slice(&0.25f32.to_le_bytes());
    f[40..44].copy_from_slice(&4u32.to_le_bytes());
    assert_eq!(audio_codec::decode(&f).unwrap().samples, vec![0.25]);
    // AIFF 16-bit stereo, 44100 Hz: extended 0x400E AC44 0...
    let mut a = b"FORM\0\0\0\0AIFFCOMM".to_vec();
    a.extend_from_slice(&18u32.to_be_bytes());
    a.extend_from_slice(&[0, 2, 0, 0, 0, 1, 0, 16, 0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
    a.extend_from_slice(b"SSND");
    a.extend_from_slice(&12u32.to_be_bytes());
    a.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0x00, 0xC0, 0x00]);
    let total = (a.len() - 8) as u32;
    a[4..8].copy_from_slice(&total.to_be_bytes());
    let p = audio_codec::decode(&a).unwrap();
    assert_eq!((p.rate, p.channels, p.samples.clone()), (44_100, 2, vec![0.5, -0.5]));
    // CAF 16-bit big-endian mono.
    let mut c = b"caff\0\x01\0\0desc".to_vec();
    c.extend_from_slice(&32i64.to_be_bytes());
    c.extend_from_slice(&48_000f64.to_be_bytes());
    c.extend_from_slice(b"lpcm");
    for v in [0u32, 2, 1, 1, 16] {
        c.extend_from_slice(&v.to_be_bytes());
    }
    c.extend_from_slice(b"data");
    c.extend_from_slice(&10i64.to_be_bytes());
    c.extend_from_slice(&[0, 0, 0, 1, 0x40, 0x00, 0x80, 0x00]);
    c.extend_from_slice(&[0, 0]);
    let p = audio_codec::decode(&c).unwrap();
    assert_eq!(p.samples, vec![0.5, -1.0, 0.0]);
    let info = audio_codec::probe(&c).unwrap();
    assert_eq!((info.format, info.frames), ("caf", 3));
    // Rejections.
    let good = wav16(48_000, 1, &[1, 2, 3, 4]);
    assert_eq!(audio_codec::decode(&good[..good.len() - 2]).unwrap_err(), AudioError::Truncated);
    assert_eq!(audio_codec::decode(b"not audio at all").unwrap_err(), AudioError::BadHeader);
    assert_eq!(audio_codec::decode(&[]).unwrap_err(), AudioError::BadHeader);
    assert_eq!(audio_codec::decode(&wav16(48_000, 3, &[1, 2, 3])).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&wav16(4_000, 1, &[1, 2])).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&wav16(48_000, 1, &[])).unwrap_err(), AudioError::BadLength);
    let mut bad_tag = good.clone();
    bad_tag[20] = 2;
    assert_eq!(audio_codec::decode(&bad_tag).unwrap_err(), AudioError::Unsupported);
    let mut caf_trunc = c.clone();
    caf_trunc.truncate(caf_trunc.len() - 3);
    assert_eq!(audio_codec::decode(&caf_trunc).unwrap_err(), AudioError::Truncated);
    let big = vec![0u8; audio_codec::MAX_FILE_BYTES + 1];
    assert_eq!(audio_codec::probe(&big).unwrap_err(), AudioError::TooLarge);
}

/// Minimal CAF with one `desc` and one `data` chunk; `samples` follow the
/// 4-byte edit count. The data chunk size is `4 + samples.len()`.
fn caf_pcm(channels: u32, bits: u32, bytes_per_packet: u32, frames_per_packet: u32, samples: &[u8]) -> Vec<u8> {
    let mut c = b"caff\0\x01\0\0desc".to_vec();
    c.extend_from_slice(&32i64.to_be_bytes());
    c.extend_from_slice(&48_000f64.to_be_bytes());
    c.extend_from_slice(b"lpcm");
    for v in [0, bytes_per_packet, frames_per_packet, channels, bits] {
        c.extend_from_slice(&v.to_be_bytes());
    }
    c.extend_from_slice(b"data");
    c.extend_from_slice(&(4 + samples.len() as i64).to_be_bytes());
    c.extend_from_slice(&[0; 4]);
    c.extend_from_slice(samples);
    c
}

/// Valid 16-bit stereo AIFF at 44.1 kHz with the given 80-bit rate field.
fn aiff_with_rate(rate: [u8; 10]) -> Vec<u8> {
    let mut a = b"FORM\0\0\0\0AIFFCOMM".to_vec();
    a.extend_from_slice(&18u32.to_be_bytes());
    a.extend_from_slice(&[0, 2, 0, 0, 0, 1, 0, 16]);
    a.extend_from_slice(&rate);
    a.extend_from_slice(b"SSND");
    a.extend_from_slice(&12u32.to_be_bytes());
    a.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0x00, 0xC0, 0x00]);
    let total = (a.len() - 8) as u32;
    a[4..8].copy_from_slice(&total.to_be_bytes());
    a
}

#[test]
fn stride_fields_and_frame_alignment_are_checked() {
    // Sanity: the helpers build decodable files.
    assert!(audio_codec::decode(&caf_pcm(1, 16, 2, 1, &[0x40, 0, 0x80, 0])).is_ok());
    assert!(audio_codec::decode(&aiff_with_rate([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0])).is_ok());
    // WAV block_align must equal channels * bits / 8.
    let mut w = wav16(48_000, 1, &[1, 2]);
    w[32] = 3;
    assert_eq!(audio_codec::decode(&w).unwrap_err(), AudioError::Unsupported);
    // CAF bytesPerPacket must match, and framesPerPacket must be 1.
    assert_eq!(audio_codec::decode(&caf_pcm(1, 16, 4, 1, &[0x40, 0, 0x80, 0])).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&caf_pcm(1, 16, 2, 2, &[0x40, 0, 0x80, 0])).unwrap_err(), AudioError::Unsupported);
    // CAF 24-bit padded to 32-bit containers is refused.
    assert_eq!(audio_codec::decode(&caf_pcm(1, 24, 4, 1, &[0, 0x40, 0, 0])).unwrap_err(), AudioError::Unsupported);
    // Data length that is not a whole number of frames.
    let mut odd = wav16(48_000, 1, &[1, 2, 3]);
    odd[40..44].copy_from_slice(&5u32.to_le_bytes());
    odd.truncate(44 + 5);
    assert_eq!(audio_codec::decode(&odd).unwrap_err(), AudioError::BadHeader);
}

#[test]
fn hostile_files_error_without_panicking() {
    // AIFF sample rate: NaN/inf encoding, overflowing exponent, zero.
    assert_eq!(audio_codec::decode(&aiff_with_rate([0x7F, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0])).unwrap_err(), AudioError::BadHeader);
    assert_eq!(audio_codec::decode(&aiff_with_rate([0xFF, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0])).unwrap_err(), AudioError::BadHeader);
    assert_eq!(audio_codec::decode(&aiff_with_rate([0x7F, 0xFE, 0x80, 0, 0, 0, 0, 0, 0, 0])).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&aiff_with_rate([0; 10])).unwrap_err(), AudioError::Unsupported);
    // Chunk sizes of 0xFFFFFFFF in AIFF COMM, SSND.
    let mut a = aiff_with_rate([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
    a[16..20].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(audio_codec::decode(&a).unwrap_err(), AudioError::BadHeader);
    let mut a = aiff_with_rate([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
    a[42..46].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(audio_codec::decode(&a).unwrap_err(), AudioError::Truncated);
    // Chunk sizes of 0xFFFFFFFF in WAV fmt and data.
    let mut w = wav16(48_000, 1, &[1, 2]);
    w[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(audio_codec::decode(&w).unwrap_err(), AudioError::BadHeader);
    let mut w = wav16(48_000, 1, &[1, 2]);
    w[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(audio_codec::decode(&w).unwrap_err(), AudioError::Truncated);
    // CAF data size -1 (no payload), 0 and i64::MAX.
    let mut c = caf_pcm(1, 16, 2, 1, &[]);
    c[56..64].copy_from_slice(&(-1i64).to_be_bytes());
    assert_eq!(audio_codec::decode(&c).unwrap_err(), AudioError::BadLength);
    let mut c = caf_pcm(1, 16, 2, 1, &[]);
    c[56..64].copy_from_slice(&0i64.to_be_bytes());
    assert_eq!(audio_codec::decode(&c).unwrap_err(), AudioError::BadHeader);
    let mut c = caf_pcm(1, 16, 2, 1, &[0x40, 0, 0x80, 0]);
    c[56..64].copy_from_slice(&i64::MAX.to_be_bytes());
    assert_eq!(audio_codec::decode(&c).unwrap_err(), AudioError::Truncated);
    // Channel counts 0 and 3 (CAF with consistent stride fields).
    assert_eq!(audio_codec::decode(&caf_pcm(0, 16, 0, 1, &[0x40, 0, 0x80, 0])).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&caf_pcm(3, 16, 6, 1, &[0; 6])).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&wav16(48_000, 0, &[1, 2])).unwrap_err(), AudioError::Unsupported);
    // Bit depths 8 and 12 (WAV and CAF).
    let mut w = wav16(48_000, 1, &[1, 2]);
    w[34] = 8;
    w[32] = 1;
    assert_eq!(audio_codec::decode(&w).unwrap_err(), AudioError::Unsupported);
    let mut w = wav16(48_000, 1, &[1, 2]);
    w[34] = 12;
    w[32] = 1;
    assert_eq!(audio_codec::decode(&w).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&caf_pcm(1, 8, 1, 1, &[0, 0])).unwrap_err(), AudioError::Unsupported);
    assert_eq!(audio_codec::decode(&caf_pcm(1, 12, 1, 1, &[0, 0])).unwrap_err(), AudioError::Unsupported);
}

fn const_pcm(value: f32, frames: usize) -> Pcm {
    Pcm { rate: OUT_RATE, channels: 1, samples: vec![value; frames] }
}
fn mix(tracks: &[AudioTrack], frames: usize, pcms: &[Pcm]) -> Vec<i16> {
    let mut load = |t: &AudioTrack| Ok(pcms[tracks.iter().position(|x| x.id == t.id).unwrap()].clone());
    mixdown(tracks, 24, frames, &mut load).unwrap()
}

#[test]
fn silence_gain_trim_and_start() {
    assert!(mix(&[], 4800, &[]).iter().all(|s| *s == 0));
    assert_eq!(mix(&[], 4800, &[]).len(), 9600);
    let mut t = track("a");
    t.gain_db = -6.0206;
    let out = mix(&[t.clone()], 48_000, &[const_pcm(0.5, 48_000)]);
    assert!((i32::from(out[100]) - 8192).abs() <= 2, "{}", out[100]);
    assert_eq!(out[100], out[101]);
    // start_tick 12 at 24 tps = 0.5 s; source ends before the output does.
    t.gain_db = 0.0;
    t.start_tick = 12;
    let out = mix(&[t.clone()], 48_000, &[const_pcm(0.25, 12_000)]);
    assert_eq!(out[2 * 23_999], 0);
    assert!(out[2 * 24_000] > 8000);
    assert_eq!(out[2 * 36_000 + 10], 0);
    // trim 100..200 ms of a ramp-free constant keeps 100 ms.
    t.start_tick = 0;
    t.trim_in_ms = 100;
    t.trim_out_ms = Some(200);
    let out = mix(&[t], 48_000, &[const_pcm(0.25, 48_000)]);
    assert!(out[2 * 4_799] != 0 && out[2 * 4_801] == 0);
}

#[test]
fn fades_ramp_linearly() {
    let mut t = track("a");
    t.fade_in_ticks = 6;
    t.fade_out_ticks = 6;
    let out = mix(&[t], 48_000, &[const_pcm(0.5, 48_000)]);
    let at = |i: usize| i32::from(out[i * 2]);
    assert_eq!(at(0), 0);
    assert!((at(12_000) - 16384).abs() <= 2); // end of the fade-in
    assert!((at(6_000) - 8192).abs() <= 4); // halfway up
    assert!((at(47_999 - 6_000) - 8192).abs() <= 4); // halfway down
    assert!(at(47_999).abs() < 8);
    for i in 1..12_000 {
        assert!(at(i) >= at(i - 1));
    }
}

#[test]
fn loop_repeats_without_seam_glitch() {
    let mut t = track("a");
    t.looped = true;
    let ramp: Vec<f32> = (0..100).map(|i| i as f32 / 200.0).collect();
    let pcm = Pcm { rate: OUT_RATE, channels: 1, samples: ramp.clone() };
    let out = mix(&[t], 450, &[pcm]);
    for i in 0..450 {
        let want = (ramp[i % 100] * 32767.0).round() as i32;
        let got = i32::from(out[i * 2]);
        assert!((got - want).abs() <= 1, "frame {i}: {got} vs {want}");
    }
}

#[test]
fn resamples_to_48k_and_mono_duplicates() {
    let t = track("a");
    let pcm = Pcm { rate: 24_000, channels: 1, samples: vec![0.5; 2_400] };
    let out = mix(&[t], 48_000, &[pcm]);
    assert!(out[2 * 4_798] > 0 && out[2 * 4_802] == 0, "{} {}", out[2 * 4_798], out[2 * 4_802]);
    assert_eq!(out[1000], out[1001]);
}

#[test]
fn ducking_lowers_music_under_voice_and_recovers() {
    let mut music = track("m");
    music.kind = AudioKind::Music;
    music.duck = Some(Duck { under: "voice".into(), amount_db: -12.0 });
    let mut voice = track("v");
    voice.kind = AudioKind::Voice;
    voice.start_tick = 12;
    let frames = 96_000;
    let out = mix(&[music, voice], frames, &[const_pcm(0.2, frames), const_pcm(0.3, 12_000)]);
    let level = |i: usize| i32::from(out[i * 2]);
    let music_only = (0.2 * 32767.0) as i32;
    assert!((level(10_000) - music_only).abs() < 5, "before voice");
    let ducked = (0.2 * 32767.0 * 0.2512) as i32 + (0.3 * 32767.0) as i32;
    assert!((level(30_000) - ducked).abs() < 200, "during voice {}", level(30_000));
    assert!((level(90_000) - music_only).abs() < 100, "recovered {}", level(90_000));
}

#[test]
fn limiter_keeps_peaks_below_ceiling() {
    let a = track("a");
    let b = track("b");
    let hot = const_pcm(0.9, 9_600);
    let out = mix(&[a, b], 9_600, &[hot.clone(), hot]);
    assert!(out.iter().all(|s| i32::from(*s).abs() <= (0.98f32 * 32767.0).ceil() as i32));
    assert!(out[9_000].abs() > 20_000);
}

#[test]
fn mixdown_is_deterministic_and_bounded() {
    let mut a = track("a");
    a.looped = true;
    a.fade_in_ticks = 3;
    a.gain_db = -3.0;
    let sine: Vec<f32> = (0..22_050).map(|i| (i as f32 * 0.05).sin() * 0.7).collect();
    let pcm = Pcm { rate: 22_050, channels: 1, samples: sine };
    let one = mix(&[a.clone()], 48_000, &[pcm.clone()]);
    let two = mix(&[a], 48_000, &[pcm]);
    assert_eq!(one, two);
    assert_eq!(frames_for_ticks(48, 24), 96_000);
    let mut load = |_: &AudioTrack| Err(MixError::Limit);
    assert_eq!(mixdown(&[], 24, audio_mix::MAX_OUT_FRAMES + 1, &mut load), Err(MixError::Limit));
    assert_eq!(mixdown(&[], 0, 10, &mut load), Err(MixError::Limit));
    assert_eq!(mixdown(&[track("z")], 24, 10, &mut load), Err(MixError::Limit));
}
