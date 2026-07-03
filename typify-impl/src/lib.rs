// Copyright 2025 Oxide Computer Company

//! typify backend implementation.

#![deny(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use conversions::SchemaCache;
use log::{debug, info};
use output::OutputSpace;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use schemars::schema::{Metadata, RootSchema, Schema};
use thiserror::Error;
use type_entry::{
    StructPropertyState, TypeEntry, TypeEntryDetails, TypeEntryNative, TypeEntryNewtype,
    WrappedValue,
};

use crate::util::{sanitize, Case};

pub use crate::util::accept_as_ident;

#[cfg(test)]
mod test_util;

mod conversions;
mod convert;
mod cycles;
mod defaults;
mod enums;
mod merge;
mod output;
mod rust_extension;
mod structs;
mod type_entry;
mod util;
mod validate;
mod value;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("unexpected value type")]
    BadValue(String, serde_json::Value),
    #[error("invalid TypeId")]
    InvalidTypeId,
    #[error("value does not conform to the given schema")]
    InvalidValue,
    #[error("invalid schema for {}: {reason}", show_type_name(.type_name.as_deref()))]
    InvalidSchema {
        type_name: Option<String>,
        reason: String,
    },
}

impl Error {
    fn invalid_value() -> Self {
        Self::InvalidValue
    }
}

#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, Error>;

fn show_type_name(type_name: Option<&str>) -> &str {
    type_name.unwrap_or("<unknown type>")
}

/// Representation of a type which may have a definition or may be built-in.
#[derive(Debug)]
pub struct Type<'a> {
    type_space: &'a TypeSpace,
    type_entry: &'a TypeEntry,
}

#[allow(missing_docs)]
/// Type details returned by Type::details() to inspect a type.
pub enum TypeDetails<'a> {
    Enum(TypeEnum<'a>),
    Struct(TypeStruct<'a>),
    Newtype(TypeNewtype<'a>),

    Option(TypeId),
    Vec(TypeId),
    Map(TypeId, TypeId),
    Set(TypeId),
    Box(TypeId),
    Tuple(Box<dyn Iterator<Item = TypeId> + 'a>),
    Array(TypeId, usize),
    Builtin(&'a str),

    Unit,
    String,
}

/// Enum type details.
pub struct TypeEnum<'a> {
    details: &'a type_entry::TypeEntryEnum,
}

/// Enum variant details.
pub enum TypeEnumVariant<'a> {
    /// Variant with no associated data.
    Simple,
    /// Tuple-type variant with at least one associated type.
    Tuple(Vec<TypeId>),
    /// Struct-type variant with named properties and types.
    Struct(Vec<(&'a str, TypeId)>),
}

/// Full information pertaining to an enum variant.
pub struct TypeEnumVariantInfo<'a> {
    /// Name.
    pub name: &'a str,
    /// Description.
    pub description: Option<&'a str>,
    /// Details for the enum variant.
    pub details: TypeEnumVariant<'a>,
}

/// Struct type details.
pub struct TypeStruct<'a> {
    details: &'a type_entry::TypeEntryStruct,
}

/// Full information pertaining to a struct property.
pub struct TypeStructPropInfo<'a> {
    /// Name.
    pub name: &'a str,
    /// Description.
    pub description: Option<&'a str>,
    /// Whether the propertty is required.
    pub required: bool,
    /// Identifies the schema for the property.
    pub type_id: TypeId,
}

/// Newtype details.
pub struct TypeNewtype<'a> {
    details: &'a type_entry::TypeEntryNewtype,
}

/// Type identifier returned from type creation and used to lookup types.
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone, Hash)]
pub struct TypeId(u64);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Name {
    Required(String),
    Suggested(String),
    Unknown,
}

impl Name {
    pub fn into_option(self) -> Option<String> {
        match self {
            Name::Required(s) | Name::Suggested(s) => Some(s),
            Name::Unknown => None,
        }
    }

    pub fn append(&self, s: &str) -> Self {
        match self {
            Name::Required(prefix) | Name::Suggested(prefix) => {
                Self::Suggested(format!("{}_{}", prefix, s))
            }
            Name::Unknown => Name::Unknown,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum RefKey {
    Root,
    Def(String),
}

/// A collection of types.
#[derive(Debug)]
pub struct TypeSpace {
    next_id: u64,

    // TODO we need this in order to inspect the collection of reference types
    // e.g. to do `all_mutually_exclusive`. In the future, we could obviate the
    // need this by keeping a single Map of referenced types whose value was an
    // enum of a "raw" or a "converted" schema.
    definitions: BTreeMap<RefKey, Schema>,

    id_to_entry: BTreeMap<TypeId, TypeEntry>,
    type_to_id: BTreeMap<TypeEntryDetails, TypeId>,

    name_to_id: BTreeMap<String, TypeId>,
    ref_to_id: BTreeMap<RefKey, TypeId>,

    uses_chrono: bool,
    uses_uuid: bool,
    uses_serde_json: bool,
    uses_regress: bool,

    settings: TypeSpaceSettings,

    cache: SchemaCache,

    // Shared functions for generating default values
    defaults: BTreeSet<DefaultImpl>,
}

impl Default for TypeSpace {
    fn default() -> Self {
        Self {
            next_id: 1,
            definitions: Default::default(),
            id_to_entry: Default::default(),
            type_to_id: Default::default(),
            name_to_id: Default::default(),
            ref_to_id: Default::default(),
            uses_chrono: Default::default(),
            uses_uuid: Default::default(),
            uses_serde_json: Default::default(),
            uses_regress: Default::default(),
            settings: Default::default(),
            cache: Default::default(),
            defaults: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefaultImpl {
    Boolean,
    I64,
    U64,
    NZU64,
}

/// Type name to use in generated code.
#[derive(Clone)]
pub struct MapType(pub syn::Type);

impl MapType {
    /// Create a new MapType from a [`str`].
    pub fn new(s: &str) -> Self {
        let map_type = syn::parse_str::<syn::Type>(s).expect("valid ident");
        Self(map_type)
    }
}

impl Default for MapType {
    fn default() -> Self {
        Self::new("::std::collections::HashMap")
    }
}

impl std::fmt::Debug for MapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MapType({})", self.0.to_token_stream())
    }
}

impl std::fmt::Display for MapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.to_token_stream().fmt(f)
    }
}

impl<'de> serde::Deserialize<'de> for MapType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

impl From<String> for MapType {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl From<&str> for MapType {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<syn::Type> for MapType {
    fn from(t: syn::Type) -> Self {
        Self(t)
    }
}

/// Settings that alter type generation.
#[derive(Default, Debug, Clone)]
pub struct TypeSpaceSettings {
    type_mod: Option<String>,
    extra_derives: Vec<String>,
    /// Derives emitted as `#[cfg_attr(feature = "...", derive(...))]` so
    /// consumers can opt in / out via a Cargo feature. See
    /// [`Self::with_conditional_derive`].
    conditional_derives: Vec<ConditionalDerive>,
    extra_attrs: Vec<String>,
    /// Attributes emitted as `#[cfg_attr(feature = "...", ...)]`. See
    /// [`Self::with_conditional_attr`].
    conditional_attrs: Vec<ConditionalAttr>,
    /// Unconditional (i.e. not cfg-gated) attributes added by
    /// [`Self::with_unconditional_attr`] / [`Self::with_unconditional_attr_for`]
    /// / [`Self::with_unconditional_attr_at`]. Each carries an
    /// [`AttrPosition`] that controls placement relative to `#[derive(...)]`.
    unconditional_attrs: Vec<UnconditionalAttr>,
    /// Unconditional derives added by [`Self::with_unconditional_derive`] /
    /// [`Self::with_unconditional_derive_for`]. When any entry matches a
    /// given type kind, the resulting `#[derive(...)]` list for that kind
    /// is taken from this `Vec` (preserving insertion order) **instead** of
    /// the historical sorted base set (`Debug`, `Clone`, `Serialize`,
    /// `Deserialize`). This lets callers specify the exact ordering of the
    /// derive list.
    unconditional_derives: Vec<UnconditionalDerive>,
    /// Policy for emitting `#[patch(name = "Option<{Inner}Patch>")]` on
    /// `Option<{InnerStruct}>` struct fields. See
    /// [`Self::with_deep_patches`].
    deep_patches: DeepPatchPolicy,
    /// Per-field predicate that takes precedence over [`Self::deep_patches`]
    /// when set. See [`Self::with_deep_patch_filter`].
    deep_patch_filter: Option<DeepPatchFilter>,
    /// Policy for emitting `Option<T> + #[serde(default, skip_serializing_if =
    /// ...)]` instead of `T + #[serde(default = "defaults::...")]` for
    /// non-required struct fields that carry a schema-level `default:`
    /// directive. See [`Self::with_defaulted_field_optionality`].
    defaulted_field_optionality: DefaultedFieldOptionality,
    /// When `true`, drop the per-field `#[serde(default,
    /// skip_serializing_if = "::std::option::Option::is_none")]` on
    /// `Option<T>` fields whose serde-attribute set would otherwise be
    /// exactly that pair. See [`Self::with_elide_option_field_defaults`].
    elide_option_field_defaults: bool,
    /// When set, every generated struct emits
    /// `#[serde(rename_all = "<case>")]` at the struct level and per-field
    /// `#[serde(rename = "...")]` attributes covered by the case transform
    /// are elided. See [`Self::with_struct_rename_all`].
    struct_rename_all: Option<String>,
    /// Optional per-field serde naming mode. When unset, typify preserves the
    /// historical `rename` / `rename_all` behavior. When set, generated struct
    /// fields carry explicit `rename` / `alias` pairs. See
    /// [`Self::with_serde_field_case`].
    serde_field_case: Option<SerdeFieldCase>,
    /// When `true`, every generated enum lacking a schema-level `default:`
    /// value gets an auto-generated `impl Default` that picks the first
    /// Simple (unit) variant. See [`Self::with_enum_first_variant_default`].
    enum_first_variant_default: bool,
    struct_builder: bool,

    unknown_crates: UnknownPolicy,
    crates: BTreeMap<String, CrateSpec>,
    map_type: MapType,

    patch: BTreeMap<String, TypeSpacePatch>,
    replace: BTreeMap<String, TypeSpaceReplace>,
    convert: Vec<TypeSpaceConversion>,

    // Overrides for native types selected by JSON Schema `format`. When
    // `None`, the upstream defaults (`::chrono::naive::NaiveDate`,
    // `::chrono::DateTime<::chrono::offset::Utc>`, `::uuid::Uuid`) are used.
    date_type: Option<String>,
    date_time_type: Option<String>,
    uuid_type: Option<String>,

    // Wire-shape knobs that relax typify's "stricter than the wire"
    // defaults; each is opt-in and defaults to the historical behavior.
    unconstrained_string: bool,
    unconstrained_int: bool,
    array_optionality: ArrayOptionality,
    default_bool_optionality: DefaultBoolOptionality,
    allof_strategy: AllOfStrategy,

    // When `true`, every generated type's doc comment includes a fenced
    // JSON-Schema block under a `# JSON schema` heading. When `false`
    // (the default), the doc comment contains only the human-readable
    // description so IDE hovers stay readable. See
    // [`Self::with_schema_in_docs`].
    include_schema_in_docs: bool,
}

/// Per-field serde naming mode for generated struct properties.
///
/// This is intentionally opt-in so existing callers keep the legacy
/// `rename` / `rename_all`-driven output unless they request a dual casing
/// surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerdeFieldCase {
    /// Serialize exact schema property names and accept snake_case aliases
    /// on input.
    Wire,
    /// Serialize snake_case property names and accept exact schema property
    /// names on input.
    Snake,
}

/// A `#[derive]` that is emitted as `#[cfg_attr(feature = "...", derive(...))]`
/// so consumers can opt-in / opt-out of a derive via a Cargo feature.
#[derive(Debug, Clone)]
pub(crate) struct ConditionalDerive {
    pub(crate) cfg: String,
    pub(crate) derive: String,
    pub(crate) kinds: TypeKindFilter,
}

/// An attribute that is emitted as `#[cfg_attr(feature = "...", ...)]`.
#[derive(Debug, Clone)]
pub(crate) struct ConditionalAttr {
    pub(crate) cfg: String,
    pub(crate) attr: String,
    pub(crate) position: AttrPosition,
    pub(crate) kinds: TypeKindFilter,
}

/// An unconditional (i.e. not cfg-gated) attribute emitted on every
/// generated type matching `kinds`. The `position` controls placement
/// relative to the main `#[derive(...)]` attribute on the type.
#[derive(Debug, Clone)]
pub(crate) struct UnconditionalAttr {
    pub(crate) attr: String,
    pub(crate) position: AttrPosition,
    pub(crate) kinds: TypeKindFilter,
}

/// An unconditional derive emitted in the main `#[derive(...)]` list of
/// every generated type matching `kinds`. When at least one
/// [`UnconditionalDerive`] matches a given type kind, the resulting
/// derive list **replaces** the historical sorted base set
/// (`Debug`, `Clone`, `::serde::Serialize`, `::serde::Deserialize`) — see
/// [`TypeSpaceSettings::with_unconditional_derive`] for details.
#[derive(Debug, Clone)]
pub(crate) struct UnconditionalDerive {
    pub(crate) derive: String,
    pub(crate) kinds: TypeKindFilter,
}

/// Where an emitted attribute is placed relative to the type's main
/// `#[derive(...)]` attribute. This matters when matching a hand-written
/// style that orders attributes deliberately — e.g. `serde_with`'s
/// `skip_serializing_none` macro must precede the derive that consumes
/// it, while `struct_patch`'s `#[patch(attribute(...))]` lines
/// conventionally follow the derive that produces them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AttrPosition {
    /// Emitted *before* the main `#[derive(...)]` attribute (and before
    /// any `#[cfg_attr(feature = ..., derive(...))]` lines). This is the
    /// historical position used by `serde_with::skip_serializing_none`
    /// and similar attribute-style macros that transform the upcoming
    /// `#[derive]`.
    #[default]
    BeforeDerive,
    /// Emitted *after* the main `#[derive(...)]` attribute and after any
    /// type-level `#[serde(...)]` line. Use this for attributes that
    /// configure the just-applied derive — e.g.
    /// `#[patch(attribute(...))]` lines that ride the
    /// `#[derive(struct_patch::Patch)]` macro.
    AfterDerive,
}

/// Selector for which generated type categories a conditional derive or
/// attribute should be applied to.
///
/// Some derives only make sense for one kind of generated type — for example
/// `struct_patch::Patch` is a `#[proc_macro_derive]` that panics on enums.
/// Use [`TypeKindFilter`] to scope a derive to the kinds it actually
/// supports.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TypeKindFilter {
    /// Apply to generated `pub struct` declarations.
    pub structs: bool,
    /// Apply to generated `pub enum` declarations.
    pub enums: bool,
    /// Apply to generated `#[serde(transparent)]` newtypes.
    pub newtypes: bool,
}

impl TypeKindFilter {
    /// Apply to all categories (the historical default).
    pub const ALL: Self = Self {
        structs: true,
        enums: true,
        newtypes: true,
    };

