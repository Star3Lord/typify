// Copyright 2025 Oxide Computer Company

//! cargo command to generate Rust code from a JSON Schema.

#![deny(missing_docs)]

use std::path::PathBuf;

use clap::{ArgGroup, Args};
use color_eyre::eyre::{Context, Result};
use typify::{
    AllOfStrategy, ArrayOptionality, CrateVers, DeepPatchPolicy, DefaultBoolOptionality,
    DefaultedFieldOptionality, TypeSpace, TypeSpaceSettings, UnknownPolicy,
};

/// A CLI for the `typify` crate that converts JSON Schema files to Rust code.
#[derive(Args)]
#[command(author, version, about)]
#[command(group(
    ArgGroup::new("build")
        .args(["builder", "no_builder"]),
))]
pub struct CliArgs {
    /// The input file to read from
    pub input: PathBuf,

    /// Whether to include a builder-style interface, this is the default.
    #[arg(short, long, default_value = "false", group = "build")]
    pub builder: bool,

    /// Inverse of `--builder`. When set the builder-style interface will not
    /// be included.
    #[arg(short = 'B', long, default_value = "false", group = "build")]
    pub no_builder: bool,

    /// Add an additional derive macro to apply to all defined types.
    #[arg(short = 'd', long = "additional-derive", value_name = "derive")]
    pub additional_derives: Vec<String>,

    /// Add an additional attribute to apply to all defined types.
    #[arg(short = 'a', long = "additional-attr", value_name = "attr")]
    pub additional_attrs: Vec<String>,

    /// The output file to write to. If not specified, the input file name will
    /// be used with a `.rs` extension.
    ///
    /// If `-` is specified, the output will be written to stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Specify each crate@version that can be assumed to be in use for types
    /// found in the schema with the x-rust-type extension.
    #[arg(long = "crate")]
    crates: Vec<CrateSpec>,

    /// Specify the map like type to use.
    #[arg(long = "map-type")]
    map_type: Option<String>,

