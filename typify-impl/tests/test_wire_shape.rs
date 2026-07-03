// Tests covering the "wire-shape" knobs added on top of upstream typify:
// `with_date_type` / `with_date_time_type` / `with_uuid_type`,
// `with_unconstrained_string`, `with_unconstrained_int`,
// `with_array_optionality`, `with_default_bool_optionality`,
// `with_allof_strategy`, `with_conditional_derive`, `with_conditional_attr`.
//
// Each test runs the generator on a small JSON-Schema fragment and asserts
// that the resulting code contains (or omits) specific patterns. We avoid
// strict whole-file snapshots so the tests survive incidental formatting
// changes.

use schemars::schema::RootSchema;
use serde_json::json;
use typify_impl::{
    AllOfStrategy, ArrayOptionality, DeepPatchPolicy, DefaultBoolOptionality,
    DefaultedFieldOptionality, TypeKindFilter, TypeSpace, TypeSpaceSettings,
};

fn generate(schema: serde_json::Value, mutate: impl FnOnce(&mut TypeSpaceSettings)) -> String {
    let root: RootSchema = serde_json::from_value(schema).unwrap();
    let mut settings = TypeSpaceSettings::default();
    mutate(&mut settings);
    let mut type_space = TypeSpace::new(&settings);
    type_space.add_root_schema(root).unwrap();
    type_space.to_stream().to_string()
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

// A tiny schema with a single date / date-time / uuid field on a struct.
fn schema_with_format(format: &str) -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Container",
        "type": "object",
        "required": ["field"],
        "properties": {
            "field": { "type": "string", "format": format }
        }
    })
}

#[test]
fn date_type_default_is_chrono_naive_date() {
    let out = generate(schema_with_format("date"), |_| {});
    assert_contains(&out, ":: chrono :: naive :: NaiveDate");
}

#[test]
fn date_type_override_replaces_chrono() {
    let out = generate(schema_with_format("date"), |s| {
        s.with_date_type("::time::Date");
    });
    assert_contains(&out, ":: time :: Date");
    assert_not_contains(&out, "chrono :: naive :: NaiveDate");
}

#[test]
fn date_time_type_override_replaces_chrono() {
    let out = generate(schema_with_format("date-time"), |s| {
        s.with_date_time_type("::time::OffsetDateTime");
    });
    assert_contains(&out, ":: time :: OffsetDateTime");
    assert_not_contains(&out, "chrono :: DateTime");
}

#[test]
fn uuid_type_override_replaces_uuid() {
    let out = generate(schema_with_format("uuid"), |s| {
        s.with_uuid_type("::std::string::String");
    });
    assert_contains(&out, ":: std :: string :: String");
    assert_not_contains(&out, "uuid :: Uuid");
}

// A schema with a single field of the given instance type and format.
fn schema_with_type_format(instance_type: &str, format: &str) -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Container",
        "type": "object",
        "required": ["field"],
        "properties": {
            "field": { "type": instance_type, "format": format }
        }
    })
}

#[test]
fn format_type_maps_unknown_string_format() {
    let out = generate(schema_with_type_format("string", "decimal"), |s| {
        s.with_format_type("string", "decimal", "::rust_decimal::Decimal");
    });
    assert_contains(&out, "field : :: rust_decimal :: Decimal");
}

#[test]
fn format_type_miss_falls_back_to_default() {
    // A mapping for a different format leaves this one on the default
    // path: unknown string formats degrade to plain String.
    let out = generate(schema_with_type_format("string", "money"), |s| {
        s.with_format_type("string", "decimal", "::rust_decimal::Decimal");
    });
    assert_contains(&out, "field : :: std :: string :: String");
    assert_not_contains(&out, "rust_decimal");
}

