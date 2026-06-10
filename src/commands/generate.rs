//! `tonin-helm generate` — render a Helm chart from `tonin.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
use tera::Tera;
use tonin_plugin::Plan;

static TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Default environments rendered unless `--envs` overrides.
const DEFAULT_ENVS: &[&str] = &["dev", "staging", "prod"];

#[derive(clap::Args)]
pub struct GenerateArgs {
    /// Path to the service directory containing `tonin.toml`.
    /// Defaults to `$TONIN_SERVICE_DIR` if set, otherwise the current directory.
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Where to write the generated chart.
    /// Defaults to `<path>/chart`.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Environments to generate `values-<env>.yaml` for.
    /// Defaults to: dev, staging, prod.
    #[arg(long = "env", value_name = "ENV")]
    pub envs: Vec<String>,
}

pub fn run(args: GenerateArgs) -> Result<()> {
    let service_dir = resolve_service_dir(args.path.as_deref())?;
    let toml_path = service_dir.join("tonin.toml");

    let base_plan =
        Plan::load(&toml_path).with_context(|| format!("loading {}", toml_path.display()))?;

    let chart_dir = args.out.unwrap_or_else(|| service_dir.join("chart"));
    let templates_dir = chart_dir.join("templates");
    std::fs::create_dir_all(&templates_dir)
        .with_context(|| format!("creating {}", templates_dir.display()))?;

    // --- Chart.yaml --------------------------------------------------------
    let ctx = build_context(&base_plan);
    render_tmpl("Chart.yaml.tmpl", &ctx, &chart_dir.join("Chart.yaml"))?;

    // --- values.yaml (base) ------------------------------------------------
    render_tmpl("values.yaml.tmpl", &ctx, &chart_dir.join("values.yaml"))?;

    // --- values-<env>.yaml -------------------------------------------------
    let env_names: Vec<String> = if args.envs.is_empty() {
        DEFAULT_ENVS.iter().map(|s| s.to_string()).collect()
    } else {
        args.envs
    };

    for env_name in &env_names {
        let env_plan = Plan::load_with_env(&toml_path, env_name)
            .with_context(|| format!("loading plan for env '{env_name}'"))?;
        let mut env_ctx = build_context(&env_plan);
        env_ctx.insert("env_name", env_name);
        render_tmpl(
            "values-env.yaml.tmpl",
            &env_ctx,
            &chart_dir.join(format!("values-{env_name}.yaml")),
        )?;
    }

    // --- templates/ (static Go template files — copy verbatim) -------------
    copy_static_templates(&templates_dir)?;

    println!("chart generated at {}", chart_dir.display());
    Ok(())
}

fn build_context(plan: &Plan) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("name", &plan.name);
    ctx.insert("version", &plan.version);
    ctx.insert("image_repository", &image_repository(&plan.image));
    ctx.insert("replicas", &plan.replicas);
    ctx.insert("max_replicas", &plan.max_replicas);
    ctx.insert("has_autoscale", &(plan.max_replicas > plan.replicas));
    ctx.insert("mcp_sidecar", &plan.mcp_sidecar);
    ctx.insert("cpu", &plan.cpu);
    ctx.insert("memory", &plan.memory);
    ctx.insert("namespace", &plan.namespace);
    ctx.insert("mesh", plan.mesh.as_str());
    ctx.insert("expose", &plan.expose);
    ctx.insert("has_database", &plan.database.is_some());
    ctx.insert("has_cache", &plan.cache.is_some());
    if let Some(db) = &plan.database {
        ctx.insert("database_engine", db.engine.as_str());
    }
    if let Some(cache) = &plan.cache {
        ctx.insert("cache_engine", cache.engine.as_str());
    }
    ctx.insert("env_name", &plan.selected_env);
    ctx
}

fn render_tmpl(tmpl_name: &str, ctx: &tera::Context, out_path: &Path) -> Result<()> {
    let tmpl_file = TEMPLATES
        .get_file(tmpl_name)
        .with_context(|| format!("missing embedded template '{tmpl_name}'"))?;
    let tmpl_src = tmpl_file
        .contents_utf8()
        .with_context(|| format!("template '{tmpl_name}' is not valid UTF-8"))?;

    let mut tera = Tera::default();
    tera.add_raw_template(tmpl_name, tmpl_src)
        .with_context(|| format!("parsing template '{tmpl_name}'"))?;

    let rendered = tera
        .render(tmpl_name, ctx)
        .with_context(|| format!("rendering template '{tmpl_name}'"))?;

    // Atomic write: temp file → rename
    let parent = out_path.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(parent)?;
    std::fs::write(tmp.path(), &rendered)?;
    tmp.persist(out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Copy every file under `templates/chart-templates/` into `out_dir` verbatim.
/// These are Go template files — Tera must not process them.
fn copy_static_templates(out_dir: &Path) -> Result<()> {
    let subdir = TEMPLATES
        .get_dir("chart-templates")
        .context("missing embedded 'chart-templates/' directory")?;
    for file in subdir.files() {
        let file_name = file
            .path()
            .file_name()
            .context("embedded file has no file name")?;
        let dest = out_dir.join(file_name);
        let tmp = tempfile::NamedTempFile::new_in(out_dir)?;
        std::fs::write(tmp.path(), file.contents())?;
        tmp.persist(&dest)
            .with_context(|| format!("writing {}", dest.display()))?;
    }
    Ok(())
}

/// Strip the `:tag` suffix from a full image string to get the repository.
fn image_repository(image: &str) -> &str {
    image.split(':').next().unwrap_or(image)
}

pub fn resolve_service_dir(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    if let Ok(dir) = std::env::var("TONIN_SERVICE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    std::env::current_dir().context("cannot determine current directory")
}
