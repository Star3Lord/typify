// Copyright 2025 Oxide Computer Company

use std::{error::Error, fs::File, io::BufReader};

use expectorate::assert_contents;
use glob::glob;
use quote::quote;
use schemars::schema::RootSchema;
use serde_json::json;
use typify::{AllOfStrategy, OptionalProperties, TypeSpace, TypeSpacePatch, TypeSpaceSettings};
use typify_impl::TypeSpaceImpl;

#[test]
fn test_schemas() {
    env_logger::init();
    // Make sure output is up to date.
    for entry in glob("tests/schemas/*.json").expect("Failed to read glob pattern") {
        let entry = entry.unwrap();
        let out_path = entry.clone().with_extension("rs");
        validate_schema(entry, out_path, &mut TypeSpaceSettings::default()).unwrap();
    }

    // Make sure it all compiles.
    trybuild::TestCases::new().pass("tests/schemas/*.rs");
}

/// Ensure that setting the global config to use a custom map type works.
#[test]
fn test_custom_map() {
    validate_schema(
        "tests/schemas/maps.json".into(),
        "tests/schemas/maps_custom.rs".into(),
        TypeSpaceSettings::default().with_map_type("std::collections::BTreeMap"),
    )
    .unwrap();

    trybuild::TestCases::new().pass("tests/schemas/maps_custom.rs");
}

/// Ensure that the `Compose` allOf strategy embeds referenced base types as
/// flattened members, and that constructions that don't fit the
/// single-inheritance idiom fall back to merging.
#[test]
fn test_all_of_compose() {
    validate_schema(
        "tests/schemas/composition.json".into(),
        "tests/schemas/composition-compose.rs".into(),
        TypeSpaceSettings::default().with_all_of_strategy(AllOfStrategy::Compose),
    )
    .unwrap();

    trybuild::TestCases::new().pass("tests/schemas/composition-compose.rs");
}

/// Ensure that the `Explicit` optional-properties policy represents every
/// non-required property as an `Option`, disregarding intrinsic and
/// schema-specified defaults.
#[test]
fn test_optional_properties_explicit() {
    validate_schema(
        "tests/schemas/types-with-defaults.json".into(),
        "tests/schemas/types-with-defaults-explicit.rs".into(),
        TypeSpaceSettings::default().with_optional_properties(OptionalProperties::Explicit),
    )
    .unwrap();

    trybuild::TestCases::new().pass("tests/schemas/types-with-defaults-explicit.rs");
}

/// Ensure that conversions apply by subset matching: a conversion matches
/// schemas that carry additional keywords, and the most specific matching
/// conversion wins.
#[test]
fn test_subset_conversions() {
    let uuid_schema = serde_json::from_value(json!({
        "type": "string",
        "format": "uuid",
    }))
    .unwrap();
    let string_schema = serde_json::from_value(json!({
        "type": "string",
    }))
    .unwrap();
    let number_schema = serde_json::from_value(json!({
        "type": "number",
    }))
    .unwrap();

    validate_schema(
        "tests/schemas/type-conversions.json".into(),
        "tests/schemas/type-conversions-subset.rs".into(),
        TypeSpaceSettings::default()
            // Less specific than the uuid conversion; wins only for plain
            // strings.
            .with_conversion(
                string_schema,
                "::std::string::String",
                [TypeSpaceImpl::Display, TypeSpaceImpl::FromStr].into_iter(),
            )
            .with_conversion(
                uuid_schema,
                "::uuid::Uuid",
                [TypeSpaceImpl::Display, TypeSpaceImpl::FromStr].into_iter(),
            )
            .with_conversion(
                number_schema,
                "::serde_json::Number",
                [TypeSpaceImpl::Display].into_iter(),
            ),
    )
    .unwrap();

    trybuild::TestCases::new().pass("tests/schemas/type-conversions-subset.rs");
}

/// Ensure that enumerated types include the enumeration in their JsonSchema
/// implementation.
#[test]
fn test_various_enums_json_schema() {
    validate_schema(
        "tests/schemas/various-enums.json".into(),
        "tests/schemas/various-enums-json-schema.rs".into(),
        TypeSpaceSettings::default().with_derive("schemars::JsonSchema".to_string()),
    )
    .unwrap();

    trybuild::TestCases::new().pass("tests/schemas/various-enums-json-schema.rs");
}

fn validate_schema(
    path: std::path::PathBuf,
    out_path: std::path::PathBuf,
    typespace: &mut TypeSpaceSettings,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let root_schema: RootSchema = serde_json::from_reader(reader)?;

    let schema_raw = json!(
        {
            "enum": [ 1, "one" ]
        }
    );
    let schema = serde_json::from_value(schema_raw).unwrap();

    let mut type_space = TypeSpace::new(
        typespace
            .with_replacement(
                "HandGeneratedType",
                "String",
                [TypeSpaceImpl::Display].into_iter(),
            )
            .with_patch(
                "TypeThatNeedsMoreDerives",
                TypeSpacePatch::default()
                    .with_rename("TypeThatHasMoreDerives")
                    .with_derive("Eq")
                    .with_derive("PartialEq"),
            )
            .with_conversion(
                schema,
                "serde_json::Value",
                [TypeSpaceImpl::Display].into_iter(),
            )
            // Our test use of the x-rust-type extension only refers to things
            // in std.
            .with_crate(
                "std",
                typify::CrateVers::Version("1.0.0".parse().unwrap()),
                None,
            )
            .with_struct_builder(true),
    );
    type_space.add_root_schema(root_schema)?;

    // Make a file with the generated code.
    let code = quote! {
        #![deny(warnings)]

        #type_space

        fn main() {}
    };
    let text = rustfmt_wrapper::rustfmt(code)?;
    assert_contents(out_path, &text);

    Ok(())
}