#[test]
fn format_type_wins_over_date_time_sugar() {
    let out = generate(schema_with_format("date-time"), |s| {
        s.with_date_time_type("::chrono::DateTime<::chrono::offset::Utc>");
        s.with_format_type("string", "date-time", "::time::OffsetDateTime");
    });
    assert_contains(&out, ":: time :: OffsetDateTime");
    assert_not_contains(&out, "chrono");
}

#[test]
fn format_type_overrides_builtin_ip_format() {
    let out = generate(schema_with_type_format("string", "ipv4"), |s| {
        s.with_format_type("string", "ipv4", "::my_net::V4");
    });
    assert_contains(&out, "field : :: my_net :: V4");
    assert_not_contains(&out, "std :: net");
}

#[test]
fn format_type_maps_integer_format() {
    let out = generate(schema_with_type_format("integer", "int64"), |s| {
        s.with_format_type("integer", "int64", "::my_crate::BigInt");
    });
    assert_contains(&out, "field : :: my_crate :: BigInt");
    assert_not_contains(&out, "field : i64");
}

#[test]
fn format_type_maps_number_format() {
    let out = generate(schema_with_type_format("number", "decimal"), |s| {
        s.with_format_type("number", "decimal", "::rust_decimal::Decimal");
    });
    assert_contains(&out, "field : :: rust_decimal :: Decimal");
    assert_not_contains(&out, "field : f64");
}

#[test]
fn format_type_keys_distinguish_instance_types() {
    // `string/int64` and `integer/int64` are distinct keys: a mapping for
    // one never leaks onto the other.
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Container",
        "type": "object",
        "required": ["as_string", "as_integer"],
        "properties": {
            "as_string": { "type": "string", "format": "int64" },
            "as_integer": { "type": "integer", "format": "int64" }
        }
    });
    let out = generate(schema, |s| {
        s.with_format_type("string", "int64", "::my_crate::StringInt");
    });
    assert_contains(&out, "as_string : :: my_crate :: StringInt");
    assert_contains(&out, "as_integer : i64");
}

#[test]
fn unconstrained_string_skips_newtype() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Holder",
        "type": "object",
        "required": ["code"],
        "properties": {
            "code": { "type": "string", "pattern": "^[A-Z]{3}$" }
        }
    });

    // Default: typify emits a `#[serde(transparent)]` newtype.
    let default_out = generate(schema.clone(), |_| {});
    assert_contains(&default_out, "# [serde (transparent)]");

    // With the knob set: plain `String`, no transparent newtype.
    let plain_out = generate(schema, |s| {
        s.with_unconstrained_string(true);
    });
    assert_not_contains(&plain_out, "# [serde (transparent)]");
    assert_contains(&plain_out, "code : :: std :: string :: String");
}

#[test]
fn unconstrained_int_skips_nonzero() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Counter",
        "type": "object",
        "required": ["count"],
        "properties": {
            "count": { "type": "integer", "format": "int32", "minimum": 1 }
        }
    });

    let default_out = generate(schema.clone(), |_| {});
    assert_contains(&default_out, "NonZeroU32");

    let plain_out = generate(schema, |s| {
        s.with_unconstrained_int(true);
    });
    assert_not_contains(&plain_out, "NonZeroU32");
    assert_contains(&plain_out, "count : i32");
}

#[test]
fn array_optionality_wraps_in_option() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "List",
        "type": "object",
        "properties": {
            "tags": {
                "type": "array",
                "items": { "type": "string" }
            }
        }
    });

    // Default: bare `Vec<T>` with `skip_serializing_if = "Vec::is_empty"`.
    // Note the `skip_serializing_if` arg is a string literal in the token
    // stream, so it appears as `"::std::vec::Vec::is_empty"` (no spaces).
    let default_out = generate(schema.clone(), |_| {});
    assert_contains(&default_out, "\"::std::vec::Vec::is_empty\"");

    // With `OptionalIfNotRequired`: `Option<Vec<T>>` + Option::is_none.
    let opt_out = generate(schema, |s| {
        s.with_array_optionality(ArrayOptionality::OptionalIfNotRequired);
    });
    assert_contains(&opt_out, "\"::std::option::Option::is_none\"");
    assert_contains(
        &opt_out,
        "tags : :: std :: option :: Option < :: std :: vec :: Vec",
    );
    assert_not_contains(&opt_out, "\"::std::vec::Vec::is_empty\"");
}

