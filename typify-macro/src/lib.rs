// Copyright 2025 Oxide Computer Company

//! typify macro implementation.

#![deny(missing_docs)]

use std::{collections::HashMap, path::Path};

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use serde::Deserialize;
use serde_tokenstream::{ParseWrapper, TokenStreamWrapper};
use syn::LitStr;
use token_utils::TypeAndImpls;
use typify_impl::{
    AllOfStrategy, ArrayOptionality, CrateVers, DeepPatchPolicy, DefaultBoolOptionality,
    DefaultedFieldOptionality, MapType, TypeKindFilter, TypeSpace, TypeSpacePatch,
    TypeSpaceSettings, UnknownPolicy,
};

mod token_utils;

/// Import types from a schema file. This may be invoked with simply a pathname
/// for a JSON Schema file (relative to `$CARGO_MANIFEST_DIR`), or it may be
/// invoked with a structured form:
/// ```
/// use typify_macro::import_types;
/// import_types!(
///     schema = "../example.json",
///     derives = [schemars::JsonSchema],
/// );
/// ```
///
/// - `schema`: string literal; the JSON schema file
///
/// - `derives`: optional array of derive macro paths; the derive macros to be
///   applied to all generated types
///
/// - `attrs`: optional array of attribute paths; the attributes to be applied
///   to all generated types
///
/// - `struct_builder`: optional boolean; (if true) generates a `::builder()`
///   method for each generated struct that can be used to specify each
///   property and construct the struct
///
/// - `unknown_crates`: optional policy regarding the handling of schemas that
///   contain the `x-rust-type` extension whose crates are not explicitly named
///   in the `crates` section. The options are `Generate` to ignore the
///   extension and generate a *de novo* type, `Allow` to use the named type
///   (which may require the addition of a new dependency to compile, and which
///   ignores version compatibility checks), or `Deny` to produce a
///   compile-time error (requiring the user to specify the crate's disposition
///   in the `crates` section).
///
/// - `crates`: optional map from crate name to the version of the crate in
///   use. Types encountered with the Rust type extension (`x-rust-type`) will
///   use types from the specified crates rather than generating them (within
///   the constraints of type compatibility).
///
/// - `patch`: optional map from type to an object with the optional members
///   `rename` and `derives`. This may be used to renamed generated types or
///   to apply additional (non-default) derive macros to them.
///
/// - `replace`: optional map from definition name to a replacement type. This
///   may be used to skip generation of the named type and use a existing Rust
///   type.
///
/// - `convert`: optional map from a JSON schema type defined in `$defs` to a
///   replacement type. This may be used to skip generation of the schema and
///   use an existing Rust type.
#[proc_macro]
pub fn import_types(item: TokenStream) -> TokenStream {
    match do_import_types(item) {
        Err(err) => err.to_compile_error().into(),
        Ok(out) => out,
    }
}

#[derive(Deserialize)]
struct MacroSettings {
    schema: ParseWrapper<LitStr>,
    #[serde(default)]
    derives: Vec<ParseWrapper<syn::Path>>,
    #[serde(default)]
    attrs: Vec<TokenStreamWrapper>,
    #[serde(default)]
    struct_builder: bool,

    #[serde(default)]
    unknown_crates: UnknownPolicy,
    #[serde(default)]
    crates: HashMap<CrateName, MacroCrateSpec>,
    #[serde(default)]
    map_type: Option<ParseWrapper<syn::Type>>,

    #[serde(default)]
    patch: HashMap<ParseWrapper<syn::Ident>, MacroPatch>,
    #[serde(default)]
    replace: HashMap<ParseWrapper<syn::Ident>, ParseWrapper<TypeAndImpls>>,
    #[serde(default)]
    convert:
        serde_tokenstream::OrderedMap<schemars::schema::SchemaObject, ParseWrapper<TypeAndImpls>>,

