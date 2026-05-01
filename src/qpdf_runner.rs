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

    pub fn extract_pages(&self, input: &Path, pages: &[u32], output: &Path) -> Result<(), AppError> {
        let page_range = format_page_range(pages);
        let output_result = Command::new(&self.executable)
            .arg(input)
            .arg("--pages")
            .arg(".")
            .arg(&page_range)
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

        for input in format_qpdf_merge_inputs(inputs) {
            command.arg(input);
        }

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

fn format_page_range(pages: &[u32]) -> String {
    pages
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
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
    use std::path::PathBuf;

    use super::{format_page_range, format_qpdf_merge_inputs};

    #[test]
    fn page_range_format_preserves_order_and_duplicates() {
        assert_eq!(format_page_range(&[3, 1, 2, 2, 5]), "3,1,2,2,5");
    }

    #[test]
    fn merge_input_format_preserves_order() {
        let inputs = vec![
            PathBuf::from("a.pdf"),
            PathBuf::from("b.pdf"),
            PathBuf::from("c.pdf"),
        ];
        assert_eq!(
            format_qpdf_merge_inputs(&inputs),
            vec!["a.pdf", "b.pdf", "c.pdf"]
        );
    }
}

fn format_qpdf_merge_inputs(inputs: &[PathBuf]) -> Vec<String> {
    inputs
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}