#[test]
fn default_bool_optionality_wraps_in_option() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Flag",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean", "default": false }
        }
    });

    // Default: bare `bool` + `serde(default)`.
    let default_out = generate(schema.clone(), |_| {});
    assert_contains(&default_out, "enabled : bool");

    let opt_out = generate(schema, |s| {
        s.with_default_bool_optionality(DefaultBoolOptionality::AlwaysOption);
    });
    assert_contains(&opt_out, "enabled : :: std :: option :: Option < bool >");
}

#[test]
fn allof_compose_emits_flatten_field() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Agency",
        "type": "object",
        "allOf": [
            { "$ref": "#/definitions/GenericAgency" },
            {
                "type": "object",
                "properties": {
                    "customerNumber": { "type": "string" }
                }
            }
        ],
        "definitions": {
            "GenericAgency": {
                "type": "object",
                "properties": {
                    "address": { "type": "string" },
                    "contactInfo": { "type": "string" }
                }
            }
        }
    });

    // Default Merge: parent struct copies base fields verbatim.
    let merge_out = generate(schema.clone(), |_| {});
    assert_contains(&merge_out, "pub address");
    assert_contains(&merge_out, "pub contact_info");
    assert_not_contains(&merge_out, "# [serde (flatten)] pub generic_agency");

    // Compose: parent gets a `#[serde(flatten)] pub generic_agency: GenericAgency`
    // field plus only the inline property.
    let compose_out = generate(schema, |s| {
        s.with_allof_strategy(AllOfStrategy::Compose);
    });
    assert_contains(
        &compose_out,
        "# [serde (flatten)] pub generic_agency : GenericAgency",
    );
    assert_contains(&compose_out, "pub customer_number");
    // The base type's fields should NOT be copied into `Agency`.
    let normalized = nws(&compose_out);
    let agency_body = normalized
        .split("pub struct Agency {")
        .nth(1)
        .and_then(|tail| tail.split('}').next())
        .expect("Agency struct should be present");
    assert!(
        !agency_body.contains("pub address :"),
        "Agency body should not contain copied `address` field: {agency_body}"
    );
    assert!(
        !agency_body.contains("pub contact_info :"),
        "Agency body should not contain copied `contact_info` field: {agency_body}"
    );
}

#[test]
fn allof_compose_falls_back_when_no_refs() {
    // No `$ref` subschemas → Compose path should bail and use Merge.
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Plain",
        "type": "object",
        "allOf": [
            { "type": "object", "properties": { "a": { "type": "string" } } },
            { "type": "object", "properties": { "b": { "type": "string" } } }
        ]
    });
    let out = generate(schema, |s| {
        s.with_allof_strategy(AllOfStrategy::Compose);
    });
    // Both fields land on the struct, no flatten.
    assert_contains(&out, "pub a");
    assert_contains(&out, "pub b");
    assert_not_contains(&out, "# [serde (flatten)]");
}

#[test]
fn conditional_derive_emits_cfg_attr() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Thing",
        "type": "object",
        "required": ["name"],
        "properties": { "name": { "type": "string" } }
    });
    let out = generate(schema, |s| {
        s.with_conditional_derive("schemars", "schemars::JsonSchema");
        s.with_conditional_derive("struct-patch", "struct_patch::Patch");
    });
    assert_contains(
        &out,
        "# [cfg_attr (feature = \"schemars\" , derive (schemars :: JsonSchema))]",
    );
    assert_contains(
        &out,
        "# [cfg_attr (feature = \"struct-patch\" , derive (struct_patch :: Patch))]",
    );
}

