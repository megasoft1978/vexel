//! Manifest and pin loading with the SHA-256 guard. Hashing shells out to
//! `/usr/bin/shasum -a 256` (no hashing dependency).

use crate::jsonx::Json;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use vexel_json::JsonDocument;

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub tier: String,
    pub kind: String,
    /// Size claimed by the manifest, when it has one.
    pub bytes: Option<u64>,
    pub sha256: String,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct Manifest {
    pub sha256: String,
    pub entries: Vec<Entry>,
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let out = Command::new("/usr/bin/shasum")
        .args(["-a", "256", "--"])
        .arg(path)
        .output()
        .map_err(|e| format!("shasum: {e}"))?;
    if !out.status.success() {
        return Err(format!("shasum failed for {}", path.display()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let hash = text.split_whitespace().next().unwrap_or("");
    if hash.len() == 64 {
        Ok(hash.to_string())
    } else {
        Err("unexpected shasum output".to_string())
    }
}

/// Accepted manifest digests from `pins.json` (`{"pins":[{"sha256":..}]}`).
pub fn load_pins(path: &Path) -> Result<Vec<String>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("pins {}: {e}", path.display()))?;
    let doc = JsonDocument::parse(&bytes).map_err(|e| format!("pins JSON: {e:?}"))?;
    let j = Json::new(&doc);
    let list = j.get(j.root(), "pins").ok_or("pins JSON has no \"pins\"")?;
    let pins: Vec<String> = j
        .elements(list)
        .into_iter()
        .filter_map(|p| j.str_at(p, "sha256"))
        .collect();
    if pins.is_empty() {
        return Err("pins JSON holds no sha256 values".to_string());
    }
    Ok(pins)
}

fn safe_relative(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(format!("unsafe manifest path {path:?}"));
    }
    Ok(p.to_path_buf())
}

/// Verifies the manifest digest against the pins, then parses it. A mismatch
/// is an error: the pipeline is never run against an unpinned manifest.
pub fn load_manifest(path: &Path, pins: &[String]) -> Result<Manifest, String> {
    let sha256 = sha256_file(path)?;
    if !pins.iter().any(|p| *p == sha256) {
        return Err(format!(
            "manifest sha256 {sha256} does not match any pin; refusing to run"
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("manifest: {e}"))?;
    let doc = JsonDocument::parse(&bytes).map_err(|e| format!("manifest JSON: {e:?}"))?;
    let j = Json::new(&doc);
    let manifest_dir = path.parent().unwrap_or(Path::new("."));
    let mut entries = Vec::new();
    if let Some(fixtures) = j.get(j.root(), "fixtures") {
        // In-repo form: paths are relative to the repository root.
        let base = manifest_dir
            .parent()
            .and_then(Path::parent)
            .ok_or("in-repo manifest is not under <root>/bench/corpus")?;
        for e in j.elements(fixtures) {
            let id = j
                .str_at(e, "fixture_id")
                .ok_or("fixture without fixture_id")?;
            let rel = j.str_at(e, "path").ok_or("fixture without path")?;
            entries.push(Entry {
                id,
                tier: "-".to_string(),
                kind: j.str_at(e, "source_origin").unwrap_or_default(),
                bytes: None,
                sha256: j.str_at(e, "sha256").ok_or("fixture without sha256")?,
                path: base.join(safe_relative(&rel)?),
            });
        }
    } else {
        // External form: a top-level array, paths relative to the manifest.
        for e in j.elements(j.root()) {
            let id = j.str_at(e, "id").ok_or("entry without id")?;
            let rel = j.str_at(e, "path").ok_or("entry without path")?;
            entries.push(Entry {
                id,
                tier: j.str_at(e, "tier").unwrap_or_default(),
                kind: j.str_at(e, "kind").unwrap_or_default(),
                bytes: j.u64_at(e, "bytes"),
                sha256: j.str_at(e, "sha256").ok_or("entry without sha256")?,
                path: manifest_dir.join(safe_relative(&rel)?),
            });
        }
    }
    if entries.is_empty() {
        return Err("manifest has no entries".to_string());
    }
    Ok(Manifest { sha256, entries })
}
