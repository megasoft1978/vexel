use vexel_json::{
    JsonBuffer, JsonBufferError, JsonDocument, JsonError, JsonType, MAX_BYTES, MAX_DEPTH,
    MAX_TOKENS,
};

#[test]
fn parses_ordered_containers_duplicate_keys_and_raw_spans() {
    let source = br#" {"a":[true,false,null,12],"s":"x\uD83D\uDE00\n","a":0} "#;
    let doc = JsonDocument::parse(source).unwrap();
    assert_eq!(doc.root(), 0);
    assert_eq!(doc.token(doc.root()).unwrap().kind, JsonType::Object);
    assert_eq!(doc.token(doc.root()).unwrap().child_count, 6);
    let first_a = doc.object_get(doc.root(), b"a").unwrap();
    assert_eq!(doc.token(first_a).unwrap().kind, JsonType::Array);
    assert_eq!(
        doc.array_get(first_a, 0).and_then(|i| doc.bool(i)),
        Some(true)
    );
    assert_eq!(
        doc.array_get(first_a, 1).and_then(|i| doc.bool(i)),
        Some(false)
    );
    assert_eq!(
        doc.array_get(first_a, 2)
            .and_then(|i| doc.token(i))
            .unwrap()
            .kind,
        JsonType::Null
    );
    assert_eq!(
        doc.array_get(first_a, 3).and_then(|i| doc.uint(i)),
        Some(12)
    );
    let string = doc.object_get(doc.root(), b"s").unwrap();
    assert_eq!(doc.decoded_string(string).unwrap(), b"x\xf0\x9f\x98\x80\n");
    assert_eq!(
        doc.raw_token(doc.root()).unwrap(),
        br#"{"a":[true,false,null,12],"s":"x\uD83D\uDE00\n","a":0}"#
    );
    assert_eq!(doc.object_get(doc.root(), b"a"), Some(first_a));
    assert_eq!(doc.array_get(first_a, 99), None);
    let catalog = format!("[{}]", vec!["{}"; 24].join(","));
    let catalog_doc = JsonDocument::parse(catalog.as_bytes()).unwrap();
    assert_eq!(
        catalog_doc.token(catalog_doc.root()).unwrap().kind,
        JsonType::Array
    );
    assert!(catalog_doc.token(catalog_doc.root()).unwrap().child_count >= 20);
}

#[test]
fn strict_malformed_and_utf8_cases_match_reference_parser() {
    for source in [
        b"".as_slice(),
        b"{",
        b"[1,]",
        br#"{"a" 1}"#,
        b"01",
        b"1e",
        br#""\uD800""#,
        br#""\uDC00""#,
        br#""\x""#,
        b"true false",
        b"[NaN]",
    ] {
        assert!(JsonDocument::parse(source).is_err(), "accepted {source:?}");
    }
    for source in [
        &b"\"\xc0\xaf\""[..],
        &b"\"\xed\xa0\x80\""[..],
        &b"\"\xf4\x90\x80\x80\""[..],
    ] {
        assert_eq!(JsonDocument::parse(source), Err(JsonError::Invalid));
    }
    assert!(
        JsonDocument::parse(b"\"nul\\u0000byte\"")
            .unwrap()
            .decoded_string(0)
            .unwrap()
            .contains(&0)
    );
    assert_eq!(
        JsonDocument::parse(br#""line\n\t\/\\\"\u00e9""#)
            .unwrap()
            .decoded_string(0)
            .unwrap(),
        "line\n\t/\\\"é".as_bytes()
    );
}

#[test]
fn numbers_obey_json_grammar_and_uint_is_checked_decimal_only() {
    for number in [b"-0".as_slice(), b"0.0", b"1e+2", b"-3E-4"] {
        let doc = JsonDocument::parse(number).unwrap();
        assert_eq!(doc.token(doc.root()).unwrap().kind, JsonType::Number);
        assert_eq!(doc.uint(doc.root()), None);
    }
    let doc = JsonDocument::parse(b"18446744073709551615").unwrap();
    assert_eq!(doc.uint(doc.root()), Some(u64::MAX));
    for number in [b"18446744073709551616".as_slice(), b"-1", b"1.0", b"1e2"] {
        let doc = JsonDocument::parse(number).unwrap();
        assert_eq!(doc.uint(doc.root()), None);
    }
    let doc = JsonDocument::parse(b"[true,false,null,0]").unwrap();
    assert_eq!(doc.bool(doc.array_get(doc.root(), 0).unwrap()), Some(true));
    assert_eq!(doc.bool(doc.array_get(doc.root(), 1).unwrap()), Some(false));
    assert_eq!(doc.bool(doc.array_get(doc.root(), 2).unwrap()), None);
}

#[test]
fn depth_token_and_byte_limits_are_enforced() {
    let accepted = format!("{}0{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert!(JsonDocument::parse(accepted.as_bytes()).is_ok());
    let rejected = format!("[{}0{}]", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert_eq!(
        JsonDocument::parse(rejected.as_bytes()),
        Err(JsonError::TooDeep)
    );
    let too_many = format!("[{}]", vec!["0"; MAX_TOKENS].join(","));
    assert_eq!(
        JsonDocument::parse(too_many.as_bytes()),
        Err(JsonError::TooManyTokens)
    );
    assert_eq!(
        JsonDocument::parse(&vec![b' '; MAX_BYTES + 1]),
        Err(JsonError::TooLarge)
    );
    assert_eq!(JsonDocument::parse_owned(b"[1]").unwrap().source(), b"[1]");
}

#[test]
fn buffer_quotes_raw_tokens_and_rolls_back_failed_appends() {
    assert_eq!(JsonBuffer::new(3, 2), Err(JsonBufferError::Limit));
    assert_eq!(JsonBuffer::new(0, 0), Err(JsonBufferError::Limit));
    let mut output = JsonBuffer::new(1, 64).unwrap();
    output.literal("prefix:").unwrap();
    let checkpoint = output.as_bytes().to_vec();
    assert_eq!(output.append(&[0; 100]), Err(JsonBufferError::Limit));
    assert_eq!(output.as_bytes(), checkpoint);
    output.append_within(0..3).unwrap();
    assert!(output.as_bytes().ends_with(b"pre"));
    assert_eq!(
        output.append_within(0..usize::MAX),
        Err(JsonBufferError::InvalidToken)
    );
    assert_eq!(
        output
            .quoted(b"\"\\\x08\x0c\n\r\t\x00\x1f\xc3\xa9")
            .unwrap(),
        ()
    );
    assert_eq!(
        &output.as_bytes()[10..],
        b"\"\\\"\\\\\\b\\f\\n\\r\\t\\u0000\\u001f\xc3\xa9\""
    );
    let doc = JsonDocument::parse(br#"{"raw": [1, true]}"#).unwrap();
    let array = doc.object_get(doc.root(), b"raw").unwrap();
    output.raw_token(&doc, array).unwrap();
    assert!(output.as_bytes().ends_with(b"[1, true]"));
    let prior = output.as_bytes().to_vec();
    let mut tiny = JsonBuffer::new(0, 2).unwrap();
    assert_eq!(tiny.quoted(b"x"), Err(JsonBufferError::Limit));
    assert!(tiny.is_empty());
    assert_eq!(output.as_bytes(), prior);
    assert_eq!(
        output.raw_token(&doc, usize::MAX),
        Err(JsonBufferError::InvalidToken)
    );
}