#[test]
fn conditional_attr_emits_cfg_attr() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Thing",
        "type": "object",
        "required": ["name"],
        "properties": { "name": { "type": "string" } }
    });
    let out = generate(schema, |s| {
        s.with_conditional_attr("strict", "serde(deny_unknown_fields)");
    });
    assert_contains(
        &out,
        "# [cfg_attr (feature = \"strict\" , serde (deny_unknown_fields))]",
    );
}

// A schema that exercises the deep-patch knob: an `Owner` struct whose
// fields cover every shape we care about — an `Option<{Struct}>` deep
// candidate, an `Option<{enum}>` (must NOT get a patch annotation), a
// `Vec<{Struct}>` (must NOT get one), a flatten base (must NOT get one),
// and a primitive option (must NOT get one).
fn deep_patch_schema() -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": {
            "Owner": {
                "type": "object",
                "allOf": [
                    { "$ref": "#/definitions/Base" },
                    {
                        "type": "object",
                        "properties": {
                            "child": { "$ref": "#/definitions/Child" },
                            "kind": { "$ref": "#/definitions/Kind" },
                            "items": {
                                "type": "array",
                                "items": { "$ref": "#/definitions/Child" }
                            },
                            "note": { "type": "string" }
                        }
                    }
                ]
            },
            "Base": {
                "type": "object",
                "properties": {
                    "base_field": { "type": "string" }
                }
            },
            "Child": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            },
            "Kind": {
                "type": "string",
                "enum": ["a", "b"]
            }
        }
    })
}

#[test]
fn deep_patches_off_emits_nothing() {
    // Default policy is `Off`: no `#[patch(name = ...)]` lines should
    // appear anywhere in the output.
    let out = generate(deep_patch_schema(), |s| {
        s.with_allof_strategy(AllOfStrategy::Compose);
    });
    assert_not_contains(&out, "# [patch (name =");
}

#[test]
fn deep_patches_all_option_structs_emits_inner_struct_only() {
    let out = generate(deep_patch_schema(), |s| {
        s.with_allof_strategy(AllOfStrategy::Compose)
            .with_deep_patches(DeepPatchPolicy::AllOptionStructs);
    });
    // Drilled `Option<Child>` should carry the patch line.
    assert_contains(&out, "# [patch (name = \"Option<ChildPatch>\")] pub child");
    // None of the other shapes should:
    //  - Option<Kind> where Kind is an enum: no Patch derive on enums.
    //  - Vec<Child>: deep-patch doesn't recurse through Vec.
    //  - Option<String>: primitive inner.
    //  - flatten base (`base: Base`): flatten fields are bare, not Option.
    let normalized = nws(&out);
    let owner_body = normalized
        .split("pub struct Owner {")
        .nth(1)
        .and_then(|tail| tail.split('}').next())
        .expect("Owner body should be present");
    assert!(
        !owner_body.contains("Option<KindPatch>"),
        "enum option must not get a patch rewrite: {owner_body}"
    );
    assert!(
        !owner_body.contains("Option<StringPatch>"),
        "primitive option must not get a patch rewrite: {owner_body}"
    );
    // Flatten base field on `Owner` (compose strategy) renders as bare
    // `pub base: Base`; it must not carry a patch line.
    assert!(
        !owner_body.contains("Option<BasePatch>"),
        "flatten base must not get a patch rewrite: {owner_body}"
    );
}

#[test]
fn deep_patch_filter_overrides_bulk_off() {
    // Bulk policy is Off, but a closure flips ON for a single field.
    let out = generate(deep_patch_schema(), |s| {
        s.with_allof_strategy(AllOfStrategy::Compose)
            .with_deep_patch_filter(|owner, field, inner| {
                owner == "Owner" && field == "child" && inner == "Child"
            });
    });
    assert_contains(&out, "# [patch (name = \"Option<ChildPatch>\")] pub child");
}