    /// Apply only to structs. Use this for derives such as
    /// `struct_patch::Patch` that the proc-macro forbids on enums.
    pub const STRUCTS: Self = Self {
        structs: true,
        enums: false,
        newtypes: false,
    };

    /// Apply only to enums.
    pub const ENUMS: Self = Self {
        structs: false,
        enums: true,
        newtypes: false,
    };

    /// Apply only to newtypes.
    pub const NEWTYPES: Self = Self {
        structs: false,
        enums: false,
        newtypes: true,
    };

    /// `true` if this filter targets the given [`TypeKind`].
    pub(crate) fn matches(&self, kind: TypeKind) -> bool {
        match kind {
            TypeKind::Struct => self.structs,
            TypeKind::Enum => self.enums,
            TypeKind::Newtype => self.newtypes,
        }
    }
}

/// The category of a generated type, used to filter conditional and
/// unconditional derives / attributes against the configured
/// [`TypeKindFilter`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeKind {
    Struct,
    Enum,
    Newtype,
}

/// Controls how arrays that are *not* in the schema's `required` list are
/// emitted on a generated struct.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum ArrayOptionality {
    /// Bare `Vec<T>` with `#[serde(default, skip_serializing_if =
    /// "Vec::is_empty")]`. This is the historical typify behavior.
    #[default]
    Bare,
    /// Wrap in `Option<Vec<T>>` with `#[serde(default, skip_serializing_if =
    /// "Option::is_none")]`, distinguishing an absent array from an empty
    /// array on the wire.
    OptionalIfNotRequired,
}

/// Controls how boolean properties that have a schema-level `default:` are
/// emitted on a generated struct.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum DefaultBoolOptionality {
    /// Bare `bool` with `#[serde(default)]`. This is the historical typify
    /// behavior.
    #[default]
    Bare,
    /// Wrap in `Option<bool>` when not required, distinguishing "field
    /// absent" from "field explicitly false".
    AlwaysOption,
}

/// Controls whether non-required struct fields carrying a schema-level
/// `default:` directive are emitted as bare `T` with `#[serde(default =
/// "defaults::...")]` or as `Option<T>` with the standard
/// `#[serde(default, skip_serializing_if = "Option::is_none")]` shape.
///
/// `Bare` reproduces typify's historical behavior: the schema default is
/// lifted into the generated type via a `defaults::` helper function and
/// the field stays bare. `AlwaysOption` drops the schema default and
/// wraps the field in `Option<T>`, which is what every other non-required
/// field looks like. The `defaults::` helper for that field is not
/// emitted at all under `AlwaysOption` — there's no caller for it.
///
/// `bool` fields are governed by [`DefaultBoolOptionality`] instead, so
/// this knob deliberately does not affect them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum DefaultedFieldOptionality {
    /// Lift the schema default into the type via a generated `defaults::`
    /// helper. Historical typify behavior.
    #[default]
    Bare,
    /// Suppress the schema default and wrap the field in `Option<T>`.
    AlwaysOption,
}

/// Controls whether `#[patch(name = "Option<{Inner}Patch>")]` is emitted
/// above struct fields of type `Option<{InnerStruct}>` so the
/// `struct_patch::Patch` derive produces a deep partial-merge shape.
///
/// `Off` reproduces upstream `struct_patch` behavior: an `Option<T>` field
/// becomes `Option<Option<T>>` in the patch type, so a partial sub-object
/// overwrites unspecified fields with `Default::default()`.
/// `AllOptionStructs` gives every `Option<{InnerStruct}>` field the
/// `#[patch(name = ...)]` rewrite so its patch field is
/// `Option<{Inner}Patch>` — a true deep partial-merge.
///
/// The knob only fires for fields whose Rust type is exactly
/// `Option<{InnerStruct}>` (or `Option<Box<{InnerStruct}>>`). Fields
/// that carry `#[serde(flatten)]`, are `Vec<...>`, or whose inner type
/// is an enum / newtype / primitive are skipped — those are not
/// Patch-able and the rewrite would not type-check.
///
/// To target a hand-curated subset of fields rather than every
/// `Option<{InnerStruct}>`, use
/// [`TypeSpaceSettings::with_deep_patch_filter`] in addition or instead.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum DeepPatchPolicy {
    /// Do not emit `#[patch(name = ...)]`. Historical behavior.
    #[default]
    Off,
    /// Emit `#[patch(name = "Option<{Inner}Patch>")]` above every
    /// `Option<{InnerStruct}>` field whose `{InnerStruct}` is a generated
    /// struct.
    AllOptionStructs,
}

/// Closure signature used by [`DeepPatchFilter`]: `(owner_struct_name,
/// field_name, inner_struct_name) -> bool`. Stored behind an `Arc` so
/// the enclosing [`TypeSpaceSettings`] can stay `Clone`.
type DeepPatchFilterFn = dyn Fn(&str, &str, &str) -> bool + Send + Sync;

/// User-supplied predicate that decides per `(owner_struct_name,
/// field_name, inner_struct_name)` whether to emit `#[patch(name =
/// "Option<{Inner}Patch>")]` for that field. Stored as an `Arc<dyn Fn>`
/// so the enclosing [`TypeSpaceSettings`] stays `Clone`.
#[derive(Clone)]
pub struct DeepPatchFilter(Arc<DeepPatchFilterFn>);

