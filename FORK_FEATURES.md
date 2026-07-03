# Fork features

This repository is a fork of [oxidecomputer/typify](https://github.com/oxidecomputer/typify)
that adds granular control over the generated code so that types produced from
OpenAPI / JSON Schema specs can match a deliberate, hand-written API-client
style (bare `Option<T>` fields, ordered derive lists, `struct_patch` deep
patches, `#[serde(flatten)]` inheritance, and so on) instead of typify's
stricter defaults.

Everything upstream still works; every feature below is opt-in via
`TypeSpaceSettings` (library), `import_types!` (macro), or `cargo typify`
(CLI) — with one deliberate default change, noted in
[Docs without embedded schema](#docs-without-embedded-schema).

The sections below map each feature to the settings API, the macro/CLI
surface, and the place in the code where it is implemented.

## Summary table

| Feature | Settings API | Macro key | CLI flag |
|---|---|---|---|
| Date type override | `with_date_type` | `date_type` | `--date-type` |
| Date-time type override | `with_date_time_type` | `date_time_type` | `--date-time-type` |
| UUID type override | `with_uuid_type` | `uuid_type` | `--uuid-type` |
| Unconstrained strings | `with_unconstrained_string` | `unconstrained_string` | `--unconstrained-string` |
| Unconstrained integers | `with_unconstrained_int` | `unconstrained_int` | `--unconstrained-int` |
| Optional arrays | `with_array_optionality` | `array_optionality` | `--array-optionality` |
| Optional defaulted bools | `with_default_bool_optionality` | `default_bool_optionality` | `--default-bool-optionality` |
| Optional defaulted fields | `with_defaulted_field_optionality` | `defaulted_field_optionality` | `--defaulted-field-optionality` |
| Elide `Option` serde noise | `with_elide_option_field_defaults` | `elide_option_field_defaults` | `--elide-option-field-defaults` |
| `allOf` composition | `with_allof_strategy` | `allof_strategy` | `--allof-strategy` |
| Conditional derives | `with_conditional_derive[_for]` | `conditional_derives` | `--conditional-derive` |
| Conditional attributes | `with_conditional_attr[_for/_at]` | `conditional_attrs` | `--conditional-attr` |
| Unconditional attributes | `with_unconditional_attr[_for/_at]` | — | — |
| Ordered derive lists | `with_unconditional_derive[_for]` | — | — |
| Struct `rename_all` + elision | `with_struct_rename_all` | — | — |
| Dual casing surfaces | `with_serde_field_case` | — | — |
| Enum first-variant `Default` | `with_enum_first_variant_default` | — | — |
| Deep patches (bulk) | `with_deep_patches` | `deep_patches` | `--deep-patches` |
| Deep patches (per-field) | `with_deep_patch_filter` | — (library only) | — |
| Schema in docs | `with_schema_in_docs` | `include_schema_in_docs` | `--include-schema-in-docs` |
| Partitioned output | `to_stream_partitioned` | — (library only) | — |
| Schema-name → Rust-name map | `definition_rust_names` | — (library only) | — |
| String newtype conveniences | always on | — | — |

## Native type overrides

**API:** `with_date_type`, `with_date_time_type`, `with_uuid_type`
**Implementation:** `typify-impl/src/convert.rs` (`convert_string`)

Override the Rust types used for `format: date`, `format: date-time`, and
`format: uuid` strings. The defaults remain `::chrono::naive::NaiveDate`,
`::chrono::DateTime<::chrono::offset::Utc>`, and `::uuid::Uuid`; overriding
(e.g. to `::std::string::String`) removes the corresponding crate dependency
from the generated code. `TypeSpace::uses_chrono()` / `uses_uuid()` report
`false` when an override is in place.

## Unconstrained strings

**API:** `with_unconstrained_string(bool)`
**Implementation:** `typify-impl/src/convert.rs` (`convert_string`)

By default, a string schema carrying `pattern`, `minLength`, or `maxLength`
becomes a `#[serde(transparent)]` newtype that validates on construction and
deserialization. With this knob on, such strings are emitted as plain
`String` — validation is the consumer's responsibility. This also avoids one
generated newtype per constrained field, a common source of type-name
explosion in large specs.

## Unconstrained integers

**API:** `with_unconstrained_int(bool)`
**Implementation:** `typify-impl/src/convert.rs` (`convert_integer`)

By default, integer schemas with `minimum: 1` map to `::std::num::NonZeroU*`.
With this knob on, they map to the plain `i32`/`u32`/… type instead.

## Array optionality

**API:** `with_array_optionality(ArrayOptionality)`
**Implementation:** `typify-impl/src/structs.rs` (`has_default`)

- `Bare` (default): non-required arrays are `Vec<T>` with
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
- `OptionalIfNotRequired`: non-required arrays are `Option<Vec<T>>`,
  preserving the wire distinction between "absent" and "empty". An explicit
  schema `default: []` is treated as absent under this mode.

## Defaulted bool optionality

**API:** `with_default_bool_optionality(DefaultBoolOptionality)`
**Implementation:** `typify-impl/src/structs.rs` (`has_default`)

- `Bare` (default): a non-required bool with a schema `default` collapses to
  bare `bool` + `#[serde(default)]`.
- `AlwaysOption`: the field becomes `Option<bool>`, preserving the
  distinction between "absent" and "explicitly false/true".

## Defaulted field optionality

**API:** `with_defaulted_field_optionality(DefaultedFieldOptionality)`
**Implementation:** `typify-impl/src/structs.rs` (`has_default`)

- `Bare` (default): a non-required field with a schema `default` stays bare
  and the default is lifted into a generated `defaults::` helper fn
  referenced from `#[serde(default = "defaults::...")]`.
- `AlwaysOption`: the schema default is dropped and the field is wrapped in
  `Option<T>` like every other non-required field; the dead `defaults::`
  helper is not emitted. Bools are governed by
  [`DefaultBoolOptionality`](#defaulted-bool-optionality) and are unaffected.
  Required fields are never wrapped.

## Elide `Option` serde noise

**API:** `with_elide_option_field_defaults(bool)`
**Implementation:** `typify-impl/src/structs.rs` (`generate_serde_attr`)

Drops the per-field `#[serde(default, skip_serializing_if =
"::std::option::Option::is_none")]` pair on `Option<T>` fields when that pair
is exactly what would have been emitted. Pair this with a struct-level
`#[serde_with::skip_serializing_none]` (see
[Unconditional attributes](#unconditional-attributes)): that attribute covers
serialize-omission, and `Option<T>` already deserializes a missing key as
`None`. Per-field `rename` / `flatten` attributes and named-default fns are
preserved.

## `allOf` composition

**API:** `with_allof_strategy(AllOfStrategy)`
**Implementation:** `typify-impl/src/convert.rs` (`convert_all_of_compose`,
`try_compose_with_sibling_properties`, hooks in `convert_all_of` and the
"subschemas with other stuff" branch of `convert_schema_object`)

- `Merge` (default): typify's historical behavior — subschemas are merged
  into one flat schema and the emitted struct contains the union of all
  properties.
- `Compose`: for the classic single-inheritance pattern

  ```yaml
  Agency:
    type: object
    properties:          # optional sibling properties
      ticketingPolicy: ...
    allOf:
      - $ref: "#/definitions/GenericAgency"
      - type: object
        properties:
          customerNumber: ...
  ```

  the generated struct embeds each `$ref` base as a
  `#[serde(flatten)] pub generic_agency: GenericAgency` field and keeps the
  inline properties as ordinary fields, rather than copying the base type's
  fields into the derived struct. Falls back to `Merge` whenever the shape
  doesn't compose cleanly (no `$ref` bases, colliding property names,
  non-object subschemas, nested combinators, …).

  Two semantic deviations to be aware of:
  1. `#[serde(flatten)]` is incompatible with
     `#[serde(deny_unknown_fields)]`, so the latter is dropped on composed
     structs.
  2. `#[serde_with::skip_serializing_none]` does not recurse into a
     flattened sub-struct; base types with `Option` fields should carry the
     attribute themselves.

## Conditional derives and attributes

**API:** `with_conditional_derive[_for]`, `with_conditional_attr[_for/_at]`
**Implementation:** `typify-impl/src/lib.rs` (settings),
`typify-impl/src/type_entry.rs` (`type_attr_lists`)

Emit `#[cfg_attr(feature = "<feature>", derive(<D>))]` /
`#[cfg_attr(feature = "<feature>", <attr>)]` on generated types so consumers
opt in via Cargo features (e.g. a `schemars` feature gating
`schemars::JsonSchema`). The `_for` variants scope emission by type kind via
[`TypeKindFilter`](#typekindfilter); `with_conditional_attr_at` additionally
chooses the position relative to the main `#[derive(...)]` via
[`AttrPosition`](#attrposition).

## Unconditional derives and attributes

**API:** `with_unconditional_derive[_for]`, `with_unconditional_attr[_for/_at]`
**Implementation:** `typify-impl/src/type_entry.rs` (`compute_derive_list`,
`type_attr_lists`)

### Ordered derive lists

`with_unconditional_derive` registers derives that are emitted **in insertion
order**. As soon as one is registered for a type kind, the historical sorted
base set (`Debug`, `Clone`, `::serde::Serialize`, `::serde::Deserialize`) is
suppressed for that kind and the caller's list is authoritative. This is the
only way to control derive ordering — needed when the target style demands
e.g. `Debug, Clone, Default, PartialEq, Serialize, Deserialize, Patch`.
Per-type patches and `with_derive` extras are appended after the caller list.
Derives that a bespoke impl replaces (e.g. `Deserialize` on constrained
newtypes) are filtered out of the caller's list too, so a conflicting derive
and manual impl are never emitted together.

### Unconditional attributes

`with_unconditional_attr*` emits verbatim attributes (no `cfg_attr` gate) —
e.g. `#[serde_with::skip_serializing_none]` before the derive, or a block of
`#[patch(attribute(...))]` lines after it. Multiple calls preserve insertion
order.

### `AttrPosition`

`BeforeDerive` (default) places the attribute above `#[derive(...)]` — for
attribute macros that transform the upcoming derive. `AfterDerive` places it
below the derive and the type-level `#[serde(...)]` — for attributes that
configure the just-applied derive (`struct_patch`'s `#[patch(attribute(...))]`
lines).

### `TypeKindFilter`

Selects which generated categories an entry applies to: `ALL`, `STRUCTS`,
`ENUMS`, `NEWTYPES`, or any combination via struct literal. Needed because
some derives are kind-specific (`struct_patch::Patch` panics on enums).

## Struct-level `rename_all` with per-field elision

**API:** `with_struct_rename_all("camelCase")`
**Implementation:** `typify-impl/src/structs.rs` (`struct_property`),
`typify-impl/src/util.rs` (`rename_all_covers_rename`),
`typify-impl/src/type_entry.rs` (`output_struct`)

Emits `#[serde(rename_all = "<case>")]` on every generated struct and drops
each per-field `#[serde(rename = "...")]` whose wire name is exactly the
`<case>` transform of the snake-cased Rust field name. Only fields whose wire
name disagrees with the case convention keep an explicit rename. All eight
serde case conventions are supported.

Because a field may now be covered by `rename_all` without a per-field
rename, the internal `StructProperty` carries the authoritative `wire_name`,
which the default-value validation (`defaults.rs`) and value-output
(`value.rs`) paths use instead of inferring the wire name from the rename.

## Dual casing surfaces (`SerdeFieldCase`)

**API:** `with_serde_field_case(SerdeFieldCase::Wire | Snake)`
**Implementation:** `typify-impl/src/structs.rs` (`generate_serde_attr`)

Generates explicit per-field `rename` + `alias` pairs instead of relying on
`rename_all`:

- `Wire`: serialize the exact schema property name, accept the snake_case
  name as an input alias.
- `Snake`: serialize the snake_case name, accept the exact schema property
  name as an input alias.

Generate the same spec twice (once per mode) to get a spec-faithful "wire"
surface and an idiomatic "api" surface whose types accept each other's JSON.
When set, `with_struct_rename_all` is not emitted (the explicit per-field
names subsume it).

## Enum first-variant `Default`

**API:** `with_enum_first_variant_default(bool)`
**Implementation:** `typify-impl/src/type_entry.rs` (`output_enum`)

Enums without a schema-level `default` get an
`impl Default` selecting the first Simple (unit) variant. This makes structs
that derive `Default` (via [ordered derive lists](#ordered-derive-lists))
compile when they contain required enum fields. Enums with a schema `default`
keep the schema-mandated impl; enums with no Simple variants are skipped.

## Deep patches for `struct_patch`

**API:** `with_deep_patches(DeepPatchPolicy)`, `with_deep_patch_filter(closure)`
**Implementation:** `typify-impl/src/type_entry.rs` (`deep_patch_attr`)

With upstream `struct_patch`, an `Option<Inner>` field becomes
`Option<Option<Inner>>` in the generated patch type — a partial sub-object
replaces the whole value. Emitting `#[patch(name = "Option<InnerPatch>")]`
above the field instead makes the patch recurse (`Option<InnerPatch>`), a
true deep partial-merge.

- `DeepPatchPolicy::AllOptionStructs` annotates every `Option<{Struct}>`
  (including `Option<Box<{Struct}>>`) field.
- `with_deep_patch_filter(|owner, field, inner| ...)` decides per field;
  when set it takes precedence over the bulk policy. Library-only (closures
  can't cross the macro boundary).

Fields that would not type-check are never annotated: `#[serde(flatten)]`
bases, `Vec<_>`, and `Option<T>` where `T` is an enum / newtype / primitive.
Pair with `with_unconditional_derive_for("Patch", TypeKindFilter::STRUCTS)`
so the annotation has a derive to ride on.

## Docs without embedded schema

**API:** `with_schema_in_docs(bool)`
**Implementation:** `typify-impl/src/type_entry.rs` (`make_doc`)

**This is the one intentional default change relative to upstream.** Doc
comments now contain only the schema `description` by default; the full
pretty-printed JSON Schema is opt-in. Two reasons:

1. IDE hovers (rust-analyzer, and editors embedding it) rendered the
   historical `<details><summary>JSON schema</summary>…</details>` block as
   unstyled wrapped text — CommonMark splits the raw-HTML wrapper around the
   fenced code block.
2. The wrapper lines used `///` (one leading space in the doc string) while
   schema lines used `#[doc = "..."]` (zero leading space); the asymmetry
   broke fenced-code indent stripping.

When enabled, the schema is emitted under a `# JSON schema` markdown heading
with a fenced ```` ```json ```` block, every line as explicit
`#[doc = "..."]` — which renders correctly in both rustdoc and hovers.

## Partitioned output

**API:** `TypeSpace::to_stream_partitioned(partition, default_module, imports_per_module)`
**Implementation:** `typify-impl/src/lib.rs`

Instead of one flat stream, emits a sequence of `pub mod <name> { ... }`
blocks. `partition` maps generated Rust type names to module names; anything
unmapped lands in `default_module`. `imports_per_module` injects a `use`
preamble into each module (e.g. `use super::shared::*;` for cross-module
references, or `use serde::{Serialize, Deserialize};` so bare derive paths
resolve). The `error` module (`ConversionError`) and the shared `defaults`
helpers are duplicated into every leaf partition so intra-module paths
(`self::error::...`, `defaults::...`) resolve without extra imports.

Module names — partition values, `default_module`, and
`imports_per_module` keys alike — may be slash-separated paths such as
`cancel_booking/request` or `shared/enums`. Nested paths are emitted as
properly nested blocks, with siblings merged under a common parent in
deterministic (BTreeMap) order:

```rust
pub mod cancel_booking {
    pub mod request { /* types + error/defaults */ }
    pub mod response { /* ... */ }
}
pub mod shared {
    pub mod enums { /* ... */ }
}
```

Only leaf partitions (and any partition that types were explicitly
assigned to) receive the duplicated `error`/`defaults` submodules;
intermediate path components are pure containers holding their child
modules plus any preamble attached to that exact path key (e.g. an
`imports_per_module` entry for `"cancel_booking"` itself). Flat
(single-segment) names behave exactly as before.

Use this to split a large OpenAPI spec into one module per operation with a
`shared` module for common types, or — with nested paths — into
per-operation `request`/`response` submodules mirroring a hand-written
client layout.

## Schema-name → Rust-name mapping

**API:** `TypeSpace::definition_rust_names()`
**Implementation:** `typify-impl/src/lib.rs`

Returns `(schema_key, rust_type_name)` pairs for every named definition
(`definitions` / `components.schemas` entry). Lets callers build the
partition map for `to_stream_partitioned` from reachability analysis keyed by
original schema names, without re-implementing typify's Pascal-case
sanitization.

## String newtype conveniences

**Implementation:** `typify-impl/src/type_entry.rs` (`output_newtype`)

Newtypes wrapping `String` always implement `AsRef<str>` and `Display`
(printing the inner value). Unconstrained string newtypes additionally get
`From<&str>`; constrained ones must go through the validating
`FromStr` / `TryFrom` path.

## Tests

The fork-specific behavior is pinned by four test suites in
`typify-impl/tests/`:

- `test_wire_shape.rs` — the wire-shape knobs (type overrides, unconstrained
  string/int, optionality knobs, `allOf` compose, conditional derives/attrs,
  deep patches, elision).
- `test_rename_elision.rs` — the `rename_all` elision matrix across all
  serde case conventions plus serde runtime round-trips proving elision
  agrees with serde.
- `test_doc_format.rs` — the doc-comment format with and without
  `with_schema_in_docs`.
- `test_partitioned_output.rs` — `to_stream_partitioned` module emission:
  nested slash-separated paths, flat/nested mixes, preambles on leaf and
  parent paths, and `error`/`defaults` duplication into leaves only.
