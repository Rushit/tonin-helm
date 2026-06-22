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

**Install tonin + tonin-helm (recommended):**
```bash
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh \
  | bash -s -- --with-tonin-helm
```

**Custom install directory:**
```bash
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh \
  | bash -s -- --with-tonin-helm --dir /usr/local/bin
```

**Via cargo-binstall:**
```bash
cargo binstall tonin-helm
```

**Build from source:**
```bash
cargo install tonin-helm
```

`tonin` dispatches to `tonin-helm` automatically once the binary is on your `$PATH` —
no extra configuration needed.

## Update

The simplest path is `tonin upgrade`, which upgrades the `tonin` CLI and every installed
plugin (including `tonin-helm`) together after showing a plan. `tonin-helm` reports its
version and required CLI via `--tonin-meta`, and warns when dispatched by an out-of-date
`tonin`. Re-running the install script directly also works and skips the download if
already up to date.

**Update tonin + tonin-helm to latest:**
```bash
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh \
  | bash -s -- --with-tonin-helm
```

**Pin a specific tonin-helm version:**
```bash
curl -sSfL https://raw.githubusercontent.com/Rushit/tonin/main/scripts/install.sh \
  | bash -s -- \
      --with-tonin-helm \
      --helm-version v0.1.1
```

**Via cargo-binstall:**
```bash
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
    deployment.yaml       # Deployment (+ migrations initContainer, stateful env)
    service.yaml          # ClusterIP Service
    hpa.yaml              # HorizontalPodAutoscaler (if autoscaling)
    ingress.yaml          # Ingress (if expose is set)
    db-statefulset.yaml   # owned database StatefulSet (if [database], not shared)
    db-service.yaml       # headless DB Service
    cache-statefulset.yaml # owned cache StatefulSet (if [cache], not shared)
    cache-service.yaml    # headless cache Service
    secret.yaml           # Secret for DB password + app secrets (values-driven)
    networkpolicy.yaml    # CiliumNetworkPolicy (if mesh = cilium)
    migration-job.yaml    # pre-upgrade hook Job (if migrations.mode = job)
    extra-manifests.yaml  # deploy-time .Values.extraManifests (escape hatch)
```

### Migrations: init-container vs. Job

`migrations.mode` selects how a declared `[migrations]` step runs:

| `migrations.mode` | What renders | Use when |
|---|---|---|
| `init-container` (default) | an initContainer on the Deployment — migrates in every pod before the server starts | owned/in-chart DB, and **dev** |
| `job` | a `pre-install`/`pre-upgrade` **hook Job** Helm gates the rollout on (migrate → success → roll out; `--atomic` rolls back on failure) | **prod** with a **managed/shared** DB |

`mode: job` requires the DB **and** the Secret to already exist, because pre-upgrade
hooks run before the chart's own StatefulSet/Secret — so pair it with
`database.shared=true` (managed DB) and `secrets.create=false` (externally-managed
Secret: Sealed Secrets / ExternalSecrets / kubectl). `migrations.env` (e.g.
`IDENTITY_MIGRATE_ONLY=1`) lands on the migration step **only**, never the server.

### MCP: in-process vs. sidecar

`mcp.mode` selects how the MCP tool surface is served (when `mcp.enabled`):

| `mcp.mode` | What renders | Use when |
|---|---|---|
| `in-process` (default) | the **same** server container exposes a second port (`service.mcpPort`); the Service exposes both | the server serves MCP in-process — tonin's `Service::new().mcp_addr(...)` runtime (no separate binary) |
| `sidecar` | a separate `<name>-mcp` container proxying to the gRPC server | only if your image actually ships a `<name>-mcp` binary |

Default is `in-process` because that matches the tonin `Service` runtime — one
binary, one container, two ports. `sidecar` is the legacy split-binary shape.

Each template is gated on `.Values`, so a service with no `[database]`, `[cache]`,
`[secrets]`, or mesh renders only the core Deployment/Service/HPA. `shared = true`
database/cache point at an external instance and render **no** StatefulSet.

Secrets are values-driven: the chart templates a `<release>-secrets` Secret from
`secrets.values` — supply real values at deploy time
(`--set secrets.values.DATABASE_PASSWORD=…` or a private values file; never commit
them). Set `secrets.create=false` to reference an externally-managed Secret instead.

### Escape hatches

`tonin helm generate` **regenerates `values.yaml`**, so don't hand-edit it. For
anything the generator doesn't model:

| Need | How |
|------|-----|
| Mesh pod annotations (e.g. cilium encryption) | generated into `podAnnotations` from `[deploy].mesh` |
| Extra env vars (server + initContainer) | deploy time: `--set-json 'extraEnv=[{"name":"X","value":"y"}]'` |
| Ad-hoc manifests (ConfigMap, etc.) | deploy time: `-f extra.yaml` populating `extraManifests` (each entry a YAML string, run through `tpl`) |
| **Durable** per-service manifest (e.g. a CronJob) | add a custom file under `chart/templates/` — generate only overwrites its own templates, so yours **survives regeneration** |

`extraEnv` / `extraManifests` default to `[]` precisely so regeneration never
clobbers them — they're meant to be supplied at deploy, not stored in the chart.

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
| `[database]` / `[cache]` | StatefulSet + headless Service (owned) or `DATABASE_URL`/`REDIS_URL` stateful env (shared) |
| `[secrets].required` | `secrets.keys` → secret-sourced env + `<release>-secrets` Secret |
| `[migrations]` (`run_on = "init-container"`) | migrations initContainer on the Deployment |
| `[callers]` + `mesh = "cilium"` | CiliumNetworkPolicy ingress allowlist |
| `[depends_on]` + `mesh = "cilium"` | CiliumNetworkPolicy egress allowlist (namespaces resolved per env via `{env}` / the table form) |
| Per-env `[deploy.<env>]` / `[database.<env>]` / … overlays, and `{env}` in namespaces | `values-<env>.yaml` overrides |

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
| 0.2.x      | 0.5.6+       | 0.5.6+    |
| 0.3.x      | 0.6.0+       | 0.6.0+    |

`tonin-helm` follows `tonin-plugin`'s semver independently. A `tonin-helm` patch bump
never requires a `tonin` CLI upgrade.

HTTP service support — `type = "http"` and `[service.http]` (a gRPC service that also
serves HTTP) — requires `tonin-plugin` and the `tonin` CLI **0.5.6+**, rendered by
`tonin-helm` **0.2.x**.

Per-environment namespaces and dependencies — `{env}` placeholders and the
Cargo-style `[depends_on]` table form (per-env override, `envs` whitelist,
`@inherit`) — require `tonin-plugin` and the `tonin` CLI **0.6.0+**, rendered by
`tonin-helm` **0.3.x**.

---

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
