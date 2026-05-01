mod cli;
mod error;
mod exit_code;
mod io_support;
mod page_spec;
mod qpdf_detector;

use clap::Parser;
use clap::error::ErrorKind;

use crate::cli::Cli;
use crate::error::AppError;
use crate::exit_code::ExitCode;

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

    if cli.common.verbose && !cli.common.quiet {
        eprintln!("info: using qpdf at {}", qpdf_path.display());
    }

    Err(AppError::General(format!(
        "command {:?} is not implemented yet",
        cli.command
    )))
}
