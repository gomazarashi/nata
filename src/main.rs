mod cli;
mod command_support;
mod error;
mod exit_code;
mod extract;
mod io_support;
mod merge;
mod page_spec;
mod pdftoppm_detector;
mod pdftoppm_runner;
mod qpdf_detector;
mod qpdf_runner;
mod render;
mod split;
mod strict;
#[cfg(test)]
mod test_support;

use clap::Parser;
use clap::error::ErrorKind;

use crate::cli::Cli;
use crate::cli::Commands;
use crate::error::AppError;
use crate::exit_code::ExitCode;
use crate::pdftoppm_runner::PdftoppmRunner;
use crate::qpdf_runner::QpdfRunner;

fn main() {
    std::process::exit(run().code());
}

fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let exit_code = match error.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => ExitCode::Success,
                _ => ExitCode::CliArgumentError,
            };
            let _ = error.print();
            return exit_code;
        }
    };

    match try_run(cli) {
        Ok(()) => ExitCode::Success,
        Err(error) => {
            eprintln!("{error}");
            error.exit_code()
        }
    }
}

fn try_run(cli: Cli) -> Result<(), AppError> {
    let qpdf_path = qpdf_detector::detect(cli.common.qpdf.as_deref())?;
    let qpdf = QpdfRunner::new(qpdf_path.clone());

    if cli.common.verbose && !cli.common.quiet {
        eprintln!("info: using qpdf at {}", qpdf_path.display());
    }

    match cli.command {
        Commands::Merge(args) => merge::run(args, &cli.common, &qpdf),
        Commands::Extract(args) => extract::run(args, &cli.common, &qpdf),
        Commands::Render(args) => {
            let pdftoppm_path = pdftoppm_detector::detect(cli.common.pdftoppm.as_deref())?;
            let pdftoppm = PdftoppmRunner::new(pdftoppm_path.clone());

            if cli.common.verbose && !cli.common.quiet {
                eprintln!("info: using pdftoppm at {}", pdftoppm_path.display());
            }

            render::run(args, &cli.common, &qpdf, &pdftoppm)
        }
        Commands::Split(args) => split::run(args, &cli.common, &qpdf),
    }
}