#[test]
fn deep_patch_filter_false_suppresses_emission() {
    // Bulk policy is AllOptionStructs, but the closure says "skip
    // everything"; the closure takes precedence.
    let out = generate(deep_patch_schema(), |s| {
        s.with_allof_strategy(AllOfStrategy::Compose)
            .with_deep_patches(DeepPatchPolicy::AllOptionStructs)
            .with_deep_patch_filter(|_, _, _| false);
    });
    assert_not_contains(&out, "# [patch (name =");
}

#[test]
fn defaulted_field_optionality_bare_lifts_default() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Holder",
        "type": "object",
        "properties": {
            "received_from": { "type": "string", "default": "ServiceDesk" },
            "wait_time":     { "type": "integer", "format": "int32", "default": 5 }
        }
    });
    let out = generate(schema, |_| {});
    // Default behavior: lift via `defaults::` helper, keep type bare.
    // Path strings are quoted token literals — no internal whitespace.
    assert_contains(&out, "default = \"defaults::holder_received_from\"");
    // Integer defaults use the shared `default_u64::<i32, N>` helper rather
    // than a per-field one, so just confirm a `defaults::` reference exists.
    assert_contains(&out, "default = \"defaults::default_u64");
    assert_contains(&out, "pub received_from : :: std :: string :: String");
    assert_contains(&out, "pub wait_time : i32");
}

#[test]
fn defaulted_field_optionality_always_option_wraps_and_drops_helper() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Holder",
        "type": "object",
        "properties": {
            "received_from": { "type": "string", "default": "ServiceDesk" },
            "wait_time":     { "type": "integer", "format": "int32", "default": 5 },
            // bool stays under the existing knob — `defaulted_field_optionality`
            // must not regress its handling.
            "flag":          { "type": "boolean", "default": false }
        }
    });
    let out = generate(schema, |s| {
        s.with_defaulted_field_optionality(DefaultedFieldOptionality::AlwaysOption);
    });
    assert_contains(
        &out,
        "pub received_from : :: std :: option :: Option < :: std :: string :: String >",
    );
    assert_contains(&out, "pub wait_time : :: std :: option :: Option < i32 >");
    // No per-field `defaults::` helper for the wrapped fields.
    assert_not_contains(&out, "holder_received_from");
    assert_not_contains(&out, "default_u64 :: < i32");
    // Bool is governed by `default_bool_optionality`, so it stays bare here.
    assert_contains(&out, "pub flag : bool");
}

#[test]
fn elide_option_field_defaults_drops_redundant_pair() {
    // Schema with an `Option<String>` (no spec default), a `Vec<T>`
    // (intrinsic-default array), and a string with a spec `default:`.
    // Only the bare `Option<T>` field should lose its per-field
    // `#[serde(default, skip_serializing_if = "Option::is_none")]`.
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Holder",
        "type": "object",
        "properties": {
            "agency_customer_number": { "type": "string" },
            "tags": { "type": "array", "items": { "type": "string" } },
            "received_from": { "type": "string", "default": "ServiceDesk" }
        }
    });

    // Default (knob OFF): the canonical pair is present on `Option<T>`.
    let default_out = generate(schema.clone(), |_| {});
    assert_contains(
        &default_out,
        "# [serde (default , skip_serializing_if = \"::std::option::Option::is_none\")] \
         pub agency_customer_number",
    );

    // Knob ON: bare `Option<T>` with NO per-field `#[serde(...)]`.
    let elided = generate(schema, |s| {
        s.with_elide_option_field_defaults(true);
    });
    let normalized = nws(&elided);
    assert!(
        normalized.contains("pub agency_customer_number : :: std :: option :: Option"),
        "agency_customer_number should remain an Option<String>: {normalized}"
    );
    assert_not_contains(
        &elided,
        "# [serde (default , skip_serializing_if = \"::std::option::Option::is_none\")] \
         pub agency_customer_number",
    );

    // `Vec<T>` keeps `skip_serializing_if = "Vec::is_empty"` — the
    // elision is scoped to Option<T> only.
    assert_contains(&elided, "\"::std::vec::Vec::is_empty\"");
    // A field with a spec `default:` keeps its `default = "defaults::..."`
    // line — that's a named-default fn, not the canonical Option pair.
    assert_contains(&elided, "default = \"defaults::holder_received_from\"");
}

