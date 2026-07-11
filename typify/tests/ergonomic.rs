// Copyright 2026 Oxide Computer Company

//! Runtime behavior of the ergonomic generation settings exposed through
//! the macro: open string enums and explicit optional properties.

use serde_json::json;

mod open {
    typify::import_types!(
        schema = "tests/schemas/open-enums.json",
        open_enum_variant = "Other",
        schema_in_docs = false,
    );
}

mod explicit {
    typify::import_types!(
        schema = "tests/schemas/types-with-defaults.json",
        optional_properties = Explicit,
    );
}

#[test]
fn test_open_enum_round_trip() {
    // Documented values deserialize into their variants.
    let known: open::Color = serde_json::from_value(json!("red")).unwrap();
    assert_eq!(known, open::Color::Red);

    // Values outside the documented set round-trip losslessly through the
    // catch-all variant.
    let unknown: open::Color = serde_json::from_value(json!("mauve")).unwrap();
    assert_eq!(unknown, open::Color::Other("mauve".to_string()));
    assert_eq!(serde_json::to_value(&unknown).unwrap(), json!("mauve"));
    assert_eq!(unknown.to_string(), "mauve");
    assert_eq!("mauve".parse::<open::Color>().unwrap(), unknown);

    // An enum that already declares the configured variant name stays
    // closed.
    assert!(serde_json::from_value::<open::AlreadyOpen>(json!("zeta")).is_err());
}

#[test]
fn test_explicit_optional_properties_round_trip() {
    // An absent property deserializes to None and is omitted from
    // serialized output ...
    let value: explicit::MrDefaultNumbers = serde_json::from_value(json!({})).unwrap();
    assert_eq!(value.little_u8, None);
    assert_eq!(serde_json::to_value(&value).unwrap(), json!({}));

    // ... whereas a property explicitly set to the schema's default value
    // is preserved: the wire distinction survives the round trip.
    let value: explicit::MrDefaultNumbers =
        serde_json::from_value(json!({ "little_u8": 2 })).unwrap();
    assert_eq!(value.little_u8, ::std::num::NonZeroU8::new(2));
    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        json!({ "little_u8": 2 }),
    );
}
