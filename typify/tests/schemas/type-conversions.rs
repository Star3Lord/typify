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
#[doc = "`ConversionHolder`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"ConversionHolder\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"properties\": {"]
#[doc = "    \"id\": {"]
#[doc = "      \"description\": \"matches a conversion despite the additional constraint and metadata\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"format\": \"uuid\","]
#[doc = "      \"maxLength\": 36"]
#[doc = "    },"]
#[doc = "    \"note\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"price\": {"]
#[doc = "      \"type\": \"number\","]
#[doc = "      \"format\": \"currency\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"sku\": {"]
#[doc = "      \"$ref\": \"#/definitions/sku\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"$comment\": \"validate schema-to-type conversions, in particular subset matching and specificity\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ConversionHolder {
    #[doc = "matches a conversion despite the additional constraint and metadata"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub note: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub price: ::std::option::Option<f64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub sku: ::std::option::Option<Sku>,
}
impl ::std::default::Default for ConversionHolder {
    fn default() -> Self {
        Self {
            id: Default::default(),
            note: Default::default(),
            price: Default::default(),
            sku: Default::default(),
        }
    }
}
impl ConversionHolder {
    pub fn builder() -> builder::ConversionHolder {
        Default::default()
    }
}
#[doc = "`Sku`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"pattern\": \"^S\","]
#[doc = "  \"$comment\": \"a named definition matching a string conversion becomes a newtype over the native string type\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct Sku(::std::string::String);
impl ::std::ops::Deref for Sku {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<Sku> for ::std::string::String {
    fn from(value: Sku) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for Sku {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("^S").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^S\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for Sku {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for Sku {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Sku {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for Sku {
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
#[doc = r" Types for composing complex structures."]
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct ConversionHolder {
        id: ::std::result::Result<::std::option::Option<::uuid::Uuid>, ::std::string::String>,
        note: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        price: ::std::result::Result<::std::option::Option<f64>, ::std::string::String>,
        sku: ::std::result::Result<::std::option::Option<super::Sku>, ::std::string::String>,
    }
    impl ::std::default::Default for ConversionHolder {
        fn default() -> Self {
            Self {
                id: Ok(Default::default()),
                note: Ok(Default::default()),
                price: Ok(Default::default()),
                sku: Ok(Default::default()),
            }
        }
    }
    impl ConversionHolder {
        pub fn id<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::uuid::Uuid>>,
            T::Error: ::std::fmt::Display,
        {
            self.id = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for id: {e}"));
            self
        }
        pub fn note<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.note = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for note: {e}"));
            self
        }
        pub fn price<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<f64>>,
            T::Error: ::std::fmt::Display,
        {
            self.price = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for price: {e}"));
            self
        }
        pub fn sku<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<super::Sku>>,
            T::Error: ::std::fmt::Display,
        {
            self.sku = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for sku: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<ConversionHolder> for super::ConversionHolder {
        type Error = super::error::ConversionError;
        fn try_from(
            value: ConversionHolder,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                id: value.id?,
                note: value.note?,
                price: value.price?,
                sku: value.sku?,
            })
        }
    }
    impl ::std::convert::From<super::ConversionHolder> for ConversionHolder {
        fn from(value: super::ConversionHolder) -> Self {
            Self {
                id: Ok(value.id),
                note: Ok(value.note),
                price: Ok(value.price),
                sku: Ok(value.sku),
            }
        }
    }
}
fn main() {}
