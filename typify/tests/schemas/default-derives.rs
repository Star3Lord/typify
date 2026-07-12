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
#[doc = "`DefaultedWidget`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"default\": {"]
#[doc = "    \"name\": \"widget\""]
#[doc = "  },"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"name\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"name\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"$comment\": \"generates its own Default impl from the schema default; the derive would conflict\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct DefaultedWidget {
    pub name: ::std::string::String,
}
impl DefaultedWidget {
    pub fn builder() -> builder::DefaultedWidget {
        Default::default()
    }
}
#[doc = "`DefaultedWidgetKind`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"default\": \"gear\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"gear\","]
#[doc = "    \"sprocket\""]
#[doc = "  ],"]
#[doc = "  \"$comment\": \"generates its own Default impl from the schema default\""]
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
pub enum DefaultedWidgetKind {
    #[serde(rename = "gear")]
    Gear,
    #[serde(rename = "sprocket")]
    Sprocket,
}
impl ::std::fmt::Display for DefaultedWidgetKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Gear => f.write_str("gear"),
            Self::Sprocket => f.write_str("sprocket"),
        }
    }
}
impl ::std::str::FromStr for DefaultedWidgetKind {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "gear" => Ok(Self::Gear),
            "sprocket" => Ok(Self::Sprocket),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for DefaultedWidgetKind {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DefaultedWidgetKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DefaultedWidgetKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::default::Default for DefaultedWidgetKind {
    fn default() -> Self {
        DefaultedWidgetKind::Gear
    }
}
#[doc = "`KindHolder`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"kind\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"kind\": {"]
#[doc = "      \"$ref\": \"#/definitions/defaulted-widget-kind\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"$comment\": \"derives Default: its required enum has a Default impl of its own\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct KindHolder {
    pub kind: DefaultedWidgetKind,
}
impl KindHolder {
    pub fn builder() -> builder::KindHolder {
        Default::default()
    }
}
#[doc = "`PlainWidget`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"count\","]
#[doc = "    \"name\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"count\": {"]
#[doc = "      \"type\": \"integer\""]
#[doc = "    },"]
#[doc = "    \"name\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"tags\": {"]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"$comment\": \"derives Default: every field type can\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct PlainWidget {
    pub count: i64,
    pub name: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub tags: ::std::vec::Vec<::std::string::String>,
}
impl PlainWidget {
    pub fn builder() -> builder::PlainWidget {
        Default::default()
    }
}
#[doc = "`SerialWidget`"]
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
#[doc = "    \"label\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"serial\": {"]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"format\": \"uint64\","]
#[doc = "      \"minimum\": 1.0"]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"$comment\": \"cannot derive Default: the required serial is a non-zero integer\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct SerialWidget {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub label: ::std::option::Option<::std::string::String>,
    pub serial: ::std::num::NonZeroU64,
}
impl SerialWidget {
    pub fn builder() -> builder::SerialWidget {
        Default::default()
    }
}
#[doc = "`SettledWidget`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"properties\": {"]
#[doc = "    \"name\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"priority\": {"]
#[doc = "      \"default\": 3,"]
#[doc = "      \"type\": \"integer\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"$comment\": \"generates its own Default impl because every property has a default\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct SettledWidget {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub name: ::std::option::Option<::std::string::String>,
    #[serde(default = "defaults::default_u64::<i64, 3>")]
    pub priority: i64,
}
impl ::std::default::Default for SettledWidget {
    fn default() -> Self {
        Self {
            name: Default::default(),
            priority: defaults::default_u64::<i64, 3>(),
        }
    }
}
impl SettledWidget {
    pub fn builder() -> builder::SettledWidget {
        Default::default()
    }
}
#[doc = "`WidgetHolder`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"widget\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"widget\": {"]
#[doc = "      \"$ref\": \"#/definitions/serial-widget\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"$comment\": \"cannot derive Default: it requires a type that cannot\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct WidgetHolder {
    pub widget: SerialWidget,
}
impl WidgetHolder {
    pub fn builder() -> builder::WidgetHolder {
        Default::default()
    }
}
#[doc = "`WidgetKind`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"gear\","]
#[doc = "    \"sprocket\""]
#[doc = "  ],"]
#[doc = "  \"$comment\": \"enums cannot derive Default without a designated variant\""]
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
pub enum WidgetKind {
    #[serde(rename = "gear")]
    Gear,
    #[serde(rename = "sprocket")]
    Sprocket,
}
impl ::std::fmt::Display for WidgetKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Gear => f.write_str("gear"),
            Self::Sprocket => f.write_str("sprocket"),
        }
    }
}
impl ::std::str::FromStr for WidgetKind {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "gear" => Ok(Self::Gear),
            "sprocket" => Ok(Self::Sprocket),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for WidgetKind {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WidgetKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WidgetKind {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = r" Types for composing complex structures."]
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct DefaultedWidget {
        name: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for DefaultedWidget {
        fn default() -> Self {
            Self {
                name: Err("no value supplied for name".to_string()),
            }
        }
    }
    impl DefaultedWidget {
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
    impl ::std::convert::TryFrom<DefaultedWidget> for super::DefaultedWidget {
        type Error = super::error::ConversionError;
        fn try_from(
            value: DefaultedWidget,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self { name: value.name? })
        }
    }
    impl ::std::convert::From<super::DefaultedWidget> for DefaultedWidget {
        fn from(value: super::DefaultedWidget) -> Self {
            Self {
                name: Ok(value.name),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct KindHolder {
        kind: ::std::result::Result<super::DefaultedWidgetKind, ::std::string::String>,
    }
    impl ::std::default::Default for KindHolder {
        fn default() -> Self {
            Self {
                kind: Err("no value supplied for kind".to_string()),
            }
        }
    }
    impl KindHolder {
        pub fn kind<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::DefaultedWidgetKind>,
            T::Error: ::std::fmt::Display,
        {
            self.kind = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for kind: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<KindHolder> for super::KindHolder {
        type Error = super::error::ConversionError;
        fn try_from(
            value: KindHolder,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self { kind: value.kind? })
        }
    }
    impl ::std::convert::From<super::KindHolder> for KindHolder {
        fn from(value: super::KindHolder) -> Self {
            Self {
                kind: Ok(value.kind),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct PlainWidget {
        count: ::std::result::Result<i64, ::std::string::String>,
        name: ::std::result::Result<::std::string::String, ::std::string::String>,
        tags: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
    }
    impl ::std::default::Default for PlainWidget {
        fn default() -> Self {
            Self {
                count: Err("no value supplied for count".to_string()),
                name: Err("no value supplied for name".to_string()),
                tags: Ok(Default::default()),
            }
        }
    }
    impl PlainWidget {
        pub fn count<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<i64>,
            T::Error: ::std::fmt::Display,
        {
            self.count = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for count: {e}"));
            self
        }
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
        pub fn tags<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.tags = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for tags: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<PlainWidget> for super::PlainWidget {
        type Error = super::error::ConversionError;
        fn try_from(
            value: PlainWidget,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                count: value.count?,
                name: value.name?,
                tags: value.tags?,
            })
        }
    }
    impl ::std::convert::From<super::PlainWidget> for PlainWidget {
        fn from(value: super::PlainWidget) -> Self {
            Self {
                count: Ok(value.count),
                name: Ok(value.name),
                tags: Ok(value.tags),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SerialWidget {
        label: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        serial: ::std::result::Result<::std::num::NonZeroU64, ::std::string::String>,
    }
    impl ::std::default::Default for SerialWidget {
        fn default() -> Self {
            Self {
                label: Ok(Default::default()),
                serial: Err("no value supplied for serial".to_string()),
            }
        }
    }
    impl SerialWidget {
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
        pub fn serial<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::num::NonZeroU64>,
            T::Error: ::std::fmt::Display,
        {
            self.serial = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for serial: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<SerialWidget> for super::SerialWidget {
        type Error = super::error::ConversionError;
        fn try_from(
            value: SerialWidget,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                label: value.label?,
                serial: value.serial?,
            })
        }
    }
    impl ::std::convert::From<super::SerialWidget> for SerialWidget {
        fn from(value: super::SerialWidget) -> Self {
            Self {
                label: Ok(value.label),
                serial: Ok(value.serial),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SettledWidget {
        name: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        priority: ::std::result::Result<i64, ::std::string::String>,
    }
    impl ::std::default::Default for SettledWidget {
        fn default() -> Self {
            Self {
                name: Ok(Default::default()),
                priority: Ok(super::defaults::default_u64::<i64, 3>()),
            }
        }
    }
    impl SettledWidget {
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {e}"));
            self
        }
        pub fn priority<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<i64>,
            T::Error: ::std::fmt::Display,
        {
            self.priority = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for priority: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<SettledWidget> for super::SettledWidget {
        type Error = super::error::ConversionError;
        fn try_from(
            value: SettledWidget,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                name: value.name?,
                priority: value.priority?,
            })
        }
    }
    impl ::std::convert::From<super::SettledWidget> for SettledWidget {
        fn from(value: super::SettledWidget) -> Self {
            Self {
                name: Ok(value.name),
                priority: Ok(value.priority),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct WidgetHolder {
        widget: ::std::result::Result<super::SerialWidget, ::std::string::String>,
    }
    impl ::std::default::Default for WidgetHolder {
        fn default() -> Self {
            Self {
                widget: Err("no value supplied for widget".to_string()),
            }
        }
    }
    impl WidgetHolder {
        pub fn widget<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::SerialWidget>,
            T::Error: ::std::fmt::Display,
        {
            self.widget = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for widget: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<WidgetHolder> for super::WidgetHolder {
        type Error = super::error::ConversionError;
        fn try_from(
            value: WidgetHolder,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                widget: value.widget?,
            })
        }
    }
    impl ::std::convert::From<super::WidgetHolder> for WidgetHolder {
        fn from(value: super::WidgetHolder) -> Self {
            Self {
                widget: Ok(value.widget),
            }
        }
    }
}
#[doc = r" Generation of default values for serde."]
pub mod defaults {
    pub(super) fn default_u64<T, const V: u64>() -> T
    where
        T: ::std::convert::TryFrom<u64>,
        <T as ::std::convert::TryFrom<u64>>::Error: ::std::fmt::Debug,
    {
        T::try_from(V).unwrap()
    }
}
fn main() {}
