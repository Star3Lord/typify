// Copyright 2026 Oxide Computer Company

//! Acceptance test for the ergonomic generation settings, reproducing an
//! API-client house style end to end: an OpenAPI document generated with
//! the wire-shape settings (subset conversions, explicit optional
//! properties, composed allOf, open string enums, description-only docs),
//! followed by a small `syn` post-pass supplying the purely decorative
//! layer (struct-level serde attributes, feature-gated derives, per-field
//! attribute elision).
//!
//! The post-pass is deliberately included here as the executable recipe
//! for decoration concerns: they operate on visible syntax and belong in
//! a consumer's post-processing, not in typify's conversion machinery.

use expectorate::assert_contents;
use quote::ToTokens;
use serde_json::json;
use syn::visit_mut::VisitMut;
use typify_impl::{AllOfStrategy, OptionalProperties, TypeSpace, TypeSpaceImpl, TypeSpaceSettings};

/// The decoration layer: everything here is additive or wire-neutral.
struct Decorate {
    /// Names of the types eligible for a `Default` derive, per
    /// [`typify_impl::Type::default_derivable`]: a house style that puts
    /// `Default` in its derive list must consult typify's type knowledge or
    /// it will emit derives that cannot compile (e.g. on structs with
    /// required non-zero integer fields).
    default_derivable: std::collections::BTreeSet<String>,
}