impl DeepPatchFilter {
    /// Whether this filter accepts the given field for deep-patch
    /// emission.
    pub(crate) fn accepts(&self, owner: &str, field: &str, inner: &str) -> bool {
        (self.0)(owner, field, inner)
    }
}

impl std::fmt::Debug for DeepPatchFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeepPatchFilter(<closure>)")
    }
}

/// Controls how schema `allOf` compositions are rendered.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum AllOfStrategy {
    /// Merge each subschema into a single flat schema and emit one struct
    /// containing the union of all properties (typify's historical
    /// behavior).
    #[default]
    Merge,
    /// For the common `allOf: [<$ref to Base>, type:object + properties:…]`
    /// pattern, emit a struct whose `Base` reference becomes a
    /// `#[serde(flatten)] pub <snake_case_base>: Base` field while the
    /// inline properties remain as ordinary fields. Falls back to the
    /// `Merge` behavior for shapes that don't permit clean composition
    /// (multiple colliding names, non-object subschemas, etc.).
    Compose,
}

#[derive(Debug, Clone)]
struct CrateSpec {
    version: CrateVers,
    rename: Option<String>,
}

/// Policy to apply to external types described by schema extensions whose
/// crates are not explicitly specified.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, serde::Deserialize)]
pub enum UnknownPolicy {
    /// Generate the type rather according to the schema.
    #[default]
    Generate,
    /// Use the specified type by path (this will result in a compile error if
    /// one of the crates is not an existing dependency). Note that this
    /// ignores compatibility requirements specified by the schema extension
    /// and may result in subtle failures if the crate used is incompatible
    /// with the version that produced the schema.
    Allow,
    /// If an unknown crate is encountered, generate a compiler warning
    /// indicating the crate that must be specified to proceed along with
    /// version constraints. This affords users an opportunity to specify the
    /// specific crate version to use (or the user may explicitly deny use of
    /// that crate).
    Deny,
}

/// Specify the version for a named crate to consider for type use (rather than
/// generating types) in the presense of a schema extension.
#[derive(Debug, Clone)]
pub enum CrateVers {
    /// An explicit version.
    Version(semver::Version),
    /// Any version.
    Any,
    /// Never use the given crate.
    Never,
}

impl CrateVers {
    /// Parse from a string
    pub fn parse(s: &str) -> Option<Self> {
        if s == "!" {
            Some(Self::Never)
        } else if s == "*" {
            Some(Self::Any)
        } else {
            Some(Self::Version(semver::Version::parse(s).ok()?))
        }
    }
}

/// Contains a set of modifications that may be applied to an existing type.
#[derive(Debug, Default, Clone)]
pub struct TypeSpacePatch {
    rename: Option<String>,
    derives: Vec<String>,
    attrs: Vec<String>,
}

/// Contains the attributes of a replacement of an existing type.
#[derive(Debug, Default, Clone)]
pub struct TypeSpaceReplace {
    replace_type: String,
    impls: Vec<TypeSpaceImpl>,
}

/// Defines a schema which will be replaced, and the attributes of the
/// replacement.
#[derive(Debug, Clone)]
struct TypeSpaceConversion {
    schema: schemars::schema::SchemaObject,
    type_name: String,
    impls: Vec<TypeSpaceImpl>,
}

#[allow(missing_docs)]
// TODO we can currently only address traits for which cycle analysis is not
// required.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TypeSpaceImpl {
    FromStr,
    FromStringIrrefutable,
    Display,
    Default,
}

impl std::str::FromStr for TypeSpaceImpl {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "FromStr" => Ok(Self::FromStr),
            "Display" => Ok(Self::Display),
            "Default" => Ok(Self::Default),
            _ => Err(format!("{} is not a valid trait specifier", s)),
        }
    }
}

impl TypeSpaceSettings {
    /// Set the name of the path prefix for types defined in this [TypeSpace].
    pub fn with_type_mod<S: AsRef<str>>(&mut self, type_mod: S) -> &mut Self {
        self.type_mod = Some(type_mod.as_ref().to_string());
        self
    }

    /// Add an additional derive macro to apply to all defined types.
    pub fn with_derive(&mut self, derive: String) -> &mut Self {
        if !self.extra_derives.contains(&derive) {
            self.extra_derives.push(derive);
        }
        self
    }

    /// Add an additional attribute to apply to all defined types.
    pub fn with_attr(&mut self, attr: String) -> &mut Self {
        if !self.extra_attrs.contains(&attr) {
            self.extra_attrs.push(attr);
        }
        self
    }

    /// For structs, include a "builder" type that can be used to construct it.
    pub fn with_struct_builder(&mut self, struct_builder: bool) -> &mut Self {
        self.struct_builder = struct_builder;
        self
    }

    /// Replace a referenced type with a named type. This causes the referenced
    /// type *not* to be generated. If the same `type_name` is specified multiple times,
    /// the last one is honored.
    pub fn with_replacement<TS: ToString, RS: ToString, I: Iterator<Item = TypeSpaceImpl>>(
        &mut self,
        type_name: TS,
        replace_type: RS,
        impls: I,
    ) -> &mut Self {
        self.replace.insert(
            type_name.to_string(),
            TypeSpaceReplace {
                replace_type: replace_type.to_string(),
                impls: impls.collect(),
            },
        );
        self
    }

    /// Modify a type with the given name. Note that specifying a type not
    /// created by the input JSON schema does **not** result in an error and is
    /// silently ignored. If the same `type_name` is specified multiple times,
    /// the last one is honored.
    pub fn with_patch<S: ToString>(
        &mut self,
        type_name: S,
        type_patch: &TypeSpacePatch,
    ) -> &mut Self {
        self.patch.insert(type_name.to_string(), type_patch.clone());
        self
    }

    /// Replace a given schema with a named type. The given schema must precisely
    /// match the schema from the input, including fields such as `description`.
    /// Typical usage is to map a schema definition to a builtin type or type
    /// provided by a crate, such as `'rust_decimal::Decimal'`. If the same schema
    /// is specified multiple times, the first one is honored.
    ///
    /// # Examples
    ///
    /// ```
    /// // Setup 'number' json type to be translated into 'rust_decimal::Decimal'
    /// use schemars::schema::{InstanceType, SchemaObject};
    /// use typify_impl::{TypeSpace, TypeSpaceImpl, TypeSpaceSettings};
    /// let mut type_space = TypeSpace::new(
    ///        TypeSpaceSettings::default()
    ///            .with_struct_builder(true)
    ///            .with_conversion(
    ///                SchemaObject {
    ///                    instance_type: Some(InstanceType::Number.into()),
    ///                    ..Default::default()
    ///                },
    ///                "::rust_decimal::Decimal",
    ///                [TypeSpaceImpl::Display].into_iter(),
    ///            ),
    ///    );
    /// ```
    pub fn with_conversion<S: ToString, I: Iterator<Item = TypeSpaceImpl>>(
        &mut self,
        schema: schemars::schema::SchemaObject,
        type_name: S,
        impls: I,
    ) -> &mut Self {
        self.convert.push(TypeSpaceConversion {
            schema,
            type_name: type_name.to_string(),
            impls: impls.collect(),
        });
        self
    }

    /// Type schemas may contain an extension (`x-rust-type`) that indicates
    /// the corresponding Rust type within a particular crate. This function
    /// changes the disposition regarding crates not otherwise specified via
    /// [`Self::with_crate`]. The default value is `false`.
    pub fn with_unknown_crates(&mut self, policy: UnknownPolicy) -> &mut Self {
        self.unknown_crates = policy;
        self
    }

    /// Type schemas may contain an extension (`x-rust-type`) that indicates
    /// the corresponding Rust type within a particular crate. This extension
    /// indicates the crate, version compatibility, type path, and type
    /// parameters. This function modifies settings to use (rather than
    /// generate) types from the given crate and version. The version should
    /// precisely match the version of the crate that you expect as a
    /// dependency.
    pub fn with_crate<S1: ToString>(
        &mut self,
        crate_name: S1,
        version: CrateVers,
        rename: Option<&String>,
    ) -> &mut Self {
        self.crates.insert(
            crate_name.to_string(),
            CrateSpec {
                version,
                rename: rename.cloned(),
            },
        );
        self
    }

    /// Specify the map-like type to be used in generated code.
    ///
    /// ## Requirements
    ///
    /// - An `is_empty` method that returns a boolean
    /// - Two generic parameters, `K` and `V`
    /// - [`Default`] + [`Clone`] + [`Debug`] +
    ///   [`Serialize`][serde::Serialize] + [`Deserialize`][serde::Deserialize]
    ///
    /// ## Examples
    ///
    /// - [`::std::collections::HashMap`]
    /// - [`::std::collections::BTreeMap`]
    /// - [`::indexmap::IndexMap`](https://docs.rs/indexmap/latest/indexmap/map/struct.IndexMap.html)
    pub fn with_map_type<T: Into<MapType>>(&mut self, map_type: T) -> &mut Self {
        self.map_type = map_type.into();
        self
    }

    /// Override the Rust type used for `{"type":"string","format":"date"}`
    /// schemas. The default is `::chrono::naive::NaiveDate`. Setting this to
    /// e.g. `::std::string::String` or `::time::Date` removes the dependency
    /// on chrono.
    ///
    /// The given string must be a fully-qualified Rust path. The type is
    /// emitted as a "native" type, so it should already implement `Debug`,
    /// `Clone`, `Serialize`, and `Deserialize` in a way that round-trips an
    /// ISO-8601 date string.
    pub fn with_date_type<S: ToString>(&mut self, type_name: S) -> &mut Self {
        self.date_type = Some(type_name.to_string());
        self
    }

    /// Override the Rust type used for
    /// `{"type":"string","format":"date-time"}` schemas. The default is
    /// `::chrono::DateTime<::chrono::offset::Utc>`. See
    /// [`Self::with_date_type`] for caveats.
    pub fn with_date_time_type<S: ToString>(&mut self, type_name: S) -> &mut Self {
        self.date_time_type = Some(type_name.to_string());
        self
    }

