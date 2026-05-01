#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    CliArgumentError = 1,
    InputPdfError = 2,
    OutputFileError = 3,
    BackendError = 4,
}

impl ExitCode {
    pub const fn code(self) -> i32 {
        self as i32
    }
}
