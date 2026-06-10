# tonin-helm

**Helm chart generation and lifecycle management for [tonin](https://github.com/Rushit/tonin) services.**

[![CI](https://img.shields.io/github/actions/workflow/status/Rushit/tonin-helm/ci.yml?branch=main)](https://github.com/Rushit/tonin-helm/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

`tonin-helm` is a `tonin` plugin. Once installed, it becomes available as:

```bash
tonin helm generate     # generate Chart.yaml + values files from tonin.toml
tonin helm upgrade      # helm upgrade --install with the right values wired up
tonin helm diff         # helm diff upgrade (requires helm-diff plugin)
tonin helm status       # helm status
tonin helm template     # helm template
# … and more
```

No Helm chart YAML to hand-write. `tonin.toml` is still the single source of truth.

---

## Install

```bash
# 1. Install script (auto-detects OS/arch, installs to ~/.cargo/bin)
#    Recommended: installs both tonin + tonin-helm in one step.
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh | bash -s -- --with-tonin-helm

# Custom install directory
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh | bash -s -- --with-tonin-helm --dir /usr/local/bin

# 2. cargo-binstall (downloads the same pre-built archive)
cargo binstall tonin-helm

# 3. Build from source
cargo install tonin-helm
```

`tonin` dispatches to `tonin-helm` automatically once the binary is on your `$PATH` —
no extra configuration needed.

## Update

Re-running the install script upgrades to the latest release and skips the download if
already up to date.

```bash
# Update tonin-helm (and tonin) together
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh | bash -s -- --with-tonin-helm

# Update to a specific tonin-helm version
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh | bash -s -- --with-tonin-helm --helm-version v0.1.1

# Via cargo-binstall
cargo binstall tonin-helm
```

---

## Usage

### Generate a Helm chart

```bash
cd my-service
tonin helm generate              # generates chart/ in the current directory
tonin helm generate --env prod   # generate only the prod values overlay
tonin helm generate --out ./out  # write to a custom output path
```

This creates:

```
chart/
  Chart.yaml              # name, version, appVersion from tonin.toml
  values.yaml             # base values (replicas, image, resources, mesh, …)
  values-dev.yaml         # dev overlay
  values-staging.yaml     # staging overlay
  values-prod.yaml        # prod overlay
  templates/
    deployment.yaml       # standard Deployment
    service.yaml          # ClusterIP Service
    hpa.yaml              # HorizontalPodAutoscaler
    ingress.yaml          # Ingress (if expose is set)
```

Re-run `tonin helm generate` after editing `tonin.toml` — don't hand-edit the generated files.

### Deploy

```bash
tonin helm upgrade my-release --env prod          # helm upgrade --install + prod values
tonin helm diff    my-release --env staging        # see what would change
tonin helm status  my-release --env dev            # current release status
tonin helm rollback my-release --env prod          # helm rollback
tonin helm history  my-release --env prod          # release history
tonin helm uninstall my-release --env prod         # helm uninstall
```

All commands automatically pass `-f chart/values.yaml -f chart/values-<env>.yaml` so you
never need to remember which values files to combine.

### Escape hatch

Pass anything directly to `helm` after `--`:

```bash
tonin helm -- repo add stable https://charts.helm.sh/stable
tonin helm -- version
```

---

## How it works

`tonin-helm` reads `tonin.toml` via `tonin-plugin` and maps the resolved `Plan` into Helm
chart values:

| `tonin.toml` field | Helm output |
|--------------------|-------------|
| `[service].name` / `version` | `Chart.yaml` name + appVersion |
| `[deploy].replicas` | `values.yaml` replicaCount |
| `[deploy].namespace` | values + `helm upgrade -n <ns>` |
| `[deploy].mesh` | mesh annotations in values |
| `[deploy].expose` | Ingress host in values |
| `[resources].cpu` / `memory` | resource requests/limits |
| `[database]` / `[cache]` | values flags for StatefulSet deps |
| Per-env `[deploy.<env>]` overlays | `values-<env>.yaml` overrides |

---

## Requirements

### helm CLI

`tonin-helm` wraps `helm` — it must be installed and on your `$PATH` before use.

See the [official Helm install guide](https://helm.sh/docs/intro/install/) for all options.
Quick install to `~/.local/bin` (no sudo):

```bash
curl -fsSL -o get_helm.sh https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-4
chmod 700 get_helm.sh
HELM_INSTALL_DIR=$HOME/.local/bin USE_SUDO=false ./get_helm.sh
rm get_helm.sh
```

Verify: `helm version`

### Other requirements

- `tonin` CLI (for the `tonin helm` dispatch)
- A `tonin.toml` in the service directory

---

## Compatibility

| tonin-helm | tonin-plugin | tonin CLI |
|------------|--------------|-----------|
| 0.1.x      | 0.5.3+       | 0.5.3+    |

`tonin-helm` follows `tonin-plugin`'s semver independently. A `tonin-helm` patch bump
never requires a `tonin` CLI upgrade.

---

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
