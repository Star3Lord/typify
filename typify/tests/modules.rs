// Copyright 2026 Oxide Computer Company

//! Exercise per-module output: partition the types of a schema into
//! multiple modules using `TypeSpace::iter_definitions`, `Type::id`, and
//! `TypeSpace::to_stream_for`.

use std::{fs::File, io::BufReader};

use expectorate::assert_contents;
use quote::quote;
use typify::{AllOfStrategy, TypeSpace, TypeSpaceSettings};

#[test]
fn test_module_assembly() {
    let file = File::open("tests/schemas/composition.json").unwrap();
    let root_schema: schemars::schema::RootSchema =
        serde_json::from_reader(BufReader::new(file)).unwrap();

    // The Compose strategy makes the derived types refer to the base types,
    // exercising cross-module references.
    let mut type_space =
        TypeSpace::new(TypeSpaceSettings::default().with_all_of_strategy(AllOfStrategy::Compose));
    type_space.add_root_schema(root_schema).unwrap();

    // Partition the definitions into two modules.
    let (bases, things): (Vec<_>, Vec<_>) = type_space
        .iter_definitions()
        .partition(|(def_name, _)| matches!(*def_name, "base-thing" | "other-base"));
    let bases = bases.into_iter().map(|(_, ty)| ty.id()).collect::<Vec<_>>();
    let things = things
        .into_iter()
        .map(|(_, ty)| ty.id())
        .collect::<Vec<_>>();

    let bases_stream = type_space.to_stream_for(&bases).unwrap();
    let things_stream = type_space.to_stream_for(&things).unwrap();

    // Each module is self-contained but for references to the other
    // module's types, which an import resolves.
    let code = quote! {
        #![deny(warnings)]

        pub mod bases {
            #bases_stream
        }

        pub mod things {
            use super::bases::*;

            #things_stream
        }

        fn main() {}
    };
    let text = rustfmt_wrapper::rustfmt(code).unwrap();
    assert_contents("tests/modules/composition-modules.rs", &text);

    trybuild::TestCases::new().pass("tests/modules/composition-modules.rs");
}
