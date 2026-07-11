# AGENTS.md

## Cursor Cloud specific instructions

`typify` is a Rust workspace (not a web app/service) that compiles JSON Schema
documents into Rust types. There is no long-running server to start; the
"application" is the `cargo-typify` CLI plus the `typify` library/macro crates.

### Toolchain
- The toolchain is pinned by `rust-toolchain.toml` (channel `1.89.0`, `default`
  profile) and is selected automatically by `rustup` inside the workspace, so
  the correct `cargo`/`rustc`/`rustfmt` are used without any manual switching.

### Standard commands
- Build: `cargo build --locked --tests`
- Lint / style (matches CI in `.github/workflows/rust.yml`): `cargo fmt -- --check`
- Test: `cargo test --locked`

### Running the CLI (non-obvious)
- `cargo-typify` is a `cargo` subcommand, so the binary expects `typify` as its
  first argument. Run it from source with:
  `cargo run -p cargo-typify -- typify <schema.json>` (note the extra `typify`).
- By default it writes `<schema>.rs` next to the input; use `--output -` to
  print generated Rust to stdout.

### Tests (non-obvious)
- Many tests are snapshot tests using the `expectorate` crate (expected output
  files live alongside the tests). If you intentionally change generated output,
  regenerate the expected files with `EXPECTORATE=overwrite cargo test` and
  review the diff before committing.
