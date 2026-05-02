use std::fs;

use crate::cli::MergeArgs;
use crate::error::AppError;
use crate::io_support::{prepare_single_output, validate_input_pdf};
use crate::qpdf_runner::QpdfRunner;

pub fn run(args: MergeArgs, qpdf: &QpdfRunner) -> Result<(), AppError> {
    if args.inputs.len() < 2 {
        return Err(AppError::MergeRequiresAtLeastTwoInputs);
    }

    for input in &args.inputs {
        validate_input_pdf(input)?;
        qpdf.show_npages(input)?;
    }
    ensure_output_differs_from_inputs(&args.output, &args.inputs)?;

    let pending = prepare_single_output(&args.output, args.overwrite)?;
    qpdf.merge_pdfs(&args.inputs, pending.temp_path())?;
    pending.finalize()?;

    Ok(())
}

fn ensure_output_differs_from_inputs(
    output: &std::path::Path,
    inputs: &[std::path::PathBuf],
) -> Result<(), AppError> {
    let output_path = fs::canonicalize(output).unwrap_or_else(|_| output.to_path_buf());

    for input in inputs {
        let input_path =
            fs::canonicalize(input).map_err(|source| AppError::InputPdfInvalid {
                path: input.clone(),
                detail: format!("failed to resolve input path: {source}"),
            })?;

        if input_path == output_path {
            return Err(AppError::OutputPathMatchesInput {
                path: output.to_path_buf(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::cli::MergeArgs;
    use crate::error::AppError;
    use tempfile::tempdir;

    use super::{ensure_output_differs_from_inputs, run};
    use crate::qpdf_runner::QpdfRunner;

    #[test]
    fn merge_requires_at_least_two_inputs() {
        let args = MergeArgs {
            inputs: vec![PathBuf::from("one.pdf")],
            output: PathBuf::from("out.pdf"),
            overwrite: false,
        };
        let qpdf = QpdfRunner::new(PathBuf::from("qpdf"));

        assert!(matches!(run(args, &qpdf), Err(AppError::MergeRequiresAtLeastTwoInputs)));
    }

    #[test]
    fn merge_output_must_differ_from_inputs() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        fs::write(&input, b"pdf").expect("input file should be created");

        assert!(matches!(
            ensure_output_differs_from_inputs(&input, &[input.clone()]),
            Err(AppError::OutputPathMatchesInput { .. })
        ));
    }
}
