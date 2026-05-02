mod cli;
mod error;
mod extract;
mod exit_code;
mod io_support;
mod merge;
mod page_spec;
mod qpdf_detector;
mod qpdf_runner;

use clap::Parser;
use clap::error::ErrorKind;

use crate::cli::Cli;
use crate::cli::Commands;
use crate::error::AppError;
use crate::exit_code::ExitCode;
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
        Commands::Merge(args) => merge::run(args, &qpdf),
        Commands::Extract(args) => extract::run(args, &qpdf),
    }
}
