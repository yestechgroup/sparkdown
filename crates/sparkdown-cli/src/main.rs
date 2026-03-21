mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sparkdown", about = "Semantic markdown processor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a sparkdown document to an output format
    Render {
        /// Input file path (use - for stdin)
        input: String,
        /// Output format: html, jsonld, turtle
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file (stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Validate semantic annotations in a document
    Validate {
        /// Input file path
        input: String,
        /// Strictness level: warn, error
        #[arg(long, default_value = "warn")]
        level: String,
    },
    /// Extract RDF triples from a document
    Extract {
        /// Input file path
        input: String,
        /// Output format: turtle, jsonld
        #[arg(short, long, default_value = "turtle")]
        format: String,
    },
    /// Initialize a new sparkdown document with frontmatter template
    Init {
        /// Output file path
        output: String,
        /// Document type (article, event, review, person)
        #[arg(short = 't', long, default_value = "article")]
        doc_type: String,
    },
    /// Manage semantic overlay sidecar files
    Overlay {
        #[command(subcommand)]
        command: OverlayCommands,
    },
}

#[derive(Subcommand)]
enum OverlayCommands {
    /// Create empty .sparkdown-sem for a .md file
    Init {
        /// Input markdown file
        input: String,
    },
    /// Run sync engine after markdown edits
    Sync {
        /// Input markdown file
        input: String,
    },
    /// Show stale/detached entities
    Status {
        /// Input markdown file
        input: String,
    },
    /// Strip anchors, produce valid Turtle
    Export {
        /// Input markdown file
        input: String,
    },
    /// Combined view (markdown + inline annotations) for debugging
    Merge {
        /// Input markdown file
        input: String,
    },
    /// Extract inline annotations from legacy .md into sidecar
    Import {
        /// Input markdown file
        input: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Render {
            input,
            format,
            output,
        } => commands::render::run(&input, &format, output.as_deref()),
        Commands::Validate { input, level } => commands::validate::run(&input, &level),
        Commands::Extract { input, format } => commands::extract::run(&input, &format),
        Commands::Init { output, doc_type } => commands::init::run(&output, &doc_type),
        Commands::Overlay { command } => match command {
            OverlayCommands::Init { input } => commands::overlay::init(&input),
            OverlayCommands::Sync { input } => commands::overlay::sync(&input),
            OverlayCommands::Status { input } => commands::overlay::status(&input),
            OverlayCommands::Export { input } => commands::overlay::export(&input),
            OverlayCommands::Merge { input } => commands::overlay::merge(&input),
            OverlayCommands::Import { input } => commands::overlay::import(&input),
        },
    }
}
