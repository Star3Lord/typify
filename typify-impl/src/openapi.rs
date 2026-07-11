// Copyright 2026 Oxide Computer Company

//! Conversion of the component schemas of an OpenAPI document into the JSON
//! Schema representation that typify uses internally.
//!
//! OpenAPI versions differ in the JSON Schema dialect used by their Schema
//! Objects: OpenAPI 3.0.x uses its own dialect--roughly a subset of draft 4
//! extended with keywords such as `nullable`--whereas OpenAPI 3.1.x and 3.2.x
//! use JSON Schema 2020-12. We normalize schemas from each dialect into the
//! draft-7-flavored model that typify consumes ([`schemars::schema`]),
//! validating along the way that all references are ones we can resolve.
//! Normalization is structural--performed field by field on the JSON value
//! rather than by key name matching--so a property that happens to be called
//! e.g. `nullable` is never mangled.
//!
//! Support for a future OpenAPI version amounts to adding an
//! [`OpenApiVersion`] variant and mapping it to the appropriate
//! [`SchemaDialect`]; keyword changes (if any) belong in the
//! dialect-specific normalization.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::{Error, Result};

/// OpenAPI document versions from which we can generate types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenApiVersion {
    V3_0,
    V3_1,
    V3_2,
}

/// The JSON Schema dialect used by the Schema Objects of an OpenAPI
/// document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaDialect {
    /// The OpenAPI 3.0 schema dialect.
    OpenApi30,
    /// JSON Schema 2020-12, the base dialect of OpenAPI 3.1 and 3.2.
    JsonSchema202012,
}

impl OpenApiVersion {
    fn parse(version: &str, location: &Location) -> Result<Self> {
        let mut parts = version.split('.');
        match (parts.next(), parts.next()) {
            (Some("3"), Some("0")) => Ok(Self::V3_0),
            (Some("3"), Some("1")) => Ok(Self::V3_1),
            (Some("3"), Some("2")) => Ok(Self::V3_2),
            _ => Err(invalid_openapi(
                location,
                format!(
                    "unsupported OpenAPI version \"{}\" \
                     (expected 3.0.x, 3.1.x, or 3.2.x)",
                    version,
                ),
            )),
        }
    }

    fn dialect(&self) -> SchemaDialect {
        match self {
            Self::V3_0 => SchemaDialect::OpenApi30,
            Self::V3_1 | Self::V3_2 => SchemaDialect::JsonSchema202012,
        }
    }
}

/// Dialect URIs (values of the `jsonSchemaDialect` field, OpenAPI 3.1 and
/// later) that we know to be equivalent to JSON Schema 2020-12 for the
/// keywords we interpret.
const KNOWN_2020_12_DIALECTS: [&str; 3] = [
    "https://spec.openapis.org/oas/3.1/dialect/base",
    "https://spec.openapis.org/oas/3.2/dialect/base",
    "https://json-schema.org/draft/2020-12/schema",
];

/// A JSON pointer into the OpenAPI document, used to give errors a precise
/// location.
#[derive(Debug, Clone)]
struct Location(String);

impl Location {
    fn root() -> Self {
        Self(String::new())
    }

    /// The location of `segment` under `self`, with RFC 6901 token escaping.
    fn child(&self, segment: &str) -> Self {
        let token = segment.replace('~', "~0").replace('/', "~1");
        Self(format!("{}/{}", self.0, token))
    }

