use std::path::PathBuf;

use thiserror::Error;

use crate::exit_code::ExitCode;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("error: {0}")]
    General(String),

    #[error("error: qpdf executable was not found\nhint: install qpdf and ensure it is available in PATH\nhint: or pass --qpdf <path>")]
    QpdfNotFound,

    #[error("error: qpdf executable is not executable: {path}")]
    QpdfNotExecutable { path: PathBuf },
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::General(_) => ExitCode::GeneralError,
            Self::QpdfNotFound | Self::QpdfNotExecutable { .. } => ExitCode::BackendError,
        }
    }
}
