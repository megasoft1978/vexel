//! Byte-identity recorder: composes every fixture and synthetic scene and
//! writes `{name: {size: {verdict, sha256}}}` JSON to `VEXEL_HASH_OUT`.
//! Skipped unless that variable is set; two runs are compared with `diff`.
mod pipeline_common;
use pipeline_common::{
    paths, rects, rects_in_group, rects_with_style, repo_root, sha256_hex, svg_files,
};
use vexel_app::compose::compose_svg;

fn record(bytes: &[u8], size: u32) -> String {
    let outcome = std::panic::catch_unwind(|| compose_svg(bytes, size, size));
    match outcome {
        Ok(Ok(frame)) => {
            let mut all = Vec::with_capacity(frame.rgba.len() + 8);
            all.extend_from_slice(&frame.width.to_le_bytes());
            all.extend_from_slice(&frame.height.to_le_bytes());
            all.extend_from_slice(&frame.rgba);
            let skipped: Vec<String> = frame
                .disclosures
                .iter()
                .map(|d| format!("{}@{:?}", d.feature, d.offset))
                .collect();
            format!(
                "\"ok\", \"sha256\": \"{}\", \"disclosures\": {:?}",
                sha256_hex(&all),
                skipped.join(";")
            )
        }
        Ok(Err(error)) => format!(
            "\"err: {}\", \"sha256\": \"\", \"disclosures\": \"\"",
            format!("{error:?}").replace('"', "'")
        ),
        Err(_) => "\"panic\", \"sha256\": \"\", \"disclosures\": \"\"".to_string(),
    }
}

#[test]
fn record_hashes() {
    let Ok(out) = std::env::var("VEXEL_HASH_OUT") else {
        return;
    };
    let root = repo_root().canonicalize().unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = svg_files(&root, &["rust/fixtures", "bench"])
        .into_iter()
        .map(|path| {
            let name = path.strip_prefix(&root).unwrap().display().to_string();
            (name, std::fs::read(&path).unwrap())
        })
        .collect();
    entries.push(("synthetic/R200".into(), rects(200)));
    entries.push(("synthetic/R200_style".into(), rects_with_style(200)));
    entries.push(("synthetic/R2000".into(), rects(2000)));
    entries.push(("synthetic/R8000".into(), rects(8000)));
    entries.push(("synthetic/P8000".into(), paths(8000)));
    entries.push(("synthetic/S8000".into(), rects_with_style(8000)));
    entries.push(("synthetic/G8000".into(), rects_in_group(8000)));
    let mut text = String::from("{\n");
    for (index, (name, bytes)) in entries.iter().enumerate() {
        let sizes: Vec<String> = [128u32, 512]
            .iter()
            .map(|&size| format!("\"{size}\": {{\"verdict\": {}}}", record(bytes, size)))
            .collect();
        let comma = if index + 1 == entries.len() { "" } else { "," };
        text.push_str(&format!("  \"{name}\": {{{}}}{comma}\n", sizes.join(", ")));
    }
    text.push_str("}\n");
    std::fs::write(out, text).unwrap();
}