    fn index(&self, index: usize) -> Self {
        Self(format!("{}/{}", self.0, index))
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

fn invalid_openapi<S: ToString>(location: &Location, reason: S) -> Error {
    Error::InvalidOpenApi {
        location: location.to_string(),
        reason: reason.to_string(),
    }
}

/// Extract the schemas of an OpenAPI document as named JSON Schemas in
/// typify's internal representation, normalized from the dialect implied by
/// the document version (and the `jsonSchemaDialect` field for OpenAPI 3.1
/// and later).
pub(crate) fn extract_schemas(document: &Value) -> Result<Vec<(String, schemars::schema::Schema)>> {
    let root = Location::root();
    let Some(document) = document.as_object() else {
        return Err(invalid_openapi(&root, "document must be an object"));
    };

    let version = match document.get("openapi") {
        Some(Value::String(version)) => OpenApiVersion::parse(version, &root.child("openapi"))?,
        Some(_) => {
            return Err(invalid_openapi(
                &root.child("openapi"),
                "expected a version string",
            ))
        }
        None if document.contains_key("swagger") => {
            return Err(invalid_openapi(
                &root,
                "Swagger 2.0 documents are not supported; \
                 convert the document to OpenAPI 3.x first",
            ))
        }
        None => {
            return Err(invalid_openapi(
                &root,
                "not an OpenAPI document (missing the `openapi` field)",
            ))
        }
    };

    let dialect = document_dialect(document, version, &root)?;

    let schemas_location = root.child("components").child("schemas");
    let schemas = match document
        .get("components")
        .and_then(|components| components.get("schemas"))
    {
        // A document without component schemas contains no types (typify
        // does not examine operations, parameters, or other document
        // contents).
        None => return Ok(Vec::new()),
        Some(Value::Object(schemas)) => schemas,
        Some(_) => return Err(invalid_openapi(&schemas_location, "expected an object")),
    };

    let schema_names = schemas.keys().cloned().collect::<BTreeSet<_>>();
    let normalizer = Normalizer {
        dialect,
        schema_names: &schema_names,
    };

    schemas
        .iter()
        .map(|(name, schema)| {
            let location = schemas_location.child(name);
            let normalized = normalizer.normalize(schema, &location)?;
            let schema = serde_json::from_value(normalized).map_err(|e| {
                invalid_openapi(
                    &location,
                    format!("schema is invalid after normalization: {}", e),
                )
            })?;
            Ok((name.clone(), schema))
        })
        .collect()
}

/// Determine the schema dialect for a document, honoring the
/// `jsonSchemaDialect` field (OpenAPI 3.1 and later).
fn document_dialect(
    document: &Map<String, Value>,
    version: OpenApiVersion,
    root: &Location,
) -> Result<SchemaDialect> {
    let dialect = version.dialect();
    if let SchemaDialect::OpenApi30 = dialect {
        return Ok(dialect);
    }

    match document.get("jsonSchemaDialect") {
        None => Ok(dialect),
        Some(Value::String(uri)) if KNOWN_2020_12_DIALECTS.contains(&uri.as_str()) => {
            Ok(SchemaDialect::JsonSchema202012)
        }
        Some(Value::String(uri)) => Err(invalid_openapi(
            &root.child("jsonSchemaDialect"),
            format!("unsupported schema dialect \"{}\"", uri),
        )),
        Some(_) => Err(invalid_openapi(
            &root.child("jsonSchemaDialect"),
            "expected a dialect URI string",
        )),
    }
}

/// How a keyword's value participates in normalization.
enum KeywordKind {
    /// The value is a subschema.
    Schema,
    /// The value is an array of subschemas.
    SchemaArray,
    /// The value is an object whose values are subschemas.
    SchemaMap,
    /// The value is a JSON reference to validate.
    Ref,
    /// The keyword is not supported and normalization fails loudly (silently
    /// passing it through would change the meaning of generated types).
    Unsupported,
    /// The value passes through unmodified (this includes keywords that the
    /// rest of typify interprets as well as those it ignores).
    Other,
}

struct Normalizer<'a> {
    dialect: SchemaDialect,
    /// Names of the schemas in `components.schemas`; the only valid
    /// reference targets.
    schema_names: &'a BTreeSet<String>,
}

impl Normalizer<'_> {
    /// Normalize a schema value: recursively normalize subschemas, then
    /// apply the dialect-specific keyword transformations to this node.
    fn normalize(&self, schema: &Value, location: &Location) -> Result<Value> {
        let schema = match (schema, self.dialect) {
            (Value::Object(map), _) => map,
            // Boolean schemas are valid JSON Schema 2020-12 ...
            (Value::Bool(_), SchemaDialect::JsonSchema202012) => return Ok(schema.clone()),
            // ... but are not part of the OpenAPI 3.0 dialect.
            (Value::Bool(_), SchemaDialect::OpenApi30) => {
                return Err(invalid_openapi(
                    location,
                    "boolean schemas are not part of the OpenAPI 3.0 schema dialect",
                ))
            }
            (Value::Array(_), _) => {
                return Err(invalid_openapi(
                    location,
                    "expected a schema, not an array of schemas",
                ))
            }
            _ => return Err(invalid_openapi(location, "expected a schema")),
        };

        let mut out = Map::new();
        for (key, value) in schema {
            let child = location.child(key);
            let value = match self.keyword_kind(key, value) {
                KeywordKind::Schema => self.normalize(value, &child)?,
                KeywordKind::SchemaArray => match value {
                    Value::Array(subschemas) => Value::Array(
                        subschemas
                            .iter()
                            .enumerate()
                            .map(|(ii, subschema)| self.normalize(subschema, &child.index(ii)))
                            .collect::<Result<_>>()?,
                    ),
                    // Leave malformed values for the deserialization check.
                    _ => value.clone(),
                },
                KeywordKind::SchemaMap => match value {
                    Value::Object(subschemas) => Value::Object(
                        subschemas
                            .iter()
                            .map(|(name, subschema)| {
                                Ok((name.clone(), self.normalize(subschema, &child.child(name))?))
                            })
                            .collect::<Result<_>>()?,
                    ),
                    _ => value.clone(),
                },
                KeywordKind::Ref => {
                    self.check_ref(value, &child)?;
                    value.clone()
                }
                KeywordKind::Unsupported => {
                    return Err(invalid_openapi(&child, unsupported_keyword_reason(key)))
                }
                KeywordKind::Other => value.clone(),
            };
            out.insert(key.clone(), value);
        }

        match self.dialect {
            SchemaDialect::OpenApi30 => self.finish_openapi_3_0(out, location),
            SchemaDialect::JsonSchema202012 => self.finish_2020_12(out, location),
        }
    }

