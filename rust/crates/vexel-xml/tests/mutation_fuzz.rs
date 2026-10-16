//! Deterministic mutation fuzzing of the `vexel-xml` tokenizer (`XmlReader`).
//! Property: every input ends in a typed status, offsets are in range and
//! monotonic, and the reader always terminates. Override the run with
//! `VEXEL_FUZZ_SEED` / `VEXEL_FUZZ_CASES` (see the failure report).
use vexel_xml::{ReaderStatus, XmlReader};

#[rustfmt::skip]
const DICT: &[&[u8]] = &[
    b"<", b">", b"</", b"/>", b"<!--", b"-->", b"<![CDATA[", b"]]>", b"<?xml ?>",
    b"<!DOCTYPE svg [<!ENTITY a 'b'>]>", b"&amp;", b"&#x1F600;", b"=\"", b"'", b"\"",
    b"xmlns:x=\"u\"", b"\xff\xfe", b"\xc0\xaf", b"\0", b"<svg>", b"</svg>",
];

fn seeds() -> Vec<Vec<u8>> {
    let mut seeds = vec![
        b"<svg width='1' height='1'><rect/><!-- c --><![CDATA[x]]></svg>".to_vec(),
        b"<?xml version='1.0'?><!DOCTYPE svg><svg><g a='1' b=\"2\"/></svg>".to_vec(),
    ];
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../bench/corpus/fixtures");
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        seeds.extend(
            paths
                .iter()
                .filter(|p| p.extension().is_some_and(|e| e == "svg"))
                .filter_map(|p| std::fs::read(p).ok()),
        );
    }
    seeds
}

fn check_reader(input: &[u8]) -> Result<(), String> {
    let mut reader = XmlReader::new(input).map_err(|s| format!("reader refused input: {s:?}"))?;
    let mut last_read = 0usize;
    let mut last_tag_end = 0usize;
    let mut bad = None;
    for _ in 0..input.len() + 4 {
        let status = reader.step(1 + input.len() % 97, |tag| {
            let ok = tag.source.start <= tag.source.end
                && tag.source.end <= input.len()
                && tag.name.start <= tag.name.end
                && tag.name.start >= tag.source.start
                && tag.name.end <= tag.source.end
                && tag.source.start >= last_tag_end;
            if ok {
                last_tag_end = tag.source.end;
            } else {
                bad = Some(format!(
                    "tag offsets out of range or not monotonic: {tag:?}"
                ));
            }
            ok
        });
        if let Some(why) = bad.take() {
            return Err(why);
        }
        let progress = reader.progress();
        if progress.bytes_read < last_read || progress.bytes_read > input.len() {
            return Err(format!(
                "bytes_read {} not monotonic/in range",
                progress.bytes_read
            ));
        }
        last_read = progress.bytes_read;
        match status {
            ReaderStatus::Ok => continue,
            ReaderStatus::Complete => {
                return if progress.bytes_read == input.len() {
                    Ok(())
                } else {
                    Err("Complete before end of input".into())
                };
            }
            ReaderStatus::Malformed | ReaderStatus::Limit | ReaderStatus::Invalid => {
                return if progress.diagnostic.offset <= input.len() {
                    Ok(())
                } else {
                    Err(format!(
                        "diagnostic offset {} out of range",
                        progress.diagnostic.offset
                    ))
                };
            }
        }
    }
    Err("reader did not terminate".into())
}

#[test]
fn mutated_documents_end_in_typed_status_with_monotonic_offsets() {
    run_cases(&seeds(), DICT, &mut check_reader);
}

#[test]
fn unmutated_seeds_are_accepted_by_the_property() {
    for seed in seeds() {
        check_reader(&seed).unwrap();
    }
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
