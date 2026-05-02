use std::path::PathBuf;

use thiserror::Error;

use crate::exit_code::ExitCode;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("error: qpdf executable was not found\nhint: install qpdf and ensure it is available in PATH\nhint: or pass --qpdf <path>")]
    QpdfNotFound,

    #[error("error: qpdf executable path is invalid: {path}")]
    QpdfNotExecutable { path: PathBuf },

    #[error("error: qpdf execution failed during {operation}: {source}")]
    QpdfExecutionFailed {
        operation: String,
        source: std::io::Error,
    },

    #[error("error: qpdf command failed during {operation}: {detail}")]
    QpdfCommandFailed {
        operation: String,
        detail: String,
    },

    #[error("error: merge requires at least two input PDFs")]
    MergeRequiresAtLeastTwoInputs,

    #[error("error: input PDF was not found: {path}")]
    InputPdfNotFound { path: PathBuf },

    #[error("error: input PDF is not a file: {path}")]
    InputPdfNotFile { path: PathBuf },

    #[error("error: input PDF is not readable as PDF: {path}\ndetail: {detail}")]
    InputPdfInvalid { path: PathBuf, detail: String },

    #[error("error: output file already exists: {path}")]
    OutputAlreadyExists { path: PathBuf },

    #[error("error: output directory does not exist: {path}")]
    OutputDirectoryNotFound { path: PathBuf },

    #[error("error: failed to create temporary output near {path}: {source}")]
    TempOutputCreateFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("error: failed to finalize output file to {path}: {source}")]
    OutputFinalizeFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("error: invalid page specification: {0}")]
    InvalidPageSpec(String),

    #[error("error: page {page} is out of range for total pages {total_pages}")]
    PageOutOfRange { page: u32, total_pages: u32 },

    #[error("error: odd/even page specification is not allowed here")]
    OddEvenNotAllowed,

    #[error("error: exactly one page must be specified")]
    SinglePageRequired,

    #[error("error: page specification resolved to no pages")]
    EmptyPageSelection,
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::QpdfNotFound
            | Self::QpdfNotExecutable { .. }
            | Self::QpdfExecutionFailed { .. }
            | Self::QpdfCommandFailed { .. } => ExitCode::BackendError,
            Self::InputPdfNotFound { .. }
            | Self::InputPdfNotFile { .. }
            | Self::InputPdfInvalid { .. } => ExitCode::InputPdfError,
            Self::OutputAlreadyExists { .. }
            | Self::OutputDirectoryNotFound { .. }
            | Self::TempOutputCreateFailed { .. }
            | Self::OutputFinalizeFailed { .. } => ExitCode::OutputFileError,
            Self::InvalidPageSpec(_)
            | Self::PageOutOfRange { .. }
            | Self::OddEvenNotAllowed
            | Self::SinglePageRequired
            | Self::EmptyPageSelection
            | Self::MergeRequiresAtLeastTwoInputs => ExitCode::CliArgumentError,
        }
    }
}
