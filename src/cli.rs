use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand, value_parser};

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

    #[arg(long, value_name = "path", global = true)]
    pub pdftoppm: Option<PathBuf>,

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
    Render(RenderArgs),
    Split(SplitArgs),
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

#[derive(Debug, Clone, Args)]
pub struct RenderArgs {
    pub input: PathBuf,

    #[arg(long, value_name = "pages", default_value = "all")]
    pub pages: String,

    #[arg(short = 'd', long = "output-dir", value_name = "dir")]
    pub output_dir: PathBuf,

    #[arg(long, value_name = "n", default_value_t = 150, value_parser = value_parser!(u32).range(1..))]
    pub dpi: u32,

    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Args)]
#[command(group(
    ArgGroup::new("split_mode")
        .args(["ranges", "every", "each_page"])
        .required(true)
        .multiple(false)
))]
pub struct SplitArgs {
    pub input: PathBuf,

    #[arg(long, value_name = "spec")]
    pub ranges: Vec<String>,

    #[arg(long, value_name = "n", value_parser = value_parser!(u32).range(1..))]
    pub every: Option<u32>,

    #[arg(long)]
    pub each_page: bool,

    #[arg(short = 'd', long = "output-dir", value_name = "dir")]
    pub output_dir: PathBuf,

    #[arg(long)]
    pub overwrite: bool,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::{Cli, Commands};

    #[test]
    fn parses_global_strict_option() {
        let cli = Cli::try_parse_from([
            "nata", "--strict", "extract", "in.pdf", "--pages", "1", "-o", "out.pdf",
        ])
        .expect("cli should parse");

        assert!(cli.common.strict);
        assert!(matches!(cli.command, Commands::Extract(_)));
    }

    #[test]
    fn parses_split_command() {
        let cli = Cli::try_parse_from(["nata", "split", "in.pdf", "--every", "2", "-d", "out"])
            .expect("cli should parse");

        let Commands::Split(args) = cli.command else {
            panic!("split command should parse");
        };
        assert_eq!(args.every, Some(2));
        assert_eq!(args.output_dir, PathBuf::from("out"));
    }

    #[test]
    fn parses_render_command_with_defaults() {
        let cli = Cli::try_parse_from(["nata", "render", "in.pdf", "-d", "out"])
            .expect("cli should parse");

        let Commands::Render(args) = cli.command else {
            panic!("render command should parse");
        };
        assert_eq!(args.pages, "all");
        assert_eq!(args.output_dir, PathBuf::from("out"));
        assert_eq!(args.dpi, 150);
    }

    #[test]
    fn split_requires_exactly_one_mode() {
        assert!(Cli::try_parse_from(["nata", "split", "in.pdf", "-d", "out"]).is_err());
        assert!(
            Cli::try_parse_from([
                "nata",
                "split",
                "in.pdf",
                "--each-page",
                "--every",
                "2",
                "-d",
                "out",
            ])
            .is_err()
        );
    }

    #[test]
    fn split_rejects_zero_every() {
        assert!(
            Cli::try_parse_from(["nata", "split", "in.pdf", "--every", "0", "-d", "out"]).is_err()
        );
    }

    #[test]
    fn render_rejects_zero_dpi() {
        assert!(
            Cli::try_parse_from(["nata", "render", "in.pdf", "-d", "out", "--dpi", "0"]).is_err()
        );
    }

    #[test]
    fn parses_global_pdftoppm_option() {
        let cli = Cli::try_parse_from([
            "nata",
            "--pdftoppm",
            "pdftoppm",
            "render",
            "in.pdf",
            "-d",
            "out",
        ])
        .expect("cli should parse");

        assert_eq!(cli.common.pdftoppm, Some(PathBuf::from("pdftoppm")));
        assert!(matches!(cli.command, Commands::Render(_)));
    }
}