    fn keyword_kind(&self, key: &str, value: &Value) -> KeywordKind {
        match (key, self.dialect) {
            ("$ref", _) => KeywordKind::Ref,

            // Keywords that would require reference or dialect machinery
            // typify doesn't have; passing them through would silently
            // change the meaning of generated types, so fail loudly. Though
            // most are not defined by the OpenAPI 3.0 dialect, we treat both
            // dialects uniformly: these keywords always signal intent we
            // cannot honor.
            (
                "$defs" | "definitions" | "$id" | "$anchor" | "$dynamicAnchor" | "$dynamicRef"
                | "$schema",
                _,
            ) => KeywordKind::Unsupported,

            // Subschema-bearing keywords common to both dialects.
            ("properties", _) => KeywordKind::SchemaMap,
            ("allOf" | "anyOf" | "oneOf", _) => KeywordKind::SchemaArray,
            ("not", _) => KeywordKind::Schema,
            // `additionalProperties` may be a boolean in both dialects.
            ("additionalProperties", _) if !value.is_boolean() => KeywordKind::Schema,

            // In the OpenAPI 3.0 dialect `items` must be a single schema; the
            // array form is reported by `normalize`. JSON Schema 2020-12
            // likewise requires a single schema (tuples use `prefixItems`),
            // but we accept the unambiguous draft-7 array form.
            ("items", SchemaDialect::OpenApi30) => KeywordKind::Schema,
            ("items", SchemaDialect::JsonSchema202012) => match value {
                Value::Array(_) => KeywordKind::SchemaArray,
                _ => KeywordKind::Schema,
            },

            // Subschema-bearing keywords specific to JSON Schema 2020-12.
            // Note that typify interprets only some of these; the rest pass
            // through (normalized) to be ignored, consistent with typify's
            // treatment of the corresponding draft 7 keywords.
            (
                "if"
                | "then"
                | "else"
                | "contains"
                | "propertyNames"
                | "unevaluatedProperties"
                | "unevaluatedItems"
                | "contentSchema",
                SchemaDialect::JsonSchema202012,
            ) => KeywordKind::Schema,
            ("prefixItems", SchemaDialect::JsonSchema202012) => KeywordKind::SchemaArray,
            ("patternProperties" | "dependentSchemas", SchemaDialect::JsonSchema202012) => {
                KeywordKind::SchemaMap
            }

            _ => KeywordKind::Other,
        }
    }

    /// Validate that a `$ref` targets a schema in `components.schemas` that
    /// exists. Anything else--external documents, other component sections,
    /// paths into a schema--would panic or misgenerate downstream, so we
    /// fail here with a precise location.
    fn check_ref(&self, value: &Value, location: &Location) -> Result<()> {
        let Some(reference) = value.as_str() else {
            return Err(invalid_openapi(location, "expected a reference string"));
        };
        let name = reference
            .strip_prefix("#/components/schemas/")
            .ok_or_else(|| {
                invalid_openapi(
                    location,
                    format!(
                        "\"{}\" is not a reference of the form \
                         \"#/components/schemas/<name>\"",
                        reference,
                    ),
                )
            })?;
        if name.contains('/') {
            return Err(invalid_openapi(
                location,
                format!(
                    "\"{}\" refers into the interior of a schema; references \
                     must target a schema in components.schemas",
                    reference,
                ),
            ));
        }
        let name = name.replace("~1", "/").replace("~0", "~");
        if !self.schema_names.contains(&name) {
            return Err(invalid_openapi(
                location,
                format!(
                    "\"{}\" does not match a schema in components.schemas",
                    reference,
                ),
            ));
        }
        Ok(())
    }