#[test]
fn elide_option_field_defaults_preserves_rename() {
    // A field whose JSON wire name (`Agency-Customer-Number`) does NOT
    // round-trip from its snake_case Rust name through any rename_all
    // pass. The per-field `#[serde(rename = "...")]` must survive
    // elision; only the `default + skip_serializing_if` pair is
    // dropped. The resulting attribute should be `#[serde(rename =
    // "...")]` with no `default, skip_serializing_if` companions.
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Holder",
        "type": "object",
        "properties": {
            "Agency-Customer-Number": { "type": "string" }
        }
    });
    let out = generate(schema, |s| {
        s.with_elide_option_field_defaults(true);
    });
    assert_contains(
        &out,
        "# [serde (rename = \"Agency-Customer-Number\")] \
         pub agency_customer_number",
    );
    assert_not_contains(
        &out,
        "skip_serializing_if = \"::std::option::Option::is_none\"",
    );
    assert_not_contains(&out, "rename = \"Agency-Customer-Number\" , default");
}

#[test]
fn elide_option_field_defaults_keeps_flatten() {
    // A nested `allOf [$ref, properties:...]` under `Compose` produces a
    // `#[serde(flatten)] pub base: Base` field. Flatten is not part of
    // the canonical pair, so the elision must leave it untouched.
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Agency",
        "type": "object",
        "allOf": [
            { "$ref": "#/definitions/GenericAgency" },
            { "type": "object", "properties": { "extra": { "type": "string" } } }
        ],
        "definitions": {
            "GenericAgency": {
                "type": "object",
                "properties": { "address": { "type": "string" } }
            }
        }
    });
    let out = generate(schema, |s| {
        s.with_allof_strategy(AllOfStrategy::Compose)
            .with_elide_option_field_defaults(true);
    });
    assert_contains(
        &out,
        "# [serde (flatten)] pub generic_agency : GenericAgency",
    );
}

#[test]
fn elide_option_field_defaults_keeps_vec_and_bool() {
    // `Option<Vec<T>>` (via `with_array_optionality`) ends up as
    // `Optional + Option(_)` — same arm as bare `Option<String>` —
    // so it SHOULD have its per-field attr elided too. But a
    // non-optional `Vec<T>` (the default `Bare` mode) keeps its
    // `default + skip_serializing_if = Vec::is_empty` pair, because
    // that arm is `(Optional, Vec(_))`, not `(Optional, Option(_))`.
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Holder",
        "type": "object",
        "properties": {
            "tags": { "type": "array", "items": { "type": "string" } },
            "enabled": { "type": "boolean", "default": false }
        }
    });
    let out = generate(schema, |s| {
        s.with_elide_option_field_defaults(true);
    });
    // Vec keeps its skip-serializing-if.
    assert_contains(&out, "\"::std::vec::Vec::is_empty\"");
    // Bool with `default: false` collapses to bare `bool` + `serde(default)`
    // under the existing path — that must survive the elision.
    assert_contains(&out, "# [serde (default)] pub enabled : bool");
}

