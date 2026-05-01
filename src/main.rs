mod cli;
mod error;
mod exit_code;
mod qpdf_detector;

use clap::Parser;

use crate::cli::Cli;
use crate::error::AppError;
use crate::exit_code::ExitCode;

fn main() {
    std::process::exit(run().code());
}

fn run() -> ExitCode {
    match try_run() {
        Ok(()) => ExitCode::Success,
        Err(error) => {
            eprintln!("{error}");
            error.exit_code()
        }
    }
}

fn try_run() -> Result<(), AppError> {
    let cli = Cli::parse();
    let qpdf_path = qpdf_detector::detect(cli.common.qpdf.as_deref())?;

    if cli.common.verbose && !cli.common.quiet {
        eprintln!("info: using qpdf at {}", qpdf_path.display());
    }

    Err(AppError::General(format!(
        "command {:?} is not implemented yet",
        cli.command
    )))
}