    /// Apply OpenAPI 3.0 dialect transformations to a schema node whose
    /// subschemas have already been normalized.
    fn finish_openapi_3_0(
        &self,
        mut map: Map<String, Value>,
        location: &Location,
    ) -> Result<Value> {
        let nullable = match map.remove("nullable") {
            None => false,
            Some(Value::Bool(nullable)) => nullable,
            Some(_) => {
                return Err(invalid_openapi(
                    &location.child("nullable"),
                    "expected a boolean",
                ))
            }
        };

        // The OpenAPI 3.0 dialect has no type arrays (`nullable` fills that
        // role); an array here most likely indicates a mislabeled OpenAPI
        // 3.1 document.
        if matches!(map.get("type"), Some(Value::Array(_))) {
            return Err(invalid_openapi(
                &location.child("type"),
                "type arrays are not part of the OpenAPI 3.0 schema dialect",
            ));
        }

        // Convert the draft 4 boolean form of exclusive bounds
        // (`exclusiveMinimum: true` alongside `minimum`) to the numeric
        // form. Numeric values--not part of the 3.0 dialect, but
        // unambiguous--pass through.
        for (bound, exclusive) in [
            ("minimum", "exclusiveMinimum"),
            ("maximum", "exclusiveMaximum"),
        ] {
            match map.get(exclusive) {
                Some(Value::Bool(true)) => match map.remove(bound) {
                    Some(value) => {
                        map.insert(exclusive.to_string(), value);
                    }
                    // An exclusivity flag without a bound is meaningless.
                    None => {
                        map.remove(exclusive);
                    }
                },
                Some(Value::Bool(false)) => {
                    map.remove(exclusive);
                }
                _ => (),
            }
        }

        example_to_examples(&mut map);

        // A null default on a non-nullable schema is invalid per the OpenAPI
        // 3.0 specification (the default must validate against the schema),
        // but appears in real-world documents as a way of saying "no
        // default". Ignoring it is the only sensible interpretation.
        if !nullable && matches!(map.get("default"), Some(Value::Null)) {
            log::debug!(
                "ignoring `default: null` on the non-nullable schema at {}",
                location,
            );
            map.remove("default");
        }

        if nullable {
            Ok(make_nullable(map))
        } else {
            Ok(Value::Object(map))
        }
    }

    /// Apply JSON Schema 2020-12 transformations to a schema node whose
    /// subschemas have already been normalized.
    fn finish_2020_12(&self, mut map: Map<String, Value>, location: &Location) -> Result<Value> {
        // Convert 2020-12 tuple constructs to the draft 7 form: the items of
        // `prefixItems` become the array form of `items`, and a sibling
        // `items` schema--which constrains any additional items--becomes
        // `additionalItems`.
        if let Some(prefix_items) = map.remove("prefixItems") {
            if let Some(items) = map.remove("items") {
                map.insert("additionalItems".to_string(), items);
            }
            map.insert("items".to_string(), prefix_items);
        }

        example_to_examples(&mut map);

        // `nullable` is not part of this dialect, but it commonly survives
        // mechanical 3.0 -> 3.1 document conversion and its intent is
        // unambiguous; honor it (loudly).
        if matches!(map.get("nullable"), Some(Value::Bool(_))) {
            let nullable = map.remove("nullable") == Some(Value::Bool(true));
            if nullable {
                log::warn!(
                    "`nullable` is not part of the OpenAPI 3.1 (and later) \
                     schema dialect; honoring it at {}",
                    location,
                );
                return Ok(make_nullable(map));
            }
        }

        Ok(Value::Object(map))
    }
}

fn unsupported_keyword_reason(key: &str) -> String {
    match key {
        "$defs" | "definitions" => format!(
            "`{}` nested within a component schema is not supported; \
             move shared schemas to components.schemas",
            key,
        ),
        _ => format!(
            "`{}` is not supported; references must be of the form \
             \"#/components/schemas/<name>\"",
            key,
        ),
    }
}

/// Fold `example` (an OpenAPI annotation predating JSON Schema's `examples`)
/// into `examples`, from which typify picks up metadata. An existing
/// `examples` value wins.
fn example_to_examples(map: &mut Map<String, Value>) {
    if let Some(example) = map.remove("example") {
        map.entry("examples")
            .or_insert_with(|| Value::Array(vec![example]));
    }
}

