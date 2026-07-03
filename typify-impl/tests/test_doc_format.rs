// Tests for the JSON-schema-in-doc-comment knob added on top of upstream
// typify (`TypeSpaceSettings::with_schema_in_docs`).
//
// The default matches upstream: every generated type's doc comment embeds
// the full pretty-printed schema in a `<details><summary>JSON
// schema</summary>` block after the schema's `description`. Turning the
// knob off keeps doc comments minimal — just the description — so
// rust-analyzer / Cursor hover popovers stay readable.

use schemars::schema::RootSchema;
use serde_json::json;
use typify_impl::{TypeSpace, TypeSpaceSettings};

fn generate(schema: serde_json::Value, mutate: impl FnOnce(&mut TypeSpaceSettings)) -> String {
    let root: RootSchema = serde_json::from_value(schema).unwrap();
    let mut settings = TypeSpaceSettings::default();
    mutate(&mut settings);
    let mut type_space = TypeSpace::new(&settings);
    type_space.add_root_schema(root).unwrap();
    type_space.to_stream().to_string()
}

fn schema_with_description() -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Widget",
        "description": "A widget that does widget things.",
        "type": "object",
        "required": ["name"],
        "properties": {
            "name": { "type": "string", "description": "The widget's name." }
        }
    })
}

#[test]
fn default_matches_upstream_details_block() {
    let out = generate(schema_with_description(), |_| {});
    // The human-readable description must always be present.
    assert!(
        out.contains("A widget that does widget things."),
        "expected description in default output:\n{out}"
    );
    // The upstream `<details>` schema block is the default.
    assert!(
        out.contains("<details><summary>JSON schema</summary>"),
        "default output must contain the upstream `<details>` block:\n{out}"
    );
    assert!(
        out.contains("```json"),
        "default output must contain a json code fence:\n{out}"
    );
}

#[test]
fn schema_in_docs_off_omits_schema_block() {
    let out = generate(schema_with_description(), |s| {
        s.with_schema_in_docs(false);
    });
    assert!(
        out.contains("A widget that does widget things."),
        "expected description in description-only output:\n{out}"
    );
    // None of the schema-block markers should appear.
    assert!(
        !out.contains("<details>"),
        "description-only output must not contain `<details>`:\n{out}"
    );
    assert!(
        !out.contains("<summary>"),
        "description-only output must not contain `<summary>`:\n{out}"
    );
    assert!(
        !out.contains("```json"),
        "description-only output must not contain a json code fence:\n{out}"
    );
}

#[test]
fn description_only_mode_still_emits_doc_attr() {
    let out = generate(schema_with_description(), |s| {
        s.with_schema_in_docs(false);
    });
    // We still want to see at least one `# [doc =` for the description so
    // hovers aren't completely empty.
    assert!(
        out.contains("# [doc ="),
        "description-only output should still emit a doc attr for the description:\n{out}"
    );
}

#[test]
fn round_trip_re_enable_matches_default() {
    // `with_schema_in_docs(false)` then `(true)` must land back on the
    // upstream default output.
    let default_out = generate(schema_with_description(), |_| {});
    let round_trip = generate(schema_with_description(), |s| {
        s.with_schema_in_docs(false);
        s.with_schema_in_docs(true);
    });
    assert_eq!(default_out, round_trip);
}
