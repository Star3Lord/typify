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
impl BaseThing {
    pub fn builder() -> builder::BaseThing {
        Default::default()
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
impl CollidingThing {
    pub fn builder() -> builder::CollidingThing {
        Default::default()
    }
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
impl DerivedThing {
    pub fn builder() -> builder::DerivedThing {
        Default::default()
    }
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
impl GrandchildThing {
    pub fn builder() -> builder::GrandchildThing {
        Default::default()
    }
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
impl MultiBaseThing {
    pub fn builder() -> builder::MultiBaseThing {
        Default::default()
    }
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
impl OtherBase {
    pub fn builder() -> builder::OtherBase {
        Default::default()
    }
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
impl SiblingThing {
    pub fn builder() -> builder::SiblingThing {
        Default::default()
    }
}
#[doc = r" Types for composing complex structures."]
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct BaseThing {
        created: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        id: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for BaseThing {
        fn default() -> Self {
            Self {
                created: Ok(Default::default()),
                id: Err("no value supplied for id".to_string()),
            }
        }
    }
    impl BaseThing {
        pub fn created<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.created = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for created: {e}"));
            self
        }
        pub fn id<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.id = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for id: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<BaseThing> for super::BaseThing {
        type Error = super::error::ConversionError;
        fn try_from(
            value: BaseThing,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                created: value.created?,
                id: value.id?,
            })
        }
    }
    impl ::std::convert::From<super::BaseThing> for BaseThing {
        fn from(value: super::BaseThing) -> Self {
            Self {
                created: Ok(value.created),
                id: Ok(value.id),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct CollidingThing {
        created: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        id: ::std::result::Result<::std::string::String, ::std::string::String>,
        other: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
    }
    impl ::std::default::Default for CollidingThing {
        fn default() -> Self {
            Self {
                created: Ok(Default::default()),
                id: Err("no value supplied for id".to_string()),
                other: Ok(Default::default()),
            }
        }
    }
    impl CollidingThing {
        pub fn created<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.created = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for created: {e}"));
            self
        }
        pub fn id<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.id = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for id: {e}"));
            self
        }
        pub fn other<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.other = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for other: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<CollidingThing> for super::CollidingThing {
        type Error = super::error::ConversionError;
        fn try_from(
            value: CollidingThing,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                created: value.created?,
                id: value.id?,
                other: value.other?,
            })
        }
    }
    impl ::std::convert::From<super::CollidingThing> for CollidingThing {
        fn from(value: super::CollidingThing) -> Self {
            Self {
                created: Ok(value.created),
                id: Ok(value.id),
                other: Ok(value.other),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct DerivedThing {
        base_thing: ::std::result::Result<super::BaseThing, ::std::string::String>,
        extra: ::std::result::Result<i64, ::std::string::String>,
    }
    impl ::std::default::Default for DerivedThing {
        fn default() -> Self {
            Self {
                base_thing: Err("no value supplied for base_thing".to_string()),
                extra: Err("no value supplied for extra".to_string()),
            }
        }
    }
    impl DerivedThing {
        pub fn base_thing<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::BaseThing>,
            T::Error: ::std::fmt::Display,
        {
            self.base_thing = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for base_thing: {e}"));
            self
        }
        pub fn extra<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<i64>,
            T::Error: ::std::fmt::Display,
        {
            self.extra = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for extra: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<DerivedThing> for super::DerivedThing {
        type Error = super::error::ConversionError;
        fn try_from(
            value: DerivedThing,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                base_thing: value.base_thing?,
                extra: value.extra?,
            })
        }
    }
    impl ::std::convert::From<super::DerivedThing> for DerivedThing {
        fn from(value: super::DerivedThing) -> Self {
            Self {
                base_thing: Ok(value.base_thing),
                extra: Ok(value.extra),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct GrandchildThing {
        derived_thing: ::std::result::Result<super::DerivedThing, ::std::string::String>,
        grandchild: ::std::result::Result<::std::option::Option<bool>, ::std::string::String>,
    }
    impl ::std::default::Default for GrandchildThing {
        fn default() -> Self {
            Self {
                derived_thing: Err("no value supplied for derived_thing".to_string()),
                grandchild: Ok(Default::default()),
            }
        }
    }
    impl GrandchildThing {
        pub fn derived_thing<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::DerivedThing>,
            T::Error: ::std::fmt::Display,
        {
            self.derived_thing = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for derived_thing: {e}"));
            self
        }
        pub fn grandchild<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<bool>>,
            T::Error: ::std::fmt::Display,
        {
            self.grandchild = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for grandchild: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<GrandchildThing> for super::GrandchildThing {
        type Error = super::error::ConversionError;
        fn try_from(
            value: GrandchildThing,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                derived_thing: value.derived_thing?,
                grandchild: value.grandchild?,
            })
        }
    }
    impl ::std::convert::From<super::GrandchildThing> for GrandchildThing {
        fn from(value: super::GrandchildThing) -> Self {
            Self {
                derived_thing: Ok(value.derived_thing),
                grandchild: Ok(value.grandchild),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct MultiBaseThing {
        base_thing: ::std::result::Result<super::BaseThing, ::std::string::String>,
        other_base: ::std::result::Result<super::OtherBase, ::std::string::String>,
    }
    impl ::std::default::Default for MultiBaseThing {
        fn default() -> Self {
            Self {
                base_thing: Err("no value supplied for base_thing".to_string()),
                other_base: Err("no value supplied for other_base".to_string()),
            }
        }
    }
    impl MultiBaseThing {
        pub fn base_thing<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::BaseThing>,
            T::Error: ::std::fmt::Display,
        {
            self.base_thing = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for base_thing: {e}"));
            self
        }
        pub fn other_base<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::OtherBase>,
            T::Error: ::std::fmt::Display,
        {
            self.other_base = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for other_base: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<MultiBaseThing> for super::MultiBaseThing {
        type Error = super::error::ConversionError;
        fn try_from(
            value: MultiBaseThing,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                base_thing: value.base_thing?,
                other_base: value.other_base?,
            })
        }
    }
    impl ::std::convert::From<super::MultiBaseThing> for MultiBaseThing {
        fn from(value: super::MultiBaseThing) -> Self {
            Self {
                base_thing: Ok(value.base_thing),
                other_base: Ok(value.other_base),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct OtherBase {
        note: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
    }
    impl ::std::default::Default for OtherBase {
        fn default() -> Self {
            Self {
                note: Ok(Default::default()),
            }
        }
    }
    impl OtherBase {
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
    }
    impl ::std::convert::TryFrom<OtherBase> for super::OtherBase {
        type Error = super::error::ConversionError;
        fn try_from(
            value: OtherBase,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self { note: value.note? })
        }
    }
    impl ::std::convert::From<super::OtherBase> for OtherBase {
        fn from(value: super::OtherBase) -> Self {
            Self {
                note: Ok(value.note),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SiblingThing {
        base_thing: ::std::result::Result<super::BaseThing, ::std::string::String>,
        label: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
    }
    impl ::std::default::Default for SiblingThing {
        fn default() -> Self {
            Self {
                base_thing: Err("no value supplied for base_thing".to_string()),
                label: Ok(Default::default()),
            }
        }
    }
    impl SiblingThing {
        pub fn base_thing<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::BaseThing>,
            T::Error: ::std::fmt::Display,
        {
            self.base_thing = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for base_thing: {e}"));
            self
        }
        pub fn label<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.label = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for label: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<SiblingThing> for super::SiblingThing {
        type Error = super::error::ConversionError;
        fn try_from(
            value: SiblingThing,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                base_thing: value.base_thing?,
                label: value.label?,
            })
        }
    }
    impl ::std::convert::From<super::SiblingThing> for SiblingThing {
        fn from(value: super::SiblingThing) -> Self {
            Self {
                base_thing: Ok(value.base_thing),
                label: Ok(value.label),
            }
        }
    }
}
fn main() {}