    /// Override the Rust type used for `{"type":"string","format":"uuid"}`
    /// schemas. The default is `::uuid::Uuid`.
    pub fn with_uuid_type<S: ToString>(&mut self, type_name: S) -> &mut Self {
        self.uuid_type = Some(type_name.to_string());
        self
    }

    /// When `true`, JSON Schema strings carrying `pattern:`, `minLength:`,
    /// or `maxLength:` constraints are emitted as plain `String` instead of
    /// a `#[serde(transparent)]` newtype that validates the constraint on
    /// construction / deserialization. Defaults to `false`.
    ///
    /// Enable this for APIs where the constraint should be enforced by the
    /// service / consumer rather than baked into the type system. It also
    /// avoids one transparent newtype per `<Struct><Field>` combination,
    /// which is a common source of name explosion for large specs.
    pub fn with_unconstrained_string(&mut self, unconstrained: bool) -> &mut Self {
        self.unconstrained_string = unconstrained;
        self
    }

    /// When `true`, integer schemas with `minimum: 1` are emitted using
    /// the plain `i32`/`i64`/etc. type instead of the corresponding
    /// `::std::num::NonZeroU*` type. Defaults to `false`.
    ///
    /// Enable this when the consuming code expects plain integers and the
    /// "minimum: 1" constraint will be enforced elsewhere.
    pub fn with_unconstrained_int(&mut self, unconstrained: bool) -> &mut Self {
        self.unconstrained_int = unconstrained;
        self
    }

    /// Control how arrays that are not in a struct's `required` list are
    /// emitted. See [`ArrayOptionality`].
    pub fn with_array_optionality(&mut self, mode: ArrayOptionality) -> &mut Self {
        self.array_optionality = mode;
        self
    }

    /// Control how booleans that carry a schema-level `default:` are emitted
    /// on non-required struct properties. See [`DefaultBoolOptionality`].
    pub fn with_default_bool_optionality(&mut self, mode: DefaultBoolOptionality) -> &mut Self {
        self.default_bool_optionality = mode;
        self
    }

    /// Control how non-required, non-bool struct properties carrying a
    /// schema-level `default:` directive are emitted. See
    /// [`DefaultedFieldOptionality`].
    pub fn with_defaulted_field_optionality(
        &mut self,
        mode: DefaultedFieldOptionality,
    ) -> &mut Self {
        self.defaulted_field_optionality = mode;
        self
    }

    /// Returns the configured defaulted-field-optionality mode.
    pub(crate) fn defaulted_field_optionality(&self) -> DefaultedFieldOptionality {
        self.defaulted_field_optionality
    }

    /// Drop the per-field `#[serde(default, skip_serializing_if =
    /// "::std::option::Option::is_none")]` on `Option<T>` fields whose
    /// serde-attribute set would otherwise be exactly that canonical
    /// pair. Defaults to `false` (typify's historical shape).
    ///
    /// The elision is safe whenever the enclosing struct already carries
    /// `#[serde_with::skip_serializing_none]` (which the caller is
    /// expected to enable separately, e.g. via
    /// [`Self::with_unconditional_attr_for`]): that attribute handles
    /// the serialize-omission, and serde's built-in `Option<T>`
    /// deserialization treats a missing JSON key as `None` without any
    /// `serde(default)` directive. The per-field line is therefore
    /// fully redundant in that setup; turning this knob on removes the
    /// visual noise.
    ///
    /// Only the canonical `default + skip_serializing_if =
    /// "::std::option::Option::is_none"` pair is elided. Fields with a
    /// named-default fn (`default = "defaults::..."`), a custom
    /// `skip_serializing_if`, `serde(flatten)`, or a non-`Option<T>`
    /// type (e.g. `Vec<T>`, `bool`, primitives) are left untouched. A
    /// `serde(rename = "...")` line is preserved alongside the elision
    /// (the rename is independent of the default / skip-serializing
    /// pair).
    pub fn with_elide_option_field_defaults(&mut self, value: bool) -> &mut Self {
        self.elide_option_field_defaults = value;
        self
    }

    /// Returns whether per-field `#[serde(default, skip_serializing_if =
    /// "Option::is_none")]` elision is enabled on `Option<T>` fields.
    pub(crate) fn elide_option_field_defaults(&self) -> bool {
        self.elide_option_field_defaults
    }

    /// Control whether `#[patch(name = "Option<{Inner}Patch>")]` is
    /// emitted above struct fields of type `Option<{InnerStruct}>` so
    /// that `struct_patch::Patch` produces a deep partial-merge shape.
    /// See [`DeepPatchPolicy`].
    ///
    /// This is the bulk knob. To target a hand-curated subset, also
    /// configure [`Self::with_deep_patch_filter`]; when both are set the
    /// filter takes precedence.
    pub fn with_deep_patches(&mut self, policy: DeepPatchPolicy) -> &mut Self {
        self.deep_patches = policy;
        self
    }

    /// Set a per-field predicate that decides whether to emit
    /// `#[patch(name = "Option<{Inner}Patch>")]` for a given field.
    ///
    /// The predicate receives `(owner_struct_name, field_name,
    /// inner_struct_name)` — every string in Rust identifier form (the
    /// owner / inner are the Pascal-case sanitized type names; the
    /// field is the snake_case Rust field name). Return `true` to emit
    /// the deep-patch line; return `false` to skip it.
    ///
    /// When set, the filter takes precedence over
    /// [`Self::with_deep_patches`]: every candidate field is asked the
    /// filter first, and only if the filter allows emission for that
    /// field does any line appear.
    pub fn with_deep_patch_filter<F>(&mut self, filter: F) -> &mut Self
    where
        F: Fn(&str, &str, &str) -> bool + Send + Sync + 'static,
    {
        self.deep_patch_filter = Some(DeepPatchFilter(Arc::new(filter)));
        self
    }

    /// Returns the configured bulk deep-patch policy.
    pub(crate) fn deep_patches(&self) -> DeepPatchPolicy {
        self.deep_patches
    }

    /// Returns the configured per-field deep-patch filter, if any.
    pub(crate) fn deep_patch_filter(&self) -> Option<&DeepPatchFilter> {
        self.deep_patch_filter.as_ref()
    }

    /// Control how schema `allOf` compositions are rendered. See
    /// [`AllOfStrategy`].
    pub fn with_allof_strategy(&mut self, strategy: AllOfStrategy) -> &mut Self {
        self.allof_strategy = strategy;
        self
    }

    /// When `true`, every generated type's doc comment includes the full
    /// pretty-printed JSON Schema under a `# JSON schema` heading and a
    /// fenced `json` code block, in addition to the schema's `description`.
    /// When `false` (the default), only the description is emitted.
    ///
    /// The JSON-schema block renders fine in `cargo doc`, but the simpler
    /// HTML renderer used by rust-analyzer hover popovers breaks on the
    /// historical `<details><summary>JSON schema</summary>...</details>`
    /// shape: CommonMark splits the disclosure widget into two raw-HTML
    /// siblings around the fenced code block, which then loses syntax
    /// highlighting and styling. Defaulting this off keeps hover popovers
    /// readable; turning it on emits a pure markdown heading + fenced code
    /// block instead, which renders cleanly in both rustdoc and hover
    /// tooltips.
    pub fn with_schema_in_docs(&mut self, value: bool) -> &mut Self {
        self.include_schema_in_docs = value;
        self
    }

    /// Add a derive macro that is emitted as
    /// `#[cfg_attr(feature = "<cfg>", derive(<derive>))]` on every generated
    /// struct, enum, and newtype. Pair this with a matching Cargo feature in
    /// the consuming crate.
    ///
    /// To restrict the derive to a subset of generated type categories (e.g.
    /// `struct_patch::Patch`, which only supports structs), use
    /// [`Self::with_conditional_derive_for`].
    pub fn with_conditional_derive<C: ToString, D: ToString>(
        &mut self,
        cfg: C,
        derive: D,
    ) -> &mut Self {
        self.with_conditional_derive_for(cfg, derive, TypeKindFilter::ALL)
    }

    /// Add a derive macro that is emitted as
    /// `#[cfg_attr(feature = "<cfg>", derive(<derive>))]` only on the
    /// generated type categories matching `kinds`. This is the form to use
    /// for derives that don't apply uniformly across structs, enums, and
    /// newtypes — e.g. `struct_patch::Patch` (structs only).
    pub fn with_conditional_derive_for<C: ToString, D: ToString>(
        &mut self,
        cfg: C,
        derive: D,
        kinds: TypeKindFilter,
    ) -> &mut Self {
        let cfg = cfg.to_string();
        let derive = derive.to_string();
        if !self
            .conditional_derives
            .iter()
            .any(|c| c.cfg == cfg && c.derive == derive)
        {
            self.conditional_derives
                .push(ConditionalDerive { cfg, derive, kinds });
        }
        self
    }

    /// Add an attribute that is emitted as
    /// `#[cfg_attr(feature = "<cfg>", <attr>)]` on every generated struct,
    /// enum, and newtype. Use [`Self::with_conditional_attr_for`] to scope
    /// the attribute to a subset of categories.
    pub fn with_conditional_attr<C: ToString, A: ToString>(&mut self, cfg: C, attr: A) -> &mut Self {
        self.with_conditional_attr_for(cfg, attr, TypeKindFilter::ALL)
    }

    /// Add an attribute that is emitted as
    /// `#[cfg_attr(feature = "<cfg>", <attr>)]` only on the generated type
    /// categories matching `kinds`. Useful for attributes like
    /// `#[serde_with::skip_serializing_none]`, which only modifies struct
    /// field serialization.
    ///
    /// The attribute is emitted before the main `#[derive(...)]` (i.e.
    /// [`AttrPosition::BeforeDerive`]). Use
    /// [`Self::with_conditional_attr_at`] to place it after the derive.
    pub fn with_conditional_attr_for<C: ToString, A: ToString>(
        &mut self,
        cfg: C,
        attr: A,
        kinds: TypeKindFilter,
    ) -> &mut Self {
        self.with_conditional_attr_at(cfg, attr, AttrPosition::BeforeDerive, kinds)
    }

