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
- **Static Go templates** (`chart-templates/`) — `deployment.yaml` (with a
  migrations initContainer + stateful/secret env injection), `service.yaml`,
  `hpa.yaml`, `ingress.yaml`, `db-statefulset.yaml`, `db-service.yaml`,
  `cache-statefulset.yaml`, `cache-service.yaml`, `secret.yaml`, and
  `networkpolicy.yaml` (CiliumNetworkPolicy), and `extra-manifests.yaml`
  (deploy-time `.Values.extraManifests`). Each is gated on `.Values` so
  absent capabilities render nothing; `shared` DB/cache render no StatefulSet.
  Escape hatches: `podAnnotations` (generated from mesh — cilium encryption),
  `extraEnv` + `extraManifests` (deploy-time, default `[]` so regeneration never
  clobbers them). Durable per-service manifests go in a custom `chart/templates/`
  file, which `generate` does not overwrite.
  MCP renders in one of two modes via `mcp.mode`: `in-process` (default — the
  same server container exposes `service.mcpPort`; matches tonin's
  `Service::new().mcp_addr(...)` runtime, one binary/two ports) or `sidecar` (a
  separate `<name>-mcp` container, only if the image ships that binary).
  Migrations render in one of two modes via `migrations.mode`: `init-container`
  (default — initContainer per pod; dev/owned-DB) or `job` (`pre-install`/
  `pre-upgrade` hook Job that gates the rollout; prod/managed-DB, needs
  `database.shared=true` + `secrets.create=false`). `migrations.env` lands on the
  migration step only.
  Mesh policy is **Cilium only** today (Istio/Linkerd are a known follow-up to
  reach full `tonin k8s generate` parity). Copied verbatim to `chart/templates/`;
  not processed by
  Tera (they contain `{{ }}` Go template syntax that would conflict).

### Dependency on tonin-plugin

`tonin-plugin` is the only internal tonin dep. It provides:
- `Plan::load_with_env(path, env)` — parses `tonin.toml` + resolves env overlays
- `select_env(hint)` — `$TONIN_ENV` → hint → `"dev"`
- All resolved types: `DatabaseSpec`, `CacheSpec`, `Mesh`, `ServiceKind`, etc.

**Version dep:** `Cargo.toml` uses `tonin-plugin = "0.6"` (crates.io). Cargo resolves
the latest compatible patch automatically. When `tonin-plugin` gains a new field that
`tonin-helm` needs, bump the lower bound here to match — `tonin-helm` 0.3.x requires
`tonin-plugin`/`tonin` **0.6.0+** (per-environment namespaces and dependencies).

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
