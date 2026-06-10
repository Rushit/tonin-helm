# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

`tonin-helm` is a `tonin` plugin binary (`tonin-<name>` dispatch protocol). It generates
Helm charts from `tonin.toml` and proxies lifecycle commands (`upgrade`, `diff`, `status`,
etc.) to the `helm` CLI with the correct values files wired up automatically.

## Common commands

Pinned toolchain: Rust `1.90` (`rust-toolchain.toml` is inherited from the tonin workspace
when developing locally; otherwise the toolchain is declared in `Cargo.toml` via
`rust-version = "1.90"`).

```bash
# Build / check
cargo build
cargo check --all-targets

# Lint + format
cargo fmt
cargo clippy --all-targets -- -D warnings

# Tests
cargo test

# Full CI gate (same as what CI runs)
make ci

# Install locally
make install          # cargo install --path .

# Run directly
cargo run -- generate --path examples/greeter
cargo run -- --tonin-describe   # prints one-line description (plugin protocol)

# Release
make release VERSION=X.Y.Z      # bumps, commits, tags, pushes → fires release CI
```

## Architecture

Single binary crate (`src/main.rs` → `tonin-helm` binary).

### Entry point (`src/main.rs`)

Two special cases handled **before** clap parses argv:

1. `--tonin-describe` — prints one-line description and exits 0. Required by the tonin
   plugin protocol so `tonin plugin list --verbose` can show a description.
2. `-- [args...]` — raw passthrough directly to `helm`, for escape-hatch use.

After those, `Cmd` enum dispatches to one of two families:

### Commands (`src/commands/`)

- **`generate.rs`** — the primary command. Reads `tonin.toml` via `tonin-plugin`, renders
  `Chart.yaml` + `values.yaml` + per-env `values-<env>.yaml` files via Tera, then copies
  static Go template files verbatim from the embedded `chart-templates/` tree.
  Output lands in `<service-dir>/chart/` (or `--out <path>`).

- **`proxy.rs`** — all other subcommands (`upgrade`, `template`, `diff`, `status`,
  `rollback`, `history`, `uninstall`). Each resolves the chart directory and the right
  values files (`chart/values.yaml` + `chart/values-<env>.yaml`), then `exec()`s into
  `helm` with those flags prepended. On Unix this is a true process replacement; on
  non-Unix it spawns and forwards the exit code.

### Templates (`templates/`)

Two categories, both embedded via `include_dir`:

- **Tera templates** (`*.tmpl`) — `Chart.yaml.tmpl`, `values.yaml.tmpl`,
  `values-env.yaml.tmpl`. Rendered with a context built from the resolved `Plan`.
- **Static Go templates** (`chart-templates/`) — `deployment.yaml`, `service.yaml`,
  `hpa.yaml`, `ingress.yaml`. Copied verbatim to `chart/templates/`; not processed by
  Tera (they contain `{{ }}` Go template syntax that would conflict).

### Dependency on tonin-plugin

`tonin-plugin` is the only internal tonin dep. It provides:
- `Plan::load_with_env(path, env)` — parses `tonin.toml` + resolves env overlays
- `select_env(hint)` — `$TONIN_ENV` → hint → `"dev"`
- All resolved types: `DatabaseSpec`, `CacheSpec`, `Mesh`, `ServiceKind`, etc.

**Git dep status:** `Cargo.toml` uses `git = "https://github.com/Rushit/tonin", tag = "v0.5.2"`.
This resolves directly from GitHub so CI needs only a single checkout and contributors
don't need the `tonin` repo alongside. Once `tonin-plugin` is published to crates.io,
replace with the plain version dep `tonin-plugin = "0.5.2"`.

## Adding a command

1. Add a variant to the `Cmd` enum in `src/main.rs`.
2. If it's a `helm` proxy (wraps `helm <subcmd>`): add a handler in `src/commands/proxy.rs`
   following the existing pattern — `CommonArgs` + `resolve_context()` + build `helm` command.
3. If it generates files: add a module under `src/commands/` and add templates under
   `templates/` as needed.
4. Update `--tonin-describe` output in `src/main.rs` if the new command changes the plugin's
   high-level purpose.

## Adding a template field

1. Add the field to `build_context()` in `src/commands/generate.rs`.
2. Use it in the relevant `.tmpl` file under `templates/`.
3. Add a test or run `cargo run -- generate --path <example>` to verify the output.

All template changes must be backward-compatible (new fields should have safe defaults so
existing `tonin.toml` files continue to render correctly).

## Versioning

`VERSION` is the source of truth. `scripts/bump-version.sh` keeps `Cargo.toml
[package].version` in sync. Use `make release VERSION=X.Y.Z` — never edit `VERSION`
or `Cargo.toml` directly.

The `tonin-plugin` version pinned in `Cargo.toml` and the `tonin` CLI version that users
have installed are independent. When `tonin-plugin` bumps (i.e. `tonin.toml` schema gains
a new field), bump `tonin-helm` too so `build_context()` can consume the new field.

## Definition of done

- `make ci` passes (fmt-check + clippy `-D warnings` + test + doc).
- New template fields have a render test or a `make gen-example` equivalent.
- Commit messages follow Conventional Commits (`feat:`, `fix:`, `chore:`, …).
