// Copyright 2026 Oxide Computer Company

use std::{error::Error, fs::File, io::BufReader};

use expectorate::assert_contents;
use glob::glob;
use quote::quote;
use serde_json::json;
use typify::TypeSpace;

#[test]
fn test_openapi_documents() {
    env_logger::init();
    // Make sure output is up to date.
    for entry in glob("tests/openapi/*.json").expect("Failed to read glob pattern") {
        let entry = entry.unwrap();
        let out_path = entry.clone().with_extension("rs");
        validate_document(entry, out_path).unwrap();
    }

    // Make sure it all compiles.
    trybuild::TestCases::new().pass("tests/openapi/*.rs");
}

fn validate_document(
    path: std::path::PathBuf,
    out_path: std::path::PathBuf,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let document: serde_json::Value = serde_json::from_reader(reader)?;

    let mut type_space = TypeSpace::default();
    type_space.add_openapi_document(&document)?;

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

mod petstore {
    typify::import_types!(schema = "tests/openapi/petstore-3-0.json");
}

#[test]
fn test_openapi_macro() {
    let pet: petstore::Pet = serde_json::from_value(json!({
        "id": 10,
        "name": "fido",
        "status": "available",
    }))
    .unwrap();
    assert_eq!(pet.status, Some(petstore::Status::Available));

    let value = serde_json::to_value(&pet).unwrap();
    assert_eq!(
        value,
        json!({
            "id": 10,
            "name": "fido",
            "status": "available",
        }),
    );
}

#[test]
fn test_openapi_document_errors() {
    for (document, error) in [
        (
            json!({ "swagger": "2.0", "info": {} }),
            "invalid OpenAPI document at #: Swagger 2.0 documents are not \
             supported; convert the document to OpenAPI 3.x first",
        ),
        (
            json!({ "openapi": "4.0.0" }),
            "invalid OpenAPI document at #/openapi: unsupported OpenAPI \
             version \"4.0.0\" (expected 3.0.x, 3.1.x, or 3.2.x)",
        ),
        (
            json!({
                "openapi": "3.0.3",
                "components": {
                    "schemas": {
                        "Pet": {
                            "type": "object",
                            "properties": {
                                "owner": {
                                    "$ref": "#/components/parameters/Owner",
                                },
                            },
                        },
                    },
                },
            }),
            "invalid OpenAPI document at \
             #/components/schemas/Pet/properties/owner/$ref: \
             \"#/components/parameters/Owner\" is not a reference of the \
             form \"#/components/schemas/<name>\"",
        ),
    ] {
        let mut type_space = TypeSpace::default();
        assert_eq!(
            type_space
                .add_openapi_document(&document)
                .unwrap_err()
                .to_string(),
            error,
        );
    }
}
