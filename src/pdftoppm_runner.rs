use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::command_support;
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
        let temp_dir =
            TempDir::new_in(parent).map_err(|source| AppError::PdftoppmExecutionFailed {
                operation: "render_page_to_png".into(),
                source,
            })?;
        let prefix = temp_dir.path().join("rendered");

        let output_result = command_support::new_command(&self.executable)
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
                detail: command_support::stderr_summary("pdftoppm", &output_result.stderr),
            });
        }

        let generated =
            find_generated_png(temp_dir.path()).ok_or_else(|| AppError::PdftoppmCommandFailed {
                operation: "render_page_to_png".into(),
                detail: "pdftoppm did not produce the expected PNG output".into(),
            })?;

        if generated.parent() != Some(temp_dir.path()) {
            return Err(AppError::PdftoppmCommandFailed {
                operation: "render_page_to_png".into(),
                detail: "pdftoppm produced an unexpected PNG output path".into(),
            });
        }

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
}

fn find_generated_png(dir: &Path) -> Option<PathBuf> {
    let mut pngs = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .collect::<Vec<_>>();

    if pngs.len() == 1 { pngs.pop() } else { None }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{PdftoppmRunner, find_generated_png};
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

        assert!(
            fs::read(&output)
                .expect("png should exist")
                .starts_with(b"png")
        );
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

        assert!(
            fs::read(&output)
                .expect("png should exist")
                .starts_with(b"png")
        );
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

    #[test]
    fn find_generated_png_accepts_zero_padded_name() {
        let dir = tempdir().expect("temp dir should exist");
        let png = dir.path().join("rendered-01.png");
        fs::write(&png, b"png").expect("png should exist");

        assert_eq!(find_generated_png(dir.path()), Some(png));
    }
}
