#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    CliArgumentError = 2,
    InputPdfError = 3,
    OutputFileError = 4,
    BackendError = 5,
    StrictViolation = 6,
}

impl ExitCode {
    pub const fn code(self) -> i32 {
        self as i32
    }
}
