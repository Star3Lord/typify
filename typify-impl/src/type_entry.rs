// Copyright 2025 Oxide Computer Company

use std::collections::{BTreeMap, BTreeSet, HashMap};

use proc_macro2::{Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use schemars::schema::{Metadata, Schema};
use syn::Path;
use unicode_ident::is_xid_continue;

use crate::{
    enums::output_variant,
    output::{OutputSpace, OutputSpaceMod},
    sanitize,
    structs::{generate_serde_attr, DefaultFunction},
    util::{get_type_name, metadata_description, unique, TypePatch},
    Case, DeepPatchPolicy, DefaultImpl, Name, Result, TypeId, TypeSpace, TypeSpaceImpl,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SchemaWrapper(Schema);

impl Eq for SchemaWrapper {}

impl Ord for SchemaWrapper {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}
impl PartialOrd for SchemaWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeEntryEnum {
    pub name: String,
    pub rename: Option<String>,
    pub description: Option<String>,
    pub default: Option<WrappedValue>,
    pub tag_type: EnumTagType,
    pub variants: Vec<Variant>,
    pub deny_unknown_fields: bool,
    pub bespoke_impls: BTreeSet<TypeEntryEnumImpl>,
    pub schema: SchemaWrapper,
}

/// Cached attributes that (mostly) result in customized impl generation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TypeEntryEnumImpl {
    AllSimpleVariants,
    UntaggedFromStr,
    UntaggedDisplay,
    /// This is a cached marker to let us know that at least one of the
    /// variants is irrefutably a string. There is currently no associated
    /// implementation that we generate.
    UntaggedFromStringIrrefutable,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeEntryStruct {
    pub name: String,
    pub rename: Option<String>,
    pub description: Option<String>,
    pub default: Option<WrappedValue>,
    pub properties: Vec<StructProperty>,
    pub deny_unknown_fields: bool,
    pub schema: SchemaWrapper,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeEntryNewtype {
    pub name: String,
    pub rename: Option<String>,
    pub description: Option<String>,
    pub default: Option<WrappedValue>,
    pub type_id: TypeId,
    pub constraints: TypeEntryNewtypeConstraints,
    pub schema: SchemaWrapper,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TypeEntryNewtypeConstraints {
    None,
    EnumValue(Vec<WrappedValue>),
    DenyValue(Vec<WrappedValue>),
    String {
        max_length: Option<u32>,
        min_length: Option<u32>,
        pattern: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TypeEntryNative {
    pub type_name: String,
    impls: Vec<TypeSpaceImpl>,
    // TODO to support const generics, this can be some sort of TypeOrValue,
    // but note that we may some day need to disambiguate char and &'static str
    // since schemars represents a char as a string of length 1.
    pub parameters: Vec<TypeId>,
}
impl TypeEntryNative {
    pub(crate) fn name_match(&self, type_name: &Name) -> bool {
        let native_name = self.type_name.rsplit("::").next().unwrap();
        !self.parameters.is_empty()
            || matches!(type_name, Name::Required(req) if req == native_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrappedValue(pub serde_json::Value);
impl WrappedValue {
    pub(crate) fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl Ord for WrappedValue {
    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}
impl PartialOrd for WrappedValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// TODO This struct needs to go away (again). The derives should go into the
// generated struct/enum/newtype structs. Same for the impls. Native types will
// also have impls. Builtin generic types such as Box or Vec will delegate to
// their subtypes (while recursive, it is necessarily terminating... though I
// suppose we could memoize it). Builtin simple types such as u64 or String
// have a static list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeEntry {
    pub details: TypeEntryDetails,
    pub extra_derives: BTreeSet<String>,
    pub extra_attrs: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TypeEntryDetails {
    Enum(TypeEntryEnum),
    Struct(TypeEntryStruct),
    Newtype(TypeEntryNewtype),

    /// Native types exported from a well-known crate.
    Native(TypeEntryNative),

    // Types from core and std.
    Option(TypeId),
    Box(TypeId),
    Vec(TypeId),
    Map(TypeId, TypeId),
    Set(TypeId),
    Array(TypeId, usize),
    Tuple(Vec<TypeId>),
    Unit,
    Boolean,
    /// Integers
    Integer(String),
    /// Floating point numbers; not Eq, Ord, or Hash
    Float(String),
    /// Strings... which we handle a little specially.
    String,
    /// serde_json::Value which we also handle specially.
    JsonValue,

    /// While these types won't very make their way out to the user, we need
    /// reference types in particular to represent simple type aliases between
    /// types named as reference targets.
    Reference(TypeId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EnumTagType {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Variant {
    pub raw_name: String,
    pub ident_name: Option<String>,
    pub description: Option<String>,
    pub details: VariantDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum VariantDetails {
    Simple,
    Item(TypeId),
    Tuple(Vec<TypeId>),
    Struct(Vec<StructProperty>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StructProperty {
    pub name: String,
    /// The property name exactly as it appears in the schema.
    pub wire_name: String,
    /// The snake_case form of the property name, used as the "API" name
    /// when a [`crate::SerdeFieldCase`] is configured.
    pub api_name: String,
    pub rename: StructPropertyRename,
    pub state: StructPropertyState,
    pub description: Option<String>,
    pub type_id: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StructPropertyRename {
    None,
    Rename(String),
    Flatten,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StructPropertyState {
    Required,
    Optional,
    Default(WrappedValue),
}

#[derive(Debug)]
pub(crate) enum DefaultKind {
    Intrinsic,
    Specific,
    Generic(DefaultImpl),
}

fn variants_unique(variants: &[Variant]) -> bool {
    unique(
        variants
            .iter()
            .map(|variant| variant.ident_name.as_ref().unwrap()),
    )
}

impl TypeEntryEnum {
    pub(crate) fn from_metadata(
        type_space: &TypeSpace,
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        tag_type: EnumTagType,
        mut variants: Vec<Variant>,
        deny_unknown_fields: bool,
        schema: Schema,
    ) -> TypeEntry {
        // Let's find some decent names for variants. We first try the simple
        // sanitization.
        variants.iter_mut().for_each(|variant| {
            let ident_name = sanitize(&variant.raw_name, Case::Pascal);
            variant.ident_name = Some(ident_name);
        });

        // If variants aren't unique, we're turn the elided characters into
        // 'x's.
        if !variants_unique(&variants) {
            variants.iter_mut().for_each(|variant| {
                let ident_name = sanitize(
                    &variant
                        .raw_name
                        .replace(|c| c == '_' || !is_xid_continue(c), "X"),
                    Case::Pascal,
                );
                variant.ident_name = Some(ident_name);
            });
        }

        // If variants still aren't unique, we fail: we'd rather not emit code
        // that can't compile
        if !variants_unique(&variants) {
            let mut counts = HashMap::new();
            variants.iter().for_each(|variant| {
                counts
                    .entry(variant.ident_name.as_ref().unwrap())
                    .and_modify(|xxx| *xxx += 1)
                    .or_insert(0);
            });
            let dups = variants
                .iter()
                .filter(|variant| *counts.get(variant.ident_name.as_ref().unwrap()).unwrap() > 0)
                .map(|variant| variant.raw_name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            panic!("Failed to make unique variant names for [{}]", dups);
        }

        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        let type_patch = TypePatch::new(type_space, name);

        let details = TypeEntryDetails::Enum(Self {
            name: type_patch.name,
            rename,
            description,
            default: None,
            tag_type,
            variants,
            deny_unknown_fields,
            bespoke_impls: Default::default(),
            schema: SchemaWrapper(schema),
        });

        TypeEntry {
            details,
            extra_derives: type_patch.derives,
            extra_attrs: type_patch.attrs,
        }
    }

    pub(crate) fn finalize(&mut self, type_space: &TypeSpace) {
        self.bespoke_impls = [
            // Not untagged with all simple variants.
            (self.tag_type != EnumTagType::Untagged
                && !self.variants.is_empty()
                && self
                    .variants
                    .iter()
                    .all(|variant| matches!(variant.details, VariantDetails::Simple)))
            .then_some(TypeEntryEnumImpl::AllSimpleVariants),
            // Untagged and all variants impl FromStr, but none **is** a
            // String (i.e. irrefutably).
            untagged_newtype_variants(
                type_space,
                &self.tag_type,
                &self.variants,
                TypeSpaceImpl::FromStr,
                Some(TypeSpaceImpl::FromStringIrrefutable),
            )
            .then_some(TypeEntryEnumImpl::UntaggedFromStr),
            // Untagged and all variants impl Display.
            untagged_newtype_variants(
                type_space,
                &self.tag_type,
                &self.variants,
                TypeSpaceImpl::Display,
                None,
            )
            .then_some(TypeEntryEnumImpl::UntaggedDisplay),
            untagged_newtype_string(type_space, &self.tag_type, &self.variants)
                .then_some(TypeEntryEnumImpl::UntaggedFromStringIrrefutable),
        ]
        .into_iter()
        .flatten()
        .collect();
    }
}

impl Variant {
    pub(crate) fn new(
        raw_name: String,
        description: Option<String>,
        details: VariantDetails,
    ) -> Self {
        Self {
            raw_name,
            ident_name: None,
            description,
            details,
        }
    }
}

impl TypeEntryStruct {
    pub(crate) fn from_metadata(
        type_space: &TypeSpace,
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        properties: Vec<StructProperty>,
        deny_unknown_fields: bool,
        schema: Schema,
    ) -> TypeEntry {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);
        let default = metadata
            .as_ref()
            .and_then(|m| m.default.as_ref())
            .cloned()
            .map(WrappedValue::new);

        let type_patch = TypePatch::new(type_space, name);

        let details = TypeEntryDetails::Struct(Self {
            name: type_patch.name,
            rename,
            description,
            default,
            properties,
            deny_unknown_fields,
            schema: SchemaWrapper(schema),
        });

        TypeEntry {
            details,
            extra_derives: type_patch.derives,
            extra_attrs: type_patch.attrs,
        }
    }
}

impl TypeEntryNewtype {
    pub(crate) fn from_metadata(
        type_space: &TypeSpace,
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        type_id: TypeId,
        schema: Schema,
    ) -> TypeEntry {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        let type_patch = TypePatch::new(type_space, name);

        let details = TypeEntryDetails::Newtype(Self {
            name: type_patch.name,
            rename,
            description,
            default: None,
            type_id,
            constraints: TypeEntryNewtypeConstraints::None,
            schema: SchemaWrapper(schema),
        });

        TypeEntry {
            details,
            extra_derives: type_patch.derives,
            extra_attrs: type_patch.attrs,
        }
    }

    pub(crate) fn from_metadata_with_enum_values(
        type_space: &TypeSpace,
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        type_id: TypeId,
        enum_values: &[serde_json::Value],
        schema: Schema,
    ) -> TypeEntry {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        let type_patch = TypePatch::new(type_space, name);

        let details = TypeEntryDetails::Newtype(Self {
            name: type_patch.name,
            rename,
            description,
            default: None,
            type_id,
            constraints: TypeEntryNewtypeConstraints::EnumValue(
                enum_values.iter().cloned().map(WrappedValue::new).collect(),
            ),
            schema: SchemaWrapper(schema),
        });

        TypeEntry {
            details,
            extra_derives: type_patch.derives,
            extra_attrs: type_patch.attrs,
        }
    }

    pub(crate) fn from_metadata_with_deny_values(
        type_space: &TypeSpace,
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        type_id: TypeId,
        enum_values: &[serde_json::Value],
        schema: Schema,
    ) -> TypeEntry {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        let type_patch = TypePatch::new(type_space, name);

        let details = TypeEntryDetails::Newtype(Self {
            name: type_patch.name,
            rename,
            description,
            default: None,
            type_id,
            constraints: TypeEntryNewtypeConstraints::DenyValue(
                enum_values.iter().cloned().map(WrappedValue::new).collect(),
            ),
            schema: SchemaWrapper(schema),
        });

        TypeEntry {
            details,
            extra_derives: type_patch.derives,
            extra_attrs: type_patch.attrs,
        }
    }

    pub(crate) fn from_metadata_with_string_validation(
        type_space: &TypeSpace,
        type_name: Name,
        metadata: &Option<Box<Metadata>>,
        type_id: TypeId,
        validation: &schemars::schema::StringValidation,
        schema: Schema,
    ) -> TypeEntry {
        let name = get_type_name(&type_name, metadata).unwrap();
        let rename = None;
        let description = metadata_description(metadata);

        let schemars::schema::StringValidation {
            max_length,
            min_length,
            pattern,
        } = validation.clone();

        let type_patch = TypePatch::new(type_space, name);

        let details = TypeEntryDetails::Newtype(Self {
            name: type_patch.name,
            rename,
            description,
            default: None,
            type_id,
            constraints: TypeEntryNewtypeConstraints::String {
                max_length,
                min_length,
                pattern,
            },
            schema: SchemaWrapper(schema),
        });

        TypeEntry {
            details,
            extra_derives: type_patch.derives,
            extra_attrs: type_patch.attrs,
        }
    }
}

impl From<TypeEntryDetails> for TypeEntry {
    fn from(details: TypeEntryDetails) -> Self {
        Self {
            details,
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }
}

impl TypeEntry {
    pub(crate) fn new_native<S: ToString>(type_name: S, impls: &[TypeSpaceImpl]) -> Self {
        TypeEntry {
            details: TypeEntryDetails::Native(TypeEntryNative {
                type_name: type_name.to_string(),
                impls: impls.to_vec(),
                parameters: Default::default(),
            }),
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }
    pub(crate) fn new_native_params<S: ToString>(type_name: S, params: &[TypeId]) -> Self {
        TypeEntry {
            details: TypeEntryDetails::Native(TypeEntryNative {
                type_name: type_name.to_string(),
                impls: Default::default(),
                parameters: params.to_vec(),
            }),
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }
    pub(crate) fn new_boolean() -> Self {
        TypeEntry {
            details: TypeEntryDetails::Boolean,
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }
    pub(crate) fn new_integer<S: ToString>(type_name: S) -> Self {
        TypeEntryDetails::Integer(type_name.to_string()).into()
    }
    pub(crate) fn new_float<S: ToString>(type_name: S) -> Self {
        TypeEntry {
            details: TypeEntryDetails::Float(type_name.to_string()),
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }

    pub(crate) fn finalize(&mut self, type_space: &mut TypeSpace) -> Result<()> {
        if let TypeEntryDetails::Enum(enum_details) = &mut self.details {
            enum_details.finalize(type_space);
        }

        self.check_defaults(type_space)
    }

    pub(crate) fn name(&self) -> Option<&String> {
        match &self.details {
            TypeEntryDetails::Enum(TypeEntryEnum { name, .. })
            | TypeEntryDetails::Struct(TypeEntryStruct { name, .. })
            | TypeEntryDetails::Newtype(TypeEntryNewtype { name, .. }) => Some(name),

            _ => None,
        }
    }

    pub(crate) fn has_impl<'a>(
        &'a self,
        type_space: &'a TypeSpace,
        impl_name: TypeSpaceImpl,
    ) -> bool {
        match &self.details {
            TypeEntryDetails::Enum(details) => match impl_name {
                TypeSpaceImpl::Default => details.default.is_some(),
                TypeSpaceImpl::FromStr => {
                    details
                        .bespoke_impls
                        .contains(&TypeEntryEnumImpl::AllSimpleVariants)
                        || details
                            .bespoke_impls
                            .contains(&TypeEntryEnumImpl::UntaggedFromStr)
                }
                TypeSpaceImpl::Display => {
                    details
                        .bespoke_impls
                        .contains(&TypeEntryEnumImpl::AllSimpleVariants)
                        || details
                            .bespoke_impls
                            .contains(&TypeEntryEnumImpl::UntaggedDisplay)
                }
                TypeSpaceImpl::FromStringIrrefutable => details
                    .bespoke_impls
                    .contains(&TypeEntryEnumImpl::UntaggedFromStringIrrefutable),
            },

            TypeEntryDetails::Struct(details) => match impl_name {
                TypeSpaceImpl::Default => details.default.is_some(),
                _ => false,
            },
            TypeEntryDetails::Newtype(details) => match (&details.constraints, impl_name) {
                (_, TypeSpaceImpl::Default) => details.default.is_some(),
                (TypeEntryNewtypeConstraints::String { .. }, TypeSpaceImpl::FromStr) => true,
                (TypeEntryNewtypeConstraints::String { .. }, TypeSpaceImpl::Display) => true,
                (TypeEntryNewtypeConstraints::None, _) => {
                    // TODO this is a lucky kludge that will need to be removed
                    // once we have proper handling of reference cycles (i.e.
                    // as opposed to containment cycles... which we **do**
                    // handle correctly). In particular output_newtype calls
                    // this to determine if it should produce a FromStr impl.
                    // This implementation could be infinitely recursive for a
                    // type such as this:
                    //     struct A(Box<A>);
                    // While this type is useless and unusable, we do--
                    // basically--support and test this. On such a type, if one
                    // were to ask `ty.has_impl(TypeSpaceImpl::Default)` it
                    // would be infinitely recursive. Fortunately the type
                    // doesn't occur in the wild (we hope) and generation
                    // doesn't rely on that particular query.

                    let type_entry = type_space.id_to_entry.get(&details.type_id).unwrap();
                    type_entry.has_impl(type_space, impl_name)
                }
                _ => false,
            },
            TypeEntryDetails::Native(details) => details.impls.contains(&impl_name),
            TypeEntryDetails::Box(type_id) => {
                if impl_name == TypeSpaceImpl::Default {
                    let type_entry = type_space.id_to_entry.get(type_id).unwrap();
                    type_entry.has_impl(type_space, impl_name)
                } else {
                    false
                }
            }

            TypeEntryDetails::JsonValue => false,

            TypeEntryDetails::Unit
            | TypeEntryDetails::Option(_)
            | TypeEntryDetails::Vec(_)
            | TypeEntryDetails::Map(_, _)
            | TypeEntryDetails::Set(_) => {
                matches!(impl_name, TypeSpaceImpl::Default)
            }

            TypeEntryDetails::Tuple(type_ids) => {
                // Default is implemented for tuples of up to 12 items long.
                matches!(impl_name, TypeSpaceImpl::Default)
                    && type_ids.len() <= 12
                    && type_ids.iter().all(|type_id| {
                        let type_entry = type_space.id_to_entry.get(type_id).unwrap();
                        type_entry.has_impl(type_space, TypeSpaceImpl::Default)
                    })
            }

            TypeEntryDetails::Array(item_id, length) => {
                // Default is implemented for arrays of up to length 32.
                if *length <= 32 && impl_name == TypeSpaceImpl::Default {
                    let type_entry = type_space.id_to_entry.get(item_id).unwrap();
                    type_entry.has_impl(type_space, impl_name)
                } else {
                    false
                }
            }

            TypeEntryDetails::Boolean => match impl_name {
                TypeSpaceImpl::Default | TypeSpaceImpl::FromStr | TypeSpaceImpl::Display => true,
                TypeSpaceImpl::FromStringIrrefutable => false,
            },
            TypeEntryDetails::Integer(_) => match impl_name {
                TypeSpaceImpl::Default | TypeSpaceImpl::FromStr | TypeSpaceImpl::Display => true,
                TypeSpaceImpl::FromStringIrrefutable => false,
            },

            TypeEntryDetails::Float(_) => match impl_name {
                TypeSpaceImpl::Default | TypeSpaceImpl::FromStr | TypeSpaceImpl::Display => true,
                TypeSpaceImpl::FromStringIrrefutable => false,
            },
            TypeEntryDetails::String => match impl_name {
                TypeSpaceImpl::Default
                | TypeSpaceImpl::FromStr
                | TypeSpaceImpl::Display
                | TypeSpaceImpl::FromStringIrrefutable => true,
            },

            TypeEntryDetails::Reference(_) => unreachable!(),
        }
    }

    pub(crate) fn output(&self, type_space: &TypeSpace, output: &mut OutputSpace) {
        let derive_set = [
            "::serde::Serialize",
            "::serde::Deserialize",
            "Debug",
            "Clone",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();

        match &self.details {
            TypeEntryDetails::Enum(enum_details) => {
                self.output_enum(type_space, output, enum_details, derive_set)
            }
            TypeEntryDetails::Struct(struct_details) => {
                self.output_struct(type_space, output, struct_details, derive_set)
            }
            TypeEntryDetails::Newtype(newtype_details) => {
                self.output_newtype(type_space, output, newtype_details, derive_set)
            }

            // We should never get here as reference types should only be used
            // in-flight, but never recorded into the type space.
            TypeEntryDetails::Reference(_) => unreachable!(),

            // Unnamed types require no definition as they're already defined.
            _ => (),
        }
    }

    fn output_enum(
        &self,
        type_space: &TypeSpace,
        output: &mut OutputSpace,
        enum_details: &TypeEntryEnum,
        mut derive_set: BTreeSet<&str>,
    ) {
        let TypeEntryEnum {
            name,
            rename,
            description,
            default,
            tag_type,
            variants,
            deny_unknown_fields,
            bespoke_impls,
            schema: SchemaWrapper(schema),
        } = enum_details;

        let doc = make_doc(type_space, name, description.as_ref(), schema);

        // The open-enum catch-all (see `with_open_string_enums`): plain
        // externally-tagged string enums gain a trailing
        // `#[serde(untagged)] <Name>(String)` variant so undocumented wire
        // values round-trip instead of failing deserialization. Enums that
        // already declare a variant with the configured name stay closed —
        // opening them would collide.
        let open_variant = type_space
            .settings
            .open_string_enums()
            .filter(|_| {
                tag_type == &EnumTagType::External
                    && bespoke_impls.contains(&TypeEntryEnumImpl::AllSimpleVariants)
            })
            .filter(|open_name| {
                variants
                    .iter()
                    .all(|variant| variant.ident_name.as_deref() != Some(open_name))
            })
            .map(|open_name| format_ident!("{}", open_name));

        // TODO this is a one-off for some useful traits; this should move into
        // the creation of the enum type.
        if variants
            .iter()
            .all(|variant| matches!(variant.details, VariantDetails::Simple))
        {
            // An opened enum owns a `String` in its catch-all, so `Copy` is
            // off the table; everything else in the historical set holds.
            if open_variant.is_some() {
                derive_set.extend(["PartialOrd", "Ord", "PartialEq", "Eq", "Hash"]);
            } else {
                derive_set.extend(["Copy", "PartialOrd", "Ord", "PartialEq", "Eq", "Hash"]);
            }
        }

        let mut serde_options = Vec::new();
        if let Some(old_name) = rename {
            serde_options.push(quote! { rename = #old_name });
        }
        match tag_type {
            EnumTagType::External => {}
            EnumTagType::Internal { tag } => {
                serde_options.push(quote! { tag = #tag });
            }
            EnumTagType::Adjacent { tag, content } => {
                serde_options.push(quote! { tag = #tag });
                serde_options.push(quote! { content = #content });
            }
            EnumTagType::Untagged => {
                serde_options.push(quote! { untagged });
            }
        }
        if *deny_unknown_fields {
            serde_options.push(quote! { deny_unknown_fields });
        }

        let serde = (!serde_options.is_empty()).then(|| {
            quote! { #[serde( #( #serde_options ),* )] }
        });

        let type_name = format_ident!("{}", name);

        let mut variants_decl = variants
            .iter()
            .map(|variant| output_variant(variant, type_space, output, name))
            .collect::<Vec<_>>();
        if let Some(open) = &open_variant {
            variants_decl.push(quote! {
                #[doc = r" Catch-all for values outside the documented set;"]
                #[doc = r" carries the raw wire string."]
                #[serde(untagged)]
                #open(::std::string::String),
            });
        }

        // It should not be possible to construct an untagged enum
        // with more than one simple variant--it would not be usable.
        if tag_type == &EnumTagType::Untagged {
            assert!(
                variants
                    .iter()
                    .filter(|variant| matches!(variant.details, VariantDetails::Simple))
                    .count()
                    <= 1
            )
        }

        // Display and FromStr impls for enums that are made exclusively of
        // simple variants (and are not untagged).
        let simple_enum_impl = bespoke_impls
            .contains(&TypeEntryEnumImpl::AllSimpleVariants)
            .then(|| {
                let (match_variants, match_strs): (Vec<_>, Vec<_>) = variants
                    .iter()
                    .map(|variant| {
                        let ident_name = variant.ident_name.as_ref().unwrap();
                        let variant_name = format_ident!("{}", ident_name);
                        (variant_name, &variant.raw_name)
                    })
                    .unzip();

                // An opened enum's ladder gains catch-all arms: `Display`
                // writes the carried string and `FromStr` becomes
                // irrefutable. The closed shape (including its `match
                // *self`, which the catch-all's `String` cannot support)
                // is preserved byte-for-byte when the knob is off.
                let display_scrutinee = match &open_variant {
                    Some(_) => quote! { self },
                    None => quote! { *self },
                };
                let display_fallback = open_variant.as_ref().map(|open| {
                    quote! { Self::#open(value) => f.write_str(value.as_str()), }
                });
                let from_str_fallback = match &open_variant {
                    Some(open) => quote! { _ => Ok(Self::#open(value.to_string())), },
                    None => quote! { _ => Err("invalid value".into()), },
                };

                quote! {
                    impl ::std::fmt::Display for #type_name {
                        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                            match #display_scrutinee {
                                #(Self::#match_variants => f.write_str(#match_strs),)*
                                #display_fallback
                            }
                        }
                    }
                    impl ::std::str::FromStr for #type_name {
                        type Err = self::error::ConversionError;

                        fn from_str(value: &str) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            match value {
                                #(#match_strs => Ok(Self::#match_variants),)*
                                #from_str_fallback
                            }
                        }
                    }
                    impl ::std::convert::TryFrom<&str> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: &str) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                    impl ::std::convert::TryFrom<&::std::string::String> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: &::std::string::String) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                    impl ::std::convert::TryFrom<::std::string::String> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: ::std::string::String) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                }
            });

        // Honor the schema-level default when present, otherwise fall back
        // to the auto-emit-first-Simple-variant setting so required-enum
        // struct fields can satisfy `#[derive(Default)]` on the
        // containing struct.
        let default_impl = match default.as_ref() {
            Some(value) => {
                let default_stream = self.output_value(type_space, &value.0, &quote! {}).unwrap();
                Some(quote! {
                    impl ::std::default::Default for #type_name {
                        fn default() -> Self {
                            #default_stream
                        }
                    }
                })
            }
            None if type_space.settings.enum_first_variant_default() => variants
                .iter()
                .find_map(|variant| match variant.details {
                    VariantDetails::Simple => variant.ident_name.as_deref(),
                    _ => None,
                })
                .map(|ident_name| {
                    let variant_name = format_ident!("{}", ident_name);
                    quote! {
                        impl ::std::default::Default for #type_name {
                            fn default() -> Self {
                                Self::#variant_name
                            }
                        }
                    }
                }),
            None => None,
        };

        let untagged_newtype_from_string_impl = bespoke_impls
            .contains(&TypeEntryEnumImpl::UntaggedFromStr)
            .then(|| {
                let variant_name = variants
                    .iter()
                    .map(|variant| format_ident!("{}", variant.ident_name.as_ref().unwrap()));

                quote! {
                    impl ::std::str::FromStr for #type_name {
                        type Err = self::error::ConversionError;

                        fn from_str(value: &str) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            #(
                                // Try to parse() into each variant.
                                if let Ok(v) = value.parse() {
                                    Ok(Self::#variant_name(v))
                                } else
                            )*
                            {
                                Err("string conversion failed for all variants".into())
                            }
                        }
                    }
                    impl ::std::convert::TryFrom<&str> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: &str) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                    impl ::std::convert::TryFrom<&::std::string::String> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: &::std::string::String) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                    impl ::std::convert::TryFrom<::std::string::String> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: ::std::string::String) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                }
            });

        let untagged_newtype_to_string_impl = bespoke_impls
            .contains(&TypeEntryEnumImpl::UntaggedDisplay)
            .then(|| {
                let variant_name = variants
                    .iter()
                    .map(|variant| format_ident!("{}", variant.ident_name.as_ref().unwrap()));

                quote! {
                    impl ::std::fmt::Display for #type_name {
                        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                            match self {
                                #(Self::#variant_name(x) => x.fmt(f),)*
                            }
                        }
                    }
                }
            });

        let convenience_from = {
            // Build a map whose key is the type ID or type IDs of the Item and
            // Tuple variants, and whose value is a tuple of the original index
            // and the variant itself. Any key that is seen multiple times has
            // a value of None.
            // TODO this requires more consideration to handle single-item
            // tuples.
            let unique_variants =
                variants
                    .iter()
                    .enumerate()
                    .fold(BTreeMap::new(), |mut map, (index, variant)| {
                        let key = match &variant.details {
                            VariantDetails::Item(type_id) => vec![type_id],
                            VariantDetails::Tuple(type_ids) => type_ids.iter().collect(),
                            _ => return map,
                        };

                        map.entry(key)
                            .and_modify(|v| *v = None)
                            .or_insert(Some((index, variant)));
                        map
                    });

            // Remove any variants that are duplicates (i.e. the value is None)
            // with the flatten(). Then order a new map according to the
            // original order of variants. The allows for the order to be
            // stable and for impl blocks to appear in the same order as their
            // corresponding variants.
            let ordered_variants = unique_variants
                .into_values()
                .flatten()
                .collect::<BTreeMap<_, _>>();

            // Generate a `From<VariantType>` impl block that converts the type
            // into the appropriate variant of the enum.
            let variant_from = ordered_variants.into_values().map(|variant| {
                match &variant.details {
                    VariantDetails::Item(type_id) => {
                        let variant_type = type_space.id_to_entry.get(type_id).unwrap();

                        // TODO Strings might conflict with the way we're
                        // dealing with TryFrom<String> right now. That
                        // includes native `::std::string::String` entries
                        // (a `with_date_time_type`-style override mapping
                        // a format to String): `From<String>` + the
                        // `TryFrom<String>` ladder collide through core's
                        // blanket impl.
                        let is_string = variant_type.details == TypeEntryDetails::String
                            || matches!(
                                &variant_type.details,
                                TypeEntryDetails::Native(native)
                                    if native.type_name == "::std::string::String"
                            );
                        (!is_string).then(|| {
                            let variant_type_ident = variant_type.type_ident(type_space, &None);
                            let variant_name =
                                format_ident!("{}", variant.ident_name.as_ref().unwrap());
                            quote! {
                                impl ::std::convert::From<#variant_type_ident> for #type_name {
                                    fn from(value: #variant_type_ident)
                                        -> Self
                                    {
                                        Self::#variant_name(value)
                                    }
                                }
                            }
                        })
                    }
                    VariantDetails::Tuple(type_ids) => {
                        let variant_type_idents = type_ids.iter().map(|type_id| {
                            type_space
                                .id_to_entry
                                .get(type_id)
                                .unwrap()
                                .type_ident(type_space, &None)
                        });
                        let variant_type_ident = if type_ids.len() != 1 {
                            quote! { ( #(#variant_type_idents),* ) }
                        } else {
                            // A single-item tuple requires a trailing
                            // comma.
                            quote! { ( #(#variant_type_idents,)* ) }
                        };
                        let variant_name =
                            format_ident!("{}", variant.ident_name.as_ref().unwrap());
                        let ii = (0..type_ids.len()).map(syn::Index::from);
                        Some(quote! {
                            impl ::std::convert::From<#variant_type_ident> for #type_name {
                                fn from(value: #variant_type_ident) -> Self {
                                    Self::#variant_name(
                                        #( value.#ii, )*
                                    )
                                }
                            }
                        })
                    }
                    _ => None,
                }
            });

            quote! {
                #( #variant_from )*
            }
        };

        let derives = compute_derive_list(
            type_space,
            crate::TypeKind::Enum,
            derive_set,
            &self.extra_derives,
            &type_space.settings.extra_derives,
        );

        let attrs = strings_to_attrs(&self.extra_attrs, &type_space.settings.extra_attrs);
        let extra_attr_lists = type_attr_lists(type_space, crate::TypeKind::Enum);

        let uncond_attrs_pre = &extra_attr_lists.uncond_pre;
        let uncond_attrs_post = &extra_attr_lists.uncond_post;
        let cond_attrs_pre = &extra_attr_lists.cond_pre;
        let cond_attrs_post = &extra_attr_lists.cond_post;
        let cond_derives = &extra_attr_lists.cond_derives;

        let item = quote! {
            #doc
            #(#attrs)*
            #(#uncond_attrs_pre)*
            #(#cond_attrs_pre)*
            #(#cond_derives)*
            #[derive(#(#derives),*)]
            #serde
            #(#uncond_attrs_post)*
            #(#cond_attrs_post)*
            pub enum #type_name {
                #(#variants_decl)*
            }

            #simple_enum_impl
            #default_impl
            #untagged_newtype_from_string_impl
            #untagged_newtype_to_string_impl
            #convenience_from
        };
        output.add_item(OutputSpaceMod::Crate, name, item);
    }

    fn output_struct(
        &self,
        type_space: &TypeSpace,
        output: &mut OutputSpace,
        struct_details: &TypeEntryStruct,
        derive_set: BTreeSet<&str>,
    ) {
        enum PropDefault {
            None(String),
            Default(TokenStream),
            Custom(TokenStream),
        }

        let TypeEntryStruct {
            name,
            rename,
            description,
            default,
            properties,
            deny_unknown_fields,
            schema: SchemaWrapper(schema),
        } = struct_details;
        let doc = make_doc(type_space, name, description.as_ref(), schema);

        // Generate the serde directives as needed. The struct-level
        // `rename_all` is emitted when the caller has configured one via
        // `with_struct_rename_all` — see the docs on that method for the
        // per-field elision rule this enables.
        let mut serde_options = Vec::new();
        if let Some(old_name) = rename {
            serde_options.push(quote! { rename = #old_name });
        }
        if type_space.settings.serde_field_case().is_none() {
            if let Some(case) = type_space.settings.struct_rename_all() {
                serde_options.push(quote! { rename_all = #case });
            }
        }
        if *deny_unknown_fields {
            serde_options.push(quote! { deny_unknown_fields });
        }
        let serde =
            (!serde_options.is_empty()).then(|| quote! { #[serde( #( #serde_options ),* )] });

        let type_name = format_ident!("{}", name);

        // The final derive list is needed before the property loop: when it
        // carries a `Patch` derive, each renamed field must also emit the
        // patch-companion naming mirror computed by `generate_serde_attr`
        // (struct_patch does not carry field serde attributes over to the
        // `{Type}Patch` companion, so an unmirrored rename makes the
        // companion address the wrong wire key).
        let derives = compute_derive_list(
            type_space,
            crate::TypeKind::Struct,
            derive_set,
            &self.extra_derives,
            &type_space.settings.extra_derives,
        );
        let has_patch_derive = derives.iter().any(|derive| {
            derive
                .to_string()
                .rsplit("::")
                .next()
                .is_some_and(|segment| segment.trim() == "Patch")
        });

        // Gather the various components for all properties.
        let mut prop_doc = Vec::new();
        let mut prop_serde = Vec::new();
        let mut prop_patch = Vec::new();
        let mut prop_default = Vec::new();
        let mut prop_name = Vec::new();
        let mut prop_error = Vec::new();
        let mut prop_type = Vec::new();
        let mut prop_type_scoped = Vec::new();

        properties.iter().for_each(|prop| {
            prop_doc.push(prop.description.as_ref().map(|d| quote! { #[doc = #d] }));
            prop_name.push(format_ident!("{}", prop.name));
            prop_error.push(format!(
                "error converting supplied value for {}: {{e}}",
                prop.name,
            ));

            let prop_type_entry = type_space.id_to_entry.get(&prop.type_id).unwrap();
            prop_type.push(prop_type_entry.type_ident(type_space, &None));
            prop_type_scoped
                .push(prop_type_entry.type_ident(type_space, &Some("super".to_string())));

            let (serde, default_fn, patch_naming_mirror) = generate_serde_attr(
                name,
                &prop.name,
                &prop.wire_name,
                &prop.api_name,
                &prop.rename,
                &prop.state,
                prop_type_entry,
                type_space,
                output,
            );

            prop_serde.push(serde);
            let deep_patch = deep_patch_attr(type_space, name, prop);
            let naming_mirror = has_patch_derive.then_some(patch_naming_mirror).flatten();
            prop_patch.push(quote! { #deep_patch #naming_mirror });
            prop_default.push(match default_fn {
                DefaultFunction::Default => PropDefault::Default(quote! {
                    Default::default()
                }),
                DefaultFunction::Custom(fn_name) => {
                    let default_fn = syn::parse_str::<Path>(&fn_name).unwrap();
                    PropDefault::Custom(quote! {
                        #default_fn()
                    })
                }
                DefaultFunction::None => {
                    let err_msg = format!("no value supplied for {}", prop.name);
                    PropDefault::None(err_msg)
                }
            });
        });

        let attrs = strings_to_attrs(&self.extra_attrs, &type_space.settings.extra_attrs);
        let extra_attr_lists = type_attr_lists(type_space, crate::TypeKind::Struct);

        let uncond_attrs_pre = &extra_attr_lists.uncond_pre;
        let uncond_attrs_post = &extra_attr_lists.uncond_post;
        let cond_attrs_pre = &extra_attr_lists.cond_pre;
        let cond_attrs_post = &extra_attr_lists.cond_post;
        let cond_derives = &extra_attr_lists.cond_derives;

        output.add_item(
            OutputSpaceMod::Crate,
            name,
            quote! {
                #doc
                #(#attrs)*
                #(#uncond_attrs_pre)*
                #(#cond_attrs_pre)*
                #(#cond_derives)*
                #[derive(#(#derives),*)]
                #serde
                #(#uncond_attrs_post)*
                #(#cond_attrs_post)*
                pub struct #type_name {
                    #(
                        #prop_doc
                        #prop_serde
                        #prop_patch
                        pub #prop_name: #prop_type,
                    )*
                }
            },
        );

        // If `Default` is in the unconditional derive list for structs,
        // skip the hand-written `impl Default` block — the derive
        // already produces one, and two would clash. The schema-provided
        // default value is still honored at deserialization time via the
        // per-field `#[serde(default = "...")]` attributes that
        // `generate_serde_attr` emits; what changes is that calling
        // `Foo::default()` in Rust returns the all-fields-default value
        // rather than reconstructing the schema-provided one.
        let unconditional_default_for_structs = type_space
            .settings
            .unconditional_derives
            .iter()
            .any(|u| u.derive == "Default" && u.kinds.matches(crate::TypeKind::Struct));

        if !unconditional_default_for_structs {
            // If there's a default value, generate an impl Default
            if let Some(value) = default {
                let default_stream = self.output_value(type_space, &value.0, &quote! {}).unwrap();
                output.add_item(
                    OutputSpaceMod::Crate,
                    name,
                    quote! {
                        impl ::std::default::Default for #type_name {
                            fn default() -> Self {
                                #default_stream
                            }
                        }
                    },
                );
            } else if let Some(prop_default) = prop_default
                .iter()
                .map(|pd| match pd {
                    PropDefault::None(_) => None,
                    PropDefault::Default(token_stream) | PropDefault::Custom(token_stream) => {
                        Some(token_stream)
                    }
                })
                .collect::<Option<Vec<_>>>()
            {
                // If all properties have a default, we can generate a Default impl
                output.add_item(
                    OutputSpaceMod::Crate,
                    name,
                    quote! {
                        impl ::std::default::Default for #type_name {
                            fn default() -> Self {
                                Self {
                                    #(
                                        #prop_name: #prop_default,
                                    )*
                                }
                            }
                        }
                    },
                )
            }
        }

        if type_space.settings.struct_builder {
            output.add_item(
                OutputSpaceMod::Crate,
                name,
                quote! {
                    impl #type_name {
                        pub fn builder() -> builder::#type_name {
                            Default::default()
                        }
                    }
                },
            );

            // If there are no properties, all of this is kind of pointless,
            // but at least this lets us avoid the lint warning.
            let value_ident = if prop_name.is_empty() {
                quote! { _value }
            } else {
                quote! { value }
            };

            let prop_default = prop_default.iter().map(|pd| match pd {
                PropDefault::None(err_msg) => quote! { Err(#err_msg.to_string()) },
                PropDefault::Default(default_fn) => quote! { Ok(#default_fn) },
                PropDefault::Custom(custom_fn) => quote! { Ok(super::#custom_fn) },
            });

            output.add_item(
                OutputSpaceMod::Builder,
                name,
                quote! {
                    #[derive(Clone, Debug)]
                    pub struct #type_name {
                        #(
                            #prop_name: ::std::result::Result<#prop_type_scoped, ::std::string::String>,
                        )*
                    }

                    impl ::std::default::Default for #type_name {
                        fn default() -> Self {
                            Self {
                                #(
                                    #prop_name: #prop_default,
                                )*
                            }
                        }
                    }

                    impl #type_name {
                        #(
                            pub fn #prop_name<T>(mut self, value: T) -> Self
                                where
                                    T: ::std::convert::TryInto<#prop_type_scoped>,
                                    T::Error: ::std::fmt::Display,
                            {
                                self.#prop_name = value.try_into()
                                    .map_err(|e| format!(#prop_error));
                                self
                            }
                        )*
                    }

                    // This is how the item is built.
                    impl ::std::convert::TryFrom<#type_name>
                        for super::#type_name
                    {
                        type Error = super::error::ConversionError;

                        fn try_from(#value_ident: #type_name)
                            -> ::std::result::Result<Self, super::error::ConversionError>
                        {
                            Ok(Self {
                                #(
                                    #prop_name: value.#prop_name?,
                                )*
                            })
                        }
                    }

                    // Construct a builder from the item.
                    impl ::std::convert::From<super::#type_name> for #type_name {
                        fn from(#value_ident: super::#type_name) -> Self {
                            Self {
                                #(
                                    #prop_name: Ok(value.#prop_name),
                                )*
                            }
                        }
                    }
                },
            );
        }
    }

    fn output_newtype<'a>(
        &self,
        type_space: &'a TypeSpace,
        output: &mut OutputSpace,
        newtype_details: &TypeEntryNewtype,
        mut derive_set: BTreeSet<&'a str>,
    ) {
        let TypeEntryNewtype {
            name,
            rename: _,
            description,
            default,
            type_id,
            constraints,
            schema: SchemaWrapper(schema),
        } = newtype_details;
        let doc = make_doc(type_space, name, description.as_ref(), schema);

        let type_name = format_ident!("{}", name);
        let inner_type = type_space.id_to_entry.get(type_id).unwrap();
        let inner_type_name = inner_type.type_ident(type_space, &None);

        let is_str = matches!(inner_type.details, TypeEntryDetails::String);

        // If this is just a wrapper around a string, we can derive some more
        // useful traits.
        if is_str {
            derive_set.extend(["PartialOrd", "Ord", "PartialEq", "Eq", "Hash"]);
        }

        derive_set.extend(type_space.settings.extra_derives.iter().map(|s| s.as_str()));

        // Derives that must not appear in the emitted `#[derive(...)]`
        // because a bespoke impl below replaces them. Unlike the
        // `derive_set.remove(...)` calls (which only affect the legacy
        // derive-set path), entries here are also filtered out of
        // caller-supplied unconditional derives — see `compute_derive_list`.
        let mut denied_derives: Vec<&str> = Vec::new();

        let constraint_impl = match constraints {
            // In the unconstrained case we proxy impls through the inner type.
            TypeEntryNewtypeConstraints::None => {
                let str_impl = is_str.then(|| {
                    quote! {
                        impl ::std::str::FromStr for #type_name {
                            type Err = ::std::convert::Infallible;

                            fn from_str(value: &str) ->
                                ::std::result::Result<Self, Self::Err>
                            {
                                Ok(Self(value.to_string()))
                            }
                        }
                    }
                });

                // When the inner type is a native override that resolves to
                // `::std::string::String` (e.g. `with_date_type("::std::string::String")`),
                // the `From<#inner_type_name>` impl above is `From<String>`,
                // and the std blanket `impl<T, U: Into<T>> TryFrom<U> for T`
                // then already provides `TryFrom<String>`. Emitting the
                // manual `TryFrom<String>` would be a conflicting impl
                // (E0119), so we skip it; conversion from `String` remains
                // available through the blanket impl.
                let inner_is_string_path = matches!(
                    inner_type_name.to_string().replace(' ', "").as_str(),
                    "::std::string::String" | "std::string::String" | "String"
                );

                // TODO see the comment in has_impl related to this case.
                let from_str_impl = (inner_type.has_impl(type_space, TypeSpaceImpl::FromStr)
                    && !is_str)
                    .then(|| {
                        let try_from_string_impl = (!inner_is_string_path).then(|| {
                            quote! {
                                impl ::std::convert::TryFrom<String> for #type_name {
                                    type Error = <#inner_type_name as
                                        ::std::str::FromStr>::Err;

                                    fn try_from(value: String) ->
                                        ::std::result::Result<Self, Self::Error>
                                    {
                                        value.parse()
                                    }
                                }
                            }
                        });

                        quote! {
                            impl ::std::str::FromStr for #type_name {
                                type Err = <#inner_type_name as
                                    ::std::str::FromStr>::Err;

                                fn from_str(value: &str) ->
                                    ::std::result::Result<Self, Self::Err>
                                {
                                    Ok(Self(value.parse()?))
                                }
                            }
                            impl ::std::convert::TryFrom<&str> for #type_name {
                                type Error = <#inner_type_name as
                                    ::std::str::FromStr>::Err;

                                fn try_from(value: &str) ->
                                    ::std::result::Result<Self, Self::Error>
                                {
                                    value.parse()
                                }
                            }
                            #try_from_string_impl
                        }
                    });

                // When the convenience surface is on, string newtypes get
                // their `Display` from the `str_convenience_impl` block
                // below; everything else proxies through the inner type.
                let display_impl = (inner_type.has_impl(type_space, TypeSpaceImpl::Display)
                    && !(is_str && type_space.string_newtype_conveniences()))
                    .then(|| {
                        quote! {
                            impl ::std::fmt::Display for #type_name {
                                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                                    self.0.fmt(f)
                                }
                            }
                        }
                    });

                quote! {
                    impl ::std::convert::From<#inner_type_name> for #type_name {
                        fn from(value: #inner_type_name) -> Self {
                            Self(value)
                        }
                    }

                    #str_impl
                    #from_str_impl
                    #display_impl
                }
            }

            TypeEntryNewtypeConstraints::DenyValue(enum_values)
            | TypeEntryNewtypeConstraints::EnumValue(enum_values) => {
                // Note that string types with enumerated values are converted
                // into simple enums rather than newtypes so we would not
                // expect to see a string as the inner type here.
                assert!(
                    matches!(constraints, TypeEntryNewtypeConstraints::DenyValue(_))
                        || !matches!(&inner_type.details, TypeEntryDetails::String)
                );

                // We're going to impl Deserialize so we can remove it
                // from the set of derived impls.
                derive_set.remove("::serde::Deserialize");
                denied_derives.extend(["::serde::Deserialize", "Deserialize"]);

                let value_output = enum_values
                    .iter()
                    .map(|value| inner_type.output_value(type_space, &value.0, &quote! {}));

                let value_string = enum_values
                    .iter()
                    .map(|value| serde_json::to_string(&value.0).unwrap());

                // As with Deserialize, serde::JsonSchema requires a custom
                // impl. If it's present in the set of derives, remove it and
                // generate something that accurately models the type.

                let has_json_schema = derive_set.remove("schemars::JsonSchema")
                    || derive_set.remove("::schemars::JsonSchema");
                let json_schema = has_json_schema.then(|| match constraints {
                    TypeEntryNewtypeConstraints::DenyValue(_) => quote! {
                        impl ::schemars::JsonSchema for #type_name {
                            fn schema_name() -> ::std::string::String {
                                #name.to_string()
                            }

                            fn json_schema(gen: &mut ::schemars::gen::SchemaGenerator)
                                -> ::schemars::schema::Schema {
                                let mut schema =
                                    <#inner_type_name as ::schemars::JsonSchema>
                                        ::json_schema(gen)
                                        .into_object();
                                let not = ::schemars::schema::SchemaObject {
                                    enum_values: ::std::option::Option::Some([
                                        #( ::serde_json::from_str(#value_string).unwrap(), )*
                                    ].into_iter().collect()),
                                    ..::std::default::Default::default()
                                };
                                schema.subschemas().not = Some(
                                    ::std::boxed::Box::new(not.into())
                                );
                                schema.into()
                            }
                        }
                    },
                    TypeEntryNewtypeConstraints::EnumValue(_) => quote! {
                        impl ::schemars::JsonSchema for #type_name {
                            fn schema_name() -> ::std::string::String {
                                #name.to_string()
                            }

                            fn json_schema(gen: &mut ::schemars::gen::SchemaGenerator)
                                -> ::schemars::schema::Schema {
                                let mut schema =
                                    <#inner_type_name as ::schemars::JsonSchema>
                                        ::json_schema(gen)
                                        .into_object();
                                schema.enum_values = ::std::option::Option::Some([
                                    #( ::serde_json::from_str(#value_string).unwrap(), )*
                                ].into_iter().collect());
                                schema.into()
                            }
                        }
                    },

                    _ => unreachable!(),
                });

                // TODO if the sub_type is a string we could probably impl
                // TryFrom<&str> as well and FromStr.

                let not = matches!(constraints, TypeEntryNewtypeConstraints::EnumValue(_))
                    .then(|| quote! { ! });

                quote! {
                    // This is effectively the constructor for this type.
                    impl ::std::convert::TryFrom<#inner_type_name> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(
                            value: #inner_type_name
                        ) -> ::std::result::Result<Self, self::error::ConversionError>
                        {
                            if #not [
                                #(#value_output,)*
                            ].contains(&value) {
                                Err("invalid value".into())
                            } else {
                                Ok(Self(value))
                            }
                        }
                    }

                    impl<'de> ::serde::Deserialize<'de> for #type_name {
                        fn deserialize<D>(
                            deserializer: D,
                        ) -> ::std::result::Result<Self, D::Error>
                        where
                            D: ::serde::Deserializer<'de>,
                        {
                            Self::try_from(
                                <#inner_type_name>::deserialize(deserializer)?,
                            )
                            .map_err(|e| {
                                <D::Error as ::serde::de::Error>::custom(
                                    e.to_string(),
                                )
                            })
                        }
                    }

                    #json_schema
                }
            }

            TypeEntryNewtypeConstraints::String {
                max_length,
                min_length,
                pattern,
            } => {
                let max = max_length.map(|v| {
                    let v = v as usize;
                    let err = format!("longer than {} characters", v);
                    quote! {
                        if value.chars().count() > #v {
                            return Err(#err.into());
                        }
                    }
                });
                let min = min_length.map(|v| {
                    let v = v as usize;
                    let err = format!("shorter than {} characters", v);
                    quote! {
                        if value.chars().count() < #v {
                            return Err(#err.into());
                        }
                    }
                });

                let pat = pattern.as_ref().map(|p| {
                    let err = format!("doesn't match pattern \"{}\"", p);
                    quote! {
                        static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(|| {
                            ::regress::Regex::new(#p).unwrap()
                        });
                        if PATTERN.find(value).is_none() {
                            return Err(#err.into());
                        }
                    }
                });

                // We're going to impl Deserialize so we can remove it
                // from the set of derived impls.
                derive_set.remove("::serde::Deserialize");
                denied_derives.extend(["::serde::Deserialize", "Deserialize"]);

                // TODO: if a user were to derive schemars::JsonSchema, it
                // wouldn't be accurate.
                quote! {
                    impl ::std::str::FromStr for #type_name {
                        type Err = self::error::ConversionError;

                        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
                            #max
                            #min
                            #pat

                            Ok(Self(value.to_string()))
                        }
                    }
                    impl ::std::convert::TryFrom<&str> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: &str) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                    impl ::std::convert::TryFrom<&::std::string::String> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: &::std::string::String) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }
                    impl ::std::convert::TryFrom<::std::string::String> for #type_name {
                        type Error = self::error::ConversionError;

                        fn try_from(value: ::std::string::String) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            value.parse()
                        }
                    }

                    impl<'de> ::serde::Deserialize<'de> for #type_name {
                        fn deserialize<D>(
                            deserializer: D,
                        ) -> ::std::result::Result<Self, D::Error>
                        where
                            D: ::serde::Deserializer<'de>,
                        {
                            ::std::string::String::deserialize(deserializer)?
                            .parse()
                            .map_err(|e: self::error::ConversionError| {
                                <D::Error as ::serde::de::Error>::custom(
                                    e.to_string(),
                                )
                            })
                        }
                    }
                }
            }
        };

        // If there are no constraints, let consumers directly access the value.
        let vis = match constraints {
            TypeEntryNewtypeConstraints::None => Some(quote! {pub}),
            _ => None,
        };

        // As for structs above: when `Default` is in the unconditional
        // derive list for newtypes, the derive already produces an impl
        // and a hand-written one would clash (E0119). The schema default
        // still applies at deserialization; `X::default()` returns the
        // inner type's default instead of the schema-provided value.
        let unconditional_default_for_newtypes = type_space
            .settings
            .unconditional_derives
            .iter()
            .any(|u| u.derive == "Default" && u.kinds.matches(crate::TypeKind::Newtype));
        let default_impl = default
            .as_ref()
            .filter(|_| !unconditional_default_for_newtypes)
            .map(|value| {
                let default_stream = self.output_value(type_space, &value.0, &quote! {}).unwrap();
                quote! {
                    impl ::std::default::Default for #type_name {
                        fn default() -> Self {
                            #default_stream
                        }
                    }
                }
            });

        // Opt-in convenience impls for string newtypes
        // (`with_string_newtype_conveniences`): expose the inner value as
        // `&str`, print it directly, and (when unconstrained) allow cheap
        // construction from a `&str`. Constrained newtypes only get the
        // read-side impls — construction must go through the validating
        // `FromStr` / `TryFrom` path.
        let str_convenience_impl = (is_str && type_space.string_newtype_conveniences()).then(|| {
            let from_str_ref = matches!(constraints, TypeEntryNewtypeConstraints::None).then(|| {
                quote! {
                    impl ::std::convert::From<&str> for #type_name {
                        fn from(value: &str) -> Self {
                            Self(value.to_string())
                        }
                    }
                }
            });
            quote! {
                impl ::std::convert::AsRef<str> for #type_name {
                    fn as_ref(&self) -> &str {
                        self.0.as_ref()
                    }
                }

                impl ::std::fmt::Display for #type_name {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        self.0.fmt(f)
                    }
                }

                #from_str_ref
            }
        });

        // This isn't the cleanest. Unlike other types, we roll in the
        // extra_derives here so that we can sniff out and override uses of
        // "schemars::JsonSchema".
        let derives = compute_derive_list_denying(
            type_space,
            crate::TypeKind::Newtype,
            derive_set,
            &self.extra_derives,
            &[],
            &denied_derives,
        );

        let attrs = strings_to_attrs(&self.extra_attrs, &type_space.settings.extra_attrs);
        let extra_attr_lists = type_attr_lists(type_space, crate::TypeKind::Newtype);

        let uncond_attrs_pre = &extra_attr_lists.uncond_pre;
        let uncond_attrs_post = &extra_attr_lists.uncond_post;
        let cond_attrs_pre = &extra_attr_lists.cond_pre;
        let cond_attrs_post = &extra_attr_lists.cond_post;
        let cond_derives = &extra_attr_lists.cond_derives;

        let item = quote! {
            #doc
            #(#attrs)*
            #(#uncond_attrs_pre)*
            #(#cond_attrs_pre)*
            #(#cond_derives)*
            #[derive(#(#derives),*)]
            #[serde(transparent)]
            #(#uncond_attrs_post)*
            #(#cond_attrs_post)*
            pub struct #type_name(#vis #inner_type_name);

            impl ::std::ops::Deref for #type_name {
                type Target = #inner_type_name;
                fn deref(&self) -> &#inner_type_name {
                    &self.0
                }
            }

            impl ::std::convert::From<#type_name> for #inner_type_name {
                fn from(value: #type_name) -> Self {
                    value.0
                }
            }

            #default_impl
            #str_convenience_impl
            #constraint_impl
        };
        output.add_item(OutputSpaceMod::Crate, name, item);
    }

    pub(crate) fn type_name(&self, type_space: &TypeSpace) -> String {
        self.type_ident(type_space, &None).to_string()
    }

    pub(crate) fn type_ident(
        &self,
        type_space: &TypeSpace,
        type_mod: &Option<String>,
    ) -> TokenStream {
        match &self.details {
            // Named types.
            TypeEntryDetails::Enum(TypeEntryEnum { name, .. })
            | TypeEntryDetails::Struct(TypeEntryStruct { name, .. })
            | TypeEntryDetails::Newtype(TypeEntryNewtype { name, .. }) => match &type_mod {
                Some(type_mod) => {
                    let type_mod = format_ident!("{}", type_mod);
                    let type_name = format_ident!("{}", name);
                    quote! { #type_mod :: #type_name }
                }
                None => {
                    let type_name = format_ident!("{}", name);
                    quote! { #type_name }
                }
            },

            TypeEntryDetails::Option(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for option");
                let inner_ident = inner_ty.type_ident(type_space, type_mod);

                // Flatten nested Option types. This would only happen if the
                // schema encoded it; it's an odd construction.
                match &inner_ty.details {
                    TypeEntryDetails::Option(_) => inner_ident,
                    _ => quote! { ::std::option::Option<#inner_ident> },
                }
            }

            TypeEntryDetails::Box(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for box");

                let item = inner_ty.type_ident(type_space, type_mod);

                quote! { ::std::boxed::Box<#item> }
            }

            TypeEntryDetails::Vec(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for array");
                let item = inner_ty.type_ident(type_space, type_mod);

                quote! { ::std::vec::Vec<#item> }
            }

            TypeEntryDetails::Map(key_id, value_id) => {
                let map_to_use = &type_space.settings.map_type;
                let key_ty = type_space
                    .id_to_entry
                    .get(key_id)
                    .expect("unresolved type id for map key");
                let value_ty = type_space
                    .id_to_entry
                    .get(value_id)
                    .expect("unresolved type id for map value");

                if key_ty.details == TypeEntryDetails::String
                    && value_ty.details == TypeEntryDetails::JsonValue
                {
                    quote! { ::serde_json::Map<::std::string::String, ::serde_json::Value> }
                } else {
                    let key_ident = key_ty.type_ident(type_space, type_mod);
                    let value_ident = value_ty.type_ident(type_space, type_mod);
                    let map_to_use = &map_to_use.0;

                    quote! { #map_to_use<#key_ident, #value_ident> }
                }
            }

            TypeEntryDetails::Set(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for set");
                let item = inner_ty.type_ident(type_space, type_mod);
                // TODO we'll want this to be a Set of some kind, but we need
                // to get the derives right first.
                quote! { Vec<#item> }
            }

            TypeEntryDetails::Tuple(items) => {
                let type_idents = items.iter().map(|item| {
                    type_space
                        .id_to_entry
                        .get(item)
                        .expect("unresolved type id for tuple")
                        .type_ident(type_space, type_mod)
                });

                if items.len() != 1 {
                    quote! { ( #(#type_idents),* ) }
                } else {
                    // A single-item tuple requires a trailing comma.
                    quote! { ( #(#type_idents,)* ) }
                }
            }

            TypeEntryDetails::Array(item_id, length) => {
                let item_ty = type_space
                    .id_to_entry
                    .get(item_id)
                    .expect("unresolved type id for array");
                let item_ident = item_ty.type_ident(type_space, type_mod);

                quote! { [#item_ident; #length]}
            }

            TypeEntryDetails::Native(TypeEntryNative {
                type_name,
                impls: _,
                parameters,
            }) => {
                let path =
                    syn::parse_str::<syn::TypePath>(type_name).expect("type path wasn't valid");

                let type_idents = (!parameters.is_empty()).then(|| {
                    let type_idents = parameters.iter().map(|type_id| {
                        type_space
                            .id_to_entry
                            .get(type_id)
                            .expect("unresolved type id for tuple")
                            .type_ident(type_space, type_mod)
                    });
                    quote! { < #(#type_idents,)* > }
                });

                quote! {
                    #path
                    #type_idents
                }
            }

            TypeEntryDetails::Unit => quote! { () },
            TypeEntryDetails::String => quote! { ::std::string::String },
            TypeEntryDetails::Boolean => quote! { bool },
            TypeEntryDetails::JsonValue => quote! { ::serde_json::Value },
            TypeEntryDetails::Integer(name) | TypeEntryDetails::Float(name) => {
                syn::parse_str::<syn::TypePath>(name)
                    .unwrap()
                    .to_token_stream()
            }

            TypeEntryDetails::Reference(_) => panic!("references should be resolved by now"),
        }
    }

    pub(crate) fn type_parameter_ident(
        &self,
        type_space: &TypeSpace,
        lifetime_name: Option<&str>,
    ) -> TokenStream {
        let lifetime = lifetime_name.map(|s| {
            vec![
                TokenTree::from(Punct::new('\'', Spacing::Joint)),
                TokenTree::from(format_ident!("{}", s)),
            ]
            .into_iter()
            .collect::<TokenStream>()
        });
        match &self.details {
            // We special-case enums for which all variants are simple to let
            // them be passed as values rather than as references.
            // TODO we should probably cache "simpleness" of all variants
            // rather than iterating every time. We'll know it when the enum is
            // constructed.
            TypeEntryDetails::Enum(TypeEntryEnum { variants, .. })
                if variants
                    .iter()
                    .all(|variant| matches!(&variant.details, VariantDetails::Simple)) =>
            {
                self.type_ident(type_space, &type_space.settings.type_mod)
            }
            TypeEntryDetails::Enum(_)
            | TypeEntryDetails::Struct(_)
            | TypeEntryDetails::Newtype(_)
            | TypeEntryDetails::Vec(_)
            | TypeEntryDetails::Map(..)
            | TypeEntryDetails::Set(_)
            | TypeEntryDetails::Box(_)
            | TypeEntryDetails::Native(_)
            | TypeEntryDetails::Array(..)
            | TypeEntryDetails::JsonValue => {
                let ident = self.type_ident(type_space, &type_space.settings.type_mod);
                quote! {
                    & #lifetime #ident
                }
            }

            TypeEntryDetails::Option(id) => {
                let inner_ty = type_space
                    .id_to_entry
                    .get(id)
                    .expect("unresolved type id for option");
                let inner_ident = inner_ty.type_parameter_ident(type_space, lifetime_name);

                // Flatten nested Option types. This would only happen if the
                // schema encoded it; it's an odd construction.
                match &inner_ty.details {
                    TypeEntryDetails::Option(_) => inner_ident,
                    _ => quote! { Option<#inner_ident> },
                }
            }

            TypeEntryDetails::Tuple(items) => {
                let type_streams = items.iter().map(|item| {
                    type_space
                        .id_to_entry
                        .get(item)
                        .expect("unresolved type id for tuple")
                        .type_parameter_ident(type_space, lifetime_name)
                });

                if items.len() != 1 {
                    quote! { ( #(#type_streams),* ) }
                } else {
                    // Single-element tuples require special handling. In
                    // particular, they must have a trailing comma or else are
                    // treated as extraneously parenthesized types.
                    quote! { ( #(#type_streams,)* ) }
                }
            }

            TypeEntryDetails::Unit
            | TypeEntryDetails::Boolean
            | TypeEntryDetails::Integer(_)
            | TypeEntryDetails::Float(_) => {
                self.type_ident(type_space, &type_space.settings.type_mod)
            }
            TypeEntryDetails::String => quote! { & #lifetime str },

            TypeEntryDetails::Reference(_) => panic!("references should be resolved by now"),
        }
    }

    pub(crate) fn describe(&self) -> String {
        match &self.details {
            TypeEntryDetails::Enum(TypeEntryEnum { name, .. }) => format!("enum {}", name),
            TypeEntryDetails::Struct(TypeEntryStruct { name, .. }) => format!("struct {}", name),
            TypeEntryDetails::Newtype(TypeEntryNewtype { name, type_id, .. }) => {
                format!("newtype {} {}", name, type_id.0)
            }

            TypeEntryDetails::Unit => "()".to_string(),
            TypeEntryDetails::Option(type_id) => format!("option {}", type_id.0),
            TypeEntryDetails::Vec(type_id) => format!("vec {}", type_id.0),
            TypeEntryDetails::Map(key_id, value_id) => {
                format!("map {} {}", key_id.0, value_id.0)
            }
            TypeEntryDetails::Set(type_id) => format!("set {}", type_id.0),
            TypeEntryDetails::Box(type_id) => format!("box {}", type_id.0),
            TypeEntryDetails::Tuple(type_ids) => {
                format!(
                    "tuple ({})",
                    type_ids
                        .iter()
                        .map(|type_id| type_id.0.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            TypeEntryDetails::Array(type_id, length) => {
                format!("array {}; {}", type_id.0, length)
            }
            TypeEntryDetails::Boolean => "bool".to_string(),
            TypeEntryDetails::Native(TypeEntryNative {
                type_name: name, ..
            })
            | TypeEntryDetails::Integer(name)
            | TypeEntryDetails::Float(name) => name.clone(),
            TypeEntryDetails::String => "string".to_string(),

            TypeEntryDetails::JsonValue => "json value".to_string(),

            TypeEntryDetails::Reference(_) => unreachable!(),
        }
    }
}

/// Decide whether to emit a `#[patch(name = "Option<{Inner}Patch>")]`
/// line above a struct field. Returns `Some(token_stream)` when the
/// configured deep-patch policy (the `with_deep_patch_filter` closure
/// if set, otherwise the [`DeepPatchPolicy`] bulk setting) accepts the
/// field, and the field's Rust type is exactly `Option<{InnerStruct}>`
/// (potentially with an interior `Box`). Returns `None` for every other
/// shape — including:
///
/// - `#[serde(flatten)]` base fields (composed via
///   `AllOfStrategy::Compose`), which are bare `T` rather than `Option<T>`
///   and would not type-check with a `#[patch(name = ...)]` rewrite anyway.
/// - `Vec<T>` and `Vec<Option<T>>`. `struct_patch` doesn't deep-merge
///   through vectors, so the rewrite would silently break compilation.
/// - `Option<T>` where `T` is a primitive, native, enum, or newtype.
///   Only generated `struct`s derive `Patch`, so there's no `{Inner}Patch`
///   target.
///
/// The closure form is consulted first; the bulk policy is only used
/// when no closure is registered.
fn deep_patch_attr(
    type_space: &TypeSpace,
    owner_name: &str,
    prop: &StructProperty,
) -> Option<TokenStream> {
    // `#[serde(flatten)]` fields are bare, not Option<T>, so skip.
    if matches!(prop.rename, StructPropertyRename::Flatten) {
        return None;
    }

    // The settings must have something to say about deep patches at all.
    let filter = type_space.settings.deep_patch_filter();
    let bulk = type_space.settings.deep_patches();
    if filter.is_none() && matches!(bulk, DeepPatchPolicy::Off) {
        return None;
    }

    // Drill: Option<{maybe Box}<{InnerStruct}>>.
    let outer = type_space.id_to_entry.get(&prop.type_id)?;
    let inner_id = match &outer.details {
        TypeEntryDetails::Option(id) => id,
        _ => return None,
    };
    let mut inner = type_space.id_to_entry.get(inner_id)?;
    if let TypeEntryDetails::Box(id) = &inner.details {
        inner = type_space.id_to_entry.get(id)?;
    }
    let inner_struct_name = match &inner.details {
        TypeEntryDetails::Struct(s) => s.name.as_str(),
        _ => return None,
    };

    // Ask the predicate first; fall back to the bulk policy.
    let allow = if let Some(filter) = filter {
        filter.accepts(owner_name, &prop.name, inner_struct_name)
    } else {
        matches!(bulk, DeepPatchPolicy::AllOptionStructs)
    };
    if !allow {
        return None;
    }

    let patch_name = format!("Option<{}Patch>", inner_struct_name);
    Some(quote! { #[patch(name = #patch_name)] })
}

/// Build the doc comment that decorates each generated type.
///
/// Always emits the human-readable description (or, if absent, the type's
/// own name in backticks), followed — unless
/// [`crate::TypeSpaceSettings::with_schema_in_docs`] is disabled on the
/// owning [`TypeSpace`] — by the upstream `<details><summary>JSON
/// schema</summary>` block containing the full pretty-printed schema.
fn make_doc(
    type_space: &TypeSpace,
    name: &str,
    description: Option<&String>,
    schema: &Schema,
) -> TokenStream {
    let desc = match description {
        Some(desc) => desc,
        None => &format!("`{}`", name),
    };

    if !type_space.include_schema_in_docs() {
        return quote! {
            #[doc = #desc]
        };
    }

    let schema_json = serde_json::to_string_pretty(schema).unwrap();
    let schema_lines = schema_json.lines();
    quote! {
        #[doc = #desc]
        ///
        /// <details><summary>JSON schema</summary>
        ///
        /// ```json
        #(
            #[doc = #schema_lines]
        )*
        /// ```
        /// </details>
    }
}

/// Build the ordered list of derive paths emitted in `#[derive(...)]` for a
/// type of the given `kind`.
///
/// When at least one [`crate::TypeSpaceSettings::with_unconditional_derive`]
/// has been registered for `kind`, the list is taken verbatim from those
/// entries (insertion order). This lets callers control the exact ordering
/// of the derive list and is the only mechanism for emitting a non-sorted
/// derive list.
///
/// Otherwise the historical behavior applies: the lexicographically sorted
/// base set is merged with `with_derive` extras and per-type derive
/// patches.
fn compute_derive_list(
    type_space: &TypeSpace,
    kind: crate::TypeKind,
    base_derive_set: BTreeSet<&str>,
    type_derives: &BTreeSet<String>,
    extra_derives: &[String],
) -> Vec<TokenStream> {
    compute_derive_list_denying(
        type_space,
        kind,
        base_derive_set,
        type_derives,
        extra_derives,
        &[],
    )
}

/// [`compute_derive_list`] with an additional deny-list of derives that a
/// bespoke impl replaces (e.g. `Deserialize` on constrained newtypes).
/// Denied entries are excluded from every source, including
/// caller-supplied unconditional derives, so the generated code never
/// contains both a derive and a conflicting manual impl.
fn compute_derive_list_denying(
    type_space: &TypeSpace,
    kind: crate::TypeKind,
    base_derive_set: BTreeSet<&str>,
    type_derives: &BTreeSet<String>,
    extra_derives: &[String],
    denied_derives: &[&str],
) -> Vec<TokenStream> {
    let unconditional: Vec<&str> = type_space
        .settings
        .unconditional_derives
        .iter()
        .filter(|u| u.kinds.matches(kind))
        .map(|u| u.derive.as_str())
        .collect();

    let derive_strs: Vec<&str> = if unconditional.is_empty() {
        // Legacy path: BTreeSet-sorted union of base set, type-specific
        // derives, and `with_derive` extras.
        let mut combined: BTreeSet<&str> = base_derive_set;
        combined.extend(extra_derives.iter().map(String::as_str));
        combined.extend(type_derives.iter().map(String::as_str));
        combined.into_iter().collect()
    } else {
        // Caller-driven path: respect insertion order verbatim and then
        // append any type-local patches / `with_derive` extras (these are
        // sorted to keep additions deterministic).
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut out: Vec<&str> = Vec::with_capacity(unconditional.len());
        for derive in &unconditional {
            if seen.insert(derive) {
                out.push(derive);
            }
        }
        let mut extras: BTreeSet<&str> = BTreeSet::new();
        extras.extend(extra_derives.iter().map(String::as_str));
        extras.extend(type_derives.iter().map(String::as_str));
        for derive in extras {
            if seen.insert(derive) {
                out.push(derive);
            }
        }
        out
    };

    derive_strs
        .into_iter()
        .filter(|derive| !denied_derives.contains(derive))
        .map(|derive| {
            syn::parse_str::<syn::Path>(derive)
                .unwrap_or_else(|e| panic!("invalid derive path {:?}: {}", derive, e))
                .into_token_stream()
        })
        .collect()
}

/// The conditional / unconditional attribute and derive lines emitted
/// around a generated type's main `#[derive(...)]`, grouped by position.
/// Built once per type by [`type_attr_lists`].
#[derive(Default)]
struct TypeAttrLists {
    /// Unconditional attributes placed before the derive.
    uncond_pre: Vec<TokenStream>,
    /// Unconditional attributes placed after the derive (and after any
    /// type-level `#[serde(...)]` line).
    uncond_post: Vec<TokenStream>,
    /// `#[cfg_attr(feature = ..., <attr>)]` lines placed before the derive.
    cond_pre: Vec<TokenStream>,
    /// `#[cfg_attr(feature = ..., <attr>)]` lines placed after the derive.
    cond_post: Vec<TokenStream>,
    /// `#[cfg_attr(feature = ..., derive(...))]` lines; always emitted
    /// before the main derive (a cfg-gated derive after the main derive
    /// would be a duplicate, not a transformation).
    cond_derives: Vec<TokenStream>,
}

/// Collect every conditional / unconditional attribute and derive
/// configured on the [`TypeSpace`] that applies to the given type `kind`,
/// preserving insertion order within each list.
fn type_attr_lists(type_space: &TypeSpace, kind: crate::TypeKind) -> TypeAttrLists {
    let mut lists = TypeAttrLists::default();

    for entry in &type_space.settings.unconditional_attrs {
        if !entry.kinds.matches(kind) {
            continue;
        }
        let attr: TokenStream = entry
            .attr
            .parse()
            .expect("unconditional attribute must parse as a token stream");
        let line = quote! { #[ #attr ] };
        match entry.position {
            crate::AttrPosition::BeforeDerive => lists.uncond_pre.push(line),
            crate::AttrPosition::AfterDerive => lists.uncond_post.push(line),
        }
    }

    for entry in &type_space.settings.conditional_attrs {
        if !entry.kinds.matches(kind) {
            continue;
        }
        let cfg = entry.cfg.as_str();
        let attr: TokenStream = entry
            .attr
            .parse()
            .expect("conditional attribute must parse as a token stream");
        let line = quote! { #[cfg_attr(feature = #cfg, #attr)] };
        match entry.position {
            crate::AttrPosition::BeforeDerive => lists.cond_pre.push(line),
            crate::AttrPosition::AfterDerive => lists.cond_post.push(line),
        }
    }

    for entry in &type_space.settings.conditional_derives {
        if !entry.kinds.matches(kind) {
            continue;
        }
        let cfg = entry.cfg.as_str();
        let derive = syn::parse_str::<syn::Path>(&entry.derive)
            .expect("conditional derive must be a valid Rust path");
        lists
            .cond_derives
            .push(quote! { #[cfg_attr(feature = #cfg, derive(#derive))] });
    }

    lists
}

fn strings_to_attrs<'a>(
    type_attrs: &'a BTreeSet<String>,
    extra_attrs: &'a [String],
) -> impl Iterator<Item = TokenStream> + 'a {
    let mut combined_attrs = BTreeSet::new();
    combined_attrs.extend(extra_attrs.iter().map(String::as_str));
    combined_attrs.extend(type_attrs.iter().map(String::as_str));
    combined_attrs
        .into_iter()
        .map(|attr| attr.parse::<TokenStream>().unwrap())
}

/// Returns true iff...
/// - the enum is untagged
/// - all variants are single items (aka newtype variants)
/// - the type of the newtype variant implements the required trait
fn untagged_newtype_variants(
    type_space: &TypeSpace,
    tag_type: &EnumTagType,
    variants: &[Variant],
    req_impl: TypeSpaceImpl,
    neg_impl: Option<TypeSpaceImpl>,
) -> bool {
    tag_type == &EnumTagType::Untagged
        && variants.iter().all(|variant| {
            // If the variant is a single item...
            match &variant.details {
                VariantDetails::Item(type_id) => Some(type_id),
                _ => None,
            }
            .map_or_else(
                || false,
                |type_id| {
                    let type_entry = type_space.id_to_entry.get(type_id).unwrap();
                    // ... and its type has the required impl
                    type_entry.has_impl(type_space, req_impl)
                        && neg_impl
                            .is_none_or(|neg_impl| !type_entry.has_impl(type_space, neg_impl))
                },
            )
        })
}

/// Returns true iff...
/// - the enum is untagged
/// - **any** variant is a single items **and** it is irrefutably a string
fn untagged_newtype_string(
    type_space: &TypeSpace,
    tag_type: &EnumTagType,
    variants: &[Variant],
) -> bool {
    tag_type == &EnumTagType::Untagged
        && variants.iter().any(|variant| {
            // If the variant is a single item...
            match &variant.details {
                VariantDetails::Item(type_id) => Some(type_id),
                _ => None,
            }
            .map_or_else(
                || false,
                |type_id| {
                    let type_entry = type_space.id_to_entry.get(type_id).unwrap();
                    // ... and it is irrefutably a string
                    type_entry.has_impl(type_space, TypeSpaceImpl::FromStringIrrefutable)
                },
            )
        })
}

#[cfg(test)]
mod tests {
    use crate::{
        type_entry::{SchemaWrapper, TypeEntry, TypeEntryStruct},
        TypeEntryDetails, TypeSpace,
    };

    #[test]
    fn test_ident() {
        let ts = TypeSpace::default();

        let type_mod = Some("the_mod".to_string());

        let t = TypeEntry::new_integer("u32");
        let ident = t.type_ident(&ts, &type_mod);
        assert_eq!(ident.to_string(), "u32");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "u32");

        let t = TypeEntry::from(TypeEntryDetails::String);
        let ident = t.type_ident(&ts, &type_mod);
        assert_eq!(ident.to_string(), ":: std :: string :: String");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "& str");
        let parameter = t.type_parameter_ident(&ts, Some("static"));
        assert_eq!(parameter.to_string(), "& 'static str");

        let t = TypeEntry::from(TypeEntryDetails::Unit);
        let ident = t.type_ident(&ts, &type_mod);
        assert_eq!(ident.to_string(), "()");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "()");

        let t = TypeEntry::from(TypeEntryDetails::Struct(TypeEntryStruct {
            name: "SomeType".to_string(),
            rename: None,
            description: None,
            default: None,
            properties: vec![],
            deny_unknown_fields: false,
            schema: SchemaWrapper(schemars::schema::Schema::Bool(false)),
        }));

        let ident = t.type_ident(&ts, &type_mod);
        assert_eq!(ident.to_string(), "the_mod :: SomeType");
        let parameter = t.type_parameter_ident(&ts, None);
        assert_eq!(parameter.to_string(), "& SomeType");
        let parameter = t.type_parameter_ident(&ts, Some("a"));
        assert_eq!(parameter.to_string(), "& 'a SomeType");
    }
}