    /// Add an attribute that is emitted as
    /// `#[cfg_attr(feature = "<cfg>", <attr>)]` at the given `position`
    /// relative to the main `#[derive(...)]`, scoped to type categories
    /// matching `kinds`.
    ///
    /// Use this for attributes that must appear after the derive (e.g.
    /// `#[patch(attribute(...))]` lines that ride the
    /// `#[derive(struct_patch::Patch)]` macro) where the historical
    /// "BeforeDerive" position would be incorrect.
    pub fn with_conditional_attr_at<C: ToString, A: ToString>(
        &mut self,
        cfg: C,
        attr: A,
        position: AttrPosition,
        kinds: TypeKindFilter,
    ) -> &mut Self {
        let cfg = cfg.to_string();
        let attr = attr.to_string();
        if !self
            .conditional_attrs
            .iter()
            .any(|c| c.cfg == cfg && c.attr == attr && c.position == position)
        {
            self.conditional_attrs.push(ConditionalAttr {
                cfg,
                attr,
                position,
                kinds,
            });
        }
        self
    }

    /// Add an attribute that is emitted **unconditionally** (no `cfg_attr`
    /// gate) on every generated type. The attribute is placed before the
    /// main `#[derive(...)]` (i.e. [`AttrPosition::BeforeDerive`]).
    ///
    /// Use this for attribute-style macros like
    /// `#[serde_with::skip_serializing_none]` that always belong on the
    /// generated type, no Cargo feature involved.
    ///
    /// To scope by type kind or place the attribute after the derive, see
    /// [`Self::with_unconditional_attr_for`] /
    /// [`Self::with_unconditional_attr_at`].
    pub fn with_unconditional_attr<A: ToString>(&mut self, attr: A) -> &mut Self {
        self.with_unconditional_attr_at(attr, AttrPosition::BeforeDerive, TypeKindFilter::ALL)
    }

    /// Add an unconditional attribute scoped to the given type kinds. The
    /// attribute is emitted before the main `#[derive(...)]`.
    pub fn with_unconditional_attr_for<A: ToString>(
        &mut self,
        attr: A,
        kinds: TypeKindFilter,
    ) -> &mut Self {
        self.with_unconditional_attr_at(attr, AttrPosition::BeforeDerive, kinds)
    }

    /// Add an unconditional attribute at the given `position` relative to
    /// the main `#[derive(...)]`, scoped to the given `kinds`.
    ///
    /// Multiple calls preserve insertion order, so a deliberately ordered
    /// block — e.g. several `#[patch(attribute(...))]` lines after the
    /// derive — can be assembled by calling this once per line.
    pub fn with_unconditional_attr_at<A: ToString>(
        &mut self,
        attr: A,
        position: AttrPosition,
        kinds: TypeKindFilter,
    ) -> &mut Self {
        let attr = attr.to_string();
        if !self
            .unconditional_attrs
            .iter()
            .any(|u| u.attr == attr && u.position == position && u.kinds == kinds)
        {
            self.unconditional_attrs.push(UnconditionalAttr {
                attr,
                position,
                kinds,
            });
        }
        self
    }

    /// Add a derive macro to the main `#[derive(...)]` list of every
    /// generated type. Multiple calls append derives in **insertion
    /// order**.
    ///
    /// # Important: this replaces the default base derive set
    ///
    /// As soon as any unconditional derive is registered for a given
    /// type kind, the historical base set
    /// (`Debug`, `Clone`, `::serde::Serialize`, `::serde::Deserialize`) is
    /// **suppressed** for that kind — the caller takes responsibility for
    /// listing every desired derive. This is the only way to control the
    /// exact ordering of the derive list (e.g. to match a hand-written
    /// style where `Default` precedes `Serialize` rather than following it
    /// alphabetically).
    ///
    /// For kinds with no registered unconditional derives, the legacy
    /// behavior (base set + `with_derive` extras, lexicographically
    /// ordered) is preserved.
    pub fn with_unconditional_derive<D: ToString>(&mut self, derive: D) -> &mut Self {
        self.with_unconditional_derive_for(derive, TypeKindFilter::ALL)
    }

    /// Add a derive scoped to the given type kinds. See
    /// [`Self::with_unconditional_derive`] for the replacement-vs-extend
    /// semantics.
    pub fn with_unconditional_derive_for<D: ToString>(
        &mut self,
        derive: D,
        kinds: TypeKindFilter,
    ) -> &mut Self {
        let derive = derive.to_string();
        if !self
            .unconditional_derives
            .iter()
            .any(|u| u.derive == derive && u.kinds == kinds)
        {
            self.unconditional_derives
                .push(UnconditionalDerive { derive, kinds });
        }
        self
    }

    /// Configure a struct-level `#[serde(rename_all = "<case>")]`
    /// attribute on every generated struct.
    ///
    /// In addition to emitting the attribute, this also enables an
    /// elision pass on per-field `#[serde(rename = "...")]`: whenever the
    /// field's original wire name equals the `<case>` transform of the
    /// snake-cased Rust field name, the per-field rename is dropped (the
    /// struct-level `rename_all` covers it). This keeps the generated
    /// output legible — only fields whose wire name disagrees with the
    /// chosen `rename_all` keep an explicit `rename`.
    ///
    /// Supported values are the standard serde set:
    /// `"lowercase"`, `"UPPERCASE"`, `"PascalCase"`, `"camelCase"`,
    /// `"snake_case"`, `"SCREAMING_SNAKE_CASE"`, `"kebab-case"`,
    /// `"SCREAMING-KEBAB-CASE"`.
    pub fn with_struct_rename_all<S: ToString>(&mut self, case: S) -> &mut Self {
        self.struct_rename_all = Some(case.to_string());
        self
    }

    /// Returns the configured struct-level `rename_all` case, if any.
    pub(crate) fn struct_rename_all(&self) -> Option<&str> {
        self.struct_rename_all.as_deref()
    }

    /// Configure explicit per-field serde names for a generated casing
    /// surface. This replaces the struct-level `rename_all` mechanism for
    /// field names: each property emits `rename` and, when distinct,
    /// `alias`. See [`SerdeFieldCase`].
    pub fn with_serde_field_case(&mut self, case: SerdeFieldCase) -> &mut Self {
        self.serde_field_case = Some(case);
        self
    }

    /// Returns the configured explicit per-field serde naming mode, if any.
    pub(crate) fn serde_field_case(&self) -> Option<SerdeFieldCase> {
        self.serde_field_case
    }

    /// When `true`, every generated enum that lacks a schema-level `default:`
    /// value will get an auto-generated
    /// `impl Default` block whose value is the first Simple (unit-like)
    /// variant in the enum's declared order.
    ///
    /// This is the simplest way to make required-enum struct fields work
    /// with a struct that derives `Default`: every enum becomes
    /// `Default`-able, so any struct that derives `Default` (whether
    /// directly or via the [`Self::with_unconditional_derive`] mechanism)
    /// will satisfy its trait bounds for required enum fields.
    ///
    /// Enums that already declare a schema-level default are unaffected —
    /// the generated impl already points at the schema-mandated value.
    ///
    /// Enums whose variants are *all* non-Simple (tuple / struct
    /// variants) cannot be defaulted to a unit variant; for those the
    /// auto-impl is skipped and `Default` remains unimplemented.
    pub fn with_enum_first_variant_default(&mut self, value: bool) -> &mut Self {
        self.enum_first_variant_default = value;
        self
    }

    /// Returns whether the enum first-variant Default auto-emission is
    /// enabled.
    pub(crate) fn enum_first_variant_default(&self) -> bool {
        self.enum_first_variant_default
    }
}

impl TypeSpacePatch {
    /// Specify the new name for patched type.
    pub fn with_rename<S: ToString>(&mut self, rename: S) -> &mut Self {
        self.rename = Some(rename.to_string());
        self
    }

    /// Specify an additional derive to apply to the patched type.
    pub fn with_derive<S: ToString>(&mut self, derive: S) -> &mut Self {
        self.derives.push(derive.to_string());
        self
    }

    /// Specify an additional attribute to apply to the patched type.
    pub fn with_attr<S: ToString>(&mut self, attr: S) -> &mut Self {
        self.attrs.push(attr.to_string());
        self
    }
}

impl TypeSpace {
    /// Create a new TypeSpace with custom settings.
    pub fn new(settings: &TypeSpaceSettings) -> Self {
        let mut cache = SchemaCache::default();

        settings.convert.iter().for_each(
            |TypeSpaceConversion {
                 schema,
                 type_name,
                 impls,
             }| {
                cache.insert(schema, type_name, impls);
            },
        );

        Self {
            settings: settings.clone(),
            cache,
            ..Default::default()
        }
    }

    /// Add a collection of types that will be used as references. Regardless
    /// of how these types are defined--*de novo* or built-in--each type will
    /// appear in the final output as a struct, enum or newtype. This method
    /// may be called multiple times, but collections of references must be
    /// self-contained; in other words, a type in one invocation may not refer
    /// to a type in another invocation.
    // TODO on an error the TypeSpace is in a weird state; we, perhaps, create
    // a child TypeSpace and then merge it in once all conversions hae
    // succeeded.
    pub fn add_ref_types<I, S>(&mut self, type_defs: I) -> Result<()>
    where
        I: IntoIterator<Item = (S, Schema)>,
        S: AsRef<str>,
    {
        self.add_ref_types_impl(
            type_defs
                .into_iter()
                .map(|(key, schema)| (RefKey::Def(key.as_ref().to_string()), schema)),
        )
    }

