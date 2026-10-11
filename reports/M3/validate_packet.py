#!/usr/bin/env python3
"""Deterministic, offline verifier/generator for the M3 owner packet."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT_JSON = ROOT / "reports/M3/evidence.json"
OUT_MD = ROOT / "reports/M3/decision.md"

# Frozen input bytes. Updating a source requires an explicit reviewed update here.
PINNED = {
    "reports/M0/evidence.json": "d4eeadf18d6da4001e0b6f2d531aec419669edc8c028d92ae481dcd500db082b",
    "reports/M0/product-evidence.md": "2336211d8e9fdef2cf5731b5cc31e15e6799ba619f75264b6379bf41b28a188f",
    "bench/results/WO-M0-03.json": "854ff1ed1cacc0483598bd1d4fcb7d1fd118bd47062876759025e4cb929bf1b9",
    "bench/results/WO-M0-03.csv": "846f26ace59c2a3d0bc1be61e8a2e24222a8b09ea3575484f7192721c80756d6",
    "bench/results/WO-M3-01.json": "f2d7d9cd7c95bd43d185ff0a9d994d21d1c5b109ece06d0eb41d5d58cd73569a",
    "artifacts/WO-M3-01/progressive-replay.json": "e589f05056582ab98d1de9f1cdff5be1756969f1293e66337e1414e12f1af789",
    "artifacts/WO-M2-04/roundtrip.json": "83ba83a9236eb3ae05d2f7c791c8b0f6f4b69d4a5bb8d423c3116d9765c53ecc",
    "artifacts/WO-M2-04/failure-atomicity.json": "44f644bcc4a279be597243ab843eafbfa104cafc9b4fcf7a5abd8fa974cf5ca0",
    "artifacts/WO-M2-03/pr-smoke.json": "551fecd5b139ab58c6a035ba933d198dcc88fc33330a9fe6dbfd45a2017931dd",
    "artifacts/WO-M2-03/scheduled-campaign.json": "746031ac58e248c2a2d4590ae11e839f86eaa9dfb7948b8f37e0e54afda0f2c2",
    "artifacts/WO-M2-05/fuzz-report.json": "c544147e95da12bb267e68af98f381ebe6b90a69a0773ab73ff00fd5d110e37c",
    "bench/results/WO-BOOT-08.json": "a9e63e81454ad47c9d223cc72169925de722f8917ea6c4fdd6625a20288a1506",
    "artifacts/WO-BOOT-08/regression-run-01.json": "65a18aacf615ae026940afda93a8c7762935432b2f381af80721359da24afe05",
    "artifacts/WO-BOOT-08/regression-run-02.json": "a9e63e81454ad47c9d223cc72169925de722f8917ea6c4fdd6625a20288a1506",
    "artifacts/WO-BOOT-08/gate-contract.json": "bc797fad33ede0c898d040deb06fdda0edd2ba19b48b3ad41321d7f055c96f2c",
    "artifacts/WO-M3-04/navigation-tests.json": "0dfe40c8f02a4243a3b8d7ba87a514082efd50a3e691bb1df6d61d44f0ec34ee",
    "artifacts/WO-M3-04/navigation-golden-manifest.json": "311a9c14328193a87435c7a0b881defd9fce23ba3d7a7a294185ed87d9b3b887",
    "artifacts/WO-M3-04/semantics.json": "223b9456dd000f127e4b2a077c92fe4bd6796fbbb7b4b091a5d2191574d51648",
    "artifacts/WO-M3-04/navigation-frame-0000.png": "f6221b0a17dc1ad5cf6675f8ea1f455c99d13226021f94461af1b818a33348a9",
    "artifacts/WO-M3-04/navigation-frame-0001.png": "642b9b9640e6504ec05ce2642a01b9bf85ff0e891afade7361c00135c977f0fc",
    "artifacts/WO-M3-04/semantics-0000.json": "b1fe92734296ac8dd2463df365dea1b5a19b195e9e8eb2d5413c293348ad6506",
    "artifacts/WO-M3-04/semantics-0001.json": "556b4301f050e285ea443c2a1adb5de7ac059e2a9cd8e7d068929e0f8897fe9c",
    "artifacts/WO-M3-04/semantics-0002.json": "8f09bbde0cbb58a11696b05b08a3046bbc9e5e071220debb1edb88abdd521c42",
    "docs/PERF.md": "7e9e6ec7ebb3840b640154cda264560c281f686708efc2ee69ae0c752f20cb80",
    "docs/adr/0001-product-and-platform-locks.md": "143ecfd9821b69b23d732eb774b576192c82eec10562a8942e1a454158429ed0",
    "docs/adr/0006-m1-04-owner-quiescence.md": "45f9e99b64d3184fc7fce5e9a6823b0fd758fd779b44ae6d1a9b120c7a9f31fa",
    "work/WO-M3-01.md": "54bd664e5061aede7a133b07869e96f9401027cc7f404b86a11ff24b8230e748",
    "work/WO-M3-02.md": "d2547fe2a339e20a7e1c01c7468010452750ee73166c494699a0a130af03bbdf",
    "work/WO-M3-03.md": "4f191c263623fe02d8ef51c7ac2a71ed80ee6fef94b00a0a7055510b21de88fe",
    "work/WO-M3-04.md": "9fd77fed0650fb718b209626b43641b99fbadf9654fbe6488555f0206289a0db",
    "work/WO-BOOT-08.md": "f7fbc767f475c0a4b264987bdd54719e8e87e4910fee4e04bffef12f5790c98f",
}

M0_EXTRA_PATHS = (
    "bench/corpus/manifest.json", "bench/corpus/manifest.sha256",
    "tests/golden/manifest.lock", "docs/corpus/PROVENANCE.md",
    "bench/results/WO-M0-02.json", "artifacts/WO-M0-02/oracle-render.txt",
    "artifacts/WO-M0-03/verify.txt", "artifacts/WO-M0-01/provenance-check.txt",
    "artifacts/WO-M0-02/verify.txt", "artifacts/WO-M0-03/local-checks.txt",
)
M0_EXTRA_HASHES = {
    "bench/corpus/manifest.json": "a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd",
    "bench/corpus/manifest.sha256": "6afa2d4c611c4ce9f5986bdb354ee92163fa8a1808ecbd6dd1408e8d8ae45f33",
    "tests/golden/manifest.lock": "a9fb0207a5dbd2ff833b090d3d49fef5c960a56bd1a68809a52430c4382bc1c4",
    "docs/corpus/PROVENANCE.md": "f803ceaabd93411d07fad0d6641854f4ebe91f8a76d4fb2e7642efb93f0c97e2",
    "bench/results/WO-M0-02.json": "92ec0bb6ca6de2ec071b87f769c8f5095a9db3c55f99dd1669eb3b6bbd3de71d",
    "artifacts/WO-M0-02/oracle-render.txt": "403fe194d37ea9cb95ff88604c98593806ea4ba0a7bcc6626851f9277a4705dd",
    "artifacts/WO-M0-03/verify.txt": "a659a8a8025b74a2d79790328f1de6e2810091cbb52588b4bce3132c27919ee4",
    "artifacts/WO-M0-01/provenance-check.txt": "11953a172b5b7ef8fcd54da3016f1e03a5f7079c8285dbd1fce6f7322f396d4f",
    "artifacts/WO-M0-02/verify.txt": "5cd9978d2c98cbac1adec83f89064707848b99631863c0c67c0f87409e363167",
    "artifacts/WO-M0-03/local-checks.txt": "5c69b7e464f2574cb51382f8fe98837885807c618ddcb3e14bc333a6ec889ada",
}


def sha(path: str) -> str:
    return hashlib.sha256((ROOT / path).read_bytes()).hexdigest()


def load(path: str):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def source_list() -> list[dict]:
    paths = dict(PINNED)
    m0 = load("reports/M0/evidence.json")
    for src in m0["sources"]:
        paths[src["path"]] = src["sha256"]
    for path in M0_EXTRA_PATHS:
        if path not in paths:
            paths[path] = M0_EXTRA_HASHES[path]
    return [{"path": p, "sha256": h} for p, h in sorted(paths.items())]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_inputs() -> list[dict]:
    m0 = load("reports/M0/evidence.json")
    # M0's index is itself pinned, and every nested source is verified as bytes.
    for path, expected in PINNED.items():
        require((ROOT / path).is_file(), f"missing source: {path}")
        require(sha(path) == expected, f"changed source hash: {path}")
    for path, expected in M0_EXTRA_HASHES.items():
        require((ROOT / path).is_file(), f"missing source: {path}")
        require(sha(path) == expected, f"changed source hash: {path}")
    for src in m0["sources"]:
        require(sha(src["path"]) == src["sha256"], f"M0 source hash mismatch: {src['path']}")
    return source_list()


def build_packet() -> dict:
    sources = validate_inputs()
    m0 = load("reports/M0/evidence.json")
    m03 = load("bench/results/WO-M0-03.json")
    perf = load("bench/results/WO-BOOT-08.json")
    runs = [load("artifacts/WO-BOOT-08/regression-run-01.json"),
            load("artifacts/WO-BOOT-08/regression-run-02.json")]
    roundtrip = load("artifacts/WO-M2-04/roundtrip.json")
    atomic = load("artifacts/WO-M2-04/failure-atomicity.json")
    fuzz_xml = [load("artifacts/WO-M2-03/pr-smoke.json"),
                load("artifacts/WO-M2-03/scheduled-campaign.json")]
    fuzz_shape = load("artifacts/WO-M2-05/fuzz-report.json")
    nav = load("artifacts/WO-M3-04/navigation-tests.json")
    gold = load("artifacts/WO-M3-04/navigation-golden-manifest.json")
    semantics = load("artifacts/WO-M3-04/semantics.json")
    gate = perf["gate"]
    require(len(runs) == 2 and all(r["trial_protocol"]["cold_order_trials"] == 5 and
                                  r["trial_protocol"]["warm_order_trials"] == 10 for r in runs),
            "BOOT-08 must contain the final two 5+10 trial runs")
    require(perf == runs[-1], "canonical BOOT-08 result differs from final recorded run")
    require([r["gate"]["status"] for r in runs] == ["warning", "fail"], "BOOT-08 gate sequence changed")
    require([r["gate"]["metrics"]["100MiB.peak_rss_bytes"]["consecutive_violations"] for r in runs] == [1, 2],
            "BOOT-08 repeated RSS regression history changed")
    require(m03["trials"] == [] and m03["summary"] == [] and not m03["availability"][0]["available"],
            "M0-03 availability/trial record changed")
    require(len(roundtrip["records"]) == 3 and all(r["matching_sha256"] for r in roundtrip["records"]),
            "M2-04 roundtrip equality changed")
    require(len(atomic["records"]) == 4 and all(r["unchanged"] and not r["temporary_files"] for r in atomic["records"]),
            "M2-04 failure atomicity changed")
    require(len(nav["checks"]) == 11 and all(v == "passed" for v in nav["checks"].values()),
            "M3-04 navigation check set changed")
    require(nav["projection_workload"]["projection_count"] == 1_000_000, "projection count changed")

    case_summary = {}
    for name, case in perf["cases"].items():
        summary = case["summary"]
        case_summary[name] = {
            "first_preview_p95_ns": summary["first_preview_p95_ns"],
            "full_index_median_ns": summary["complete_index_median_ns"],
            "maximum_step_ns": summary["max_step_ns"],
            "peak_rss_bytes": summary["peak_rss_bytes"],
            "trial_counts": {"cold_order": len(case["cold_order_trials"]), "warm_order": len(case["warm_order_trials"])},
            "progressive_preview_before_completion": all(t["preview_before_index_complete"] for t in case["cold_order_trials"] + case["warm_order_trials"]),
        }
    metrics = {}
    for key, value in gate["metrics"].items():
        metrics[key] = {k: value[k] for k in ("value", "status", "consecutive_violations", "absolute_limit", "baseline_value", "regression_limit", "tolerance_fraction")}
    packet = {
        "schema_version": 1,
        "sources": sources,
        "hardware_toolchain": perf["hardware_toolchain"],
        "progressive_index": {
            "cases": case_summary,
            "protocol": perf["trial_protocol"],
            "gate_result": gate["status"],
            "baseline_commit": runs[-1]["baseline_provenance"]["commit"],
            "baseline_report_sha256": runs[-1]["baseline_provenance"]["report_sha256"],
        },
        "performance_gate": {
            "run_timestamps_utc": [r["run_timestamp_utc"] for r in runs],
            "per_run": [{"timestamp_utc": r["run_timestamp_utc"], "overall_status": r["gate"]["status"],
                         "metrics": {k: {x: v[x] for x in ("value", "status", "consecutive_violations")} for k, v in r["gate"]["metrics"].items()}}
                        for r in runs],
            "final_metrics": metrics,
            "absolute_limits": gate["absolute_limits"],
            "relative_regression_note": "100 MiB peak RSS is below the 500 MiB absolute cap, but fails the 5% relative baseline gate after two comparable violations.",
            "unavailable": {
                "pan_zoom_event_to_present_p95_ns": perf["stretch_targets"]["pan_zoom_event_to_present_p95_ns"],
                "app_bundle_bytes": perf["stretch_targets"]["app_bundle"],
            },
        },
        "m2_04_source_preservation": {
            "byte_equal_cases": len(roundtrip["records"]),
            "byte_equal_all": all(r["matching_sha256"] for r in roundtrip["records"]),
            "failure_atomicity_cases": len(atomic["records"]),
            "failure_atomicity_all": all(r["unchanged"] and not r["temporary_files"] for r in atomic["records"]),
            "latency": {"value": None, "reason": "M2-04 records byte equality and failure atomicity; no save latency measurement was made."},
        },
        "fuzz_and_sanitizers": {
            "m2_03_xml_campaigns": [{"duration_seconds": r["duration_seconds"], "iterations": r["iterations"], "status": r["status"], "sanitizer_output": r["sanitizer_output"]} for r in fuzz_xml],
            "m2_05_shape_fuzz": {k: fuzz_shape[k] for k in ("cases", "crashes", "invariant_failures", "sanitizer_reports", "sanitizer_stderr_bytes", "result", "sanitizer")},
            "sanitizer_ctest": "Debug/ASan/UBSan acceptance evidence is recorded in source work-order artifacts; these fuzz campaigns reported no sanitizer output.",
        },
        "viewer_navigation": {
            "checks": nav["checks"], "projection_count": nav["projection_workload"]["projection_count"],
            "frames": nav["projection_workload"]["frames"], "scene_items_per_frame": nav["projection_workload"]["scene_items_per_frame"],
            "checksum_fnv1a64": nav["projection_workload"]["checksum_fnv1a64"],
            "screenshots": [{"path": f"artifacts/WO-M3-04/{p}", "sha256": h} for p, h in sorted(gold["rendered_screen_files"].items())],
            "semantic_snapshots": [{"path": f"artifacts/WO-M3-04/{p}", "sha256": h} for p, h in sorted(gold["legacy_files"].items()) if p.startswith("semantics-")],
            "presentation_timing": {"value": None, "reason": perf["stretch_targets"]["pan_zoom_event_to_present_p95_ns"]["reason"]},
        },
        "competitors": {
            "available": [x["id"] for x in m03["availability"] if x["available"]],
            "unavailable": [{"name": x["id"], "reason": x["reason"]} for x in m03["availability"] if not x["available"]],
            "render_trials": len(m03["trials"]), "compatible_vexel_render": m03["vexel_version_probe"]["render_available"],
            "vexel_reason": m03["vexel_version_probe"]["render_unavailable_reason"],
        },
        "synthetic_corpus": {
            "fixtures": m0["counts"]["synthetic_corpus_fixtures"], "goldens": m0["counts"]["pinned_golden_fixtures"],
            "limits": ["Engineering coverage only; not customer evidence.", "Unknown representativeness; 24 goldens are not an SVG conformance claim.", "Generated stress documents and oracle-render timing are not Vexel rendering performance."],
        },
        "owner_user_pilot_counts": {
            "owner_files_and_tasks": m0["counts"]["owner_provided_files_and_tasks"],
            "interviews": m0["counts"]["recorded_interviews"], "user_outcomes": m0["counts"]["user_outcome_records"],
            "pilots": 0, "pilot_count_basis": "M0 product evidence states no owner responses or user outcomes are available; no pilot record is cited.",
            "real_files_tasks_requested": m0["counts"]["requested_owner_examples"],
        },
        "known_failures_and_gaps": [
            "Final BOOT-08 gate fails on repeated relative 100 MiB RSS regression; its absolute RSS threshold passes.",
            "Pan event-to-present p95 and app-bundle size are unavailable with explicit reasons.",
            "M0-03 has no competitor render trials; Inkscape and Donner are unavailable and Vexel has no compatible SVG render command.",
            "M0–M3 uses synthetic engineering coverage; no owner files, tasks, interviews, user outcomes, or pilots are recorded.",
            "The viewer review material is static screenshots and semantic snapshots plus scripted checks; no live customer pilot is claimed.",
        ],
        "confidence_and_limits": {
            "measured_claims": "High confidence for counts and measurements that are directly recorded in hash-pinned source artifacts; all source bytes and derived values are checked locally.",
            "customer_demand": "No evidence; zero owner/user/pilot records.",
            "generalization": "Measurements describe the named Apple M5 Max / macOS 27.0 / Apple Clang 21.0.0 environment and synthetic workloads only.",
            "product_scope": "Apple Silicon macOS 13+ target; no claim of complete SVG compatibility.",
        },
        "owner_decision": {
            "status": "open", "choice": None,
            "options": [
                {"choice": "proceed", "implication": "Authorize M4/M5 planning after reviewing real representative tasks; technical progress alone does not validate demand."},
                {"choice": "pause", "implication": "Hold M4/M5 until five representative files/tasks and outcomes are reviewed; preserves the current evidence boundary."},
                {"choice": "pivot", "implication": "Redirect scope based on owner-provided tasks; requires evidence identifying a better target workflow."},
                {"choice": "stop", "implication": "End product work if the evidence does not justify further investment; retain the packet for the decision record."},
            ],
            "request": "Please provide five representative real files/tasks, each with the task, expected successful result, and any sensitivity or retention limits.",
        },
    }
    validate_schema(packet)
    return packet


def validate_schema(p: dict) -> None:
    expected = {"schema_version", "sources", "hardware_toolchain", "progressive_index", "performance_gate", "m2_04_source_preservation", "fuzz_and_sanitizers", "viewer_navigation", "competitors", "synthetic_corpus", "owner_user_pilot_counts", "known_failures_and_gaps", "confidence_and_limits", "owner_decision"}
    require(set(p) == expected, "unknown or missing top-level packet fields")
    require(p["schema_version"] == 1, "unsupported schema version")
    require(p["owner_decision"]["status"] == "open" and p["owner_decision"]["choice"] is None, "human decision must remain open")
    require([o["choice"] for o in p["owner_decision"]["options"]] == ["proceed", "pause", "pivot", "stop"], "owner options changed")
    require(p["owner_user_pilot_counts"]["real_files_tasks_requested"] == 5, "five-item request missing")
    require(p["owner_user_pilot_counts"]["owner_files_and_tasks"] == p["owner_user_pilot_counts"]["interviews"] == p["owner_user_pilot_counts"]["user_outcomes"] == p["owner_user_pilot_counts"]["pilots"] == 0, "M0 evidence counts changed")
    require(p["performance_gate"]["final_metrics"]["100MiB.peak_rss_bytes"]["status"] == "fail", "repeated relative RSS gate failure missing")
    require(p["performance_gate"]["final_metrics"]["100MiB.peak_rss_bytes"]["value"] < p["performance_gate"]["final_metrics"]["100MiB.peak_rss_bytes"]["absolute_limit"], "RSS absolute/relative distinction inconsistent")
    for item in p["sources"]:
        require(set(item) == {"path", "sha256"} and re.fullmatch(r"[0-9a-f]{64}", item["sha256"]), "invalid source entry")


def render_markdown(p: dict) -> str:
    gate = p["performance_gate"]
    lines = [
        "# M3 viewer proof and owner decision", "",
        "> Review material is the recorded screenshots, semantic snapshots, and scripted checks linked below. This packet does not claim a live customer pilot.", "",
        "## Viewer review", "",
        "- [Navigation frame 1](../../artifacts/WO-M3-04/navigation-frame-0000.png)",
        "- [Navigation frame 2](../../artifacts/WO-M3-04/navigation-frame-0001.png)",
        "- [Semantic snapshot: active progress](../../artifacts/WO-M3-04/semantics-0000.json)",
        "- [Semantic snapshot: cancel request](../../artifacts/WO-M3-04/semantics-0001.json)",
        "- [Semantic snapshot: terminal state](../../artifacts/WO-M3-04/semantics-0002.json)",
        "- [Navigation checks and projection evidence](../../artifacts/WO-M3-04/navigation-tests.json)", "",
        "## Recorded evidence", "",
        f"- Hardware/compiler: {p['hardware_toolchain']['machine_model']}, {p['hardware_toolchain']['chip']}, {p['hardware_toolchain']['os']}, {p['hardware_toolchain']['compiler']}.",
        f"- BOOT-08 protocol: {p['progressive_index']['protocol']['cold_order_trials']} cold-order + {p['progressive_index']['protocol']['warm_order_trials']} warm-order runs per size. Cold/warm labels are run order; filesystem cache was not flushed.",
        "- Final BOOT-08 values (p95 first preview; median full index; maximum step; peak RSS):", "",
        "| Case | First preview p95 | Full index median | Max step | Peak RSS |",
        "|---|---:|---:|---:|---:|",
    ]
    for case, v in p["progressive_index"]["cases"].items():
        lines.append(f"| {case} | {v['first_preview_p95_ns']} ns | {v['full_index_median_ns']} ns | {v['maximum_step_ns']} ns | {v['peak_rss_bytes']} bytes |")
    lines += ["", f"- Overall final gate: **{p['progressive_index']['gate_result']}**. Final 100 MiB RSS is {gate['final_metrics']['100MiB.peak_rss_bytes']['value']} bytes against an absolute limit of {gate['final_metrics']['100MiB.peak_rss_bytes']['absolute_limit']} bytes: the absolute limit passes, while the relative 5% baseline gate fails after two consecutive violations.",
              f"- Pan timing: unavailable — {gate['unavailable']['pan_zoom_event_to_present_p95_ns']['reason']}",
              f"- App bundle: unavailable — {gate['unavailable']['app_bundle_bytes']['reason']}",
              f"- M2-04: {p['m2_04_source_preservation']['byte_equal_cases']} byte-equal round trips and {p['m2_04_source_preservation']['failure_atomicity_cases']} failure-atomicity cases passed.",
              f"- Fuzz: M2-03 campaigns completed {p['fuzz_and_sanitizers']['m2_03_xml_campaigns'][0]['iterations']} and {p['fuzz_and_sanitizers']['m2_03_xml_campaigns'][1]['iterations']} cases; M2-05 completed {p['fuzz_and_sanitizers']['m2_05_shape_fuzz']['cases']} cases with {p['fuzz_and_sanitizers']['m2_05_shape_fuzz']['crashes']} crashes and {p['fuzz_and_sanitizers']['m2_05_shape_fuzz']['sanitizer_reports']} sanitizer reports.",
              f"- Navigation: {len(p['viewer_navigation']['checks'])} recorded checks passed; projection workload {p['viewer_navigation']['projection_count']} items, checksum `{p['viewer_navigation']['checksum_fnv1a64']}`. No pan-to-present timing is claimed.",
              f"- Competitors: Inkscape and Donner unavailable; {p['competitors']['render_trials']} render trials; {p['competitors']['vexel_reason']}.",
              f"- Synthetic corpus: {p['synthetic_corpus']['fixtures']} fixtures and {p['synthetic_corpus']['goldens']} goldens. These are engineering coverage, not customer evidence or complete SVG compatibility.",
              f"- Owner files/tasks: {p['owner_user_pilot_counts']['owner_files_and_tasks']}; interviews: {p['owner_user_pilot_counts']['interviews']}; outcomes: {p['owner_user_pilot_counts']['user_outcomes']}; pilots: {p['owner_user_pilot_counts']['pilots']}.", "",
              "## Owner decision", "", "**Decision: OPEN.** No choice is recorded by this packet.", "",
              p["owner_decision"]["request"], ""]
    for option in p["owner_decision"]["options"]:
        lines.append(f"- **{option['choice']}** — {option['implication']}")
    lines += ["", "## Confidence and limitations", "", p["confidence_and_limits"]["measured_claims"], "", p["confidence_and_limits"]["customer_demand"], "", p["confidence_and_limits"]["generalization"], "", "## Checked source SHA-256", "", "Every packet source below is repository-relative and verified by the offline validator.", "", "| Repository path | SHA-256 |", "|---|---|"]
    lines.extend(f"| `{s['path']}` | `{s['sha256']}` |" for s in p["sources"])
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        packet = build_packet()
        json_bytes = (json.dumps(packet, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()
        md_bytes = render_markdown(packet).encode()
        if args.write:
            OUT_JSON.write_bytes(json_bytes)
            OUT_MD.write_bytes(md_bytes)
            print(f"PASS wrote {OUT_JSON.relative_to(ROOT)} and {OUT_MD.relative_to(ROOT)} ({len(packet['sources'])} checked sources)")
        else:
            require(OUT_JSON.read_bytes() == json_bytes, "evidence.json differs from deterministic source-derived packet")
            require(OUT_MD.read_bytes() == md_bytes, "decision.md differs from deterministic source-derived Markdown")
            json.loads(OUT_JSON.read_text(encoding="utf-8"))
            print(f"PASS checked {len(packet['sources'])} source hashes, derived values, schema, and Markdown consistency")
        return 0
    except (OSError, KeyError, TypeError, ValueError) as exc:
        print(f"FAIL {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
