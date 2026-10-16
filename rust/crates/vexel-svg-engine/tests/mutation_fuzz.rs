//! Deterministic mutation fuzzing of the `vexel-svg-engine` decode pipeline
//! (parse, session, finish) seeded from `bench/corpus/fixtures/*.svg`.
//! Property: every input yields `Ok` (scene or pattern suspension) or a typed
//! `Err`, never a panic. Override with `VEXEL_FUZZ_SEED` / `VEXEL_FUZZ_CASES`.
mod common;
use common::{Control, try_decode_outcome};

#[rustfmt::skip]
const DICT: &[&[u8]] = &[
    b"<g>", b"</g>", b"<rect width='1e999' height='-1'/>", b"transform='matrix(1 2 3 4 5 6)'",
    b"d='M0 0L1e308 1e308Z'", b"fill='url(#a)'", b"<use href='#a'/>", b"<style>*{fill:red}</style>",
    b"viewBox='0 0 0 0'", b"stroke-width='nan'", b"<![CDATA[", b"&#0;", b"\xff", b"<defs>",
    b"<clipPath id='a'>", b"<linearGradient id='a'/>", b"opacity='2'", b"points='1,2 3'",
];

fn seeds() -> Vec<Vec<u8>> {
    let mut seeds = vec![
        b"<svg width='16' height='16'><g transform='translate(1 2)'><path d='M0 0L8 0L8 8Z' fill='red'/><rect width='4' height='4' stroke='blue'/></g></svg>".to_vec(),
    ];
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../bench/corpus/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.sort();
    seeds.extend(
        paths
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "svg"))
            .filter_map(|p| std::fs::read(p).ok()),
    );
    seeds
}

#[test]
fn mutated_svg_decodes_or_fails_with_a_typed_error() {
    run_cases(&seeds(), DICT, &mut |input| {
        // Ok(Scene/NeedPattern) and Err(Diagnostic) are both valid outcomes.
        let _ = try_decode_outcome(input, &mut Control::default());
        Ok(())
    });
}

#[test]
fn fixture_seeds_include_the_corpus() {
    assert!(seeds().len() > 20, "corpus fixtures were not found");
}

// ---- shared deterministic fuzz harness (copied into each fuzz test file) ----
use std::panic::{AssertUnwindSafe, catch_unwind};

/// xorshift64* generator; deterministic for a given seed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    let Ok(t) = std::env::var(name) else {
        return default;
    };
    let t = t.trim().replace('_', "");
    let n = match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => t.parse(),
    };
    n.unwrap_or_else(|_| panic!("{name}={t:?} is not an integer"))
}

/// One mutation: bit flip, byte replace, delete range, duplicate range,
/// truncate, or token-dictionary insert.
fn mutate(rng: &mut Rng, data: &mut Vec<u8>, dict: &[&[u8]]) {
    let len = data.len();
    let (a, n) = (rng.below(len.max(1)), 1 + rng.below(24));
    let end = (a + n).min(len);
    match rng.below(6) {
        0 if len > 0 => data[a] ^= 1 << rng.below(8),
        1 if len > 0 => data[a] = rng.next() as u8,
        2 if len > 0 => drop(data.drain(a..end)),
        3 if len > 0 => {
            let copy = data[a..end].to_vec();
            let at = rng.below(len + 1);
            data.splice(at..at, copy);
        }
        4 if len > 0 => data.truncate(a),
        _ if !dict.is_empty() => {
            let at = rng.below(len + 1);
            data.splice(at..at, dict[rng.below(dict.len())].iter().copied());
        }
        _ => {}
    }
    data.truncate(64 * 1024);
}

/// Runs mutated inputs through `property`; a typed `Err` or a panic fails the
/// test with seed, case index and the input as hex.
fn run_cases(
    seeds: &[Vec<u8>],
    dict: &[&[u8]],
    property: &mut dyn FnMut(&[u8]) -> Result<(), String>,
) {
    let seed = env_u64("VEXEL_FUZZ_SEED", 0x5EED_0001);
    let mut rng = Rng(seed | 1);
    for case in 0..env_u64("VEXEL_FUZZ_CASES", 2000) {
        let mut input = seeds[rng.below(seeds.len())].clone();
        for _ in 0..1 + rng.below(4) {
            mutate(&mut rng, &mut input, dict);
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| property(&input)));
        let why = match outcome {
            Ok(Ok(())) => continue,
            Ok(Err(why)) => why,
            Err(_) => "panic".to_string(),
        };
        let hex: String = input.iter().take(256).map(|b| format!("{b:02x}")).collect();
        panic!(
            "fuzz failure: seed=0x{seed:X} case={case} len={} reason={why}\ninput(hex, first 256 bytes)={hex}\nreproduce: VEXEL_FUZZ_SEED=0x{seed:X} VEXEL_FUZZ_CASES={}",
            input.len(),
            case + 1
        );
    }
}

#[test]
#[ignore = "demonstrates the failure report format; run with --ignored"]
fn reproducer() {
    run_cases(&[b"seed input".to_vec()], &[], &mut |_| {
        Err("injected failing property".to_string())
    });
}
// ---- end shared harness ----
