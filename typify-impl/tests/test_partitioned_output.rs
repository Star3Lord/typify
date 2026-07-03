// Tests covering `to_stream_partitioned`, in particular the nested
// (slash-separated) module paths: nested emission, flat/nested mixes,
// preambles attached to leaf and parent paths, and the `error` /
// `defaults` duplication landing in leaves only.
//
// Following the style of `test_wire_shape.rs`: each test generates from a
// small JSON-Schema fragment and asserts on whitespace-collapsed patterns
// of the emitted token text, so incidental formatting changes don't break
// anything.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use schemars::schema::RootSchema;
use serde_json::json;
use typify_impl::{TypeSpace, TypeSpaceSettings};

/// Three named definitions: two structs (one referencing the other) and a
/// simple enum, enough to spread across several partitions.
fn sample_schema() -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": {
            "Alpha": {
                "type": "object",
                "properties": { "name": { "type": "string" } }
            },
            "Beta": {
                "type": "object",
                "properties": { "alpha": { "$ref": "#/definitions/Alpha" } }
            },
            "Color": {
                "type": "string",
                "enum": ["red", "green"]
            }
        }
    })
}

fn generate_partitioned(
    partition: &[(&str, &str)],
    default_module: &str,
    imports: &[(&str, &str)],
) -> String {
    let root: RootSchema = serde_json::from_value(sample_schema()).unwrap();
    let mut type_space = TypeSpace::new(&TypeSpaceSettings::default());
    type_space.add_root_schema(root).unwrap();

    let partition: HashMap<String, String> = partition
        .iter()
        .map(|(ty, module)| (ty.to_string(), module.to_string()))
        .collect();
    let imports: HashMap<String, TokenStream> = imports
        .iter()
        .map(|(module, preamble)| (module.to_string(), preamble.parse().unwrap()))
        .collect();

    type_space
        .to_stream_partitioned(&partition, default_module, &imports)
        .to_string()
}

fn nws(s: &str) -> String {
    // Collapse all whitespace so test patterns don't fight formatting.
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_contains(haystack: &str, needle: &str) {
    let hay = nws(haystack);
    let nee = nws(needle);
    assert!(
        hay.contains(&nee),
        "expected to find:\n  {nee}\nin generated output:\n  {hay}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str) {
    let hay = nws(haystack);
    let nee = nws(needle);
    assert!(
        !hay.contains(&nee),
        "expected NOT to find:\n  {nee}\nbut it appears in:\n  {hay}"
    );
}

/// Position of `needle` in the whitespace-collapsed `haystack`, for
/// asserting relative ordering of emitted items.
fn pos(haystack: &str, needle: &str) -> usize {
    let hay = nws(haystack);
    let nee = nws(needle);
    hay.find(&nee)
        .unwrap_or_else(|| panic!("expected to find:\n  {nee}\nin generated output:\n  {hay}"))
}

fn count(haystack: &str, needle: &str) -> usize {
    nws(haystack).matches(&nws(needle)).count()
}

#[test]
fn nested_paths_emit_nested_modules() {
    let out = generate_partitioned(
        &[("Alpha", "ops/alpha"), ("Color", "shared/enums")],
        "shared/common",
        &[],
    );

    // Slash-separated paths become nested blocks; `shared/enums` and
    // `shared/common` merge under a single `pub mod shared`.
    assert_contains(&out, "pub mod ops { pub mod alpha {");
    assert_eq!(count(&out, "pub mod shared {"), 1);
    assert_contains(&out, "pub mod common {");
    assert_contains(&out, "pub mod enums {");

    // Types land in their partitions: modules in BTree order are
    // ops < shared, and within shared: common < enums. Beta is unmapped
    // and falls into the default module (shared/common).
    assert!(pos(&out, "pub mod ops") < pos(&out, "pub struct Alpha"));
    assert!(pos(&out, "pub struct Alpha") < pos(&out, "pub mod shared"));
    assert!(pos(&out, "pub mod common") < pos(&out, "pub struct Beta"));
    assert!(pos(&out, "pub struct Beta") < pos(&out, "pub mod enums"));
    assert!(pos(&out, "pub mod enums") < pos(&out, "pub enum Color"));

    // Every leaf gets its own `error` submodule; the pure containers
    // (`ops`, `shared`) do not add their own.
    assert_eq!(count(&out, "pub mod error {"), 3);
}

#[test]
fn flat_and_nested_paths_mix() {
    let out = generate_partitioned(
        &[("Alpha", "alpha"), ("Beta", "cancel/request")],
        "shared/common",
        &[],
    );

    assert_contains(&out, "pub mod alpha {");
    assert_contains(&out, "pub mod cancel { pub mod request {");
    assert_contains(&out, "pub mod shared { pub mod common {");

    assert!(pos(&out, "pub mod alpha") < pos(&out, "pub struct Alpha"));
    assert!(pos(&out, "pub struct Alpha") < pos(&out, "pub mod cancel"));
    assert!(pos(&out, "pub mod cancel") < pos(&out, "pub struct Beta"));
    assert!(pos(&out, "pub struct Beta") < pos(&out, "pub mod shared"));
    assert!(pos(&out, "pub mod shared") < pos(&out, "pub enum Color"));

    // Three leaves (alpha, cancel/request, shared/common), three error
    // submodules.
    assert_eq!(count(&out, "pub mod error {"), 3);
}

#[test]
fn imports_attach_to_leaf_and_parent_paths() {
    let out = generate_partitioned(
        &[("Alpha", "ops/alpha")],
        "shared",
        &[
            ("ops/alpha", "use super::super::shared::*;"),
            ("ops", "use ::serde::Serialize;"),
        ],
    );

    // The parent path's preamble sits directly inside `ops`, before the
    // nested `alpha` block, which carries its own leaf preamble.
    assert_contains(
        &out,
        "pub mod ops { use :: serde :: Serialize ; pub mod alpha { use super :: super :: shared ::*;",
    );

    // The parent is a pure container: no `error` submodule directly
    // under `ops` — only the two leaves (ops/alpha and the default
    // `shared`) get one.
    assert_eq!(count(&out, "pub mod error {"), 2);
}

#[test]
fn imports_materialize_empty_nested_modules() {
    // `shared/enums` receives no types, but appears as an imports key —
    // it must still be emitted (a sibling module may glob-import it).
    let out = generate_partitioned(
        &[],
        "shared/common",
        &[("shared/enums", "use ::serde::Serialize;")],
    );

    assert_contains(&out, "pub mod shared { pub mod common {");
    assert_contains(&out, "pub mod enums { use :: serde :: Serialize ;");
    assert_eq!(count(&out, "pub mod shared {"), 1);
}

#[test]
fn flat_partition_shape_unchanged() {
    // The historical flat behavior: one top-level block per module name,
    // preamble first, `error` submodule duplicated into every module.
    let out = generate_partitioned(
        &[("Alpha", "alpha_mod")],
        "shared",
        &[("alpha_mod", "use super::shared::*;")],
    );

    assert_contains(&out, "pub mod alpha_mod { use super :: shared ::*;");
    assert_contains(&out, "pub mod shared {");
    assert_not_contains(&out, "pub mod ops");
    assert!(pos(&out, "pub mod alpha_mod") < pos(&out, "pub struct Alpha"));
    assert!(pos(&out, "pub struct Alpha") < pos(&out, "pub mod shared"));
    assert_eq!(count(&out, "pub mod error {"), 2);
}
