#[cfg(test)]
use std::fs;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tempfile::{Builder, TempPath};

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

    if path.exists() {
        if !path.is_file() {
            return Err(AppError::OutputPathNotFile {
                path: path.to_path_buf(),
            });
        }

        if !overwrite {
            return Err(AppError::OutputAlreadyExists {
                path: path.to_path_buf(),
            });
        }
    }

    Ok(())
}

pub fn prepare_single_output(path: &Path, overwrite: bool) -> Result<PendingOutput, AppError> {
    validate_single_output(path, overwrite)?;

    let parent = output_parent_dir(path)?;
    let temp_path = Builder::new()
        .prefix(".nata-")
        .tempfile_in(parent)
        .map_err(|source| AppError::TempOutputCreateFailed {
            path: path.to_path_buf(),
            source,
        })?
        .into_temp_path();

    Ok(PendingOutput {
        final_path: path.to_path_buf(),
        temp_path,
    })
}

pub fn ensure_output_directory(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        if !path.is_dir() {
            return Err(AppError::OutputPathNotDirectory {
                path: path.to_path_buf(),
            });
        }
        return Ok(());
    }

    std::fs::create_dir_all(path).map_err(|source| AppError::OutputDirectoryCreateFailed {
        path: path.to_path_buf(),
        source,
    })
}

pub fn prepare_multiple_outputs(
    paths: &[PathBuf],
    overwrite: bool,
) -> Result<Vec<PendingOutput>, AppError> {
    let mut seen = HashSet::new();
    let mut prepared = Vec::with_capacity(paths.len());

    for path in paths {
        if !seen.insert(path.clone()) {
            return Err(AppError::OutputAlreadyExists { path: path.clone() });
        }
        prepared.push(prepare_single_output(path, overwrite)?);
    }

    Ok(prepared)
}

pub struct PendingOutput {
    final_path: PathBuf,
    temp_path: TempPath,
}

impl PendingOutput {
    pub fn temp_path(&self) -> &Path {
        self.temp_path.as_ref()
    }

    #[cfg(test)]
    pub fn write_all(&mut self, contents: &[u8]) -> Result<(), AppError> {
        fs::write(self.temp_path(), contents).map_err(|source| AppError::TempOutputCreateFailed {
            path: self.final_path.clone(),
            source,
        })
    }

    pub fn finalize(self) -> Result<PathBuf, AppError> {
        let PendingOutput {
            final_path,
            temp_path,
        } = self;

        temp_path
            .persist(&final_path)
            .map(|_| final_path.clone())
            .map_err(|error| AppError::OutputFinalizeFailed {
                path: final_path,
                source: error.error,
            })
    }
}

fn output_parent_dir(path: &Path) -> Result<&Path, AppError> {
    match path.parent() {
        Some(parent) if parent.as_os_str().is_empty() => Ok(Path::new(".")),
        Some(parent) => Ok(parent),
        None => Err(AppError::OutputDirectoryNotFound {
            path: path.to_path_buf(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{
        ensure_output_directory, prepare_multiple_outputs, prepare_single_output,
        validate_input_pdf, validate_single_output,
    };
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
    fn relative_output_in_current_directory_is_allowed() {
        validate_single_output(Path::new("out.pdf"), false)
            .expect("relative output in current directory should be allowed");
    }

    #[test]
    fn existing_output_directory_is_rejected_even_with_overwrite() {
        let dir = tempdir().expect("temp dir should exist");
        let output = dir.path().join("existing-dir");
        fs::create_dir(&output).expect("directory should be created");

        assert!(matches!(
            validate_single_output(&output, true),
            Err(AppError::OutputPathNotFile { .. })
        ));
    }

    #[test]
    fn finalize_promotes_temporary_file() {
        let dir = tempdir().expect("temp dir should exist");
        let output = dir.path().join("out.pdf");
        let mut pending =
            prepare_single_output(&output, false).expect("pending output should be created");
        assert!(pending.temp_path().exists());

        pending
            .write_all(b"nata-output")
            .expect("temporary file should be writable");
        let finalized = pending.finalize().expect("finalize should succeed");

        assert_eq!(finalized, output);
        assert_eq!(
            fs::read(&output).expect("output should exist"),
            b"nata-output"
        );
    }

    #[test]
    fn finalize_overwrites_existing_output() {
        let dir = tempdir().expect("temp dir should exist");
        let output = dir.path().join("out.pdf");
        fs::write(&output, b"existing").expect("output file should be created");
        let mut pending =
            prepare_single_output(&output, true).expect("pending output should be created");

        pending
            .write_all(b"nata-output")
            .expect("temporary file should be writable");
        let finalized = pending.finalize().expect("finalize should succeed");

        assert_eq!(finalized, output);
        assert_eq!(
            fs::read(&output).expect("output should exist"),
            b"nata-output"
        );
    }

    #[test]
    fn dropped_pending_output_does_not_leave_final_file() {
        let dir = tempdir().expect("temp dir should exist");
        let output = dir.path().join("out.pdf");
        let pending =
            prepare_single_output(&output, false).expect("pending output should be created");
        let temp_path = pending.temp_path().to_path_buf();
        drop(pending);

        assert!(!output.exists());
        assert!(!temp_path.exists());
    }

    #[test]
    fn ensure_output_directory_creates_missing_directory() {
        let dir = tempdir().expect("temp dir should exist");
        let output_dir = dir.path().join("nested").join("out");

        ensure_output_directory(&output_dir).expect("directory should be created");

        assert!(output_dir.is_dir());
    }

    #[test]
    fn prepare_multiple_outputs_rejects_existing_files_without_overwrite() {
        let dir = tempdir().expect("temp dir should exist");
        let first = dir.path().join("first.pdf");
        let second = dir.path().join("second.pdf");
        fs::write(&second, b"existing").expect("existing output should be created");

        let error = match prepare_multiple_outputs(&[first, second.clone()], false) {
            Ok(_) => panic!("existing output should be rejected"),
            Err(error) => error,
        };

        assert!(matches!(error, AppError::OutputAlreadyExists { path } if path == second));
    }
}