    /// Specify the policy unknown crates found in schemas with the
    /// x-rust-type extension.
    #[arg(
        long = "unknown-crates",
        value_parser = ["generate", "allow", "deny"]
    )]
    unknown_crates: Option<String>,

    /// Override the Rust type used for `{"type":"string","format":"date"}`
    /// schemas. The default is `::chrono::naive::NaiveDate`. Setting this
    /// to e.g. `::std::string::String` removes the chrono dependency.
    #[arg(long = "date-type", value_name = "PATH")]
    date_type: Option<String>,

    /// Override the Rust type used for
    /// `{"type":"string","format":"date-time"}` schemas. The default is
    /// `::chrono::DateTime<::chrono::offset::Utc>`.
    #[arg(long = "date-time-type", value_name = "PATH")]
    date_time_type: Option<String>,

    /// Override the Rust type used for `{"type":"string","format":"uuid"}`
    /// schemas. The default is `::uuid::Uuid`.
    #[arg(long = "uuid-type", value_name = "PATH")]
    uuid_type: Option<String>,

    /// Emit plain `String` instead of a per-field `#[serde(transparent)]`
    /// newtype for strings carrying `pattern` / `minLength` / `maxLength`
    /// constraints. Validation must then be enforced elsewhere.
    #[arg(long = "unconstrained-string", default_value = "false")]
    unconstrained_string: bool,

    /// Emit plain integer types (e.g. `i32`) instead of the corresponding
    /// `NonZeroU*` for integer schemas with `minimum: 1`.
    #[arg(long = "unconstrained-int", default_value = "false")]
    unconstrained_int: bool,

    /// Control how non-required array properties are rendered:
    /// `bare` (default) is `Vec<T>` with `#[serde(default,
    /// skip_serializing_if = "Vec::is_empty")]`; `optional` is
    /// `Option<Vec<T>>`.
    #[arg(
        long = "array-optionality",
        value_parser = ["bare", "optional"]
    )]
    array_optionality: Option<String>,

    /// Control how `bool` properties with a schema `default` are rendered:
    /// `bare` (default) is `bool` with `#[serde(default)]`; `option` is
    /// `Option<bool>`.
    #[arg(
        long = "default-bool-optionality",
        value_parser = ["bare", "option"]
    )]
    default_bool_optionality: Option<String>,

    /// Control how non-required, non-bool properties carrying a schema
    /// `default` are rendered: `bare` (default) keeps the field bare with
    /// `#[serde(default = "defaults::...")]`; `option` wraps the field in
    /// `Option<T>` and drops the schema default.
    #[arg(
        long = "defaulted-field-optionality",
        value_parser = ["bare", "option"]
    )]
    defaulted_field_optionality: Option<String>,

    /// Drop the per-field `#[serde(default, skip_serializing_if =
    /// "::std::option::Option::is_none")]` pair on `Option<T>` fields.
    /// Pair with a struct-level `#[serde_with::skip_serializing_none]`
    /// (e.g. via `--additional-attr`) so the omission semantics are
    /// preserved.
    #[arg(long = "elide-option-field-defaults", default_value = "false")]
    elide_option_field_defaults: bool,

    /// Emit `#[patch(name = "Option<{Inner}Patch>")]` above every
    /// `Option<{InnerStruct}>` field so that `struct_patch::Patch`
    /// produces a deep partial-merge type rather than a shallow
    /// `Option<Option<T>>`. Pair with a `struct_patch::Patch` derive.
    #[arg(long = "deep-patches", default_value = "false")]
    deep_patches: bool,

    /// Control how schema `allOf` compositions are rendered: `merge`
    /// (default) produces one flat struct containing the union of all
    /// properties; `compose` emits `#[serde(flatten)]` fields for `$ref`
    /// subschemas with inline subschemas folded in as ordinary fields.
    #[arg(
        long = "allof-strategy",
        value_parser = ["merge", "compose"]
    )]
    allof_strategy: Option<String>,

    /// Omit the pretty-printed JSON Schema `<details>` block from each
    /// generated type's doc comment (emitted by default, matching
    /// upstream), keeping IDE hover popovers minimal.
    #[arg(long = "no-schema-in-docs", default_value = "false")]
    no_schema_in_docs: bool,

    /// Add `AsRef<str>` / `Display` (and `From<&str>` for unconstrained
    /// ones) convenience impls to string-wrapping newtypes.
    #[arg(long = "string-newtype-conveniences", default_value = "false")]
    string_newtype_conveniences: bool,

    /// Add a `#[cfg_attr(feature = "<feature>", derive(<derive>))]` to every
    /// generated struct, enum, and newtype. Pass as `feature=DerivePath`
    /// (e.g. `--conditional-derive schemars=schemars::JsonSchema`).
    #[arg(
        long = "conditional-derive",
        value_name = "FEATURE=DERIVE",
        value_parser = parse_conditional
    )]
    conditional_derives: Vec<ConditionalSpec>,

    /// Add a `#[cfg_attr(feature = "<feature>", <attr>)]` to every generated
    /// struct, enum, and newtype. Pass as `feature=attr` (e.g.
    /// `--conditional-attr strict="serde(deny_unknown_fields)"`).
    #[arg(
        long = "conditional-attr",
        value_name = "FEATURE=ATTR",
        value_parser = parse_conditional
    )]
    conditional_attrs: Vec<ConditionalSpec>,
}

#[derive(Debug, Clone)]
struct ConditionalSpec {
    cfg: String,
    body: String,
}

fn parse_conditional(s: &str) -> std::result::Result<ConditionalSpec, String> {
    let (cfg, body) = s
        .split_once('=')
        .ok_or_else(|| "expected `feature=value`".to_string())?;
    if cfg.is_empty() || body.is_empty() {
        return Err("expected `feature=value` with non-empty parts".to_string());
    }
    Ok(ConditionalSpec {
        cfg: cfg.to_string(),
        body: body.to_string(),
    })
}

impl CliArgs {
    /// Output path.
    pub fn output_path(&self) -> Option<PathBuf> {
        match &self.output {
            Some(output_path) => {
                if output_path == &PathBuf::from("-") {
                    None
                } else {
                    Some(output_path.clone())
                }
            }
            None => {
                let mut output = self.input.clone();
                output.set_extension("rs");
                Some(output)
            }
        }
    }

