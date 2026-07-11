// Copyright 2025 Oxide Computer Company

//! cargo command to generate Rust code from a JSON Schema or OpenAPI
//! document.

#![deny(missing_docs)]

use std::path::PathBuf;

use clap::{ArgGroup, Args};
use color_eyre::eyre::{Context, Result};
use typify::{
    AllOfStrategy, CrateVers, OptionalProperties, TypeSpace, TypeSpaceSettings, UnknownPolicy,
};

/// A CLI for the `typify` crate that converts JSON Schema files to Rust code.
#[derive(Args)]
#[command(author, version, about)]
#[command(group(
    ArgGroup::new("build")
        .args(["builder", "no_builder"]),
))]
pub struct CliArgs {
    /// The input file to read from: a JSON Schema document or an OpenAPI
    /// (3.0.x, 3.1.x, or 3.2.x) document, in JSON
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

    /// Specify the representation of non-required object properties:
    /// "explicit" represents every one as an Option<T>.
    #[arg(
        long = "optional-properties",
        value_parser = ["collapsed", "explicit"]
    )]
    optional_properties: Option<String>,

    /// Specify the interpretation of allOf constructions: "compose" embeds
    /// referenced base types as flattened members.
    #[arg(
        long = "all-of-strategy",
        value_parser = ["merge", "compose"]
    )]
    all_of_strategy: Option<String>,

    /// Give every plain string enum a catch-all variant of this name that
    /// losslessly round-trips values outside the documented set.
    #[arg(long = "open-enum-variant", value_name = "name")]
    open_enum_variant: Option<String>,

    /// Limit generated doc comments to the schema's description rather than
    /// embedding the whole JSON schema.
    #[arg(long = "no-schema-in-docs", default_value = "false")]
    no_schema_in_docs: bool,
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

/// Generate Rust code for the selected JSON Schema or OpenAPI document.
pub fn convert(args: &CliArgs) -> Result<String> {
    let content = std::fs::read_to_string(&args.input)
        .wrap_err_with(|| format!("Failed to open input file: {}", &args.input.display()))?;

    let input = serde_json::from_str::<serde_json::Value>(&content)
        .wrap_err("Failed to parse input file as JSON")?;

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

    if let Some(optional_properties) = &args.optional_properties {
        let policy = match optional_properties.as_str() {
            "collapsed" => OptionalProperties::Collapsed,
            "explicit" => OptionalProperties::Explicit,
            _ => unreachable!(),
        };
        settings.with_optional_properties(policy);
    }

    if let Some(all_of_strategy) = &args.all_of_strategy {
        let strategy = match all_of_strategy.as_str() {
            "merge" => AllOfStrategy::Merge,
            "compose" => AllOfStrategy::Compose,
            _ => unreachable!(),
        };
        settings.with_all_of_strategy(strategy);
    }

    if let Some(open_enum_variant) = &args.open_enum_variant {
        settings.with_open_enum_variant(open_enum_variant);
    }

    if args.no_schema_in_docs {
        settings.with_schema_in_docs(false);
    }

    let mut type_space = TypeSpace::new(&settings);
    // An `openapi` member indicates an OpenAPI document; anything else is
    // treated as a JSON Schema.
    if input.get("openapi").is_some() {
        type_space
            .add_openapi_document(&input)
            .wrap_err("OpenAPI document conversion failed")?;
    } else {
        let schema = serde_json::from_value::<schemars::schema::RootSchema>(input)
            .wrap_err("Failed to parse input file as JSON Schema")?;
        type_space
            .add_root_schema(schema)
            .wrap_err("Schema conversion failed")?;
    }

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

    #[test]
    fn test_output_parsing_stdout() {
        let args = CliArgs {
            input: PathBuf::from("input.json"),
            builder: false,
            additional_derives: vec![],
            additional_attrs: vec![],
            output: Some(PathBuf::from("-")),
            no_builder: false,
            crates: vec![],
            map_type: None,
            unknown_crates: Default::default(),
            optional_properties: Default::default(),
            all_of_strategy: Default::default(),
            open_enum_variant: Default::default(),
            no_schema_in_docs: Default::default(),
        };

        assert_eq!(args.output_path(), None);
    }

    #[test]
    fn test_output_parsing_file() {
        let args = CliArgs {
            input: PathBuf::from("input.json"),
            builder: false,
            additional_derives: vec![],
            additional_attrs: vec![],
            output: Some(PathBuf::from("some_file.rs")),
            no_builder: false,
            crates: vec![],
            map_type: None,
            unknown_crates: Default::default(),
            optional_properties: Default::default(),
            all_of_strategy: Default::default(),
            open_enum_variant: Default::default(),
            no_schema_in_docs: Default::default(),
        };

        assert_eq!(args.output_path(), Some(PathBuf::from("some_file.rs")));
    }

    #[test]
    fn test_output_parsing_default() {
        let args = CliArgs {
            input: PathBuf::from("input.json"),
            builder: false,
            additional_derives: vec![],
            additional_attrs: vec![],
            output: None,
            no_builder: false,
            crates: vec![],
            map_type: None,
            unknown_crates: Default::default(),
            optional_properties: Default::default(),
            all_of_strategy: Default::default(),
            open_enum_variant: Default::default(),
            no_schema_in_docs: Default::default(),
        };

        assert_eq!(args.output_path(), Some(PathBuf::from("input.rs")));
    }

    #[test]
    fn test_use_btree_map() {
        let args = CliArgs {
            input: PathBuf::from("input.json"),
            builder: false,
            additional_derives: vec![],
            additional_attrs: vec![],
            output: None,
            no_builder: false,
            crates: vec![],
            map_type: Some("::std::collections::BTreeMap".to_string()),
            unknown_crates: Default::default(),
            optional_properties: Default::default(),
            all_of_strategy: Default::default(),
            open_enum_variant: Default::default(),
            no_schema_in_docs: Default::default(),
        };

        assert_eq!(
            args.map_type,
            Some("::std::collections::BTreeMap".to_string())
        );
    }

    #[test]
    fn test_builder_as_default_style() {
        let args = CliArgs {
            input: PathBuf::from("input.json"),
            builder: false,
            additional_derives: vec![],
            additional_attrs: vec![],
            output: None,
            no_builder: false,
            crates: vec![],
            map_type: None,
            unknown_crates: Default::default(),
            optional_properties: Default::default(),
            all_of_strategy: Default::default(),
            open_enum_variant: Default::default(),
            no_schema_in_docs: Default::default(),
        };

        assert!(args.use_builder());
    }

    #[test]
    fn test_no_builder() {
        let args = CliArgs {
            input: PathBuf::from("input.json"),
            builder: false,
            additional_derives: vec![],
            additional_attrs: vec![],
            output: None,
            no_builder: true,
            crates: vec![],
            map_type: None,
            unknown_crates: Default::default(),
            optional_properties: Default::default(),
            all_of_strategy: Default::default(),
            open_enum_variant: Default::default(),
            no_schema_in_docs: Default::default(),
        };

        assert!(!args.use_builder());
    }

    #[test]
    fn test_builder_opt_in() {
        let args = CliArgs {
            input: PathBuf::from("input.json"),
            builder: true,
            additional_derives: vec![],
            additional_attrs: vec![],
            output: None,
            no_builder: false,
            crates: vec![],
            map_type: None,
            unknown_crates: Default::default(),
            optional_properties: Default::default(),
            all_of_strategy: Default::default(),
            open_enum_variant: Default::default(),
            no_schema_in_docs: Default::default(),
        };

        assert!(args.use_builder());
    }
}
