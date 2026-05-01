use std::fs;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use crate::error::AppError;

pub fn validate_input_pdf(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::InputPdfNotFound {
            path: path.to_path_buf(),
        });
    }

    if !path.is_file() {
        return Err(AppError::InputPdfNotFile {
            path: path.to_path_buf(),
        });
    }

    Ok(())
}

pub fn validate_single_output(path: &Path, overwrite: bool) -> Result<(), AppError> {
    let parent = output_parent_dir(path)?;
    if !parent.exists() {
        return Err(AppError::OutputDirectoryNotFound {
            path: parent.to_path_buf(),
        });
    }

    if path.exists() && !overwrite {
        return Err(AppError::OutputAlreadyExists {
            path: path.to_path_buf(),
        });
    }

    Ok(())
}

pub fn prepare_single_output(path: &Path, overwrite: bool) -> Result<PendingOutput, AppError> {
    validate_single_output(path, overwrite)?;

    let parent = output_parent_dir(path)?;
    let temp_file = NamedTempFile::new_in(parent).map_err(|source| AppError::TempOutputCreateFailed {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(PendingOutput {
        final_path: path.to_path_buf(),
        temp_file,
    })
}

pub struct PendingOutput {
    final_path: PathBuf,
    temp_file: NamedTempFile,
}

impl PendingOutput {
    pub fn temp_path(&self) -> &Path {
        self.temp_file.path()
    }

    pub fn write_all(&mut self, contents: &[u8]) -> Result<(), AppError> {
        fs::write(self.temp_file.path(), contents).map_err(|source| AppError::TempOutputCreateFailed {
            path: self.final_path.clone(),
            source,
        })
    }

    pub fn finalize(self) -> Result<PathBuf, AppError> {
        let PendingOutput {
            final_path,
            temp_file,
        } = self;

        temp_file
            .persist(&final_path)
            .map(|_| final_path.clone())
            .map_err(|error| AppError::OutputFinalizeFailed {
                path: final_path,
                source: error.error,
            })
    }
}

fn output_parent_dir(path: &Path) -> Result<&Path, AppError> {
    path.parent().ok_or_else(|| AppError::OutputDirectoryNotFound {
        path: path.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{prepare_single_output, validate_input_pdf, validate_single_output};
    use crate::error::AppError;

    #[test]
    fn missing_input_is_rejected() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("missing.pdf");
        assert!(matches!(
            validate_input_pdf(&input),
            Err(AppError::InputPdfNotFound { .. })
        ));
    }

    #[test]
    fn existing_output_requires_overwrite() {
        let dir = tempdir().expect("temp dir should exist");
        let output = dir.path().join("out.pdf");
        fs::write(&output, b"existing").expect("output file should be created");
        assert!(matches!(
            validate_single_output(&output, false),
            Err(AppError::OutputAlreadyExists { .. })
        ));
        validate_single_output(&output, true).expect("overwrite should allow existing output");
    }

    #[test]
    fn finalize_promotes_temporary_file() {
        let dir = tempdir().expect("temp dir should exist");
        let output = dir.path().join("out.pdf");
        let mut pending = prepare_single_output(&output, false).expect("pending output should be created");
        assert!(pending.temp_path().exists());

        pending
            .write_all(b"nata-output")
            .expect("temporary file should be writable");
        let finalized = pending.finalize().expect("finalize should succeed");

        assert_eq!(finalized, output);
        assert_eq!(fs::read(&output).expect("output should exist"), b"nata-output");
    }

    #[test]
    fn dropped_pending_output_does_not_leave_final_file() {
        let dir = tempdir().expect("temp dir should exist");
        let output = dir.path().join("out.pdf");
        let pending = prepare_single_output(&output, false).expect("pending output should be created");
        let temp_path = pending.temp_path().to_path_buf();
        drop(pending);

        assert!(!output.exists());
        assert!(!temp_path.exists());
    }
}
