//! Deterministic mutation fuzzing of `vexel_mcp_stdio::serve` over mutated line
//! streams. Property: `serve` never panics and always terminates; each
//! response line it writes parses with `vexel-json`. Override with
//! `VEXEL_FUZZ_SEED` / `VEXEL_FUZZ_CASES`.
use std::io::Cursor;
use vexel_json::JsonDocument;
use vexel_mcp_stdio::serve;
use vexel_protocol::{ProtocolBackend, ProtocolSession, ToolResponse};

struct Stub;
impl ProtocolBackend for Stub {
    fn call_tool(&mut self, _: &JsonDocument<'_>, _: usize, _: &str) -> Option<ToolResponse> {
        Some(ToolResponse {
            structured_json: b"{}".to_vec(),
            content_json: Vec::new(),
            is_error: false,
        })
    }
}

#[rustfmt::skip]
const DICT: &[&[u8]] = &[
    b"\n", b"\r\n", b"\r", b"\n\n", b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n",
    b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n", b"{", b"\"", b"\0", b"\xff\xfe",
];

fn seeds() -> Vec<Vec<u8>> {
    vec![
        b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\r\n".to_vec(),
        b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"x\",\"arguments\":{}}}".to_vec(),
        b"\n\r\n{}\n[]\n".to_vec(),
    ]
}

fn check_stream(input: &[u8]) -> Result<(), String> {
    let mut output = Vec::new();
    // Ok and a typed StdioError are both terminating outcomes.
    let _ = serve(
        &mut Cursor::new(input),
        &mut output,
        &mut ProtocolSession::default(),
        &mut Stub,
    );
    for line in output.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        JsonDocument::parse(line).map_err(|e| format!("response line does not parse: {e:?}"))?;
    }
    Ok(())
}

#[test]
fn mutated_line_streams_terminate_with_parseable_responses() {
    run_cases(&seeds(), DICT, &mut check_stream);
}

#[test]
fn seed_streams_are_accepted() {
    for seed in seeds() {
        check_stream(&seed).unwrap();
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