/// Rewrite a schema--stripped of `nullable` and with subschemas already
/// normalized--to admit null in the way typify best interprets for the
/// schema's shape.
fn make_nullable(mut map: Map<String, Value>) -> Value {
    // A typed schema admits null through a type array; typify converts
    // `[T, "null"]` into `Option<T>`.
    let has_type = match map.get_mut("type") {
        Some(Value::String(single)) if single != "null" => {
            let single = std::mem::take(single);
            map.insert(
                "type".to_string(),
                Value::Array(vec![
                    Value::String(single),
                    Value::String("null".to_string()),
                ]),
            );
            // With this shape, typify uses `title` to name the non-null
            // inner type. Specs frequently title schemas with their own
            // names, which would make the inner type collide with the
            // definition itself (e.g. `X(Option<X>)`); the title is
            // redundant with the definition name, so drop it. (The
            // description carries the useful documentation and remains.)
            map.remove("title");
            true
        }
        Some(Value::Array(types)) => {
            let null = Value::String("null".to_string());
            if !types.contains(&null) {
                types.push(null);
                if types.len() == 2 {
                    // We created the `[T, "null"]` shape; see above.
                    map.remove("title");
                }
            }
            true
        }
        Some(_) => true,
        None => false,
    };

    if has_type || map.contains_key("enum") {
        // Enumerated values must include null for it to be a valid value;
        // typify prunes the null into the `Option`.
        if let Some(Value::Array(values)) = map.get_mut("enum") {
            if !values.contains(&Value::Null) {
                values.push(Value::Null);
            }
        }
        return Value::Object(map);
    }

    // A subschema union admits null through an additional null subschema;
    // wrapping the whole construction instead would obscure the union from
    // typify's `oneOf`/`anyOf` interpretation.
    for key in ["oneOf", "anyOf"] {
        if let Some(Value::Array(subschemas)) = map.get_mut(key) {
            let admits_null = subschemas
                .iter()
                .any(|subschema| subschema.get("type").and_then(Value::as_str) == Some("null"));
            if !admits_null {
                subschemas.push(serde_json::json!({ "type": "null" }));
            }
            return Value::Object(map);
        }
    }

    // An unconstrained schema already admits null.
    if map.is_empty() {
        return Value::Object(map);
    }

    // For anything else--references, `allOf` constructions--admit null by
    // wrapping; typify converts a two-element `anyOf` with a null schema
    // into `Option<T>`.
    let mut wrapper = Map::new();
    wrapper.insert(
        "anyOf".to_string(),
        Value::Array(vec![
            Value::Object(map),
            serde_json::json!({ "type": "null" }),
        ]),
    );
    Value::Object(wrapper)
}

#[cfg(test)]
mod tests {
    use schemars::schema::Schema;
    use serde_json::{json, Value};

    use super::{extract_schemas, Location, Normalizer, OpenApiVersion, SchemaDialect};
    use crate::Error;

    fn normalize(dialect: SchemaDialect, schema: Value) -> Result<Value, Error> {
        let schema_names = ["Referenced".to_string()].into_iter().collect();
        let normalizer = Normalizer {
            dialect,
            schema_names: &schema_names,
        };
        normalizer.normalize(&schema, &Location::root())
    }

    fn normalize_3_0(schema: Value) -> Result<Value, Error> {
        normalize(SchemaDialect::OpenApi30, schema)
    }

    fn normalize_2020_12(schema: Value) -> Result<Value, Error> {
        normalize(SchemaDialect::JsonSchema202012, schema)
    }

    #[track_caller]
    fn assert_invalid<T: std::fmt::Debug>(
        result: Result<T, Error>,
        location: &str,
        reason_fragment: &str,
    ) {
        match result {
            Err(Error::InvalidOpenApi {
                location: l,
                reason,
            }) => {
                assert_eq!(l, location);
                assert!(
                    reason.contains(reason_fragment),
                    "reason {:?} does not contain {:?}",
                    reason,
                    reason_fragment,
                );
            }
            other => panic!("expected an InvalidOpenApi error; got {:?}", other),
        }
    }

    #[test]
    fn test_version_parse() {
        for (version, expected) in [
            ("3.0.0", OpenApiVersion::V3_0),
            ("3.0.4", OpenApiVersion::V3_0),
            ("3.0", OpenApiVersion::V3_0),
            ("3.1.0", OpenApiVersion::V3_1),
            ("3.1.1", OpenApiVersion::V3_1),
            ("3.2.0", OpenApiVersion::V3_2),
        ] {
            assert_eq!(
                OpenApiVersion::parse(version, &Location::root()).unwrap(),
                expected,
            );
        }

        for version in ["2.0", "3", "3.3.0", "4.0.0", "three"] {
            assert_invalid(
                OpenApiVersion::parse(version, &Location::root()),
                "#",
                "unsupported OpenAPI version",
            );
        }
    }

