# Fork features

This fork of [oxidecomputer/typify](https://github.com/oxidecomputer/typify)
adds two layers on top of upstream:

1. **OpenAPI ingestion** — types generated directly from the component
   schemas of OpenAPI 3.0.x / 3.1.x / 3.2.x documents
   (`TypeSpace::add_openapi_document`, auto-detected by `cargo typify` and
   `import_types!`; see the README's "OpenAPI documents" section).
2. **Ergonomic generation mechanisms** — a small set of opt-in settings for
   producing API-client-style types, described below.

With no settings applied, output is byte-identical to upstream; every
pre-existing golden file in this repository is unchanged.

This document also serves as the migration guide from the retired
`ergonomic-codegen` branch: that branch's ~20 knobs are replaced by six
coherent mechanisms plus documented post-processing recipes.

## The six mechanisms

### 1. Subset-matching conversions

`TypeSpaceSettings::with_conversion` now matches by **subset**: a conversion
applies to input schemas that carry additional keywords beyond those the
conversion specifies, the most specific match wins, and metadata is ignored.
This makes `convert` usable on real-world schemas and replaces every
type-mapping knob of the old branch:

| Old knob | Replacement |
|---|---|
| `with_uuid_type(T)` | `convert = { { type = "string", format = "uuid" } = T }` |
| `with_date_type(T)` / `with_date_time_type(T)` | `format = "date"` / `"date-time"` conversions |
| `with_format_type(it, f, T)` | a conversion specifying `type` and `format` |
| `with_unconstrained_string(true)` | `{ type = "string" }` → `String` (note: also takes precedence over built-in format handling; add more-specific format conversions to carve out exceptions) |
| `with_unconstrained_int(true)` | per-format integer conversions |

### 2. Optional-properties policy

`with_optional_properties(OptionalProperties::Explicit)` represents **every**
non-required property as `Option<T>`, preserving the wire distinction between
absent and default-valued properties. Replaces `with_array_optionality`,
`with_default_bool_optionality`, and `with_defaulted_field_optionality`.

### 3. `allOf` composition

`with_all_of_strategy(AllOfStrategy::Compose)` interprets single-inheritance
`allOf` constructions (including sibling-properties forms) as structs
embedding each referenced base as a `#[serde(flatten)]` member. Constructions
that don't provably fit — non-object subschemas, colliding property names,
bases whose properties can't be enumerated — fall back to `Merge`.

### 4. Open string enums

`with_open_enum_variant("Other")` gives every plain string enum a trailing
`#[serde(untagged)] Other(String)` catch-all: undocumented wire values
round-trip losslessly. `FromStr` becomes irrefutable; opened enums drop
`Copy`; enums already declaring the name stay closed.

### 5. Docs without embedded schema

`with_schema_in_docs(false)` limits doc comments to the schema description
(the embedded JSON schema block remains the default).

### 6. Module organization and introspection

`TypeSpace::to_stream_for(&[TypeId])` generates a self-contained stream for a
subset of types (including exactly the shared error/defaults items those
types need); `TypeSpace::iter_definitions()` pairs definition names with
their types; `Type::id()` identifies types for subset selection. Consumers
assemble arbitrary module trees (see `typify/tests/modules.rs`), adding `use`
items for cross-module references. Replaces the old in-typify partitioned
emitter (`to_stream_partitioned` / `definition_rust_names`).

`rust_type_ident` / `rust_field_ident` (the identifier forms typify
generates for schema names) are exported for pre-generation selector
resolution, and `Type::default_derivable()` answers whether a type has — or
could, with a `Default` derive applied throughout, have — a `Default`
implementation (see "`Default` is type knowledge" below).

### `Default` is type knowledge

Some types can never satisfy `Default` (non-zero integers from
`"minimum": 1` schemas, enums without a schema default, and anything
requiring one of those) and some generate a `Default` impl of their own,
with which a derive would conflict. typify now omits a `Default` requested
via `with_derive`/patches from exactly those types, and exposes the
analysis as `Type::default_derivable()` — a house style that adds `Default`
to derive lists in post-processing must consult it (see the acceptance
test's `Decorate` pass) instead of deriving unconditionally.

All of 2–5 are exposed through `import_types!`
(`optional_properties = Explicit`, `all_of_strategy = Compose`,
`open_enum_variant = "Other"`, `schema_in_docs = false`) and `cargo typify`
(`--optional-properties`, `--all-of-strategy`, `--open-enum-variant`,
`--no-schema-in-docs`); 1 rides the existing `convert` syntax; 6 is
library-only.

## Decoration concerns: post-processing recipes

Everything below operates on **visible syntax** and is deliberately *not* a
typify setting; the executable recipe for each is the `syn` post-pass in
`typify-impl/tests/test_api_client.rs`, which reproduces the API-client house
style end to end.

| Old branch feature | Recipe |
|---|---|
| conditional / unconditional derives & attrs, ordering, positions, kind filters | visit `ItemStruct` / `ItemEnum`; insert attributes before or after the `derive` attr; rewrite the derive list |
| elide `Option` serde noise | paired with a struct-level `#[serde_with::skip_serializing_none]`: parse each field's `#[serde(...)]`, drop `default` + `skip_serializing_if = "..is_none"`, keep `rename` etc. (wire-neutral) |
| `struct_patch` deep patches + rename mirroring | add `#[patch(...)]` attrs in the same pass; identify `Option<generated struct>` fields via `TypeSpace::iter_types` / `TypeDetails` |
| struct `rename_all` + per-field elision | wire-neutral cosmetic: verify the case transform covers each field, then replace per-field renames |
| enum first-variant `Default` / untagged-`oneOf` `Default` synthesis | append an `impl Default` item |
| string newtype conveniences (`AsRef` / `Display` / `From<&str>`) | append impls; constrained-ness is visible via the presence of the validating `TryFrom` |
| partitioned / nested module output | assemble modules with `to_stream_for` (mechanism 6) |

## Dropped outright

`SerdeFieldCase` (dual casing surfaces) and the bulk
`DeepPatchPolicy::AllOptionStructs` had no consumers and were not carried
over.

## Consumer workarounds made unnecessary

Downstream feedback (the ferrotype migration and its real-world audit)
surfaced gaps that have since moved into typify; consumers carrying these
workarounds can delete them:

| Workaround | Superseded by |
|---|---|
| stripping string constraints in a lowering pass so a `{type: string}` conversion can't clobber string enums | conversions never match schemas whose `enum`/`const` they don't specify |
| hoisting the inners of nullable wrappers into synthetic `{name}Inner` definitions | Option-forming constructions name their inner types distinctly |
| omitting `default: null` from lowered documents | the type-array split no longer copies the schema default onto the inner type |
| dropping typify's manual `TryFrom<String>` where a `From<String>` blanket impl collides | native `::std::string::String` newtype inners take the string path (no manual `TryFrom` ladder) |
| a local port of typify's `sanitize` for selector resolution | exported `rust_type_ident` / `rust_field_ident` |
| unconditional `Default` in decoration derive lists (E0277 on required non-zero fields) | consult `Type::default_derivable()`; typify's own `with_derive("Default")` self-filters |