    fn add_ref_types_impl<I>(&mut self, type_defs: I) -> Result<()>
    where
        I: IntoIterator<Item = (RefKey, Schema)>,
    {
        // Gather up all types to make things a little more convenient.
        let definitions = type_defs.into_iter().collect::<Vec<_>>();

        // Assign IDs to reference types before actually converting them. We'll
        // need these in the case of forward (or circular) references.
        let base_id = self.next_id;
        let def_len = definitions.len() as u64;
        self.next_id += def_len;

        for (index, (ref_name, schema)) in definitions.iter().enumerate() {
            self.ref_to_id
                .insert(ref_name.clone(), TypeId(base_id + index as u64));
            self.definitions.insert(ref_name.clone(), schema.clone());
        }

        // Convert all types; note that we use the type id assigned from the
        // previous step because each type may create additional types. This
        // effectively is doing the work of `add_type_with_name` but for a
        // batch of types.
        for (index, (ref_name, schema)) in definitions.into_iter().enumerate() {
            info!(
                "converting type: {:?} with schema {}",
                ref_name,
                serde_json::to_string(&schema).unwrap()
            );

            // Check for manually replaced types. Proceed with type conversion
            // if there is none; use the specified type if there is.
            let type_id = TypeId(base_id + index as u64);

            let maybe_replace = match &ref_name {
                RefKey::Root => None,
                RefKey::Def(def_name) => {
                    let check_name = sanitize(def_name, Case::Pascal);
                    self.settings.replace.get(&check_name)
                }
            };

            match maybe_replace {
                None => {
                    let type_name = if let RefKey::Def(name) = ref_name {
                        Name::Required(name.clone())
                    } else {
                        Name::Unknown
                    };
                    self.convert_ref_type(type_name, schema, type_id)?
                }

                Some(replace_type) => {
                    let type_entry = TypeEntry::new_native(
                        replace_type.replace_type.clone(),
                        &replace_type.impls.clone(),
                    );
                    self.id_to_entry.insert(type_id, type_entry);
                }
            }
        }

        // Eliminate cycles. It's sufficient to only start from referenced
        // types as a reference is required to make a cycle.
        self.break_cycles(base_id..base_id + def_len);

        // Finalize all created types.
        for index in base_id..self.next_id {
            let type_id = TypeId(index);
            let mut type_entry = self.id_to_entry.get(&type_id).unwrap().clone();
            debug!("finalizing type entry: {} {:#?}", index, &type_entry);
            type_entry.finalize(self)?;
            self.id_to_entry.insert(type_id, type_entry);
        }

        Ok(())
    }

    fn convert_ref_type(&mut self, type_name: Name, schema: Schema, type_id: TypeId) -> Result<()> {
        let (mut type_entry, metadata) = self.convert_schema(type_name.clone(), &schema)?;
        let default = metadata
            .as_ref()
            .and_then(|m| m.default.as_ref())
            .cloned()
            .map(WrappedValue::new);
        let type_entry = match &mut type_entry.details {
            // The types that are already named are good to go.
            TypeEntryDetails::Enum(details) => {
                details.default = default;
                type_entry
            }
            TypeEntryDetails::Struct(details) => {
                details.default = default;
                type_entry
            }
            TypeEntryDetails::Newtype(details) => {
                details.default = default;
                type_entry
            }

            // If the type entry is a reference, then this definition is a
            // simple alias to another type in this list of definitions
            // (which may nor may not have already been converted). We
            // simply create a newtype with that type ID.
            TypeEntryDetails::Reference(type_id) => TypeEntryNewtype::from_metadata(
                self,
                type_name,
                metadata,
                type_id.clone(),
                schema.clone(),
            ),

            TypeEntryDetails::Native(native) if native.name_match(&type_name) => type_entry,

            // For types that don't have names, this is effectively a type
            // alias which we treat as a newtype.
            _ => {
                info!(
                    "type alias {:?} {}\n{:?}",
                    type_name,
                    serde_json::to_string_pretty(&schema).unwrap(),
                    metadata
                );
                let subtype_id = self.assign_type(type_entry);
                TypeEntryNewtype::from_metadata(
                    self,
                    type_name,
                    metadata,
                    subtype_id,
                    schema.clone(),
                )
            }
        };
        // TODO need a type alias?
        if let Some(entry_name) = type_entry.name() {
            self.name_to_id.insert(entry_name.clone(), type_id.clone());
        }
        self.id_to_entry.insert(type_id, type_entry);
        Ok(())
    }

    /// Add a new type and return a type identifier that may be used in
    /// function signatures or embedded within other types.
    pub fn add_type(&mut self, schema: &Schema) -> Result<TypeId> {
        self.add_type_with_name(schema, None)
    }

    /// Add a new type with a name hint and return a the components necessary
    /// to use the type for various components of a function signature.
    pub fn add_type_with_name(
        &mut self,
        schema: &Schema,
        name_hint: Option<String>,
    ) -> Result<TypeId> {
        let base_id = self.next_id;

        let name = match name_hint {
            Some(s) => Name::Suggested(s),
            None => Name::Unknown,
        };
        let (type_id, _) = self.id_for_schema(name, schema)?;

        // Finalize all created types.
        for index in base_id..self.next_id {
            let type_id = TypeId(index);
            let mut type_entry = self.id_to_entry.get(&type_id).unwrap().clone();
            type_entry.finalize(self)?;
            self.id_to_entry.insert(type_id, type_entry);
        }

        Ok(type_id)
    }

    /// Add all the types contained within a RootSchema including any
    /// referenced types and the top-level type (if there is one and it has a
    /// title).
    pub fn add_root_schema(&mut self, schema: RootSchema) -> Result<Option<TypeId>> {
        let RootSchema {
            meta_schema: _,
            schema,
            definitions,
        } = schema;

        let mut defs = definitions
            .into_iter()
            .map(|(key, schema)| (RefKey::Def(key), schema))
            .collect::<Vec<_>>();

        // Does the root type have a name (otherwise... ignore it)
        let root_type = schema
            .metadata
            .as_ref()
            .and_then(|m| m.title.as_ref())
            .is_some();

        if root_type {
            defs.push((RefKey::Root, schema.into()));
        }

        self.add_ref_types_impl(defs)?;

        if root_type {
            Ok(self.ref_to_id.get(&RefKey::Root).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get a type given its ID.
    pub fn get_type(&self, type_id: &TypeId) -> Result<Type<'_>> {
        let type_entry = self.id_to_entry.get(type_id).ok_or(Error::InvalidTypeId)?;
        Ok(Type {
            type_space: self,
            type_entry,
        })
    }

    /// Whether the generated code needs `chrono` crate.
    pub fn uses_chrono(&self) -> bool {
        self.uses_chrono
    }

    /// Whether the generated code needs [regress] crate.
    pub fn uses_regress(&self) -> bool {
        self.uses_regress
    }

    /// Whether the generated code needs [serde_json] crate.
    pub fn uses_serde_json(&self) -> bool {
        self.uses_serde_json
    }

    /// Whether the generated code needs `uuid` crate.
    pub fn uses_uuid(&self) -> bool {
        self.uses_uuid
    }

    /// Whether the schema-in-docs knob is on; controls whether
    /// [`crate::type_entry::make_doc`] embeds the full JSON Schema.
    pub(crate) fn include_schema_in_docs(&self) -> bool {
        self.settings.include_schema_in_docs
    }

    /// Iterate over all types including those defined in this [TypeSpace] and
    /// those referred to by those types.
    pub fn iter_types(&self) -> impl Iterator<Item = Type<'_>> {
        self.id_to_entry.values().map(move |type_entry| Type {
            type_space: self,
            type_entry,
        })
    }

    /// All code for processed types.
    pub fn to_stream(&self) -> TokenStream {
        let mut output = OutputSpace::default();
        self.fill_error_mod(&mut output);
        self.id_to_entry
            .values()
            .for_each(|type_entry| type_entry.output(self, &mut output));
        self.fill_defaults_mod(&mut output);
        output.into_stream()
    }

    /// All code for processed types, grouped into per-module blocks.
    ///
    /// `partition` maps each generated type's Rust name (as returned by
    /// [`Type::name()`]) to a target module name. Types not present in
    /// `partition` are placed in `default_module`. The resulting
    /// [`TokenStream`] is a sequence of `pub mod <name> { ... }` blocks
    /// — one per distinct destination module — produced in stable order
    /// (lexicographically by module name).
    ///
    /// `imports_per_module` lets the caller inject a preamble of `use`
    /// statements at the top of each module body. Use this for
    /// cross-module references (e.g. `use super::shared::*;` so types in
    /// per-operation modules can name shared types directly) and for
    /// bringing trait paths into scope so bare derives resolve (e.g.
    /// `use serde::{Serialize, Deserialize};`).
    ///
    /// The `error` module (containing `ConversionError`) is emitted as
    /// a submodule inside every partition so that any per-module
    /// `self::error::ConversionError` reference in a bespoke enum impl
    /// resolves correctly. The shared `defaults` module is likewise
    /// duplicated into every partition.
    pub fn to_stream_partitioned(
        &self,
        partition: &std::collections::HashMap<String, String>,
        default_module: &str,
        imports_per_module: &std::collections::HashMap<String, TokenStream>,
    ) -> TokenStream {
        // Bucket each emitted type into a per-module `OutputSpace`.
        let mut per_module: BTreeMap<String, OutputSpace> = BTreeMap::new();

        // Ensure the default module exists even if the partition map
        // happens to cover every generated type, and materialize every
        // module the caller references in `imports_per_module` — another
        // module's preamble may glob-import it (e.g. `use super::foo::*;`)
        // even when no generated type landed there.
        per_module.entry(default_module.to_string()).or_default();
        for name in imports_per_module.keys() {
            per_module.entry(name.clone()).or_default();
        }

        for type_entry in self.id_to_entry.values() {
            // Only Struct/Enum/Newtype produce a top-level `pub` item;
            // everything else is a structural helper (Option, Vec,
            // Reference, …) and skipping it matches what
            // `TypeEntry::output` already does.
            match &type_entry.details {
                type_entry::TypeEntryDetails::Struct(_)
                | type_entry::TypeEntryDetails::Enum(_)
                | type_entry::TypeEntryDetails::Newtype(_) => {}
                _ => continue,
            }
            let name = type_entry.type_name(self);
            let module = partition
                .get(&name)
                .map(String::as_str)
                .unwrap_or(default_module)
                .to_string();
            let space = per_module.entry(module).or_default();
            type_entry.output(self, space);
        }

        // Every module gets its own `error` submodule so that
        // `self::error::ConversionError` references in bespoke enum impls
        // resolve to the right path no matter which module the enum
        // ended up in.
        //
        // The shared default-fn helpers (e.g. `default_u64`) are likewise
        // baked into every partition's `defaults` submodule alongside any
        // type-local default fns emitted during `type_entry.output()`.
        // Duplicating the helpers is preferable to cross-module imports,
        // which would collide with the local `pub mod defaults` blocks;
        // the helpers are tiny so the duplication is negligible.
        for space in per_module.values_mut() {
            self.fill_error_mod(space);
            self.fill_defaults_mod(space);
        }

        let mods = per_module.into_iter().map(|(name, output_space)| {
            let mod_ident = format_ident!("{}", name);
            let mod_body = output_space.into_stream();
            let preamble = imports_per_module.get(&name).cloned().unwrap_or_default();
            quote! {
                pub mod #mod_ident {
                    #preamble
                    #mod_body
                }
            }
        });

        quote! {
            #(#mods)*
        }
    }