    /// Whether builder-style interface was selected.
    pub fn use_builder(&self) -> bool {
        !self.no_builder
    }
}

#[derive(Debug, Clone)]
struct CrateSpec {
    name: String,
    version: CrateVers,
    rename: Option<String>,
}

impl std::str::FromStr for CrateSpec {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn is_crate(s: &str) -> bool {
            s.starts_with(|cc: char| cc.is_alphabetic() || cc == '_')
                && !s.contains(|cc: char| !cc.is_alphanumeric() && cc != '-' && cc != '_')
        }

        fn convert(s: &str) -> Option<CrateSpec> {
            let (rename, s) = if let Some(ii) = s.find('=') {
                let rename = &s[..ii];
                let rest = &s[ii + 1..];
                if !is_crate(rename) {
                    return None;
                }
                (Some(rename.to_string()), rest)
            } else {
                (None, s)
            };

            let ii = s.find('@')?;
            let crate_str = &s[..ii];
            let vers_str = &s[ii + 1..];

            if !is_crate(crate_str) {
                return None;
            }
            let version = CrateVers::parse(vers_str)?;

            Some(CrateSpec {
                name: crate_str.to_string(),
                version,
                rename,
            })
        }

        convert(s).ok_or("crate specifier must be of the form 'cratename@version'")
    }
}

