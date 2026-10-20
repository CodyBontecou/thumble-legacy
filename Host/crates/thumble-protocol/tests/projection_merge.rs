use thumble_protocol::{merge_json_projection, ProjectionMergeError};

fn merge(source: &str, before: &str, after: &str) -> String {
    String::from_utf8(
        merge_json_projection(
            source.as_bytes(),
            before.as_bytes(),
            after.as_bytes(),
            1024 * 1024,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn edits_known_leaves_without_normalizing_opaque_values_or_unchanged_numbers() {
    let source = r#"{
  "name" : "Old", "layout": {"x":0.30000000000000000000000000000001,
    "style":{"fill":"red","future":{"integer":18446744073709551617,"decimal":0.12345678901234567890123456789}}},
  "\u006eotes": {"keep":true}
}"#;
    let before = r#"{"name":"Old","layout":{"x":0.3,"style":{"fill":"red"}}}"#;
    let after = r#"{"name":"New","layout":{"x":0.3,"style":{"fill":"blue"}}}"#;
    let expected = source
        .replace(r#""Old""#, r#""New""#)
        .replace(r#""red""#, r#""blue""#);
    assert_eq!(merge(source, before, after), expected);
    assert_eq!(merge(source, before, before), source);
    assert_eq!(merge(&expected, after, after), expected);
}

#[test]
fn root_whitespace_survives_changes_and_noops_and_counts_toward_the_bound() {
    let source = " \n {\"x\":1} \t\n";
    assert_eq!(merge(source, r#"{"x":1}"#, r#"{"x":1}"#), source);
    assert_eq!(
        merge(source, r#"{"x":1}"#, r#"{"x":2}"#),
        source.replace(":1", ":2")
    );
    assert_eq!(
        merge_json_projection(
            source.as_bytes(),
            br#"{"x":1}"#,
            br#"{"x":"long"}"#,
            source.len()
        ),
        Err(ProjectionMergeError::TooLarge)
    );
}

#[test]
fn objects_add_remove_and_update_fields_with_pointer_escapes_and_normalized_keys() {
    let source = r#"{"a/b":{"~key":1,"future":true},"e\u0301":1,"clear":{"known":1}}"#;
    let result = merge(
        source,
        r#"{"a/b":{"~key":1},"é":1,"clear":{"known":1}}"#,
        r#"{"a/b":{"~key":2},"é":2,"added":null}"#,
    );
    assert_eq!(
        result,
        r#"{"a/b":{"~key":2,"future":true},"e\u0301":2,"added":null}"#
    );
}

#[test]
fn erasing_or_retyping_unknown_containers_requires_recovery() {
    let source = br#"{"style":{"known":"red","future":{"keep":true}}}"#;
    let before = br#"{"style":{"known":"red"}}"#;
    for after in [
        br#"{}"#.as_slice(),
        br#"{"style":null}"#,
        br#"{"style":"blue"}"#,
    ] {
        assert_eq!(
            merge_json_projection(source, before, after, 4096),
            Err(ProjectionMergeError::OpaqueConflict)
        );
    }
    assert_eq!(
        source,
        br#"{"style":{"known":"red","future":{"keep":true}}}"#
    );
    assert_eq!(
        merge(
            r#"{"style":{"known":"red"}}"#,
            r#"{"style":{"known":"red"}}"#,
            "{}"
        ),
        "{}"
    );
}

#[test]
fn a_new_projected_key_cannot_overwrite_unknown_source_data() {
    assert_eq!(
        merge_json_projection(br#"{"new":{"future":true}}"#, b"{}", br#"{"new":1}"#, 4096),
        Err(ProjectionMergeError::OpaqueConflict)
    );
    assert_eq!(
        merge(r#"{"new":null}"#, "{}", r#"{"new":{"paint":true}}"#),
        r#"{"new":{"paint":true}}"#
    );
    assert_eq!(merge(r#"{"new":1}"#, "{}", r#"{"new":1}"#), r#"{"new":1}"#);
}

#[test]
fn identity_arrays_keep_metadata_with_the_same_control_when_reordered() {
    let source = r#"{"items":[{"id":"a","x":0.1,"future":18446744073709551617},{"id":"b","x":0.2,"future":0.12345678901234567890123456789}]}"#;
    let before = r#"{"items":[{"id":"a","x":0.1},{"id":"b","x":0.2}]}"#;
    let after = r#"{"items":[{"id":"b","x":0.8},{"id":"a","x":0.1},{"id":"c","x":0.4}]}"#;
    assert_eq!(
        merge(source, before, after),
        r#"{"items":[{"id":"b","x":0.8,"future":0.12345678901234567890123456789},{"id":"a","x":0.1,"future":18446744073709551617},{"id":"c","x":0.4}]}"#
    );
}

#[test]
fn identity_matching_accepts_uuid_case_but_not_arbitrary_case_changes() {
    let lower = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let upper = lower.to_uppercase();
    let source = format!(r#"[{{"id":"{lower}","x":1,"future":true}}]"#);
    let before = format!(r#"[{{"id":"{upper}","x":1}}]"#);
    let after = format!(r#"[{{"id":"{upper}","x":2}}]"#);
    assert_eq!(
        merge(&source, &before, &after),
        source.replace(r#""x":1"#, r#""x":2"#)
    );
    assert_eq!(
        merge_json_projection(
            br#"[{"id":"a","future":true}]"#,
            br#"[{"id":"A"}]"#,
            b"[]",
            4096
        ),
        Err(ProjectionMergeError::OpaqueConflict)
    );
}

#[test]
fn uuid_shaped_map_key_aliases_cannot_create_shadow_owners() {
    let lower = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let upper = lower.to_uppercase();
    let before = format!(r#"{{"{upper}":{{"x":1}}}}"#);
    let after = format!(r#"{{"{upper}":{{"x":2}}}}"#);
    for source in [
        format!(r#"{{"{lower}":{{"x":1,"opaque":true}}}}"#),
        format!(r#"{{"{lower}":{{"x":1,"opaque":true}},"{upper}":{{"x":1}}}}"#),
    ] {
        for next in [&after, &"{}".to_owned()] {
            assert_eq!(
                merge_json_projection(source.as_bytes(), before.as_bytes(), next.as_bytes(), 4096),
                Err(ProjectionMergeError::OpaqueConflict)
            );
        }
    }
}

#[test]
fn removed_or_unprojected_controls_with_metadata_never_disappear() {
    for (source, before) in [
        (r#"[{"id":"a","future":true}]"#, r#"[{"id":"a"}]"#),
        (r#"[{"id":"a"}]"#, r#"[{"id":"different"}]"#),
    ] {
        assert_eq!(
            merge_json_projection(source.as_bytes(), before.as_bytes(), b"[]", 4096),
            Err(ProjectionMergeError::OpaqueConflict)
        );
    }
    assert_eq!(
        merge(
            r#"[{"id":"a","x":1},{"id":"b"}]"#,
            r#"[{"id":"a","x":1},{"id":"b"}]"#,
            r#"[{"id":"a","x":2}]"#
        ),
        r#"[{"id":"a","x":2}]"#
    );
}

#[test]
fn arrays_without_identities_do_not_guess_metadata_ownership() {
    assert_eq!(merge("[1,2,3]", "[1,2,3]", "[3,2,1]"), "[3,2,1]");
    let source = br#"[{"value":1,"future":"first"},{"value":2}]"#;
    assert_eq!(
        merge_json_projection(
            source,
            br#"[{"value":1},{"value":2}]"#,
            br#"[{"value":2},{"value":1}]"#,
            4096
        ),
        Err(ProjectionMergeError::OpaqueConflict)
    );
    assert_eq!(merge("[1,2]", "[1,2]", "[1]"), "[1]");
}

#[test]
fn duplicate_keys_duplicate_identities_and_size_fail_without_partial_results() {
    for invalid in [
        br#"{"x":1,"x":2}"#.as_slice(),
        r#"{"é":1,"e\u0301":2}"#.as_bytes(),
    ] {
        for inputs in [
            (invalid, b"{}".as_slice(), b"{}".as_slice()),
            (b"{}", invalid, b"{}"),
            (b"{}", b"{}", invalid),
        ] {
            assert_eq!(
                merge_json_projection(inputs.0, inputs.1, inputs.2, 4096),
                Err(ProjectionMergeError::InvalidJson)
            );
        }
    }
    assert_eq!(
        merge_json_projection(
            br#"[{"id":"a"},{"id":"b"}]"#,
            br#"[{"id":"a"},{"id":"b"}]"#,
            br#"[{"id":"a"},{"id":"a"}]"#,
            4096
        ),
        Err(ProjectionMergeError::InvalidProjection)
    );
    let result = merge_json_projection(
        br#"{"opaque":"123456","x":1}"#,
        br#"{"x":1}"#,
        br#"{"x":"12345678901234567890"}"#,
        30,
    );
    assert!(matches!(
        result,
        Err(ProjectionMergeError::Edit(_)) | Err(ProjectionMergeError::TooLarge)
    ));
    assert_eq!(
        merge_json_projection(b"{}", b"{}", b"{}", 1),
        Err(ProjectionMergeError::TooLarge)
    );
}

#[test]
fn cold_merge_runs_on_a_constrained_thread() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let source =
                r#"{"controls":[{"id":"a","layout":{"x":0.2,"opaque":18446744073709551617}}]}"#;
            let before = r#"{"controls":[{"id":"a","layout":{"x":0.2}}]}"#;
            let after = r#"{"controls":[{"id":"a","layout":{"x":0.8}}]}"#;
            let mut source = source.to_owned();
            let mut before = before.to_owned();
            let mut after = after.to_owned();
            for _ in 0..24 {
                source = format!(r#"{{"child":{source}}}"#);
                before = format!(r#"{{"child":{before}}}"#);
                after = format!(r#"{{"child":{after}}}"#);
            }
            assert_eq!(
                merge(&source, &before, &after),
                source.replace("0.2", "0.8")
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