#[test]
fn defaulted_field_optionality_does_not_wrap_required_fields() {
    // Required fields with `default:` should stay bare (the spec default
    // is honored at construction time, not at the type level).
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Holder",
        "type": "object",
        "required": ["received_from"],
        "properties": {
            "received_from": { "type": "string", "default": "ServiceDesk" }
        }
    });
    let out = generate(schema, |s| {
        s.with_defaulted_field_optionality(DefaultedFieldOptionality::AlwaysOption);
    });
    assert_contains(&out, "pub received_from : :: std :: string :: String");
    assert_not_contains(&out, "pub received_from : :: std :: option :: Option");
}

#[test]
fn conditional_derive_for_structs_skips_enums() {
    // Schema with both a struct (Thing) and an enum (Color) so we can
    // verify a structs-only conditional derive lands on the struct but
    // not on the enum.
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": {
            "Thing": {
                "type": "object",
                "required": ["name"],
                "properties": { "name": { "type": "string" } }
            },
            "Color": {
                "type": "string",
                "enum": ["red", "green", "blue"]
            }
        }
    });
    let out = generate(schema, |s| {
        s.with_conditional_derive_for(
            "struct-patch",
            "struct_patch::Patch",
            TypeKindFilter::STRUCTS,
        );
    });
    let normalized = nws(&out);
    let thing = normalized
        .split("pub struct Thing")
        .next()
        .expect("Thing struct context");
    assert!(
        thing.contains("cfg_attr (feature = \"struct-patch\" , derive (struct_patch :: Patch))"),
        "structs-only cfg_attr should appear before Thing struct"
    );
    let color_before = normalized
        .split("pub enum Color")
        .next()
        .expect("Color enum context");
    let color_idx = color_before.len();
    let thing_idx = normalized
        .find("pub struct Thing")
        .expect("Thing struct present");
    // The cfg_attr should appear in the prelude before Thing but NOT in the
    // attribute block that's between the Color enum's docs and `pub enum
    // Color`. Walk a small window before the enum and check.
    let window_start = color_idx.saturating_sub(200);
    let color_window = &color_before[window_start..];
    assert!(
        !color_window.contains("derive (struct_patch :: Patch)"),
        "enum Color must NOT carry the structs-only cfg_attr; window: {color_window}"
    );
    // Sanity: Color and Thing are both emitted.
    assert!(thing_idx > 0, "Thing should be present in output");
}

// A named unconstrained string definition (becomes a newtype) and a
// pattern-constrained one (becomes a validating newtype).
fn string_newtype_schema() -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": {
            "PlainName": { "type": "string" },
            "CodeName": { "type": "string", "pattern": "^[a-z]+$" }
        }
    })
}

#[test]
fn string_newtype_conveniences_default_off_matches_upstream() {
    let out = generate(string_newtype_schema(), |_| {});
    // Upstream surface only: no `AsRef<str>` and no `From<&str>` on
    // either newtype.
    assert_not_contains(&out, "impl :: std :: convert :: AsRef < str > for PlainName");
    assert_not_contains(&out, "impl :: std :: convert :: From < & str > for PlainName");
    assert_not_contains(&out, "impl :: std :: convert :: AsRef < str > for CodeName");
    assert_not_contains(&out, "impl :: std :: fmt :: Display for CodeName");
}

#[test]
fn string_newtype_conveniences_add_read_and_construct_impls() {
    let out = generate(string_newtype_schema(), |s| {
        s.with_string_newtype_conveniences(true);
    });
    // Unconstrained: read-side impls plus cheap `From<&str>` construction.
    assert_contains(&out, "impl :: std :: convert :: AsRef < str > for PlainName");
    assert_contains(&out, "impl :: std :: fmt :: Display for PlainName");
    assert_contains(&out, "impl :: std :: convert :: From < & str > for PlainName");
    // Constrained: read-side impls only — construction must go through
    // the validating FromStr / TryFrom path.
    assert_contains(&out, "impl :: std :: convert :: AsRef < str > for CodeName");
    assert_contains(&out, "impl :: std :: fmt :: Display for CodeName");
    assert_not_contains(&out, "impl :: std :: convert :: From < & str > for CodeName");
}