/// Generate Rust code for the selected JSON Schema.
pub fn convert(args: &CliArgs) -> Result<String> {
    let content = std::fs::read_to_string(&args.input)
        .wrap_err_with(|| format!("Failed to open input file: {}", &args.input.display()))?;

    let schema = serde_json::from_str::<schemars::schema::RootSchema>(&content)
        .wrap_err("Failed to parse input file as JSON Schema")?;

    let mut settings = TypeSpaceSettings::default();
    settings.with_struct_builder(args.use_builder());

    for derive in &args.additional_derives {
        settings.with_derive(derive.clone());
    }

    for attr in &args.additional_attrs {
        settings.with_attr(attr.clone());
    }

    for CrateSpec {
        name,
        version,
        rename,
    } in &args.crates
    {
        settings.with_crate(name, version.clone(), rename.as_ref());
    }

    if let Some(map_type) = &args.map_type {
        settings.with_map_type(map_type.as_str());
    }

    if let Some(unknown_crates) = &args.unknown_crates {
        let unknown_crates = match unknown_crates.as_str() {
            "generate" => UnknownPolicy::Generate,
            "allow" => UnknownPolicy::Allow,
            "deny" => UnknownPolicy::Deny,
            _ => unreachable!(),
        };
        settings.with_unknown_crates(unknown_crates);
    }

    if let Some(date_type) = &args.date_type {
        settings.with_date_type(date_type);
    }
    if let Some(date_time_type) = &args.date_time_type {
        settings.with_date_time_type(date_time_type);
    }
    if let Some(uuid_type) = &args.uuid_type {
        settings.with_uuid_type(uuid_type);
    }
    if args.unconstrained_string {
        settings.with_unconstrained_string(true);
    }
    if args.unconstrained_int {
        settings.with_unconstrained_int(true);
    }
    if let Some(mode) = &args.array_optionality {
        let mode = match mode.as_str() {
            "bare" => ArrayOptionality::Bare,
            "optional" => ArrayOptionality::OptionalIfNotRequired,
            _ => unreachable!(),
        };
        settings.with_array_optionality(mode);
    }
    if let Some(mode) = &args.default_bool_optionality {
        let mode = match mode.as_str() {
            "bare" => DefaultBoolOptionality::Bare,
            "option" => DefaultBoolOptionality::AlwaysOption,
            _ => unreachable!(),
        };
        settings.with_default_bool_optionality(mode);
    }
    if let Some(mode) = &args.defaulted_field_optionality {
        let mode = match mode.as_str() {
            "bare" => DefaultedFieldOptionality::Bare,
            "option" => DefaultedFieldOptionality::AlwaysOption,
            _ => unreachable!(),
        };
        settings.with_defaulted_field_optionality(mode);
    }
    if args.elide_option_field_defaults {
        settings.with_elide_option_field_defaults(true);
    }
    if args.deep_patches {
        settings.with_deep_patches(DeepPatchPolicy::AllOptionStructs);
    }
    if let Some(strategy) = &args.allof_strategy {
        let strategy = match strategy.as_str() {
            "merge" => AllOfStrategy::Merge,
            "compose" => AllOfStrategy::Compose,
            _ => unreachable!(),
        };
        settings.with_allof_strategy(strategy);
    }
    if args.no_schema_in_docs {
        settings.with_schema_in_docs(false);
    }
    if args.string_newtype_conveniences {
        settings.with_string_newtype_conveniences(true);
    }
    for conditional in &args.conditional_derives {
        settings.with_conditional_derive(&conditional.cfg, &conditional.body);
    }
    for conditional in &args.conditional_attrs {
        settings.with_conditional_attr(&conditional.cfg, &conditional.body);
    }

    let mut type_space = TypeSpace::new(&settings);
    type_space
        .add_root_schema(schema)
        .wrap_err("Schema conversion failed")?;

    let intro = "#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]
";

    let contents = format!("{intro}\n{}", type_space.to_stream());

    let contents = rustfmt_wrapper::rustfmt(contents).wrap_err("Failed to format Rust code")?;

    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_with(
        input: &str,
        output: Option<PathBuf>,
        no_builder: bool,
        map_type: Option<String>,
    ) -> CliArgs {
        CliArgs {
            input: PathBuf::from(input),
            builder: false,
            additional_derives: vec![],
            additional_attrs: vec![],
            output,
            no_builder,
            crates: vec![],
            map_type,
            unknown_crates: Default::default(),
            date_type: None,
            date_time_type: None,
            uuid_type: None,
            unconstrained_string: false,
            unconstrained_int: false,
            array_optionality: None,
            default_bool_optionality: None,
            defaulted_field_optionality: None,
            elide_option_field_defaults: false,
            deep_patches: false,
            allof_strategy: None,
            no_schema_in_docs: false,
            string_newtype_conveniences: false,
            conditional_derives: vec![],
            conditional_attrs: vec![],
        }
    }

    #[test]
    fn test_output_parsing_stdout() {
        let args = args_with("input.json", Some(PathBuf::from("-")), false, None);
        assert_eq!(args.output_path(), None);
    }

    #[test]
    fn test_output_parsing_file() {
        let args = args_with(
            "input.json",
            Some(PathBuf::from("some_file.rs")),
            false,
            None,
        );
        assert_eq!(args.output_path(), Some(PathBuf::from("some_file.rs")));
    }

    #[test]
    fn test_output_parsing_default() {
        let args = args_with("input.json", None, false, None);
        assert_eq!(args.output_path(), Some(PathBuf::from("input.rs")));
    }

    #[test]
    fn test_use_btree_map() {
        let args = args_with(
            "input.json",
            None,
            false,
            Some("::std::collections::BTreeMap".to_string()),
        );
        assert_eq!(
            args.map_type,
            Some("::std::collections::BTreeMap".to_string())
        );
    }

    #[test]
    fn test_builder_as_default_style() {
        let args = args_with("input.json", None, false, None);
        assert!(args.use_builder());
    }

    #[test]
    fn test_no_builder() {
        let args = args_with("input.json", None, true, None);
        assert!(!args.use_builder());
    }

    #[test]
    fn test_builder_opt_in() {
        let mut args = args_with("input.json", None, false, None);
        args.builder = true;
        assert!(args.use_builder());
    }

    #[test]
    fn test_parse_conditional_ok() {
        let spec = parse_conditional("schemars=schemars::JsonSchema").unwrap();
        assert_eq!(spec.cfg, "schemars");
        assert_eq!(spec.body, "schemars::JsonSchema");
    }

    #[test]
    fn test_parse_conditional_err() {
        assert!(parse_conditional("nope").is_err());
        assert!(parse_conditional("=value").is_err());
        assert!(parse_conditional("feature=").is_err());
    }
}
