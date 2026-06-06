use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod compat;
mod cross_build;
mod report;
mod suggest;
mod target_db;

/// Cross-compilation compatibility checker for Rust projects
#[derive(Parser)]
#[command(name = "cross-compile-checker", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List known Rust target triples
    Targets {
        /// Filter by OS (e.g., linux, windows, macos)
        #[arg(long)]
        os: Option<String>,
        /// Filter by architecture (e.g., x86_64, aarch64)
        #[arg(long)]
        arch: Option<String>,
    },
    /// Check which targets are compatible with a Cargo.toml
    Compat {
        /// Path to Cargo.toml
        #[arg(default_value = "Cargo.toml")]
        path: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run cargo check for multiple targets
    Check {
        /// Comma-separated target triples (default: popular targets)
        #[arg(long, value_delimiter = ',')]
        targets: Option<Vec<String>>,
        /// Path to the Rust project
        #[arg(default_value = ".")]
        project: PathBuf,
    },
    /// Generate a compatibility matrix report
    Report {
        /// Path to the Rust project
        #[arg(default_value = ".")]
        project: PathBuf,
        /// Output format: table, json, markdown
        #[arg(long, default_value = "table")]
        format: String,
        /// Comma-separated target triples
        #[arg(long, value_delimiter = ',')]
        targets: Option<Vec<String>>,
    },
    /// Suggest CI targets based on popularity and compatibility
    Suggest {
        /// Path to Cargo.toml
        #[arg(default_value = "Cargo.toml")]
        path: PathBuf,
        /// Maximum number of suggestions
        #[arg(long, default_value = "5")]
        top: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Targets { os, arch } => {
            target_db::list_targets(os.as_deref(), arch.as_deref())?;
        }
        Commands::Compat { path, json } => {
            compat::run(&path, json)?;
        }
        Commands::Check { targets, project } => {
            cross_build::run(&project, targets.as_deref())?;
        }
        Commands::Report {
            project,
            format,
            targets,
        } => {
            report::run(&project, &format, targets.as_deref())?;
        }
        Commands::Suggest { path, top } => {
            suggest::run(&path, top)?;
        }
    }

    Ok(())
}
