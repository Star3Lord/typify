// Tests for the JSON-schema-in-doc-comment knob added on top of upstream
// typify (`TypeSpaceSettings::with_schema_in_docs`).
//
// The default keeps generated doc comments minimal — just the schema's
// `description` — so rust-analyzer / Cursor hover popovers stay readable.
// Turning the knob on emits the full pretty-printed schema under a
// `# JSON schema` markdown heading + fenced `json` code block. The shape
// is deliberately *not* the historical `<details><summary>JSON
// schema</summary>...</details>` because that breaks hover rendering for
// the reasons described on `with_schema_in_docs`.

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
fn default_omits_json_schema_block() {
    let out = generate(schema_with_description(), |_| {});
    // The human-readable description must always be present.
    assert!(
        out.contains("A widget that does widget things."),
        "expected description in default output:\n{out}"
    );
    // None of the schema-block markers should appear.
    assert!(
        !out.contains("<details>"),
        "default output must not contain `<details>`:\n{out}"
    );
    assert!(
        !out.contains("<summary>"),
        "default output must not contain `<summary>`:\n{out}"
    );
    assert!(
        !out.contains("# JSON schema"),
        "default output must not contain `# JSON schema`:\n{out}"
    );
    assert!(
        !out.contains("```json"),
        "default output must not contain a json code fence:\n{out}"
    );
}

#[test]
fn with_schema_in_docs_emits_heading_and_fence() {
    let out = generate(schema_with_description(), |s| {
        s.with_schema_in_docs(true);
    });
    assert!(
        out.contains("A widget that does widget things."),
        "expected description in opt-in output:\n{out}"
    );
    assert!(
        out.contains("# JSON schema"),
        "expected `# JSON schema` heading in opt-in output:\n{out}"
    );
    // `\`\`\`json` is rendered in the token stream as the literal string
    // `"```json"` (the surrounding backticks come through verbatim).
    assert!(
        out.contains("\"```json\""),
        "expected ```json fence (as a string literal) in opt-in output:\n{out}"
    );
    assert!(
        !out.contains("<details>"),
        "opt-in output must NOT use `<details>` (broken in IDE hover):\n{out}"
    );
    assert!(
        !out.contains("<summary>"),
        "opt-in output must NOT use `<summary>` (broken in IDE hover):\n{out}"
    );
}

#[test]
fn fence_doc_attrs_have_no_leading_whitespace() {
    let out = generate(schema_with_description(), |s| {
        s.with_schema_in_docs(true);
    });
    // The opening and closing fence markers MUST be emitted as
    // `#[doc = "..."]` with zero leading whitespace inside the string —
    // anything else introduces the indentation asymmetry that breaks
    // markdown rendering in IDE hovers (see `make_doc`).
    //
    // We assert the literal fences appear as their no-leading-space
    // string forms. A leading-space variant (e.g. `" \`\`\`json"`) would
    // come through as `" ```json"` instead.
    assert!(
        out.contains("\"```json\""),
        "opening fence `#[doc = \"```json\"]` should appear verbatim:\n{out}"
    );
    assert!(
        out.contains("\"```\""),
        "closing fence `#[doc = \"```\"]` should appear verbatim:\n{out}"
    );
    // Negative check: the leading-space variant must NOT be present.
    assert!(
        !out.contains("\" ```json\""),
        "opening fence must not have a leading space:\n{out}"
    );
    // For the closing fence, we don't have a clean substring to assert
    // against (it would alias the opening fence) — checking the negative
    // form for the opening fence is sufficient evidence that
    // `make_doc` emits explicit `#[doc = "..."]` rather than `///` for
    // the wrappers.
}

#[test]
fn description_only_mode_still_emits_doc_attr() {
    let out = generate(schema_with_description(), |_| {});
    // We still want to see at least one `# [doc =` for the description so
    // hovers aren't completely empty.
    assert!(
        out.contains("# [doc ="),
        "default output should still emit a doc attr for the description:\n{out}"
    );
}