    #[test]
    fn test_dialects() {
        assert_eq!(OpenApiVersion::V3_0.dialect(), SchemaDialect::OpenApi30);
        assert_eq!(
            OpenApiVersion::V3_1.dialect(),
            SchemaDialect::JsonSchema202012,
        );
        assert_eq!(
            OpenApiVersion::V3_2.dialect(),
            SchemaDialect::JsonSchema202012,
        );
    }

    #[test]
    fn test_nullable_typed() {
        assert_eq!(
            normalize_3_0(json!({
                "title": "Pet",
                "description": "a pet",
                "type": "string",
                "nullable": true,
            }))
            .unwrap(),
            json!({
                "description": "a pet",
                "type": ["string", "null"],
            }),
        );
    }

    #[test]
    fn test_nullable_typed_enum() {
        // The null member is added so that it remains a valid value.
        assert_eq!(
            normalize_3_0(json!({
                "type": "string",
                "enum": ["dog", "cat"],
                "nullable": true,
            }))
            .unwrap(),
            json!({
                "type": ["string", "null"],
                "enum": ["dog", "cat", null],
            }),
        );

        // ... but not duplicated if already present.
        assert_eq!(
            normalize_3_0(json!({
                "type": "string",
                "enum": ["dog", null],
                "nullable": true,
            }))
            .unwrap(),
            json!({
                "type": ["string", "null"],
                "enum": ["dog", null],
            }),
        );
    }

    #[test]
    fn test_nullable_untyped_enum() {
        assert_eq!(
            normalize_3_0(json!({
                "enum": ["dog", "cat"],
                "nullable": true,
            }))
            .unwrap(),
            json!({
                "enum": ["dog", "cat", null],
            }),
        );
    }

    #[test]
    fn test_nullable_union() {
        assert_eq!(
            normalize_3_0(json!({
                "oneOf": [
                    { "type": "string" },
                    { "type": "integer" },
                ],
                "nullable": true,
            }))
            .unwrap(),
            json!({
                "oneOf": [
                    { "type": "string" },
                    { "type": "integer" },
                    { "type": "null" },
                ],
            }),
        );
    }

    #[test]
    fn test_nullable_ref() {
        assert_eq!(
            normalize_3_0(json!({
                "$ref": "#/components/schemas/Referenced",
                "nullable": true,
            }))
            .unwrap(),
            json!({
                "anyOf": [
                    { "$ref": "#/components/schemas/Referenced" },
                    { "type": "null" },
                ],
            }),
        );
    }

    #[test]
    fn test_nullable_empty() {
        // An unconstrained schema already admits null.
        assert_eq!(
            normalize_3_0(json!({ "nullable": true })).unwrap(),
            json!({}),
        );
    }

    #[test]
    fn test_nullable_noise() {
        assert_eq!(
            normalize_3_0(json!({
                "type": "string",
                "nullable": false,
            }))
            .unwrap(),
            json!({ "type": "string" }),
        );

        assert_invalid(
            normalize_3_0(json!({ "nullable": "yes" })),
            "#/nullable",
            "expected a boolean",
        );
    }

    #[test]
    fn test_nullable_within_properties() {
        // Normalization must be structural: a *property* named "nullable"
        // is data, not a keyword.
        assert_eq!(
            normalize_3_0(json!({
                "type": "object",
                "properties": {
                    "nullable": { "type": "boolean" },
                },
            }))
            .unwrap(),
            json!({
                "type": "object",
                "properties": {
                    "nullable": { "type": "boolean" },
                },
            }),
        );
    }

    #[test]
    fn test_exclusive_bounds() {
        assert_eq!(
            normalize_3_0(json!({
                "type": "integer",
                "minimum": 1,
                "exclusiveMinimum": true,
                "maximum": 10,
                "exclusiveMaximum": false,
            }))
            .unwrap(),
            json!({
                "type": "integer",
                "exclusiveMinimum": 1,
                "maximum": 10,
            }),
        );

        // An exclusivity flag without a bound is dropped.
        assert_eq!(
            normalize_3_0(json!({
                "type": "integer",
                "exclusiveMinimum": true,
            }))
            .unwrap(),
            json!({ "type": "integer" }),
        );

        // The numeric form passes through in both dialects.
        let numeric = json!({
            "type": "integer",
            "exclusiveMinimum": 1,
        });
        assert_eq!(normalize_3_0(numeric.clone()).unwrap(), numeric);
        assert_eq!(normalize_2020_12(numeric.clone()).unwrap(), numeric);
    }