    /// Map from the original schema definition key (e.g. an OpenAPI
    /// `components/schemas/<key>` name or a JSON Schema `definitions`
    /// entry) to the Rust type name typify emits for that schema.
    ///
    /// Use this when computing a partition for
    /// [`Self::to_stream_partitioned`] from reachability data keyed by the
    /// original schema names: the value returned here matches
    /// [`Type::name()`] exactly, so the partition map can be keyed
    /// correctly without re-implementing typify's Pascal-case
    /// sanitization rules.
    pub fn definition_rust_names(&self) -> Vec<(String, String)> {
        self.ref_to_id
            .iter()
            .filter_map(|(key, type_id)| {
                let RefKey::Def(schema_key) = key else {
                    return None;
                };
                let entry = self.id_to_entry.get(type_id)?;
                Some((schema_key.clone(), entry.type_name(self)))
            })
            .collect()
    }

    /// Add the `ConversionError` submodule into `output`; it's fine if this
    /// is unused.
    fn fill_error_mod(&self, output: &mut OutputSpace) {
        output.add_item(
            output::OutputSpaceMod::Error,
            "",
            quote! {
                /// Error from a `TryFrom` or `FromStr` implementation.
                pub struct ConversionError(::std::borrow::Cow<'static, str>);

                impl ::std::error::Error for ConversionError {}
                impl ::std::fmt::Display for ConversionError {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>)
                        -> Result<(), ::std::fmt::Error>
                    {
                        ::std::fmt::Display::fmt(&self.0, f)
                    }
                }

                impl ::std::fmt::Debug for ConversionError {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>)
                        -> Result<(), ::std::fmt::Error>
                    {
                        ::std::fmt::Debug::fmt(&self.0, f)
                    }
                }
                impl From<&'static str> for ConversionError {
                    fn from(value: &'static str) -> Self {
                        Self(value.into())
                    }
                }
                impl From<String> for ConversionError {
                    fn from(value: String) -> Self {
                        Self(value.into())
                    }
                }
            },
        );
    }

    /// Add every shared default-fn into `output`'s `defaults` submodule.
    fn fill_defaults_mod(&self, output: &mut OutputSpace) {
        self.defaults
            .iter()
            .for_each(|x| output.add_item(output::OutputSpaceMod::Defaults, "", x.into()));
    }

    /// Allocated the next TypeId.
    fn assign(&mut self) -> TypeId {
        let id = TypeId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Assign a TypeId for a TypeEntry. This handles resolving references,
    /// checking for duplicate type definitions (e.g. to make sure there aren't
    /// two conflicting types of the same name), and deduplicates various
    /// flavors of built-in types.
    fn assign_type(&mut self, ty: TypeEntry) -> TypeId {
        if let TypeEntryDetails::Reference(type_id) = ty.details {
            type_id
        } else if let Some(name) = ty.name() {
            // If there's already a type of this name, we make sure it's
            // identical. Note that this covers all user-defined types.

            // TODO there are many different choices we might make here
            // that could differ depending on the texture of the schema.
            // For example, a schema might use the string "Response" in a
            // bunch of places and if that were the case we might expect
            // them to be different and resolve that by renaming or scoping
            // them in some way.
            if let Some(type_id) = self.name_to_id.get(name) {
                // TODO we'd like to verify that the type is structurally the
                // same, but the types may not be functionally equal. This is a
                // consequence of types being "finalized" after each type
                // addition. This further emphasized the need for a more
                // deliberate, multi-pass approach.
                type_id.clone()
            } else {
                let type_id = self.assign();
                self.name_to_id.insert(name.clone(), type_id.clone());
                self.id_to_entry.insert(type_id.clone(), ty);
                type_id
            }
        } else if let Some(type_id) = self.type_to_id.get(&ty.details) {
            type_id.clone()
        } else {
            let type_id = self.assign();
            self.type_to_id.insert(ty.details.clone(), type_id.clone());
            self.id_to_entry.insert(type_id.clone(), ty);
            type_id
        }
    }

    /// Convert a schema to a TypeEntry and assign it a TypeId.
    ///
    /// This is used for sub-types such as the type of an array or the types of
    /// properties of a struct.
    fn id_for_schema<'a>(
        &mut self,
        type_name: Name,
        schema: &'a Schema,
    ) -> Result<(TypeId, &'a Option<Box<Metadata>>)> {
        let (mut type_entry, metadata) = self.convert_schema(type_name, schema)?;
        if let Some(metadata) = metadata {
            let default = metadata.default.clone().map(WrappedValue::new);
            match &mut type_entry.details {
                TypeEntryDetails::Enum(details) => {
                    details.default = default;
                }
                TypeEntryDetails::Struct(details) => {
                    details.default = default;
                }
                TypeEntryDetails::Newtype(details) => {
                    details.default = default;
                }
                _ => (),
            }
        }
        let type_id = self.assign_type(type_entry);
        Ok((type_id, metadata))
    }

    /// Create an Option<T> from a pre-assigned TypeId and assign it an ID.
    fn id_to_option(&mut self, id: &TypeId) -> TypeId {
        self.assign_type(TypeEntryDetails::Option(id.clone()).into())
    }

    // Create an Option<T> from a TypeEntry by assigning it type.
    fn type_to_option(&mut self, ty: TypeEntry) -> TypeEntry {
        TypeEntryDetails::Option(self.assign_type(ty)).into()
    }

    /// Create a Box<T> from a pre-assigned TypeId and assign it an ID.
    fn id_to_box(&mut self, id: &TypeId) -> TypeId {
        self.assign_type(TypeEntryDetails::Box(id.clone()).into())
    }
}

impl ToTokens for TypeSpace {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.to_stream())
    }
}