    /// Override the Rust type used for `format: date` strings. See
    /// [`typify_impl::TypeSpaceSettings::with_date_type`].
    #[serde(default)]
    date_type: Option<ParseWrapper<syn::Path>>,
    /// Override the Rust type used for `format: date-time` strings. See
    /// [`typify_impl::TypeSpaceSettings::with_date_time_type`].
    #[serde(default)]
    date_time_type: Option<ParseWrapper<syn::Type>>,
    /// Override the Rust type used for `format: uuid` strings. See
    /// [`typify_impl::TypeSpaceSettings::with_uuid_type`].
    #[serde(default)]
    uuid_type: Option<ParseWrapper<syn::Path>>,
    /// Map `"<instance-type>/<format>"` keys (e.g. `"string/decimal"`,
    /// `"integer/int64"`) to Rust types. See
    /// [`typify_impl::TypeSpaceSettings::with_format_type`].
    #[serde(default)]
    format_types: HashMap<String, ParseWrapper<syn::Type>>,
    /// Emit plain `String` for constrained strings. See
    /// [`typify_impl::TypeSpaceSettings::with_unconstrained_string`].
    #[serde(default)]
    unconstrained_string: bool,
    /// Emit plain integers instead of `NonZero*` types. See
    /// [`typify_impl::TypeSpaceSettings::with_unconstrained_int`].
    #[serde(default)]
    unconstrained_int: bool,
    /// Wrap non-required arrays in `Option<Vec<T>>`. See
    /// [`typify_impl::TypeSpaceSettings::with_array_optionality`].
    #[serde(default)]
    array_optionality: ArrayOptionality,
    /// Wrap defaulted bools in `Option<bool>`. See
    /// [`typify_impl::TypeSpaceSettings::with_default_bool_optionality`].
    #[serde(default)]
    default_bool_optionality: DefaultBoolOptionality,
    /// Controls whether non-required, non-bool struct fields with a
    /// schema-level `default:` are emitted as bare `T` (the typify
    /// historical shape) or wrapped in `Option<T>`. See
    /// [`typify_impl::TypeSpaceSettings::with_defaulted_field_optionality`].
    #[serde(default)]
    defaulted_field_optionality: DefaultedFieldOptionality,
    /// Drop the per-field `#[serde(default, skip_serializing_if = ...)]`
    /// pair on `Option<T>` fields. Pair with a struct-level
    /// `#[serde_with::skip_serializing_none]` (typically added via `attrs`
    /// or `conditional_attrs`) so the omission semantics are preserved. See
    /// [`typify_impl::TypeSpaceSettings::with_elide_option_field_defaults`].
    #[serde(default)]
    elide_option_field_defaults: bool,
    /// Bulk policy for `#[patch(name = "Option<{Inner}Patch>")]` emission
    /// on `Option<{InnerStruct}>` fields. See
    /// [`typify_impl::TypeSpaceSettings::with_deep_patches`]. The
    /// closure-based predicate (`with_deep_patch_filter`) is library-only
    /// and intentionally not exposed via this macro.
    #[serde(default)]
    deep_patches: DeepPatchPolicy,
    /// Render `allOf` compositions with `#[serde(flatten)]` fields. See
    /// [`typify_impl::TypeSpaceSettings::with_allof_strategy`].
    #[serde(default)]
    allof_strategy: AllOfStrategy,
    /// Embed the full pretty-printed JSON Schema in each generated type's
    /// doc comment (the upstream `<details>` block). Defaults to `true`,
    /// matching upstream; set `false` to keep IDE hovers minimal. See
    /// [`typify_impl::TypeSpaceSettings::with_schema_in_docs`].
    #[serde(default)]
    include_schema_in_docs: Option<bool>,
    /// Add `AsRef<str>` / `Display` (and `From<&str>` for unconstrained
    /// ones) to string-wrapping newtypes. See
    /// [`typify_impl::TypeSpaceSettings::with_string_newtype_conveniences`].
    #[serde(default)]
    string_newtype_conveniences: bool,
    /// Cfg-gated derives applied per type kind. See
    /// [`typify_impl::TypeSpaceSettings::with_conditional_derive_for`].
    #[serde(default)]
    conditional_derives: Vec<MacroConditional>,
    /// Cfg-gated attributes applied per type kind. See
    /// [`typify_impl::TypeSpaceSettings::with_conditional_attr_for`].
    #[serde(default)]
    conditional_attrs: Vec<MacroConditional>,
}

#[derive(Deserialize)]
struct MacroConditional {
    feature: String,
    body: TokenStreamWrapper,
    /// Optional scope. One of `"structs"`, `"enums"`, `"newtypes"`, or
    /// `"all"` (default). Controls which generated type categories the
    /// conditional derive / attribute is applied to.
    #[serde(default)]
    kinds: Option<String>,
}