    #[test]
    fn test_example_to_examples() {
        assert_eq!(
            normalize_3_0(json!({
                "type": "string",
                "example": "fido",
            }))
            .unwrap(),
            json!({
                "type": "string",
                "examples": ["fido"],
            }),
        );

        // An existing `examples` value wins.
        assert_eq!(
            normalize_2020_12(json!({
                "type": "string",
                "example": "fido",
                "examples": ["rex"],
            }))
            .unwrap(),
            json!({
                "type": "string",
                "examples": ["rex"],
            }),
        );
    }

    #[test]
    fn test_null_default() {
        // Real-world 3.0 documents use `default: null` on non-nullable
        // schemas to mean "no default"; taking it literally would make the
        // schema unsatisfiable.
        assert_eq!(
            normalize_3_0(json!({
                "type": "string",
                "default": null,
            }))
            .unwrap(),
            json!({ "type": "string" }),
        );

        // On a nullable schema, null is a legitimate default.
        assert_eq!(
            normalize_3_0(json!({
                "type": "string",
                "default": null,
                "nullable": true,
            }))
            .unwrap(),
            json!({
                "type": ["string", "null"],
                "default": null,
            }),
        );
    }

    #[test]
    fn test_3_0_illegal_constructs() {
        assert_invalid(
            normalize_3_0(json!({ "type": ["string", "null"] })),
            "#/type",
            "type arrays",
        );
        assert_invalid(
            normalize_3_0(json!({
                "type": "array",
                "items": [{ "type": "string" }],
            })),
            "#/items",
            "not an array of schemas",
        );
        assert_invalid(
            normalize_3_0(json!({
                "type": "object",
                "additionalProperties": { "not": true },
            })),
            "#/additionalProperties/not",
            "boolean schemas",
        );
    }

    #[test]
    fn test_prefix_items() {
        assert_eq!(
            normalize_2020_12(json!({
                "type": "array",
                "prefixItems": [
                    { "type": "string" },
                    { "type": "integer" },
                ],
            }))
            .unwrap(),
            json!({
                "type": "array",
                "items": [
                    { "type": "string" },
                    { "type": "integer" },
                ],
            }),
        );

        assert_eq!(
            normalize_2020_12(json!({
                "type": "array",
                "prefixItems": [{ "type": "string" }],
                "items": { "type": "integer" },
            }))
            .unwrap(),
            json!({
                "type": "array",
                "items": [{ "type": "string" }],
                "additionalItems": { "type": "integer" },
            }),
        );
    }

    #[test]
    fn test_2020_12_passthrough() {
        // Constructs typify's internal representation already models pass
        // through unmodified.
        let schema = json!({
            "type": ["string", "integer"],
            "const": "x",
            "examples": ["x"],
            "x-custom": { "$comment": "vendor extensions ride along" },
        });
        assert_eq!(normalize_2020_12(schema.clone()).unwrap(), schema);

        // ... as do boolean schemas.
        assert_eq!(
            normalize_2020_12(json!({
                "type": "object",
                "additionalProperties": false,
            }))
            .unwrap(),
            json!({
                "type": "object",
                "additionalProperties": false,
            }),
        );
    }

    #[test]
    fn test_2020_12_nullable_leniency() {
        // `nullable` is 3.0-only, but commonly survives document conversion;
        // its intent is unambiguous.
        assert_eq!(
            normalize_2020_12(json!({
                "type": "string",
                "nullable": true,
            }))
            .unwrap(),
            json!({ "type": ["string", "null"] }),
        );

        // A nullable type array gains a null member.
        assert_eq!(
            normalize_2020_12(json!({
                "type": ["string", "integer"],
                "nullable": true,
            }))
            .unwrap(),
            json!({ "type": ["string", "integer", "null"] }),
        );
    }

    #[test]
    fn test_unsupported_keywords() {
        assert_invalid(
            normalize_2020_12(json!({
                "$defs": { "Inner": { "type": "string" } },
                "$ref": "#/$defs/Inner",
            })),
            "#/$defs",
            "nested within a component schema",
        );
        assert_invalid(
            normalize_2020_12(json!({ "$dynamicRef": "#meta" })),
            "#/$dynamicRef",
            "`$dynamicRef` is not supported",
        );
        assert_invalid(
            normalize_3_0(json!({
                "definitions": { "Inner": { "type": "string" } },
            })),
            "#/definitions",
            "nested within a component schema",
        );
    }

