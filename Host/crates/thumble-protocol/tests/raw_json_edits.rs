use thumble_protocol::{edit_json_values, JsonEditError, JsonValueEdit};
fn set<'a>(pointer: &'a str, value: &'a [u8]) -> JsonValueEdit<'a> {
    JsonValueEdit {
        pointer,
        value: Some(value),
    }
}
fn remove(pointer: &str) -> JsonValueEdit<'_> {
    JsonValueEdit {
        pointer,
        value: None,
    }
}

#[test]
fn only_the_addressed_value_bytes_change_including_escaped_and_unicode_keys() {
    let source = br#" { "metadata": {"large":18446744073709551616, "decimal":1.234567890123456789, "escaped":"\u0061"}, "value" : 1 } "#;
    let result = edit_json_values(source, &[set("/value", b"2")], 4096).unwrap();
    let expected = std::str::from_utf8(source)
        .unwrap()
        .replace("\"value\" : 1", "\"value\" : 2");
    assert_eq!(result, expected.as_bytes());
    let source = r#"{"a\/b~c":1,"e\u0301":2,"":3}"#.as_bytes();
    let result = edit_json_values(
        source,
        &[set("/a~1b~0c", b"4"), set("/é", b"5"), set("/", b"6")],
        4096,
    )
    .unwrap();
    assert_eq!(result, r#"{"a\/b~c":4,"e\u0301":5,"":6}"#.as_bytes());
}
#[test]
fn object_add_remove_first_middle_last_and_only_member_preserve_other_values() {
    for (path, expected) in [
        ("/a", r#"{  "b" : 2, "c":3 }"#),
        ("/b", r#"{ "a":1,  "c":3 }"#),
        ("/c", r#"{ "a":1, "b" : 2 }"#),
    ] {
        assert_eq!(
            edit_json_values(br#"{ "a":1, "b" : 2, "c":3 }"#, &[remove(path)], 4096).unwrap(),
            expected.as_bytes()
        );
    }
    assert_eq!(
        edit_json_values(br#"{ "a":1 }"#, &[remove("/a")], 4096).unwrap(),
        b"{  }"
    );
    let empty = edit_json_values(b"{\n}", &[set("/a", b"1"), set("/b", b"2")], 4096).unwrap();
    assert_eq!(empty, b"{\"a\":1,\"b\":2\n}");
    assert_eq!(
        edit_json_values(
            br#"{"a":1}"#,
            &[set("/new", br#"{"nested":[true,null]}"#)],
            4096
        )
        .unwrap(),
        br#"{"a":1,"new":{"nested":[true,null]}}"#
    );
}
#[test]
fn ordered_array_edits_use_updated_indices_and_append_without_normalizing_values() {
    let source = br#"{"array":[ 1.00, "\u0061", 3 ]}"#;
    let result = edit_json_values(
        source,
        &[
            remove("/array/0"),
            set("/array/1", b"42"),
            set("/array/-", b"43"),
        ],
        4096,
    )
    .unwrap();
    assert_eq!(result, br#"{"array":[  "\u0061", 42,43 ]}"#);
    assert_eq!(
        edit_json_values(br#"[1, 2, 3]"#, &[remove("/2")], 4096).unwrap(),
        b"[1, 2]"
    );
    assert_eq!(
        edit_json_values(br#"[1]"#, &[remove("/0"), set("/0", b"2")], 4096).unwrap(),
        b"[2]"
    );
}
#[test]
fn malformed_ambiguous_missing_and_oversized_edits_return_no_partial_document() {
    let source = br#"{"a":[1]}"#;
    for path in ["", "a", "/a/~2", "/a/01", "/a/-1", "/a/9", "/a/0/value"] {
        assert!(
            edit_json_values(source, &[set(path, b"2")], 4096).is_err(),
            "{path}"
        );
    }
    assert_eq!(
        edit_json_values(source, &[remove("/absent")], 4096),
        Err(JsonEditError::MissingValue)
    );
    assert_eq!(
        edit_json_values(source, &[set("/new", br#"{"a":1,"\u0061":2}"#)], 4096),
        Err(JsonEditError::InvalidJson)
    );
    assert!(edit_json_values(br#"{"a":1,"a":2}"#, &[], 4096).is_err());
    assert_eq!(
        edit_json_values(source, &[set("/a/0", b"123456789")], source.len()),
        Err(JsonEditError::TooLarge)
    );
    assert!(edit_json_values(source, &[set("/a/0", b"2"), remove("/absent")], 4096).is_err());
    assert_eq!(source, br#"{"a":[1]}"#);
    assert_eq!(
        edit_json_values(source, &vec![set("/a/0", b"1"); 513], 4096),
        Err(JsonEditError::TooManyEdits)
    );
}