impl Type<'_> {
    /// The name of the type as a String.
    pub fn name(&self) -> String {
        let Type {
            type_space,
            type_entry,
        } = self;
        type_entry.type_name(type_space)
    }

    /// The identifier for the type as might be used for a function return or
    /// defining the type of a member of a struct..
    pub fn ident(&self) -> TokenStream {
        let Type {
            type_space,
            type_entry,
        } = self;
        type_entry.type_ident(type_space, &type_space.settings.type_mod)
    }

    /// The identifier for the type as might be used for a parameter in a
    /// function signature. In general: simple types are the same as
    /// [Type::ident] and complex types prepend a `&`.
    pub fn parameter_ident(&self) -> TokenStream {
        let Type {
            type_space,
            type_entry,
        } = self;
        type_entry.type_parameter_ident(type_space, None)
    }

    /// The identifier for the type as might be used for a parameter in a
    /// function signature along with a lifetime parameter. In general: simple
    /// types are the same as [Type::ident] and complex types prepend a
    /// `&'<lifetime>`.
    pub fn parameter_ident_with_lifetime(&self, lifetime: &str) -> TokenStream {
        let Type {
            type_space,
            type_entry,
        } = self;
        type_entry.type_parameter_ident(type_space, Some(lifetime))
    }

    /// A textual description of the type appropriate for debug output.
    pub fn describe(&self) -> String {
        self.type_entry.describe()
    }

    /// Get details about the type.
    pub fn details(&self) -> TypeDetails<'_> {
        match &self.type_entry.details {
            // Named user-defined types
            TypeEntryDetails::Enum(details) => TypeDetails::Enum(TypeEnum { details }),
            TypeEntryDetails::Struct(details) => TypeDetails::Struct(TypeStruct { details }),
            TypeEntryDetails::Newtype(details) => TypeDetails::Newtype(TypeNewtype { details }),

            // Compound types
            TypeEntryDetails::Option(type_id) => TypeDetails::Option(type_id.clone()),
            TypeEntryDetails::Vec(type_id) => TypeDetails::Vec(type_id.clone()),
            TypeEntryDetails::Map(key_id, value_id) => {
                TypeDetails::Map(key_id.clone(), value_id.clone())
            }
            TypeEntryDetails::Set(type_id) => TypeDetails::Set(type_id.clone()),
            TypeEntryDetails::Box(type_id) => TypeDetails::Box(type_id.clone()),
            TypeEntryDetails::Tuple(types) => TypeDetails::Tuple(Box::new(types.iter().cloned())),
            TypeEntryDetails::Array(type_id, length) => {
                TypeDetails::Array(type_id.clone(), *length)
            }

            // Builtin types
            TypeEntryDetails::Unit => TypeDetails::Unit,
            TypeEntryDetails::Native(TypeEntryNative {
                type_name: name, ..
            })
            | TypeEntryDetails::Integer(name)
            | TypeEntryDetails::Float(name) => TypeDetails::Builtin(name.as_str()),
            TypeEntryDetails::Boolean => TypeDetails::Builtin("bool"),
            TypeEntryDetails::String => TypeDetails::String,
            TypeEntryDetails::JsonValue => TypeDetails::Builtin("::serde_json::Value"),

            // Only used during processing; shouldn't be visible at this point
            TypeEntryDetails::Reference(_) => unreachable!(),
        }
    }

    /// Checks if the type has the associated impl.
    pub fn has_impl(&self, impl_name: TypeSpaceImpl) -> bool {
        let Type {
            type_space,
            type_entry,
        } = self;
        type_entry.has_impl(type_space, impl_name)
    }

    /// Provides the the type identifier for the builder if one exists.
    pub fn builder(&self) -> Option<TokenStream> {
        let Type {
            type_space,
            type_entry,
        } = self;

        if !type_space.settings.struct_builder {
            return None;
        }

        match &type_entry.details {
            TypeEntryDetails::Struct(type_entry::TypeEntryStruct { name, .. }) => {
                match &type_space.settings.type_mod {
                    Some(type_mod) => {
                        let type_mod = format_ident!("{}", type_mod);
                        let type_name = format_ident!("{}", name);
                        Some(quote! { #type_mod :: builder :: #type_name })
                    }
                    None => {
                        let type_name = format_ident!("{}", name);
                        Some(quote! { builder :: #type_name })
                    }
                }
            }
            _ => None,
        }
    }
}

impl<'a> TypeEnum<'a> {
    /// Get name and information of each enum variant.
    pub fn variants(&'a self) -> impl Iterator<Item = (&'a str, TypeEnumVariant<'a>)> {
        self.variants_info().map(|info| (info.name, info.details))
    }

    /// Get all information for each enum variant.
    pub fn variants_info(&'a self) -> impl Iterator<Item = TypeEnumVariantInfo<'a>> {
        self.details.variants.iter().map(move |variant| {
            let details = match &variant.details {
                type_entry::VariantDetails::Simple => TypeEnumVariant::Simple,
                // The distinction between a lone item variant and a tuple
                // variant with a single item is only relevant internally.
                type_entry::VariantDetails::Item(type_id) => {
                    TypeEnumVariant::Tuple(vec![type_id.clone()])
                }
                type_entry::VariantDetails::Tuple(types) => TypeEnumVariant::Tuple(types.clone()),
                type_entry::VariantDetails::Struct(properties) => TypeEnumVariant::Struct(
                    properties
                        .iter()
                        .map(|prop| (prop.name.as_str(), prop.type_id.clone()))
                        .collect(),
                ),
            };
            TypeEnumVariantInfo {
                name: variant.ident_name.as_ref().unwrap(),
                description: variant.description.as_deref(),
                details,
            }
        })
    }
}

impl<'a> TypeStruct<'a> {
    /// Get name and type of each property.
    pub fn properties(&'a self) -> impl Iterator<Item = (&'a str, TypeId)> {
        self.details
            .properties
            .iter()
            .map(move |prop| (prop.name.as_str(), prop.type_id.clone()))
    }

    /// Get all information about each struct property.
    pub fn properties_info(&'a self) -> impl Iterator<Item = TypeStructPropInfo<'a>> {
        self.details
            .properties
            .iter()
            .map(move |prop| TypeStructPropInfo {
                name: prop.name.as_str(),
                description: prop.description.as_deref(),
                required: matches!(&prop.state, StructPropertyState::Required),
                type_id: prop.type_id.clone(),
            })
    }
}

impl TypeNewtype<'_> {
    /// Get the inner type of the newtype struct.
    pub fn inner(&self) -> TypeId {
        self.details.type_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use schema::Schema;
    use schemars::{schema_for, JsonSchema};
    use serde::Serialize;
    use serde_json::json;
    use std::collections::HashSet;

    use crate::{
        output::OutputSpace,
        test_util::validate_output,
        type_entry::{TypeEntryEnum, VariantDetails},
        Name, TypeEntryDetails, TypeSpace, TypeSpaceSettings,
    };

    #[allow(dead_code)]
    #[derive(Serialize, JsonSchema)]
    struct Blah {
        blah: String,
    }

    #[allow(dead_code)]
    #[derive(Serialize, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    //#[serde(untagged)]
    //#[serde(tag = "type", content = "content")]
    enum E {
        /// aaa
        A,
        /// bee
        B,
        /// cee
        //C(Vec<String>),
        C(Blah),
        /// dee
        D {
            /// double D
            dd: String,
        },
        // /// eff
        // F(
        //     /// eff.0
        //     u32,
        //     /// eff.1
        //     u32,
        // ),
    }

    #[allow(dead_code)]
    #[derive(JsonSchema)]
    #[serde(rename_all = "camelCase")]
    struct Foo {
        /// this is bar
        #[serde(default)]
        bar: Option<String>,
        baz_baz: i32,
        /// eeeeee!
        e: E,
    }

    #[test]
    fn test_simple() {
        let schema = schema_for!(Foo);
        println!("{:#?}", schema);
        let mut type_space = TypeSpace::default();
        type_space.add_ref_types(schema.definitions).unwrap();
        let (ty, _) = type_space
            .convert_schema_object(
                Name::Unknown,
                &schemars::schema::Schema::Object(schema.schema.clone()),
                &schema.schema,
            )
            .unwrap();

        println!("{:#?}", ty);

        let mut output = OutputSpace::default();
        ty.output(&type_space, &mut output);
        println!("{}", output.into_stream());

        for ty in type_space.id_to_entry.values() {
            println!("{:#?}", ty);
            let mut output = OutputSpace::default();
            ty.output(&type_space, &mut output);
            println!("{}", output.into_stream());
        }
    }

    #[test]
    fn test_external_references() {
        let schema = json!({
            "$schema": "http://json-schema.org/draft-04/schema#",
            "definitions": {
                "somename": {
                    "$ref": "#/definitions/someothername",
                    "required": [ "someproperty" ]
                },
                "someothername": {
                    "type": "object",
                    "properties": {
                        "someproperty": {
                            "type": "string"
                        }
                    }
                }
            }
        });
        let schema = serde_json::from_value(schema).unwrap();
        println!("{:#?}", schema);
        let settings = TypeSpaceSettings::default();
        let mut type_space = TypeSpace::new(&settings);
        type_space.add_root_schema(schema).unwrap();
        let tokens = type_space.to_stream().to_string();
        println!("{}", tokens);
        assert!(tokens
            .contains(" pub struct Somename { pub someproperty : :: std :: string :: String , }"))
    }

    #[test]
    fn test_convert_enum_string() {
        #[allow(dead_code)]
        #[derive(JsonSchema)]
        #[serde(rename_all = "camelCase")]
        enum SimpleEnum {
            DotCom,
            Grizz,
            Kenneth,
        }

        let schema = schema_for!(SimpleEnum);
        println!("{:#?}", schema);

        let mut type_space = TypeSpace::default();
        type_space.add_ref_types(schema.definitions).unwrap();
        let (ty, _) = type_space
            .convert_schema_object(
                Name::Unknown,
                &schemars::schema::Schema::Object(schema.schema.clone()),
                &schema.schema,
            )
            .unwrap();

        match ty.details {
            TypeEntryDetails::Enum(TypeEntryEnum { variants, .. }) => {
                for variant in &variants {
                    assert_eq!(variant.details, VariantDetails::Simple);
                }
                let var_names = variants
                    .iter()
                    .map(|variant| variant.ident_name.as_ref().unwrap().clone())
                    .collect::<HashSet<_>>();
                assert_eq!(
                    var_names,
                    ["DotCom", "Grizz", "Kenneth",]
                        .iter()
                        .map(ToString::to_string)
                        .collect::<HashSet<_>>()
                );
            }
            _ => {
                let mut output = OutputSpace::default();
                ty.output(&type_space, &mut output);
                println!("{}", output.into_stream());
                panic!();
            }
        }
    }

    #[test]
    fn test_string_enum_with_null() {
        let original_schema = json!({ "$ref": "xxx"});
        let enum_values = vec![
            json!("Shadrach"),
            json!("Meshach"),
            json!("Abednego"),
            json!(null),
        ];

        let mut type_space = TypeSpace::default();
        let (te, _) = type_space
            .convert_enum_string(
                Name::Required("OnTheGo".to_string()),
                &serde_json::from_value(original_schema).unwrap(),
                &None,
                &enum_values,
                None,
            )
            .unwrap();

        if let TypeEntryDetails::Option(id) = &te.details {
            let ote = type_space.id_to_entry.get(id).unwrap();
            if let TypeEntryDetails::Enum(TypeEntryEnum { variants, .. }) = &ote.details {
                let variants = variants
                    .iter()
                    .map(|v| match v.details {
                        VariantDetails::Simple => v.ident_name.as_ref().unwrap().clone(),
                        _ => panic!("unexpected variant type"),
                    })
                    .collect::<HashSet<_>>();

                assert_eq!(
                    variants,
                    enum_values
                        .iter()
                        .flat_map(|j| j.as_str().map(ToString::to_string))
                        .collect::<HashSet<_>>()
                );
            } else {
                panic!("not the sub-type we expected {:#?}", te)
            }
        } else {
            panic!("not the type we expected {:#?}", te)
        }
    }

    #[test]
    fn test_alias() {
        #[allow(dead_code)]
        #[derive(JsonSchema, Schema)]
        struct Stuff(Vec<String>);

        #[allow(dead_code)]
        #[derive(JsonSchema, Schema)]
        struct Things {
            a: String,
            b: Stuff,
        }

        validate_output::<Things>();
    }

    #[test]
    fn test_builder_name() {
        #[allow(dead_code)]
        #[derive(JsonSchema)]
        struct TestStruct {
            x: u32,
        }

        let mut type_space = TypeSpace::default();
        let schema = schema_for!(TestStruct);
        let type_id = type_space.add_root_schema(schema).unwrap().unwrap();
        let ty = type_space.get_type(&type_id).unwrap();

        assert!(ty.builder().is_none());

        let mut type_space = TypeSpace::new(TypeSpaceSettings::default().with_struct_builder(true));
        let schema = schema_for!(TestStruct);
        let type_id = type_space.add_root_schema(schema).unwrap().unwrap();
        let ty = type_space.get_type(&type_id).unwrap();

        assert_eq!(
            ty.builder().map(|ts| ts.to_string()),
            Some("builder :: TestStruct".to_string())
        );

        let mut type_space = TypeSpace::new(
            TypeSpaceSettings::default()
                .with_type_mod("types")
                .with_struct_builder(true),
        );
        let schema = schema_for!(TestStruct);
        let type_id = type_space.add_root_schema(schema).unwrap().unwrap();
        let ty = type_space.get_type(&type_id).unwrap();

        assert_eq!(
            ty.builder().map(|ts| ts.to_string()),
            Some("types :: builder :: TestStruct".to_string())
        );

        #[allow(dead_code)]
        #[derive(JsonSchema)]
        enum TestEnum {
            X,
            Y,
        }
        let mut type_space = TypeSpace::new(
            TypeSpaceSettings::default()
                .with_type_mod("types")
                .with_struct_builder(true),
        );
        let schema = schema_for!(TestEnum);
        let type_id = type_space.add_root_schema(schema).unwrap().unwrap();
        let ty = type_space.get_type(&type_id).unwrap();
        assert!(ty.builder().is_none());
    }
}
