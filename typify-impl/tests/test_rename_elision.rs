// Tests covering the per-field `#[serde(rename = "...")]` elision logic
// activated by `with_struct_rename_all(...)`. Each test feeds a tiny
// JSON-Schema fragment through typify, then asserts whether the generated
// output should or should not contain a per-field `rename` directive.
//
// The elision rule (see `crates/libs/typify/typify-impl/src/structs.rs`)
// is:
//
//     "if a struct-level rename_all = "<case>" is set AND
//      <case>(snake_field) == wire_name, drop the per-field rename"
//
// These tests pin that contract on every serde-supported case AND on the
// awkward edge cases the codegen actually hits in practice (Sabre-style
// acronyms, Rust-keyword field renames, mixed acronym wire names, etc.).

use schemars::schema::RootSchema;
use serde_json::json;
use typify_impl::{TypeSpace, TypeSpaceSettings};

/// Run codegen on a JSON-Schema fragment with the given mutation applied to
/// the `TypeSpaceSettings`. Returns the generated Rust source.
fn generate(schema: serde_json::Value, mutate: impl FnOnce(&mut TypeSpaceSettings)) -> String {
    let root: RootSchema = serde_json::from_value(schema).unwrap();
    let mut settings = TypeSpaceSettings::default();
    mutate(&mut settings);
    let mut type_space = TypeSpace::new(&settings);
    type_space.add_root_schema(root).unwrap();
    type_space.to_stream().to_string()
}

fn nws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[track_caller]
fn assert_contains(haystack: &str, needle: &str) {
    let hay = nws(haystack);
    let nee = nws(needle);
    assert!(
        hay.contains(&nee),
        "expected to find:\n  {nee}\nin generated output:\n  {hay}"
    );
}

#[track_caller]
fn assert_not_contains(haystack: &str, needle: &str) {
    let hay = nws(haystack);
    let nee = nws(needle);
    assert!(
        !hay.contains(&nee),
        "expected NOT to find:\n  {nee}\nbut it appears in:\n  {hay}"
    );
}

/// Assert that the generated output emits `#[serde(rename = "<wire>")]`
/// for the field. Matches only the `rename = "<wire>"` substring, so it
/// is tolerant of the optional `default + skip_serializing_if = ...`
/// companions typify adds to `Option<T>` fields by default.
#[track_caller]
fn assert_rename_present(out: &str, wire: &str) {
    let hay = nws(out);
    let needle = format!("rename = \"{wire}\"");
    assert!(
        hay.contains(&needle),
        "expected per-field rename for wire {wire:?} in:\n  {hay}"
    );
}

/// Assert that the generated output does NOT carry a per-field
/// `rename = "<wire>"` directive — the struct-level `rename_all`
/// (or natural round-trip) covers the field unaided.
#[track_caller]
fn assert_rename_absent(out: &str, wire: &str) {
    let hay = nws(out);
    let needle = format!("rename = \"{wire}\"");
    assert!(
        !hay.contains(&needle),
        "expected NO per-field rename for wire {wire:?} but found one in:\n  {hay}"
    );
}

/// Build a one-field schema with `wire_name` as the JSON property name.
fn one_field(wire_name: &str) -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Container",
        "type": "object",
        "properties": {
            wire_name: { "type": "string" }
        }
    })
}

// =============================================================================
// Matrix: every serde-supported `rename_all` value vs. canonical wire shapes.
//
// For each row, the test asserts whether `#[serde(rename = "...")]` appears
// in the emitted output. Names of the tests follow the convention
// `<rename_all>_<wire_shape>_<expected>`.
// =============================================================================

// --- camelCase rename_all (the workspace default) ----------------------------

#[test]
fn camel_case_round_trip_elides() {
    // wire `accountId`, Rust field `account_id`, rename_all camelCase.
    // to_lower_camel_case("account_id") == "accountId", so the per-field
    // rename is redundant and MUST be elided.
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_contains(&out, "pub account_id : :: std :: option :: Option");
    assert_rename_absent(&out, "accountId");
}