    #[test]
    fn test_refs() {
        // Valid references pass through.
        let schema = json!({ "$ref": "#/components/schemas/Referenced" });
        assert_eq!(normalize_3_0(schema.clone()).unwrap(), schema);

        assert_invalid(
            normalize_3_0(json!({ "$ref": "#/components/schemas/Missing" })),
            "#/$ref",
            "does not match a schema",
        );
        assert_invalid(
            normalize_3_0(json!({ "$ref": "#/components/parameters/PetId" })),
            "#/$ref",
            "#/components/schemas/<name>",
        );
        assert_invalid(
            normalize_3_0(json!({ "$ref": "other.json#/components/schemas/Pet" })),
            "#/$ref",
            "#/components/schemas/<name>",
        );
        assert_invalid(
            normalize_3_0(json!({
                "$ref": "#/components/schemas/Referenced/properties/x",
            })),
            "#/$ref",
            "refers into the interior of a schema",
        );
        assert_invalid(
            normalize_2020_12(json!({
                "properties": { "x": { "$ref": 42 } },
            })),
            "#/properties/x/$ref",
            "expected a reference string",
        );
    }

    #[test]
    fn test_escaped_ref() {
        let schema_names = ["a/b~c".to_string()].into_iter().collect();
        let normalizer = Normalizer {
            dialect: SchemaDialect::OpenApi30,
            schema_names: &schema_names,
        };
        let schema = json!({ "$ref": "#/components/schemas/a~1b~0c" });
        assert_eq!(
            normalizer.normalize(&schema, &Location::root()).unwrap(),
            schema,
        );
    }

    #[test]
    fn test_extract_schemas() {
        let document = json!({
            "openapi": "3.0.3",
            "info": { "title": "test", "version": "1.0.0" },
            "paths": {},
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "tag": {
                                "$ref": "#/components/schemas/Tag",
                            },
                        },
                        "required": ["name"],
                    },
                    "Tag": {
                        "type": "string",
                        "nullable": true,
                    },
                },
            },
        });

        let schemas = extract_schemas(&document).unwrap();
        assert_eq!(
            schemas
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            vec!["Pet", "Tag"],
        );

        let Schema::Object(tag) = &schemas[1].1 else {
            panic!("expected an object schema");
        };
        assert_eq!(
            tag.instance_type,
            Some(
                vec![
                    schemars::schema::InstanceType::String,
                    schemars::schema::InstanceType::Null,
                ]
                .into()
            ),
        );
    }

    #[test]
    fn test_extract_schemas_empty() {
        // A document without component schemas contains no types.
        let document = json!({
            "openapi": "3.1.0",
            "info": { "title": "test", "version": "1.0.0" },
            "paths": {},
        });
        assert_eq!(extract_schemas(&document).unwrap(), Vec::new());
    }

    #[test]
    fn test_extract_schemas_errors() {
        assert_invalid(
            extract_schemas(&json!([])),
            "#",
            "document must be an object",
        );
        assert_invalid(
            extract_schemas(&json!({ "swagger": "2.0" })),
            "#",
            "Swagger 2.0",
        );
        assert_invalid(
            extract_schemas(&json!({ "openapi": "4.0.0" })),
            "#/openapi",
            "unsupported OpenAPI version",
        );
        assert_invalid(
            extract_schemas(&json!({ "info": {} })),
            "#",
            "missing the `openapi` field",
        );
        assert_invalid(
            extract_schemas(&json!({
                "openapi": "3.1.0",
                "jsonSchemaDialect": "http://json-schema.org/draft-07/schema#",
            })),
            "#/jsonSchemaDialect",
            "unsupported schema dialect",
        );

        // The document dialect is respected: this is valid 3.1 but not 3.0.
        let nullable_array = json!({
            "components": {
                "schemas": {
                    "Tag": { "type": ["string", "null"] },
                },
            },
        });
        let mut document = nullable_array.clone();
        document["openapi"] = json!("3.0.0");
        assert_invalid(
            extract_schemas(&document),
            "#/components/schemas/Tag/type",
            "type arrays",
        );
        let mut document = nullable_array;
        document["openapi"] = json!("3.1.0");
        extract_schemas(&document).unwrap();
    }

    #[test]
    fn test_extract_schemas_dialect_field() {
        let document = json!({
            "openapi": "3.2.0",
            "jsonSchemaDialect": "https://spec.openapis.org/oas/3.2/dialect/base",
            "components": {
                "schemas": {
                    "Tags": {
                        "type": "array",
                        "prefixItems": [{ "type": "string" }],
                    },
                },
            },
        });
        let schemas = extract_schemas(&document).unwrap();
        assert_eq!(schemas.len(), 1);
    }
}
