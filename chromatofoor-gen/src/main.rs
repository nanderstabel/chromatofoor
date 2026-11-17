#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_arguments,
    clippy::ptr_arg
)]
mod bounded_context;
mod common;
mod domain;
mod graph;
mod integration;
mod presentation;
mod runtime;
mod testing;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Generates Rust scaffolding or visualizes ESDM YAML"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long)]
    input: Option<PathBuf>,

    #[arg(short, long)]
    features: Option<PathBuf>,

    #[arg(long)]
    infra: Option<PathBuf>,

    #[arg(long)]
    lifecycle: Option<PathBuf>,

    #[arg(short, long)]
    out_dir: Option<PathBuf>,

    #[arg(long, default_value = "../../chromatofoor")]
    chromatofoor_path: String,

    #[arg(long)]
    emit_graph: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Visualize an exported domain model graph JSON
    Visualize {
        /// Path to the model-graph.json file
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Visualize { input }) = cli.command {
        return graph::run_visualizer_server(&input);
    }

    let input = cli.input.ok_or_else(|| {
        anyhow::anyhow!(
            "error: the following required arguments were not provided:\n  --input <INPUT>\n\nFor more information, try '--help'."
        )
    })?;
    let out_dir = cli.out_dir.ok_or_else(|| {
        anyhow::anyhow!(
            "error: the following required arguments were not provided:\n  --out-dir <OUT_DIR>\n\nFor more information, try '--help'."
        )
    })?;

    runtime::run_codegen(runtime::CodegenOptions {
        input,
        features: cli.features,
        infra: cli.infra,
        lifecycle: cli.lifecycle,
        out_dir,
        chromatofoor_path: cli.chromatofoor_path,
        emit_graph: cli.emit_graph,
    })
}
