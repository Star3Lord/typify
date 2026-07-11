#![deny(warnings)]
pub mod bases {
    #[doc = r" Error types."]
    pub mod error {
        #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }
        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
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
    }
    #[doc = "`BaseThing`"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"type\": \"object\","]
    #[doc = "  \"required\": ["]
    #[doc = "    \"id\""]
    #[doc = "  ],"]
    #[doc = "  \"properties\": {"]
    #[doc = "    \"created\": {"]
    #[doc = "      \"type\": \"string\""]
    #[doc = "    },"]
    #[doc = "    \"id\": {"]
    #[doc = "      \"type\": \"string\""]
    #[doc = "    }"]
    #[doc = "  }"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct BaseThing {
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub created: ::std::option::Option<::std::string::String>,
        pub id: ::std::string::String,
    }
    #[doc = "`OtherBase`"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"type\": \"object\","]
    #[doc = "  \"properties\": {"]
    #[doc = "    \"note\": {"]
    #[doc = "      \"type\": \"string\""]
    #[doc = "    }"]
    #[doc = "  }"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct OtherBase {
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub note: ::std::option::Option<::std::string::String>,
    }
    impl ::std::default::Default for OtherBase {
        fn default() -> Self {
            Self {
                note: Default::default(),
            }
        }
    }
}
pub mod things {
    use super::bases::*;
    #[doc = r" Error types."]
    pub mod error {
        #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }
        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
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
    }
    #[doc = "the inline property collides with a base property, so this cannot compose and merges instead"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"description\": \"the inline property collides with a base property, so this cannot compose and merges instead\","]
    #[doc = "  \"allOf\": ["]
    #[doc = "    {"]
    #[doc = "      \"$ref\": \"#/definitions/base-thing\""]
    #[doc = "    },"]
    #[doc = "    {"]
    #[doc = "      \"type\": \"object\","]
    #[doc = "      \"properties\": {"]
    #[doc = "        \"id\": {"]
    #[doc = "          \"type\": \"string\""]
    #[doc = "        },"]
    #[doc = "        \"other\": {"]
    #[doc = "          \"type\": \"string\""]
    #[doc = "        }"]
    #[doc = "      }"]
    #[doc = "    }"]
    #[doc = "  ]"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct CollidingThing {
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub created: ::std::option::Option<::std::string::String>,
        pub id: ::std::string::String,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub other: ::std::option::Option<::std::string::String>,
    }
    #[doc = "the classic single-inheritance idiom"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"description\": \"the classic single-inheritance idiom\","]
    #[doc = "  \"allOf\": ["]
    #[doc = "    {"]
    #[doc = "      \"$ref\": \"#/definitions/base-thing\""]
    #[doc = "    },"]
    #[doc = "    {"]
    #[doc = "      \"type\": \"object\","]
    #[doc = "      \"required\": ["]
    #[doc = "        \"extra\""]
    #[doc = "      ],"]
    #[doc = "      \"properties\": {"]
    #[doc = "        \"extra\": {"]
    #[doc = "          \"type\": \"integer\""]
    #[doc = "        }"]
    #[doc = "      }"]
    #[doc = "    }"]
    #[doc = "  ]"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct DerivedThing {
        #[serde(flatten)]
        pub base_thing: BaseThing,
        pub extra: i64,
    }
    #[doc = "composition through a composed base"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"description\": \"composition through a composed base\","]
    #[doc = "  \"allOf\": ["]
    #[doc = "    {"]
    #[doc = "      \"$ref\": \"#/definitions/derived-thing\""]
    #[doc = "    },"]
    #[doc = "    {"]
    #[doc = "      \"type\": \"object\","]
    #[doc = "      \"properties\": {"]
    #[doc = "        \"grandchild\": {"]
    #[doc = "          \"type\": \"boolean\""]
    #[doc = "        }"]
    #[doc = "      }"]
    #[doc = "    }"]
    #[doc = "  ]"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct GrandchildThing {
        #[serde(flatten)]
        pub derived_thing: DerivedThing,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub grandchild: ::std::option::Option<bool>,
    }
    #[doc = "`MultiBaseThing`"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"allOf\": ["]
    #[doc = "    {"]
    #[doc = "      \"$ref\": \"#/definitions/base-thing\""]
    #[doc = "    },"]
    #[doc = "    {"]
    #[doc = "      \"$ref\": \"#/definitions/other-base\""]
    #[doc = "    }"]
    #[doc = "  ]"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct MultiBaseThing {
        #[serde(flatten)]
        pub base_thing: BaseThing,
        #[serde(flatten)]
        pub other_base: OtherBase,
    }
    #[doc = "non-object subschemas never compose"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"description\": \"non-object subschemas never compose\","]
    #[doc = "  \"allOf\": ["]
    #[doc = "    {"]
    #[doc = "      \"type\": \"string\""]
    #[doc = "    },"]
    #[doc = "    {"]
    #[doc = "      \"type\": \"string\","]
    #[doc = "      \"maxLength\": 5"]
    #[doc = "    }"]
    #[doc = "  ]"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct ScalarThing(::std::string::String);
    impl ::std::ops::Deref for ScalarThing {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<ScalarThing> for ::std::string::String {
        fn from(value: ScalarThing) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for ScalarThing {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 5usize {
                return Err("longer than 5 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for ScalarThing {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for ScalarThing {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for ScalarThing {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for ScalarThing {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    #[doc = "sibling properties alongside the allOf"]
    #[doc = r""]
    #[doc = r" <details><summary>JSON schema</summary>"]
    #[doc = r""]
    #[doc = r" ```json"]
    #[doc = "{"]
    #[doc = "  \"description\": \"sibling properties alongside the allOf\","]
    #[doc = "  \"type\": \"object\","]
    #[doc = "  \"allOf\": ["]
    #[doc = "    {"]
    #[doc = "      \"$ref\": \"#/definitions/base-thing\""]
    #[doc = "    }"]
    #[doc = "  ],"]
    #[doc = "  \"properties\": {"]
    #[doc = "    \"label\": {"]
    #[doc = "      \"type\": \"string\""]
    #[doc = "    }"]
    #[doc = "  }"]
    #[doc = "}"]
    #[doc = r" ```"]
    #[doc = r" </details>"]
    #[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
    pub struct SiblingThing {
        #[serde(flatten)]
        pub base_thing: BaseThing,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub label: ::std::option::Option<::std::string::String>,
    }
}
fn main() {}