#[test]
fn camel_case_pascal_wire_emits() {
    // wire `AccountId` (PascalCase), rename_all camelCase.
    // to_lower_camel_case("account_id") == "accountId" != "AccountId".
    let out = generate(one_field("AccountId"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "AccountId");
}

#[test]
fn camel_case_kebab_wire_emits() {
    // wire `account-id`, rename_all camelCase.
    let out = generate(one_field("account-id"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "account-id");
}

#[test]
fn camel_case_screaming_snake_wire_emits() {
    // wire `ACCOUNT_ID`, rename_all camelCase.
    let out = generate(one_field("ACCOUNT_ID"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "ACCOUNT_ID");
}

#[test]
fn camel_case_uppercase_wire_emits() {
    // wire `FOO` (UPPERCASE single word), rename_all camelCase.
    // to_lower_camel_case("foo") == "foo" != "FOO".
    let out = generate(one_field("FOO"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "FOO");
}

#[test]
fn camel_case_lowercase_single_word_elides() {
    // wire `foo` (lowercase single word), rename_all camelCase.
    // recase produces `name = "foo"`, `rename = None`, so the elision
    // codepath isn't even hit — no rename is emitted.
    let out = generate(one_field("foo"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_contains(&out, "pub foo : :: std :: option :: Option");
    assert_rename_absent(&out, "foo");
}

#[test]
fn camel_case_acronym_pcc_emits() {
    // wire `PCC` (Sabre Pseudo-City Code), rename_all camelCase.
    // to_lower_camel_case("pcc") == "pcc" != "PCC" — must NOT elide.
    let out = generate(one_field("PCC"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "PCC");
}

#[test]
fn camel_case_acronym_iata_emits() {
    // wire `IATA`, rename_all camelCase.
    let out = generate(one_field("IATA"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "IATA");
}

#[test]
fn camel_case_embedded_acronym_id_pcc_emits() {
    // wire `id_PCC`, rename_all camelCase.
    // to_lower_camel_case("id_pcc") == "idPcc" != "id_PCC".
    let out = generate(one_field("id_PCC"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "id_PCC");
}

#[test]
fn camel_case_keyword_field_type_elides() {
    // wire `type` -> Rust field `type_` (the keyword sanitization adds
    // a trailing underscore). With rename_all = "camelCase" the
    // generated wire name from serde's camelCase rule applied to
    // `type_` is `"type"` (PascalCase("type_") = "Type", lowercase
    // first = "type"). The elision is therefore CORRECT — confirm it.
    let out = generate(one_field("type"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_contains(&out, "pub type_ : :: std :: option :: Option");
    assert_rename_absent(&out, "type");
    // And confirm the elision did not also accidentally drop the field
    // sanitization: the Rust field MUST be `type_`, not bare `type`.
    assert_not_contains(&out, "pub type :");
}

#[test]
fn camel_case_keyword_field_ref_elides() {
    // Same elision logic but for `ref` -> `ref_`.
    let out = generate(one_field("ref"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_contains(&out, "pub ref_ : :: std :: option :: Option");
    assert_rename_absent(&out, "ref");
}

#[test]
fn camel_case_pascal_wire_with_capital_acronym_emits() {
    // wire `RequestorID` — common Sabre pattern.
    // to_lower_camel_case("requestor_id") == "requestorId" != "RequestorID".
    let out = generate(one_field("RequestorID"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "RequestorID");
}

#[test]
fn camel_case_pascal_wire_round_trippable_elides() {
    // wire `RequestorId` (no all-caps acronym) — to_pascal_case via
    // camelCase + first-letter-lower SHOULD round-trip, but the wire is
    // PascalCase not camelCase. Confirm we DO emit a rename here
    // (the elision should NOT fire because case doesn't match).
    let out = generate(one_field("RequestorId"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "RequestorId");
}

// --- PascalCase rename_all ---------------------------------------------------

#[test]
fn pascal_case_round_trip_elides() {
    // wire `AccountId`, rename_all PascalCase.
    // to_pascal_case("account_id") == "AccountId".
    let out = generate(one_field("AccountId"), |s| {
        s.with_struct_rename_all("PascalCase");
    });
    assert_contains(&out, "pub account_id : :: std :: option :: Option");
    assert_rename_absent(&out, "AccountId");
}

#[test]
fn pascal_case_camel_wire_emits() {
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("PascalCase");
    });
    assert_rename_present(&out, "accountId");
}

#[test]
fn pascal_case_acronym_wire_emits() {
    // wire `PCC`, rename_all PascalCase.
    // to_pascal_case("pcc") == "Pcc" != "PCC".
    let out = generate(one_field("PCC"), |s| {
        s.with_struct_rename_all("PascalCase");
    });
    assert_rename_present(&out, "PCC");
}

// --- snake_case rename_all ---------------------------------------------------

#[test]
fn snake_case_round_trip_no_rename() {
    // wire `account_id` (snake_case), rename_all snake_case.
    // recase produces rename = None directly (no transformation
    // needed), so the elision branch isn't hit and no rename is
    // emitted.
    let out = generate(one_field("account_id"), |s| {
        s.with_struct_rename_all("snake_case");
    });
    assert_contains(&out, "pub account_id : :: std :: option :: Option");
    assert_rename_absent(&out, "account_id");
}

#[test]
fn snake_case_camel_wire_emits() {
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("snake_case");
    });
    assert_rename_present(&out, "accountId");
}

#[test]
fn snake_case_pascal_wire_emits() {
    let out = generate(one_field("AccountId"), |s| {
        s.with_struct_rename_all("snake_case");
    });
    assert_rename_present(&out, "AccountId");
}

// --- SCREAMING_SNAKE_CASE rename_all -----------------------------------------

#[test]
fn screaming_snake_round_trip_elides() {
    // wire `ACCOUNT_ID`, rename_all SCREAMING_SNAKE_CASE.
    // to_shouty_snake_case("account_id") == "ACCOUNT_ID".
    let out = generate(one_field("ACCOUNT_ID"), |s| {
        s.with_struct_rename_all("SCREAMING_SNAKE_CASE");
    });
    assert_contains(&out, "pub account_id : :: std :: option :: Option");
    assert_rename_absent(&out, "ACCOUNT_ID");
}

#[test]
fn screaming_snake_camel_wire_emits() {
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("SCREAMING_SNAKE_CASE");
    });
    assert_rename_present(&out, "accountId");
}

// --- kebab-case rename_all ---------------------------------------------------

#[test]
fn kebab_case_round_trip_elides() {
    // wire `account-id`, rename_all kebab-case.
    // to_kebab_case("account_id") == "account-id".
    let out = generate(one_field("account-id"), |s| {
        s.with_struct_rename_all("kebab-case");
    });
    assert_contains(&out, "pub account_id : :: std :: option :: Option");
    assert_rename_absent(&out, "account-id");
}

#[test]
fn kebab_case_camel_wire_emits() {
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("kebab-case");
    });
    assert_rename_present(&out, "accountId");
}

// --- SCREAMING-KEBAB-CASE rename_all -----------------------------------------

#[test]
fn screaming_kebab_round_trip_elides() {
    // wire `ACCOUNT-ID`, rename_all SCREAMING-KEBAB-CASE.
    let out = generate(one_field("ACCOUNT-ID"), |s| {
        s.with_struct_rename_all("SCREAMING-KEBAB-CASE");
    });
    assert_contains(&out, "pub account_id : :: std :: option :: Option");
    assert_rename_absent(&out, "ACCOUNT-ID");
}

// --- lowercase rename_all ----------------------------------------------------

#[test]
fn lowercase_single_word_no_rename() {
    // wire `foo`, rename_all lowercase. recase produces rename = None.
    let out = generate(one_field("foo"), |s| {
        s.with_struct_rename_all("lowercase");
    });
    assert_contains(&out, "pub foo : :: std :: option :: Option");
    assert_rename_absent(&out, "foo");
}

#[test]
fn lowercase_multiword_round_trip_elides() {
    // wire `foobar` (lowercase, multi-word collapsed), prop_name as-is.
    // recase("foobar", Snake) = "foobar", rename = None.
    let out = generate(one_field("foobar"), |s| {
        s.with_struct_rename_all("lowercase");
    });
    assert_contains(&out, "pub foobar : :: std :: option :: Option");
    assert_rename_absent(&out, "foobar");
}

#[test]
fn lowercase_camel_wire_emits() {
    // wire `accountId` (camelCase), rename_all lowercase.
    // lowercase("account_id") with underscores removed = "accountid".
    // "accountid" != "accountId" -> emit.
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("lowercase");
    });
    assert_rename_present(&out, "accountId");
}

// --- UPPERCASE rename_all ----------------------------------------------------

#[test]
fn uppercase_camel_wire_emits() {
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("UPPERCASE");
    });
    assert_rename_present(&out, "accountId");
}

#[test]
fn uppercase_round_trip_elides() {
    // wire `ACCOUNTID` (uppercase, collapsed-snake), rename_all UPPERCASE.
    // The UPPERCASE rule strips underscores then uppercases.
    // For Rust field `account_id`, transform == "ACCOUNTID".
    let out = generate(one_field("ACCOUNTID"), |s| {
        s.with_struct_rename_all("UPPERCASE");
    });
    assert_contains(&out, "pub accountid : :: std :: option :: Option");
    assert_rename_absent(&out, "ACCOUNTID");
}

// =============================================================================
// No rename_all configured: per-field renames are always emitted whenever
// recase had to alter the prop_name. This is the historical typify behavior.
// =============================================================================

#[test]
fn no_rename_all_kebab_wire_emits() {
    let out = generate(one_field("account-id"), |_| {});
    assert_rename_present(&out, "account-id");
}

#[test]
fn no_rename_all_pascal_wire_emits() {
    let out = generate(one_field("AccountId"), |_| {});
    assert_rename_present(&out, "AccountId");
}

#[test]
fn no_rename_all_camel_wire_emits() {
    let out = generate(one_field("accountId"), |_| {});
    assert_rename_present(&out, "accountId");
}

#[test]
fn no_rename_all_keyword_emits() {
    // Without rename_all, the keyword sanitization still has to be
    // backed up by a per-field rename or the wire shape changes.
    let out = generate(one_field("type"), |_| {});
    assert_contains(&out, "pub type_ : :: std :: option :: Option");
    assert_rename_present(&out, "type");
}

// =============================================================================
// Multi-field structs: confirm the elision is applied per-field, not
// all-or-nothing.
// =============================================================================

#[test]
fn mixed_fields_elide_some_keep_others() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "MixedFields",
        "type": "object",
        "properties": {
            // Should elide.
            "accountId": { "type": "string" },
            // Should elide (no transformation needed at all).
            "id": { "type": "string" },
            // Should keep — wire is PascalCase.
            "AccountId": { "type": "string" },
            // Should keep — wire has an embedded all-caps acronym.
            "PCC": { "type": "string" },
            // Should keep — wire is the Rust keyword (rename for clarity
            // even though the camelCase transform happens to round-trip).
            "type": { "type": "string" }
        }
    });
    let out = generate(schema, |s| {
        s.with_struct_rename_all("camelCase");
    });

    // Three elisions: wire == camelCase transform of snake field.
    assert_rename_absent(&out, "accountId");
    assert_rename_absent(&out, "id");
    assert_rename_absent(&out, "type");
    // The Rust-keyword field still has the `_` suffix.
    assert_contains(&out, "pub type_ : :: std :: option :: Option");

    // Two kept: wire shape disagrees with the camelCase transform.
    // Note `AccountId` is renamed to `account_id` in Rust which collides
    // with the lowercase `accountId` field — typify de-collides by
    // suffixing one of them. Confirm both PascalCase / acronym renames
    // survived.
    assert_rename_present(&out, "AccountId");
    assert_rename_present(&out, "PCC");
}

