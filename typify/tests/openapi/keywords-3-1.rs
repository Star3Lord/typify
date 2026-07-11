#![deny(warnings)]
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
#[doc = "`Coordinates`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"array\","]
#[doc = "  \"items\": ["]
#[doc = "    {"]
#[doc = "      \"type\": \"number\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"type\": \"number\""]
#[doc = "    }"]
#[doc = "  ],"]
#[doc = "  \"maxItems\": 2,"]
#[doc = "  \"minItems\": 2"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Coordinates(pub (f64, f64));
impl ::std::ops::Deref for Coordinates {
    type Target = (f64, f64);
    fn deref(&self) -> &(f64, f64) {
        &self.0
    }
}
impl ::std::convert::From<Coordinates> for (f64, f64) {
    fn from(value: Coordinates) -> Self {
        value.0
    }
}
impl ::std::convert::From<(f64, f64)> for Coordinates {
    fn from(value: (f64, f64)) -> Self {
        Self(value)
    }
}
#[doc = "`IdOrName`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": ["]
#[doc = "    \"integer\","]
#[doc = "    \"string\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum IdOrName {
    Integer(i64),
    String(::std::string::String),
}
impl ::std::fmt::Display for IdOrName {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            Self::Integer(x) => x.fmt(f),
            Self::String(x) => x.fmt(f),
        }
    }
}
impl ::std::convert::From<i64> for IdOrName {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}
#[doc = "A tuple with a label followed by measurements"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A tuple with a label followed by measurements\","]
#[doc = "  \"type\": \"array\","]
#[doc = "  \"items\": ["]
#[doc = "    {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  ],"]
#[doc = "  \"additionalItems\": {"]
#[doc = "    \"type\": \"integer\""]
#[doc = "  },"]
#[doc = "  \"maxItems\": 3,"]
#[doc = "  \"minItems\": 3"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct LabeledRow(pub (::std::string::String, i64, i64));
impl ::std::ops::Deref for LabeledRow {
    type Target = (::std::string::String, i64, i64);
    fn deref(&self) -> &(::std::string::String, i64, i64) {
        &self.0
    }
}
impl ::std::convert::From<LabeledRow> for (::std::string::String, i64, i64) {
    fn from(value: LabeledRow) -> Self {
        value.0
    }
}
impl ::std::convert::From<(::std::string::String, i64, i64)> for LabeledRow {
    fn from(value: (::std::string::String, i64, i64)) -> Self {
        Self(value)
    }
}
#[doc = "A converted document may carry 3.0-isms such as nullable"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A converted document may carry 3.0-isms such as nullable\","]
#[doc = "  \"type\": ["]
#[doc = "    \"string\","]
#[doc = "    \"null\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Nickname(pub ::std::option::Option<::std::string::String>);
impl ::std::ops::Deref for Nickname {
    type Target = ::std::option::Option<::std::string::String>;
    fn deref(&self) -> &::std::option::Option<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<Nickname> for ::std::option::Option<::std::string::String> {
    fn from(value: Nickname) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::option::Option<::std::string::String>> for Nickname {
    fn from(value: ::std::option::Option<::std::string::String>) -> Self {
        Self(value)
    }
}
#[doc = "`Pet`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"name\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"age\": {"]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"maximum\": 30.0,"]
#[doc = "      \"exclusiveMinimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"location\": {"]
#[doc = "      \"$ref\": \"#/components/schemas/Coordinates\""]
#[doc = "    },"]
#[doc = "    \"name\": {"]
#[doc = "      \"examples\": ["]
#[doc = "        \"fido\""]
#[doc = "      ],"]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"nickname\": {"]
#[doc = "      \"type\": ["]
#[doc = "        \"string\","]
#[doc = "        \"null\""]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Pet {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub age: ::std::option::Option<::std::num::NonZeroU64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub location: ::std::option::Option<Coordinates>,
    pub name: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub nickname: ::std::option::Option<::std::string::String>,
}
#[doc = "`Pets`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"array\","]
#[doc = "  \"items\": {"]
#[doc = "    \"$ref\": \"#/components/schemas/Pet\""]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Pets(pub ::std::vec::Vec<Pet>);
impl ::std::ops::Deref for Pets {
    type Target = ::std::vec::Vec<Pet>;
    fn deref(&self) -> &::std::vec::Vec<Pet> {
        &self.0
    }
}
impl ::std::convert::From<Pets> for ::std::vec::Vec<Pet> {
    fn from(value: Pets) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::vec::Vec<Pet>> for Pets {
    fn from(value: ::std::vec::Vec<Pet>) -> Self {
        Self(value)
    }
}
fn main() {}
