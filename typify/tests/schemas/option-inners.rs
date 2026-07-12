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
#[doc = "a nullable enum whose null default belongs to the Option, not the inner enum"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"a nullable enum whose null default belongs to the Option, not the inner enum\","]
#[doc = "  \"default\": null,"]
#[doc = "  \"type\": ["]
#[doc = "    \"string\","]
#[doc = "    \"null\""]
#[doc = "  ],"]
#[doc = "  \"enum\": ["]
#[doc = "    \"on\","]
#[doc = "    \"off\","]
#[doc = "    null"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct DefaultedPhase(pub ::std::option::Option<DefaultedPhaseInner>);
impl ::std::ops::Deref for DefaultedPhase {
    type Target = ::std::option::Option<DefaultedPhaseInner>;
    fn deref(&self) -> &::std::option::Option<DefaultedPhaseInner> {
        &self.0
    }
}
impl ::std::convert::From<DefaultedPhase> for ::std::option::Option<DefaultedPhaseInner> {
    fn from(value: DefaultedPhase) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::option::Option<DefaultedPhaseInner>> for DefaultedPhase {
    fn from(value: ::std::option::Option<DefaultedPhaseInner>) -> Self {
        Self(value)
    }
}
#[doc = "a nullable enum whose null default belongs to the Option, not the inner enum"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"a nullable enum whose null default belongs to the Option, not the inner enum\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"on\","]
#[doc = "    \"off\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum DefaultedPhaseInner {
    #[serde(rename = "on")]
    On,
    #[serde(rename = "off")]
    Off,
}
impl ::std::fmt::Display for DefaultedPhaseInner {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::On => f.write_str("on"),
            Self::Off => f.write_str("off"),
        }
    }
}
impl ::std::str::FromStr for DefaultedPhaseInner {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "on" => Ok(Self::On),
            "off" => Ok(Self::Off),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for DefaultedPhaseInner {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DefaultedPhaseInner {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DefaultedPhaseInner {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "an anyOf-with-null wrap around an inline object"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"an anyOf-with-null wrap around an inline object\","]
#[doc = "  \"anyOf\": ["]
#[doc = "    {"]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"serial\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"serial\": {"]
#[doc = "          \"type\": \"string\""]
#[doc = "        }"]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"type\": \"null\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct NullablePart(pub ::std::option::Option<NullablePartInner>);
impl ::std::ops::Deref for NullablePart {
    type Target = ::std::option::Option<NullablePartInner>;
    fn deref(&self) -> &::std::option::Option<NullablePartInner> {
        &self.0
    }
}
impl ::std::convert::From<NullablePart> for ::std::option::Option<NullablePartInner> {
    fn from(value: NullablePart) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::option::Option<NullablePartInner>> for NullablePart {
    fn from(value: ::std::option::Option<NullablePartInner>) -> Self {
        Self(value)
    }
}
#[doc = "`NullablePartInner`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"serial\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"serial\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct NullablePartInner {
    pub serial: ::std::string::String,
}
impl NullablePartInner {
    pub fn builder() -> builder::NullablePartInner {
        Default::default()
    }
}
#[doc = "an anyOf-with-null wrap around a reference keeps the referenced type"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"an anyOf-with-null wrap around a reference keeps the referenced type\","]
#[doc = "  \"anyOf\": ["]
#[doc = "    {"]
#[doc = "      \"$ref\": \"#/definitions/widget-part\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"type\": \"null\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct NullableRef(pub ::std::option::Option<WidgetPart>);
impl ::std::ops::Deref for NullableRef {
    type Target = ::std::option::Option<WidgetPart>;
    fn deref(&self) -> &::std::option::Option<WidgetPart> {
        &self.0
    }
}
impl ::std::convert::From<NullableRef> for ::std::option::Option<WidgetPart> {
    fn from(value: NullableRef) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::option::Option<WidgetPart>> for NullableRef {
    fn from(value: ::std::option::Option<WidgetPart>) -> Self {
        Self(value)
    }
}
#[doc = "a string enum with a literal null member"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"a string enum with a literal null member\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"up\","]
#[doc = "    \"down\","]
#[doc = "    null"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Phase(pub ::std::option::Option<PhaseInner>);
impl ::std::ops::Deref for Phase {
    type Target = ::std::option::Option<PhaseInner>;
    fn deref(&self) -> &::std::option::Option<PhaseInner> {
        &self.0
    }
}
impl ::std::convert::From<Phase> for ::std::option::Option<PhaseInner> {
    fn from(value: Phase) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::option::Option<PhaseInner>> for Phase {
    fn from(value: ::std::option::Option<PhaseInner>) -> Self {
        Self(value)
    }
}
#[doc = "a string enum with a literal null member"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"a string enum with a literal null member\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"up\","]
#[doc = "    \"down\","]
#[doc = "    null"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum PhaseInner {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
}
impl ::std::fmt::Display for PhaseInner {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Up => f.write_str("up"),
            Self::Down => f.write_str("down"),
        }
    }
}
impl ::std::str::FromStr for PhaseInner {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for PhaseInner {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for PhaseInner {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PhaseInner {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "`WidgetPart`"]
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
#[doc = "    \"name\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct WidgetPart {
    pub name: ::std::string::String,
}
impl WidgetPart {
    pub fn builder() -> builder::WidgetPart {
        Default::default()
    }
}
#[doc = r" Types for composing complex structures."]
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct NullablePartInner {
        serial: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for NullablePartInner {
        fn default() -> Self {
            Self {
                serial: Err("no value supplied for serial".to_string()),
            }
        }
    }
    impl NullablePartInner {
        pub fn serial<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.serial = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for serial: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<NullablePartInner> for super::NullablePartInner {
        type Error = super::error::ConversionError;
        fn try_from(
            value: NullablePartInner,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                serial: value.serial?,
            })
        }
    }
    impl ::std::convert::From<super::NullablePartInner> for NullablePartInner {
        fn from(value: super::NullablePartInner) -> Self {
            Self {
                serial: Ok(value.serial),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct WidgetPart {
        name: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for WidgetPart {
        fn default() -> Self {
            Self {
                name: Err("no value supplied for name".to_string()),
            }
        }
    }
    impl WidgetPart {
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<WidgetPart> for super::WidgetPart {
        type Error = super::error::ConversionError;
        fn try_from(
            value: WidgetPart,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self { name: value.name? })
        }
    }
    impl ::std::convert::From<super::WidgetPart> for WidgetPart {
        fn from(value: super::WidgetPart) -> Self {
            Self {
                name: Ok(value.name),
            }
        }
    }
}
fn main() {}
