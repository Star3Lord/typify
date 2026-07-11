// Copyright 2026 Oxide Computer Company

use schemars::schema::SchemaObject;
use serde_json::{Map, Value};

use crate::{type_entry::TypeEntry, TypeSpaceImpl};

/// Schema-to-type conversions specified in the settings.
///
/// A conversion matches an input schema if every keyword the conversion
/// schema specifies is present in the input schema with an equal value;
/// the input schema may specify additional keywords. Metadata (title,
/// description, examples, etc.) is ignored on both sides. Of the
/// conversions that match a given schema, the most specific--the one that
/// specifies the most keywords--is used; ties are broken in favor of the
/// conversion specified first.
#[derive(Debug, Default)]
pub(crate) struct SchemaCache {
    conversions: Vec<(Map<String, Value>, TypeEntry)>,
}

/// The keywords of a schema as a JSON object, with metadata excluded.
fn keywords(schema: &SchemaObject) -> Map<String, Value> {
    let schema = SchemaObject {
        metadata: None,
        ..schema.clone()
    };
    match serde_json::to_value(schema) {
        Ok(Value::Object(map)) => map,
        _ => unreachable!("schema objects serialize to JSON objects"),
    }
}

impl SchemaCache {
    pub fn insert(&mut self, schema: &SchemaObject, type_name: &String, impls: &[TypeSpaceImpl]) {
        let type_entry = TypeEntry::new_native(type_name, impls);
        self.conversions.push((keywords(schema), type_entry));
    }

    pub fn lookup(&self, search_schema: &SchemaObject) -> Option<TypeEntry> {
        let search = keywords(search_schema);
        self.conversions
            .iter()
            .filter(|(conversion, _)| {
                conversion
                    .iter()
                    .all(|(keyword, value)| search.get(keyword) == Some(value))
            })
            // The most specific conversion wins; `>` (not `>=`) retains the
            // earliest-specified conversion among equals.
            .fold(
                None,
                |best: Option<&(Map<_, _>, TypeEntry)>, candidate| match best {
                    Some((keywords, _)) if keywords.len() >= candidate.0.len() => best,
                    _ => Some(candidate),
                },
            )
            .map(|(_, type_entry)| type_entry.clone())
    }
}

#[cfg(test)]
mod tests {
    use schemars::schema::SchemaObject;

    use super::SchemaCache;
    use crate::type_entry::{TypeEntryDetails, TypeEntryNative};

    fn schema(value: serde_json::Value) -> SchemaObject {
        serde_json::from_value(value).unwrap()
    }

    fn cache(conversions: &[serde_json::Value]) -> SchemaCache {
        let mut cache = SchemaCache::default();
        for (ii, conversion) in conversions.iter().enumerate() {
            cache.insert(&schema(conversion.clone()), &format!("type_{}", ii), &[]);
        }
        cache
    }

    #[track_caller]
    fn assert_lookup(cache: &SchemaCache, search: serde_json::Value, expected: Option<&str>) {
        let result = cache.lookup(&schema(search));
        let name = result.as_ref().map(|entry| match &entry.details {
            TypeEntryDetails::Native(TypeEntryNative { type_name, .. }) => type_name.as_str(),
            _ => panic!("unexpected entry type"),
        });
        assert_eq!(name, expected);
    }

    #[test]
    fn test_subset_match() {
        let cache = cache(&[serde_json::json!({
            "type": "string",
            "format": "uuid",
        })]);

        // An exact match.
        assert_lookup(
            &cache,
            serde_json::json!({ "type": "string", "format": "uuid" }),
            Some("type_0"),
        );
        // Additional keywords on the input do not preclude a match ...
        assert_lookup(
            &cache,
            serde_json::json!({
                "type": "string",
                "format": "uuid",
                "maxLength": 36,
            }),
            Some("type_0"),
        );
        // ... but every specified keyword must match exactly.
        assert_lookup(
            &cache,
            serde_json::json!({ "type": "string", "format": "date" }),
            None,
        );
        assert_lookup(&cache, serde_json::json!({ "type": "string" }), None);
    }

    #[test]
    fn test_metadata_ignored() {
        let cache = cache(&[serde_json::json!({
            "title": "IgnoredTitle",
            "type": "number",
        })]);

        assert_lookup(
            &cache,
            serde_json::json!({
                "description": "one honest number",
                "type": "number",
            }),
            Some("type_0"),
        );
    }

    #[test]
    fn test_specificity() {
        let cache = cache(&[
            serde_json::json!({ "type": "string" }),
            serde_json::json!({ "type": "string", "format": "uuid" }),
        ]);

        // The most specific matching conversion wins, regardless of the
        // order in which conversions were specified.
        assert_lookup(
            &cache,
            serde_json::json!({ "type": "string", "format": "uuid" }),
            Some("type_1"),
        );
        assert_lookup(
            &cache,
            serde_json::json!({ "type": "string", "pattern": "^x" }),
            Some("type_0"),
        );
    }

    #[test]
    fn test_first_wins_among_equals() {
        let cache = cache(&[
            serde_json::json!({ "type": "integer", "format": "int64" }),
            serde_json::json!({ "type": "integer", "multipleOf": 2.0 }),
        ]);

        // Two matches of equal specificity: the first specified is used.
        assert_lookup(
            &cache,
            serde_json::json!({
                "type": "integer",
                "format": "int64",
                "multipleOf": 2.0,
            }),
            Some("type_0"),
        );
    }

    #[test]
    fn test_deep_equality() {
        let cache = cache(&[serde_json::json!({
            "enum": [1, "one"],
        })]);

        assert_lookup(
            &cache,
            serde_json::json!({ "enum": [1, "one"], "type": "string" }),
            Some("type_0"),
        );
        // Values are compared deeply; a different enum does not match.
        assert_lookup(&cache, serde_json::json!({ "enum": ["one", 1] }), None);
    }

    #[test]
    fn test_empty_conversion_matches_everything() {
        // A conversion specifying no keywords matches every schema (at the
        // lowest possible specificity).
        let cache = cache(&[
            serde_json::json!({}),
            serde_json::json!({ "type": "boolean" }),
        ]);

        assert_lookup(
            &cache,
            serde_json::json!({ "type": "string" }),
            Some("type_0"),
        );
        assert_lookup(
            &cache,
            serde_json::json!({ "type": "boolean" }),
            Some("type_1"),
        );
    }
}