impl MacroConditional {
    fn kinds_filter(&self) -> TypeKindFilter {
        match self.kinds.as_deref() {
            None | Some("all") => TypeKindFilter::ALL,
            Some("structs") => TypeKindFilter::STRUCTS,
            Some("enums") => TypeKindFilter::ENUMS,
            Some("newtypes") => TypeKindFilter::NEWTYPES,
            Some(other) => {
                panic!(
                    "unknown kinds filter `{}`: expected one of `all`, \
                     `structs`, `enums`, `newtypes`",
                    other
                )
            }
        }
    }
}

struct MacroCrateSpec {
    original: Option<String>,
    version: CrateVers,
}

impl<'de> Deserialize<'de> for MacroCrateSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ss = String::deserialize(deserializer)?;

        let (original, vers_str) = if let Some(ii) = ss.find('@') {
            let original_str = &ss[..ii];
            let rest = &ss[ii + 1..];
            if !is_crate(original_str) {
                return Err(<D::Error as serde::de::Error>::invalid_value(
                    serde::de::Unexpected::Str(&ss),
                    &"valid crate name",
                ));
            }

            (Some(original_str.to_string()), rest)
        } else {
            (None, ss.as_ref())
        };

        let Some(version) = CrateVers::parse(vers_str) else {
            return Err(<D::Error as serde::de::Error>::invalid_value(
                serde::de::Unexpected::Str(&ss),
                &"valid version",
            ));
        };

        Ok(Self { original, version })
    }
}

#[derive(Hash, PartialEq, Eq)]
struct CrateName(String);
impl<'de> Deserialize<'de> for CrateName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ss = String::deserialize(deserializer)?;

        if is_crate(&ss) {
            Ok(Self(ss))
        } else {
            Err(<D::Error as serde::de::Error>::invalid_value(
                serde::de::Unexpected::Str(&ss),
                &"valid crate name",
            ))
        }
    }
}

fn is_crate(s: &str) -> bool {
    s.starts_with(|cc: char| cc.is_alphabetic() || cc == '_')
        && !s.contains(|cc: char| !cc.is_alphanumeric() && cc != '_' && cc != '-')
}

#[derive(Deserialize)]
struct MacroPatch {
    #[serde(default)]
    rename: Option<String>,
    #[serde(default)]
    derives: Vec<ParseWrapper<syn::Path>>,
    #[serde(default)]
    attrs: Vec<TokenStreamWrapper>,
}

impl From<MacroPatch> for TypeSpacePatch {
    fn from(a: MacroPatch) -> Self {
        let mut s = Self::default();
        a.rename.iter().for_each(|rename| {
            s.with_rename(rename);
        });
        a.derives.iter().for_each(|derive| {
            s.with_derive(derive.to_token_stream());
        });
        a.attrs.iter().for_each(|attr| {
            s.with_attr(attr.to_token_stream());
        });
        s
    }
}