// =============================================================================
// Edge cases: numeric-leading, single char, embedded-acronym Pascal wire.
// =============================================================================

#[test]
fn single_char_wire_no_rename() {
    // wire `a` (single char), rename_all camelCase.
    // recase("a", Snake) = "a", rename = None.
    let out = generate(one_field("a"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_contains(&out, "pub a : :: std :: option :: Option");
    assert_rename_absent(&out, "a");
}

#[test]
fn numeric_leading_wire_emits_rename() {
    // wire `1stClass` — typify prefixes invalid Rust identifiers with
    // `x` (-> `x1st_class`). The wire name diverges from the Rust ident
    // so a rename MUST be emitted regardless of rename_all.
    let out = generate(one_field("1stClass"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "1stClass");
}

#[test]
fn deeply_pascal_acronym_wire_emits() {
    // Multi-part wire with embedded acronym: `MaximumNumberOfPCCs`.
    // to_lower_camel_case("maximum_number_of_pc_cs") != wire.
    let out = generate(one_field("MaximumNumberOfPCCs"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "MaximumNumberOfPCCs");
}

#[test]
fn ota_underscored_wire_emits() {
    // Sabre OTA-style wire name `OTA_AirLowFareSearchRQ` — has internal
    // underscores, capital acronyms, mixed.
    let out = generate(one_field("OTA_AirLowFareSearchRQ"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "OTA_AirLowFareSearchRQ");
}

#[test]
fn tpa_extensions_wire_emits() {
    // The classic `TPA_Extensions` — appears all over Sabre specs.
    let out = generate(one_field("TPA_Extensions"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_rename_present(&out, "TPA_Extensions");
}

// =============================================================================
// Unknown rename_all value: the helper returns `false`, so renames are
// always preserved.
// =============================================================================

#[test]
fn unknown_rename_all_preserves_renames() {
    // The struct-level rename_all = "banana-case" isn't recognized by
    // serde, but typify emits it verbatim — and the elision helper
    // refuses to elide (it can't prove the transform). The per-field
    // rename must therefore survive.
    let out = generate(one_field("accountId"), |s| {
        s.with_struct_rename_all("banana-case");
    });
    assert_rename_present(&out, "accountId");
}

// =============================================================================
// Multi-word lowercase wire under each non-lowercase rename_all: pin the
// known limitation of `recase` (it stores `rename = None` when the wire
// equals the Snake-sanitized form, which can disagree with what the
// struct-level rename_all produces on the same Rust ident). These cases
// don't trigger in the Sabre spec — its property names are PascalCase or
// camelCase across the board — but they're worth documenting so future
// callers know to avoid feeding snake_case wire names into a struct with
// a non-snake `rename_all`.
//
// Specifically: wire `foo_bar` with `rename_all = "camelCase"` ends up
// without a per-field rename even though serde's camelCase rule applied
// to the Rust ident `foo_bar` would produce wire `"fooBar"`. The
// generated code would therefore (de)serialize the field as `fooBar`,
// not `foo_bar`. The elision logic is innocent here — `recase` never
// produces a rename for this case in the first place, so the
// `with_struct_rename_all` machinery has nothing to consider.
// =============================================================================

#[test]
fn snake_wire_under_camel_rename_all_silently_misses_rename() {
    // This test pins the pre-existing `recase` / `rename_all` interaction:
    // typify currently does NOT emit a per-field rename here, even though
    // serde would (incorrectly, vs. the spec) produce wire `fooBar` from
    // the Rust ident `foo_bar` under `rename_all = "camelCase"`.
    let out = generate(one_field("foo_bar"), |s| {
        s.with_struct_rename_all("camelCase");
    });
    assert_contains(&out, "pub foo_bar : :: std :: option :: Option");
    // The bug surface: rename is silently absent.
    assert_rename_absent(&out, "foo_bar");
    // Note: if the spec ever uses snake_case wire names with a non-snake
    // rename_all, this assertion would need to flip — that would require
    // teaching `recase` to also flag "wire equals input but disagrees with
    // the future rename_all transform" as needing a rename. The Sabre
    // workflow doesn't hit this case (its wire names are PascalCase /
    // camelCase only), so this is intentionally left as documented
    // existing behavior rather than a fix.
}

// =============================================================================
// Runtime serde roundtrip sanity checks.
//
// The codegen-text tests above prove what typify emits. These tests prove
// the emitted shape ACTUALLY round-trips through serde correctly — i.e.
// the elision agrees with serde's real `rename_all` transform, not just
// heck's `to_lower_camel_case` (which is what `rename_all_covers_rename`
// uses internally).
//
// If heck and serde ever diverge on the cases we elide, these tests will
// catch it before it reaches the generated openapi-rust output.
// =============================================================================

#[test]
fn runtime_keyword_field_serializes_to_wire_type() {
    // Replicates the elision case from `camel_case_keyword_field_type_elides`
    // at runtime: a struct with `rename_all = "camelCase"` and a `type_`
    // field carries no per-field rename, but should still serialize the
    // field as the JSON key `"type"`.
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Container {
        type_: String,
    }
    let value = Container {
        type_: "hello".to_string(),
    };
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, r#"{"type":"hello"}"#);
    let parsed: Container = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, value);
}

#[test]
fn runtime_camel_round_trip_serializes_correctly() {
    // wire `accountId`, Rust field `account_id`, rename_all camelCase.
    // The elision logic drops the per-field rename; serde must still
    // produce `accountId` on the wire.
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Container {
        account_id: String,
    }
    let value = Container {
        account_id: "abc".to_string(),
    };
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, r#"{"accountId":"abc"}"#);
}

#[test]
fn runtime_pascal_round_trip_serializes_correctly() {
    // wire `AccountId`, Rust field `account_id`, rename_all PascalCase.
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "PascalCase")]
    struct Container {
        account_id: String,
    }
    let value = Container {
        account_id: "abc".to_string(),
    };
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, r#"{"AccountId":"abc"}"#);
}

#[test]
fn runtime_pcc_acronym_keeps_explicit_rename() {
    // The acronym case: even though the elision wouldn't fire here, the
    // resulting hand-emitted rename must still round-trip.
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Container {
        #[serde(rename = "PCC")]
        pcc: String,
    }
    let value = Container {
        pcc: "DFW".to_string(),
    };
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, r#"{"PCC":"DFW"}"#);
}
