//! The original 200000-case editor sequences, with their original seeds,
//! branch choices, XML validation, fixed prefix/suffix, and round-trip oracles.
mod common;
use common::*;
use vexel_document_edit::*;
fn random(state: &mut u32) -> u32 {
    let mut v = *state;
    v ^= v << 13;
    v ^= v >> 17;
    v ^= v << 5;
    *state = v;
    v
}
#[test]
fn test_randomized_history_200000() {
    let source = b"<svg><rect x='0'/></svg>";
    let limits = EditLimits {
        history_commands: 200000,
        history_bytes: 4 * 1024 * 1024,
    };
    let mut edit = Editor::create(&std::sync::Arc::new(source.to_vec()), limits)
        .editor
        .unwrap();
    let mut state = 0x9e3779b9u32;
    let mut current = [0u8; 64];
    for _step in 0..200000 {
        let choice = random(&mut state) % 3;
        if choice == 0
            || (choice == 1 && edit.history_cursor() == 0)
            || (choice == 2 && edit.history_cursor() == edit.history_count())
        {
            let value = b'0' + (random(&mut state) % 10) as u8;
            edit.validate_replace(
                SourceSpan {
                    offset: 5,
                    length: 13,
                },
                14,
                1,
                &[value],
                false,
                true,
            )
            .unwrap();
            edit.commit_replace(14, 1, &[value], 1, false, HistoryAction::EditAttribute)
                .unwrap();
        } else if choice == 1 {
            edit.undo().unwrap();
        } else {
            edit.redo().unwrap();
        }
        let length = edit.materialize(&mut current).unwrap();
        let snapshot = &current[..length];
        valid(snapshot);
        assert_eq!(length, source.len());
        assert_eq!(&snapshot[..14], &source[..14]);
        assert_eq!(&snapshot[15..], &source[15..]);
        assert!(edit.history_bytes() <= limits.history_bytes);
        assert_eq!(
            edit.allocation_stats().cache_bytes,
            0,
            "raw source states do not rebuild query cache"
        );
    }
    while edit.history_cursor() < edit.history_count() {
        edit.redo().unwrap();
    }
    let final_bytes = bytes(&edit);
    while edit.history_cursor() > 0 {
        edit.undo().unwrap();
    }
    assert_eq!(bytes(&edit), source);
    valid(&bytes(&edit));
    while edit.history_cursor() < edit.history_count() {
        edit.redo().unwrap();
    }
    assert_eq!(bytes(&edit), final_bytes);
    valid(&bytes(&edit));
    assert!(edit.history_bytes() <= limits.history_bytes);
}
#[test]
fn test_randomized_xml_fragments_200000() {
    let mut edit = create(b"<svg/>");
    let mut seed = 0x57465835u32;
    let (mut accepted, mut malformed, mut rejected) = (0usize, 0usize, 0usize);
    for _iteration in 0..200000 {
        let choice = random(&mut seed) % 5;
        let value = random(&mut seed);
        let (markup, expected) = match choice {
            0 => (format!("<rect data-v='{value}'/>"), EditError::Ok),
            1 => (format!("<!--c--><circle id='r{value}'/>"), EditError::Ok),
            2 => (format!("<g><text>{value}</text></g>"), EditError::Ok),
            3 => (
                format!("<rect><circle>{value}</rect>"),
                EditError::MalformedXml,
            ),
            _ => ("<rect/><circle/>".into(), EditError::InvalidOperation),
        };
        assert!(!markup.is_empty() && markup.len() < 128);
        let root = edit.root().unwrap();
        let result = edit.insert_xml_fragment(root, markup.as_bytes());
        match result {
            Ok(_) => {
                assert_eq!(expected, EditError::Ok);
                accepted += 1;
                edit.undo().unwrap();
            }
            Err(error) => {
                assert_eq!(error, expected);
                if error == EditError::MalformedXml {
                    malformed += 1;
                } else {
                    rejected += 1;
                }
            }
        }
        assert_eq!(edit.byte_length(), 6);
        assert!(!edit.is_dirty());
    }
    assert_eq!(bytes(&edit), b"<svg/>");
    assert!(accepted > 0 && malformed > 0 && rejected > 0);
    eprintln!(
        "fragment cases: 200000 seed 0x57465835 accepted={accepted} malformed={malformed} rejected={rejected}"
    );
}
