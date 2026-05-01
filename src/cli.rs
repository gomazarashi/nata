use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "nata", version, about = "PDFをページ単位で編集するCLIツール")]
pub struct Cli {
    #[command(flatten)]
    pub common: CommonOptions,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Args)]
pub struct CommonOptions {
    #[arg(long, value_name = "path", global = true)]
    pub qpdf: Option<PathBuf>,

    #[arg(long, global = true)]
    pub strict: bool,

    #[arg(long, global = true)]
    pub quiet: bool,

    #[arg(long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Merge,
    Extract,
    Remove,
    Reorder,
    Insert,
    Replace,
    Split,
    Rotate,
}
