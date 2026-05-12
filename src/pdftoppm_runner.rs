use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct PdftoppmRunner {
    executable: PathBuf,
}

impl PdftoppmRunner {
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }

    pub fn render_page_to_png(
        &self,
        input: &Path,
        page: u32,
        dpi: u32,
        output: &Path,
    ) -> Result<(), AppError> {
        let parent = output.parent().unwrap_or_else(|| Path::new("."));
        let temp_dir = TempDir::new_in(parent).map_err(|source| AppError::PdftoppmExecutionFailed {
            operation: "render_page_to_png".into(),
            source,
        })?;
        let prefix = temp_dir.path().join("rendered");

        let output_result = self
            .new_command()
            .arg("-f")
            .arg(page.to_string())
            .arg("-l")
            .arg(page.to_string())
            .arg("-r")
            .arg(dpi.to_string())
            .arg("-png")
            .arg(input)
            .arg(&prefix)
            .output()
            .map_err(|source| AppError::PdftoppmExecutionFailed {
                operation: "render_page_to_png".into(),
                source,
            })?;

        if !output_result.status.success() {
            return Err(AppError::PdftoppmCommandFailed {
                operation: "render_page_to_png".into(),
                detail: stderr_summary(&output_result.stderr),
            });
        }

        let generated = prefix.with_file_name(format!("rendered-{page}.png"));
        if !generated.is_file() {
            return Err(AppError::PdftoppmCommandFailed {
                operation: "render_page_to_png".into(),
                detail: "pdftoppm did not produce the expected PNG output".into(),
            });
        }

        fs::rename(&generated, output).map_err(|source| AppError::PdftoppmExecutionFailed {
            operation: "render_page_to_png".into(),
            source,
        })?;

        Ok(())
    }

    fn new_command(&self) -> Command {
        #[cfg(windows)]
        {
            if is_windows_batch_wrapper(&self.executable) {
                let mut command = Command::new("cmd.exe");
                command.arg("/C").arg(&self.executable);
                return command;
            }
        }

        Command::new(&self.executable)
    }
}

#[cfg(windows)]
fn is_windows_batch_wrapper(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "cmd" | "bat"))
}

fn stderr_summary(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        "pdftoppm did not provide error details".into()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::PdftoppmRunner;
    use crate::error::AppError;
    use crate::test_support::create_pdftoppm_probe;

    #[test]
    fn render_page_to_png_moves_generated_file_to_output() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("page.png");
        fs::write(&input, b"pdf").expect("input should exist");

        let runner = PdftoppmRunner::new(create_pdftoppm_probe(dir.path(), true, false, None));

        runner
            .render_page_to_png(&input, 2, 150, &output)
            .expect("render should succeed");

        assert!(fs::read(&output).expect("png should exist").starts_with(b"png"));
    }

    #[test]
    fn render_page_to_png_accepts_requested_page_numbered_output() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("page.png");
        fs::write(&input, b"pdf").expect("input should exist");

        let runner = PdftoppmRunner::new(create_pdftoppm_probe(dir.path(), true, false, None));

        runner
            .render_page_to_png(&input, 2, 150, &output)
            .expect("render should succeed");

        assert!(fs::read(&output).expect("png should exist").starts_with(b"png"));
    }

    #[test]
    fn render_page_to_png_maps_command_failures() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("page.png");
        fs::write(&input, b"pdf").expect("input should exist");

        let runner = PdftoppmRunner::new(create_pdftoppm_probe(dir.path(), false, false, None));

        assert!(matches!(
            runner.render_page_to_png(&input, 2, 150, &output),
            Err(AppError::PdftoppmCommandFailed { operation, detail })
                if operation == "render_page_to_png" && detail.contains("render failure")
        ));
    }
}
