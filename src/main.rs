//! ousia-atscale CLI entry point.
//!
//! Subcommands:
//!   ground   — propose BFO category for each model element
//!   annotate — emit grounded overlay JSON
//!   report   — print coverage statistics
//!   export   — emit grounded model as RDF (Turtle or OWL/XML)
//!
//! All offline paths read from a --model <json-file>. The optional --from-mcp
//! path gates itself with an actionable error when the MCP connector is absent.

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use ousia_atscale::{annotate, mapper::Mapper, rdf, report::CoverageReport, AtscaleError, AtscaleModel};
use std::path::PathBuf;

fn main() {
    sigpipe::reset();
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ground { model, from_mcp } => cmd_ground(model, from_mcp),
        Commands::Annotate { model, out, from_mcp } => cmd_annotate(model, out, from_mcp),
        Commands::Report { model, from_mcp } => cmd_report(model, from_mcp),
        Commands::Export { model, from_mcp, format, out } => {
            cmd_export(model, from_mcp, format, out)
        }
    }
}

#[derive(Parser)]
#[command(
    name = "ousia-atscale",
    version,
    about = "BFO grounding bridge for AtScale semantic layer models"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// RDF output format for the `export` subcommand.
#[derive(Clone, Debug, ValueEnum)]
enum RdfFormat {
    /// Turtle (.ttl) — compact, human-readable RDF.
    Turtle,
    /// OWL/XML (.owl) — XML serialization with owl:imports.
    Owl,
}

#[derive(Subcommand)]
enum Commands {
    /// Propose a BFO category mapping for each element in an AtScale model.
    Ground {
        /// Path to an AtScale model JSON file (describe_model output).
        #[arg(long)]
        model: Option<PathBuf>,
        /// Pull model live from the attached AtScale MCP connector.
        #[arg(long)]
        from_mcp: Option<String>,
    },
    /// Emit a BFO-grounded annotation overlay without mutating the source model.
    Annotate {
        /// Path to an AtScale model JSON file.
        #[arg(long)]
        model: Option<PathBuf>,
        /// Output path for the grounded overlay JSON.
        #[arg(long, default_value = "grounded.json")]
        out: PathBuf,
        /// Pull model live from the attached AtScale MCP connector.
        #[arg(long)]
        from_mcp: Option<String>,
    },
    /// Print BFO grounding coverage for a model (% elements with a mapping).
    Report {
        /// Path to an AtScale model JSON file.
        #[arg(long)]
        model: Option<PathBuf>,
        /// Pull model live from the attached AtScale MCP connector.
        #[arg(long)]
        from_mcp: Option<String>,
    },
    /// Emit the grounded model as RDF (Turtle or OWL/XML).
    ///
    /// Each model element becomes an OWL named individual typed to its BFO class,
    /// annotated with philosophicalGrounding, domainModule, and aristotelianDefinition.
    /// The output includes owl:imports of BFO so a downstream reasoner can classify it.
    Export {
        /// Path to an AtScale model JSON file.
        #[arg(long)]
        model: Option<PathBuf>,
        /// Pull model live from the attached AtScale MCP connector.
        #[arg(long)]
        from_mcp: Option<String>,
        /// RDF serialization format.
        #[arg(long, value_enum, default_value = "turtle")]
        format: RdfFormat,
        /// Output file path. Writes to stdout if omitted.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn load_model(model_path: Option<PathBuf>, from_mcp: Option<String>) -> Result<AtscaleModel> {
    if let Some(ref _mcp_ref) = from_mcp {
        // The live MCP path requires the AtScale connector to be attached in
        // the current interactive session. In headless / build-auto contexts
        // the connector is absent — we gate explicitly (AC #6).
        return Err(AtscaleError::McpNotAttached.into());
    }
    match model_path {
        Some(path) => {
            let p = path.to_string_lossy().to_string();
            Ok(AtscaleModel::from_file(&p)?)
        }
        None => Err(anyhow::anyhow!(
            "Either --model <json> or --from-mcp <catalog.schema.table> is required."
        )),
    }
}

fn cmd_ground(model_path: Option<PathBuf>, from_mcp: Option<String>) -> Result<()> {
    let model = load_model(model_path, from_mcp)?;
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model);

    let model_label = format!("{}.{}.{}", model.catalog, model.schema, model.table);
    println!("BFO Grounding — {}\n", model_label);
    println!(
        "{:<35} {:<15} {:<35} Rationale",
        "Name", "ElementType", "BFO Category"
    );
    println!("{}", "-".repeat(130));
    for elem in &grounded {
        // Truncate rationale for single-line display.
        let rat_short: String = elem.rationale.chars().take(60).collect();
        let rat_display = if elem.rationale.len() > 60 {
            format!("{}…", rat_short)
        } else {
            rat_short
        };
        println!(
            "{:<35} {:<15} {:<35} {}",
            elem.name,
            elem.element_type,
            elem.bfo_category.label(),
            rat_display
        );
    }
    Ok(())
}

fn cmd_annotate(model_path: Option<PathBuf>, out: PathBuf, from_mcp: Option<String>) -> Result<()> {
    let model = load_model(model_path, from_mcp)?;
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model);
    let overlay = annotate::emit_overlay(&model, &grounded);
    let out_str = out.to_string_lossy().to_string();
    overlay.write_to_file(&out_str)?;
    println!("Grounded overlay written to: {}", out_str);
    println!("  Elements annotated : {}", overlay.annotations.len());
    println!("  Overlay version    : {}", overlay.overlay_version);
    Ok(())
}

fn cmd_report(model_path: Option<PathBuf>, from_mcp: Option<String>) -> Result<()> {
    let model = load_model(model_path, from_mcp)?;
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model);
    let report = CoverageReport::build(&grounded);
    let model_label = format!("{}.{}.{}", model.catalog, model.schema, model.table);
    report.print(&model_label);
    Ok(())
}

fn cmd_export(
    model_path: Option<PathBuf>,
    from_mcp: Option<String>,
    format: RdfFormat,
    out: Option<PathBuf>,
) -> Result<()> {
    use std::io::Write;

    let model = load_model(model_path, from_mcp)?;
    let mapper = Mapper::new();
    let grounded = mapper.ground_model(&model);

    match format {
        RdfFormat::Turtle => {
            let bytes = rdf::emit_turtle(&model.catalog, &model.schema, &model.table, &grounded)?;
            match out {
                Some(ref path) => {
                    std::fs::write(path, &bytes)?;
                    eprintln!(
                        "Turtle written to: {} ({} bytes, {} individuals)",
                        path.display(),
                        bytes.len(),
                        grounded.len()
                    );
                }
                None => {
                    std::io::stdout().write_all(&bytes)?;
                }
            }
        }
        RdfFormat::Owl => {
            let xml = rdf::emit_owlxml(&model.catalog, &model.schema, &model.table, &grounded)?;
            match out {
                Some(ref path) => {
                    std::fs::write(path, xml.as_bytes())?;
                    eprintln!(
                        "OWL/XML written to: {} ({} bytes, {} individuals)",
                        path.display(),
                        xml.len(),
                        grounded.len()
                    );
                }
                None => {
                    print!("{xml}");
                }
            }
        }
    }
    Ok(())
}
