//! Corpus smoke for DOCTYPE internal entities (needs the external real corpus).
use vexel_app::compose::{ComposeError, compose_svg};

/// Run with `--ignored` when the corpus is present. Entity references must
/// no longer fail in the XML layer; the file may still be refused later by
/// the decoder (which compares raw attribute values), which is reported only.
#[test]
#[ignore]
fn columbiarivermap_gets_past_the_xml_layer() {
    let path = std::env::var("VEXEL_CORPUS_COLUMBIA").unwrap_or_else(|_| {
        format!(
            "{}/Desktop/Freelance/vexel-corpus-real/files/real/columbiarivermap-svg.svg",
            std::env::var("HOME").unwrap()
        )
    });
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let result = compose_svg(&bytes, 512, 512);
    eprintln!("columbiarivermap: {:?}", result.as_ref().err());
    if let Err(error) = result {
        assert!(
            !matches!(error, ComposeError::Malformed { detail: "XML", .. })
                && !matches!(
                    error,
                    ComposeError::Unsupported {
                        feature: "DTD entity",
                        ..
                    }
                ),
            "{error:?}"
        );
    }
}