fn do_import_types(item: TokenStream) -> Result<TokenStream, syn::Error> {
    // Allow the caller to give us either a simple string or a compound object.
    let (schema, settings) = if let Ok(ll) = syn::parse::<LitStr>(item.clone()) {
        (ll, TypeSpaceSettings::default())
    } else {
        let MacroSettings {
            schema,
            derives,
            replace,
            patch,
            struct_builder,
            convert,
            unknown_crates,
            crates,
            map_type,
            attrs,
            date_type,
            date_time_type,
            uuid_type,
            format_types,
            unconstrained_string,
            unconstrained_int,
            array_optionality,
            default_bool_optionality,
            defaulted_field_optionality,
            elide_option_field_defaults,
            deep_patches,
            allof_strategy,
            include_schema_in_docs,
            string_newtype_conveniences,
            conditional_derives,
            conditional_attrs,
        } = serde_tokenstream::from_tokenstream(&item.into())?;
        let mut settings = TypeSpaceSettings::default();
        derives.into_iter().for_each(|derive| {
            settings.with_derive(derive.to_token_stream().to_string());
        });
        attrs.into_iter().for_each(|attr| {
            settings.with_attr(attr.to_token_stream().to_string());
        });
        settings.with_struct_builder(struct_builder);

        patch.into_iter().for_each(|(type_name, patch)| {
            settings.with_patch(type_name.to_token_stream(), &patch.into());
        });
        replace.into_iter().for_each(|(type_name, type_and_impls)| {
            let (replace_type, impls) = type_and_impls.into_inner().into_name_and_impls();
            settings.with_replacement(type_name.to_token_stream(), replace_type, impls.into_iter());
        });
        convert.into_iter().for_each(|(schema, type_and_impls)| {
            let (type_name, impls) = type_and_impls.into_inner().into_name_and_impls();
            settings.with_conversion(schema, type_name, impls);
        });

        crates.into_iter().for_each(
            |(CrateName(crate_name), MacroCrateSpec { original, version })| {
                if let Some(original_crate) = original {
                    settings.with_crate(original_crate, version, Some(&crate_name));
                } else {
                    settings.with_crate(crate_name, version, None);
                }
            },
        );
        settings.with_unknown_crates(unknown_crates);

        if let Some(map_type) = map_type {
            settings.with_map_type(MapType(map_type.into_inner()));
        }

        if let Some(date_type) = date_type {
            settings.with_date_type(date_type.to_token_stream().to_string());
        }
        if let Some(date_time_type) = date_time_type {
            settings.with_date_time_type(date_time_type.to_token_stream().to_string());
        }
        if let Some(uuid_type) = uuid_type {
            settings.with_uuid_type(uuid_type.to_token_stream().to_string());
        }
        for (key, rust_type) in format_types {
            let Some((instance_type, format)) = key.split_once('/') else {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "format_types key {key:?} must be \
                         \"<instance-type>/<format>\", e.g. \"string/date-time\"",
                    ),
                ));
            };
            settings.with_format_type(
                instance_type,
                format,
                rust_type.to_token_stream().to_string(),
            );
        }
        settings.with_unconstrained_string(unconstrained_string);
        settings.with_unconstrained_int(unconstrained_int);
        settings.with_array_optionality(array_optionality);
        settings.with_default_bool_optionality(default_bool_optionality);
        settings.with_defaulted_field_optionality(defaulted_field_optionality);
        settings.with_elide_option_field_defaults(elide_option_field_defaults);
        settings.with_deep_patches(deep_patches);
        settings.with_allof_strategy(allof_strategy);
        if let Some(include_schema_in_docs) = include_schema_in_docs {
            settings.with_schema_in_docs(include_schema_in_docs);
        }
        settings.with_string_newtype_conveniences(string_newtype_conveniences);
        for conditional in conditional_derives {
            let kinds = conditional.kinds_filter();
            settings.with_conditional_derive_for(
                &conditional.feature,
                conditional.body.to_token_stream().to_string(),
                kinds,
            );
        }
        for conditional in conditional_attrs {
            let kinds = conditional.kinds_filter();
            settings.with_conditional_attr_for(
                &conditional.feature,
                conditional.body.to_token_stream().to_string(),
                kinds,
            );
        }

        (schema.into_inner(), settings)
    };

    let dir = std::env::var("CARGO_MANIFEST_DIR").map_or_else(
        |_| std::env::current_dir().unwrap(),
        |s| Path::new(&s).to_path_buf(),
    );

    let path = dir.join(schema.value());

    let root_schema: schemars::schema::RootSchema =
        serde_json::from_reader(std::fs::File::open(&path).map_err(|e| {
            syn::Error::new(
                schema.span(),
                format!("couldn't read file {}: {}", schema.value(), e),
            )
        })?)
        .unwrap();

    let mut type_space = TypeSpace::new(&settings);
    type_space
        .add_root_schema(root_schema)
        .map_err(|e| into_syn_err(e, schema.span()))?;

    let path_str = path.to_string_lossy();
    let output = quote! {
        #type_space

        // Force a rebuild when the given file is modified.
        const _: &str = include_str!(#path_str);
    };

    Ok(output.into())
}

fn into_syn_err(e: typify_impl::Error, span: proc_macro2::Span) -> syn::Error {
    syn::Error::new(span, e.to_string())
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::MacroSettings;

    #[test]
    fn test_settings() {
        let item = quote! {
            schema = "foo.json",
            derives = [::foo::Foo, ::bar::Bar],
            replace = {
                Baz = ::baz::Baz,
            },
            struct_builder = true,
            map_type = ::my::map::Type,
        };

        let MacroSettings { .. } = serde_tokenstream::from_tokenstream(&item).unwrap();
    }
}
