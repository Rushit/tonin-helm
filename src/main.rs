//! `tonin-helm` — Helm chart generation and lifecycle management for tonin services.
//!
//! Invoked either directly (`tonin-helm generate`) or via the tonin plugin
//! dispatch (`tonin helm generate`). When dispatched by tonin, the env vars
//! `TONIN_SERVICE_DIR` and `TONIN_CLI_VERSION` are set automatically.
//!
//! ## Usage
//!
//! ```text
//! tonin helm generate            # render chart/ from tonin.toml
//! tonin helm upgrade --env prod  # helm upgrade --install with context auto-resolved
//! tonin helm template            # helm template with context
//! tonin helm diff                # helm diff upgrade (requires helm-diff plugin)
//! tonin helm status              # helm status
//! tonin helm rollback            # helm rollback
//! tonin helm history             # helm history
//! tonin helm uninstall           # helm uninstall
//! tonin helm -- lint chart/      # raw passthrough to helm
//! ```

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{generate, proxy};

#[derive(Parser)]
#[command(
    name = "tonin-helm",
    version,
    about = "Helm chart generation and lifecycle management for tonin services",
    long_about = "Reads tonin.toml and either generates a Helm chart (generate) or \
                  wraps helm commands with auto-resolved release name, namespace, \
                  and values files.

EXAMPLES

  Generate chart from tonin.toml:
    tonin helm generate
    tonin helm generate --out ./deploy/chart

  Deploy to a cluster:
    tonin helm upgrade --env prod --atomic --wait
    tonin helm diff    --env prod
    tonin helm status

  Raw helm passthrough (prefix with --):
    tonin helm -- lint chart/
    tonin helm -- get manifest identity -n agnitiv

ENVIRONMENT

  TONIN_SERVICE_DIR   Set by `tonin` dispatch; the service directory.
  TONIN_CLI_VERSION   Set by `tonin` dispatch; the calling CLI version.
  TONIN_ENV           Default environment overlay (overridden by --env)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a Helm chart from `tonin.toml`.
    ///
    /// Writes `chart/Chart.yaml`, `chart/values.yaml`,
    /// `chart/values-<env>.yaml` (one per env), and the generic
    /// `chart/templates/` Go template files.
    Generate(generate::GenerateArgs),

    /// Deploy to a cluster: `helm upgrade --install` with context auto-resolved.
    Upgrade(proxy::UpgradeArgs),

    /// Render chart templates locally: `helm template` with context.
    Template(proxy::SimpleArgs),

    /// Show a diff against the live release (requires helm-diff plugin).
    Diff(proxy::SimpleArgs),

    /// Show the status of the live release.
    Status(proxy::SimpleArgs),

    /// Roll back the release to a previous revision.
    Rollback(proxy::SimpleArgs),

    /// Show the revision history of the release.
    History(proxy::SimpleArgs),

    /// Remove the release from the cluster.
    Uninstall(proxy::SimpleArgs),
}

fn main() -> Result<()> {
    // tonin plugin list --verbose calls this flag to get a one-line description.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--tonin-describe") {
        println!("Helm chart generation and lifecycle management for tonin services");
        return Ok(());
    }

    // `tonin helm -- <args>` arrives here as `["--", ...]` after tonin strips "helm".
    // clap can't parse `--` as a subcommand, so handle it before parsing.
    if args.get(1).map(String::as_str) == Some("--") {
        return proxy::run_raw(args[2..].to_vec());
    }

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Generate(a) => generate::run(a),
        Cmd::Upgrade(a) => proxy::run_upgrade(a),
        Cmd::Template(a) => proxy::run_template(a),
        Cmd::Diff(a) => proxy::run_diff(a),
        Cmd::Status(a) => proxy::run_status(a),
        Cmd::Rollback(a) => proxy::run_rollback(a),
        Cmd::History(a) => proxy::run_history(a),
        Cmd::Uninstall(a) => proxy::run_uninstall(a),
    }
}
