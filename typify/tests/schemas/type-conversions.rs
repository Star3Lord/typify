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
}
impl ::std::default::Default for ConversionHolder {
    fn default() -> Self {
        Self {
            id: Default::default(),
            note: Default::default(),
            price: Default::default(),
        }
    }
}
impl ConversionHolder {
    pub fn builder() -> builder::ConversionHolder {
        Default::default()
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
    }
    impl ::std::default::Default for ConversionHolder {
        fn default() -> Self {
            Self {
                id: Ok(Default::default()),
                note: Ok(Default::default()),
                price: Ok(Default::default()),
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
            })
        }
    }
    impl ::std::convert::From<super::ConversionHolder> for ConversionHolder {
        fn from(value: super::ConversionHolder) -> Self {
            Self {
                id: Ok(value.id),
                note: Ok(value.note),
                price: Ok(value.price),
            }
        }
    }
}
fn main() {}
