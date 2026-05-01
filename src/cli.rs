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
    Merge(MergeArgs),
    Extract(ExtractArgs),
    Remove,
    Reorder,
    Insert,
    Replace,
    Split,
    Rotate,
}

#[derive(Debug, Clone, Args)]
pub struct MergeArgs {
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,

    #[arg(short = 'o', long, value_name = "file")]
    pub output: PathBuf,

    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ExtractArgs {
    pub input: PathBuf,

    #[arg(long, value_name = "pages")]
    pub pages: String,

    #[arg(short = 'o', long, value_name = "file")]
    pub output: PathBuf,

    #[arg(long)]
    pub overwrite: bool,
}
