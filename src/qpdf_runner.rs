use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct QpdfRunner {
    executable: PathBuf,
}

impl QpdfRunner {
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }

    pub fn show_npages(&self, input: &Path) -> Result<u32, AppError> {
        let output = Command::new(&self.executable)
            .arg("--show-npages")
            .arg(input)
            .output()
            .map_err(|source| AppError::QpdfExecutionFailed {
                operation: "show_npages".into(),
                source,
            })?;

        if !output.status.success() {
            return Err(AppError::InputPdfInvalid {
                path: input.to_path_buf(),
                detail: stderr_summary(&output.stderr),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let page_count = stdout.trim().parse::<u32>().map_err(|_| AppError::InputPdfInvalid {
            path: input.to_path_buf(),
            detail: format!("unexpected qpdf --show-npages output: {}", stdout.trim()),
        })?;

        Ok(page_count)
    }

    pub fn extract_pages(&self, input: &Path, page_range: &str, output: &Path) -> Result<(), AppError> {
        let output_result = Command::new(&self.executable)
            .arg(input)
            .arg("--pages")
            .arg(".")
            .arg(page_range)
            .arg("--")
            .arg(output)
            .output()
            .map_err(|source| AppError::QpdfExecutionFailed {
                operation: "extract_pages".into(),
                source,
            })?;

        if !output_result.status.success() {
            return Err(AppError::QpdfCommandFailed {
                operation: "extract_pages".into(),
                detail: stderr_summary(&output_result.stderr),
            });
        }

        Ok(())
    }

    pub fn merge_pdfs(&self, inputs: &[PathBuf], output: &Path) -> Result<(), AppError> {
        let mut command = Command::new(&self.executable);
        command.arg("--empty").arg("--pages");
        command.args(inputs);

        let output_result = command
            .arg("--")
            .arg(output)
            .output()
            .map_err(|source| AppError::QpdfExecutionFailed {
                operation: "merge_pdfs".into(),
                source,
            })?;

        if !output_result.status.success() {
            return Err(AppError::QpdfCommandFailed {
                operation: "merge_pdfs".into(),
                detail: stderr_summary(&output_result.stderr),
            });
        }

        Ok(())
    }
}

fn stderr_summary(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        "qpdf did not provide error details".into()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
}
