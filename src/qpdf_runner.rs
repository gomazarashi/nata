use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tempfile::NamedTempFile;

use crate::command_support;
use crate::error::AppError;
use crate::strict::StrictInspection;

const STRICT_INSPECTION_OPERATION: &str = "strict inspection (qpdf --json)";

#[derive(Debug, Clone)]
pub struct QpdfRunner {
    executable: PathBuf,
}

impl QpdfRunner {
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }

    pub fn show_npages(&self, input: &Path) -> Result<u32, AppError> {
        let output = command_support::new_command(&self.executable)
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
                detail: command_support::stderr_summary("qpdf", &output.stderr),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let page_count = stdout
            .trim()
            .parse::<u32>()
            .map_err(|_| AppError::InputPdfInvalid {
                path: input.to_path_buf(),
                detail: format!("unexpected qpdf --show-npages output: {}", stdout.trim()),
            })?;

        Ok(page_count)
    }

    pub fn inspect_strict_features(&self, input: &Path) -> Result<StrictInspection, AppError> {
        let json_file = NamedTempFile::new().map_err(|source| AppError::QpdfExecutionFailed {
            operation: STRICT_INSPECTION_OPERATION.into(),
            source,
        })?;
        let json_output = json_file
            .reopen()
            .map_err(|source| AppError::QpdfExecutionFailed {
                operation: STRICT_INSPECTION_OPERATION.into(),
                source,
            })?;

        let output = command_support::new_command(&self.executable)
            .arg("--json")
            .arg(input)
            .stdout(Stdio::from(json_output))
            .stderr(Stdio::piped())
            .output()
            .map_err(|source| AppError::QpdfExecutionFailed {
                operation: STRICT_INSPECTION_OPERATION.into(),
                source,
            })?;

        if !output.status.success() {
            return Err(AppError::QpdfCommandFailed {
                operation: STRICT_INSPECTION_OPERATION.into(),
                detail: command_support::stderr_summary("qpdf", &output.stderr),
            });
        }

        let json_input =
            File::open(json_file.path()).map_err(|source| AppError::QpdfExecutionFailed {
                operation: STRICT_INSPECTION_OPERATION.into(),
                source,
            })?;

        StrictInspection::from_qpdf_json_reader(json_input).map_err(|detail| {
            AppError::QpdfCommandFailed {
                operation: STRICT_INSPECTION_OPERATION.into(),
                detail,
            }
        })
    }

    pub fn extract_pages(
        &self,
        input: &Path,
        page_range: &str,
        output: &Path,
    ) -> Result<(), AppError> {
        let output_result = command_support::new_command(&self.executable)
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
                detail: command_support::stderr_summary("qpdf", &output_result.stderr),
            });
        }

        Ok(())
    }

    pub fn merge_pdfs(&self, inputs: &[PathBuf], output: &Path) -> Result<(), AppError> {
        let mut command = command_support::new_command(&self.executable);
        command.arg("--empty").arg("--pages");
        command.args(inputs);

        let output_result = command.arg("--").arg(output).output().map_err(|source| {
            AppError::QpdfExecutionFailed {
                operation: "merge_pdfs".into(),
                source,
            }
        })?;

        if !output_result.status.success() {
            return Err(AppError::QpdfCommandFailed {
                operation: "merge_pdfs".into(),
                detail: command_support::stderr_summary("qpdf", &output_result.stderr),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::{QpdfRunner, STRICT_INSPECTION_OPERATION};
    use crate::error::AppError;
    use crate::strict::StrictFeature;

    #[test]
    fn inspect_strict_features_reads_from_temp_file_output() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        fs::write(&input, b"pdf").expect("input file should exist");
        let qpdf = QpdfRunner::new(create_qpdf_json_probe(
            dir.path(),
            br#"{"outlines":{"first":"1 0 R"}}"#,
            true,
        ));

        let inspection = qpdf
            .inspect_strict_features(&input)
            .expect("strict inspection should succeed");

        assert_eq!(inspection.features(), &[StrictFeature::Outlines]);
    }

    #[test]
    fn inspect_strict_features_keeps_backend_error_mapping() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        fs::write(&input, b"pdf").expect("input file should exist");
        let qpdf = QpdfRunner::new(create_qpdf_json_probe(dir.path(), br#"{}"#, false));

        assert!(matches!(
            qpdf.inspect_strict_features(&input),
            Err(AppError::QpdfCommandFailed { operation, detail })
                if operation == STRICT_INSPECTION_OPERATION && detail.contains("strict json failure")
        ));
    }

    fn create_qpdf_json_probe(dir: &Path, json: &[u8], succeed: bool) -> PathBuf {
        #[cfg(windows)]
        {
            let script_path = dir.join("qpdf.cmd");
            let json_text = String::from_utf8_lossy(json);
            let script = if succeed {
                format!(
                    "@echo off\r\nif \"%1\"==\"--json\" (\r\n  echo {json_text}\r\n  exit /b 0\r\n)\r\n>&2 echo unexpected call\r\nexit /b 1\r\n"
                )
            } else {
                "@echo off\r\nif \"%1\"==\"--json\" (\r\n  >&2 echo strict json failure\r\n  exit /b 2\r\n)\r\n>&2 echo unexpected call\r\nexit /b 1\r\n".to_string()
            };
            fs::write(&script_path, script).expect("probe script should be created");
            script_path
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let script_path = dir.join("qpdf");
            let json_text = String::from_utf8_lossy(json);
            let script = if succeed {
                format!(
                    "#!/bin/sh\nif [ \"$1\" = \"--json\" ]; then\n  cat <<'EOF'\n{json_text}\nEOF\n  exit 0\nfi\necho unexpected call >&2\nexit 1\n"
                )
            } else {
                "#!/bin/sh\nif [ \"$1\" = \"--json\" ]; then\n  echo strict json failure >&2\n  exit 2\nfi\necho unexpected call >&2\nexit 1\n".to_string()
            };
            fs::write(&script_path, script).expect("probe script should be created");
            let mut permissions = fs::metadata(&script_path)
                .expect("probe script should exist")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script_path, permissions)
                .expect("probe script should be executable");
            script_path
        }
    }
}