impl VisitMut for Decorate {
    fn visit_item_struct_mut(&mut self, item: &mut syn::ItemStruct) {
        // `#[serde_with::skip_serializing_none]` must precede the derive.
        let derive_index = item
            .attrs
            .iter()
            .position(|attr| attr.path().is_ident("derive"))
            .unwrap_or(item.attrs.len());
        item.attrs.insert(
            derive_index,
            syn::parse_quote! { #[serde_with::skip_serializing_none] },
        );
        if self.default_derivable.contains(&item.ident.to_string()) {
            for attr in &mut item.attrs {
                if attr.path().is_ident("derive") {
                    let derives = attr.parse_args_with(
                        syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                    );
                    if let Ok(mut derives) = derives {
                        derives.push(syn::parse_quote! { Default });
                        *attr = syn::parse_quote! { #[derive( #derives )] };
                    }
                }
            }
        }
        item.attrs.push(
            syn::parse_quote! { #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))] },
        );
        syn::visit_mut::visit_item_struct_mut(self, item);
    }

    fn visit_item_enum_mut(&mut self, item: &mut syn::ItemEnum) {
        item.attrs.push(
            syn::parse_quote! { #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))] },
        );
        syn::visit_mut::visit_item_enum_mut(self, item);
    }

    fn visit_field_mut(&mut self, field: &mut syn::Field) {
        // With `skip_serializing_none` at the struct level, the per-field
        // `default` + `skip_serializing_if = "..is_none"` pair it subsumes
        // is noise; eliding it is wire-neutral. Other serde options (such
        // as `rename`) are preserved.
        for attr in &mut field.attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            let Ok(options) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                continue;
            };
            let is_none_skip = |meta: &syn::Meta| match meta {
                syn::Meta::NameValue(syn::MetaNameValue {
                    path,
                    value:
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(value),
                            ..
                        }),
                    ..
                }) => {
                    path.is_ident("skip_serializing_if")
                        && value.value() == "::std::option::Option::is_none"
                }
                _ => false,
            };
            if !options.iter().any(is_none_skip) {
                continue;
            }
            let retained = options
                .into_iter()
                .filter(|meta| !is_none_skip(meta) && !meta.path().is_ident("default"))
                .collect::<Vec<_>>();
            *attr = syn::parse_quote! { #[serde( #( #retained ),* )] };
        }
        // Attributes left empty disappear entirely.
        let empty: syn::Attribute = syn::parse_quote! { #[serde()] };
        field.attrs.retain(|attr| {
            attr.to_token_stream().to_string() != empty.to_token_stream().to_string()
        });
        syn::visit_mut::visit_field_mut(self, field);
    }
}

#[test]
fn test_api_client_style() {
    let document = json!({
        "openapi": "3.0.3",
        "info": { "title": "Booking", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "GenericAgency": {
                    "type": "object",
                    "properties": {
                        "agencyId": {
                            "type": "string",
                            "format": "uuid"
                        },
                        "name": {
                            "type": "string"
                        }
                    },
                    "required": [ "agencyId" ]
                },
                "Agency": {
                    "description": "An agency extending the generic base",
                    "type": "object",
                    "properties": {
                        "ticketingPolicy": {
                            "type": "string"
                        },
                        "flights": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/FlightReference"
                            }
                        },
                        "cancelAll": {
                            "type": "boolean",
                            "default": false
                        },
                        "status": {
                            "$ref": "#/components/schemas/AgencyStatus"
                        }
                    },
                    "allOf": [
                        {
                            "$ref": "#/components/schemas/GenericAgency"
                        }
                    ]
                },
                "FlightReference": {
                    "type": "object",
                    "properties": {
                        "flightNumber": {
                            "type": "string"
                        },
                        "operatingCarrier": {
                            "type": "string",
                            "nullable": true
                        }
                    },
                    "required": [ "flightNumber" ]
                },
                "Ticket": {
                    "type": "object",
                    "properties": {
                        "couponNumber": {
                            "type": "integer",
                            "format": "uint64",
                            "minimum": 1
                        }
                    },
                    "required": [ "couponNumber" ]
                },
                "AgencyStatus": {
                    "type": "string",
                    "enum": [ "active", "suspended" ]
                }
            }
        }
    });

    let uuid_schema = serde_json::from_value(json!({
        "type": "string",
        "format": "uuid",
    }))
    .unwrap();

    let mut type_space = TypeSpace::new(
        TypeSpaceSettings::default()
            .with_conversion(
                uuid_schema,
                "::uuid::Uuid",
                [TypeSpaceImpl::Display, TypeSpaceImpl::FromStr].into_iter(),
            )
            .with_optional_properties(OptionalProperties::Explicit)
            .with_all_of_strategy(AllOfStrategy::Compose)
            .with_open_enum_variant("Other")
            .with_schema_in_docs(false),
    );
    type_space.add_openapi_document(&document).unwrap();

    // The house style adds Default to struct derive lists; consult typify
    // for the types that can satisfy it.
    let default_derivable = type_space
        .iter_types()
        .filter(|ty| ty.default_derivable())
        .map(|ty| ty.name())
        .collect();

    let mut file = syn::parse2::<syn::File>(type_space.to_stream()).unwrap();
    Decorate { default_derivable }.visit_file_mut(&mut file);
    let actual = rustfmt_wrapper::rustfmt(file.to_token_stream().to_string()).unwrap();

    assert_contents("tests/api-client.out", &actual);

    // The signatures of each mechanism:
    // subset conversion despite extra keywords (A)
    assert!(actual.contains("pub agency_id: ::uuid::Uuid"));
    // explicit optional properties (B)
    assert!(actual.contains("pub flights: ::std::option::Option<::std::vec::Vec<FlightReference>>"));
    assert!(actual.contains("pub cancel_all: ::std::option::Option<bool>"));
    // composed inheritance (C)
    assert!(actual.contains("#[serde(flatten)]"));
    assert!(actual.contains("pub generic_agency: GenericAgency"));
    // open string enum (D)
    assert!(actual.contains("Other(::std::string::String)"));
    // description-only docs (E)
    assert!(actual.contains("An agency extending the generic base"));
    assert!(!actual.contains("JSON schema"));
    // decoration post-pass
    assert!(actual.contains("#[serde_with::skip_serializing_none]"));
    assert!(actual.contains("#[cfg_attr(feature = \"schemars\", derive(schemars::JsonSchema))]"));
    assert!(!actual.contains("skip_serializing_if"));
    // Default lands on the struct whose fields can satisfy it--and not on
    // the one with a required non-zero field, nor on those containing the
    // uuid (for which no Default impl was claimed).
    let derives_default = |type_name: &str| {
        let definition = actual
            .split(&format!("pub struct {}", type_name))
            .next()
            .unwrap();
        definition
            .rsplit("#[derive(")
            .next()
            .unwrap()
            .split(")]")
            .next()
            .unwrap()
            .contains("Default")
    };
    assert!(derives_default("FlightReference"));
    assert!(!derives_default("Ticket"));
    assert!(!derives_default("Agency"));
}
