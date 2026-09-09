use std::fs;

use crate::cli::CommonOptions;
use crate::cli::MergeArgs;
use crate::error::AppError;
use crate::io_support::{prepare_single_output, validate_input_pdf};
use crate::qpdf_runner::QpdfRunner;
use crate::strict;

pub fn run(args: MergeArgs, common: &CommonOptions, qpdf: &QpdfRunner) -> Result<(), AppError> {
    if args.inputs.len() < 2 {
        return Err(AppError::MergeRequiresAtLeastTwoInputs);
    }

    for input in &args.inputs {
        validate_input_pdf(input)?;
    }
    ensure_output_differs_from_inputs(&args.output, &args.inputs)?;

    for input in &args.inputs {
        qpdf.show_npages(input)?;
        strict::enforce_on_input(input, common.strict, common.verbose && !common.quiet, qpdf)?;
    }

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
        let input_path = fs::canonicalize(input).map_err(|source| AppError::InputPdfInvalid {
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

    use crate::cli::CommonOptions;
    use crate::cli::MergeArgs;
    use crate::error::AppError;
    use tempfile::tempdir;

    use super::{ensure_output_differs_from_inputs, run};
    use crate::qpdf_runner::QpdfRunner;
    use crate::test_support::create_invalid_pdf_qpdf_probe;

    #[test]
    fn merge_requires_at_least_two_inputs() {
        let args = MergeArgs {
            inputs: vec![PathBuf::from("one.pdf")],
            output: PathBuf::from("out.pdf"),
            overwrite: false,
        };
        let common = CommonOptions {
            qpdf: None,
            pdftoppm: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let qpdf = QpdfRunner::new(PathBuf::from("qpdf"));

        assert!(matches!(
            run(args, &common, &qpdf),
            Err(AppError::MergeRequiresAtLeastTwoInputs)
        ));
    }

    #[test]
    fn merge_output_must_differ_from_inputs() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        fs::write(&input, b"pdf").expect("input file should be created");

        assert!(matches!(
            ensure_output_differs_from_inputs(&input, std::slice::from_ref(&input)),
            Err(AppError::OutputPathMatchesInput { .. })
        ));
    }

    #[test]
    fn merge_keeps_input_pdf_error_class_in_strict_mode() {
        let dir = tempdir().expect("temp dir should exist");
        let input_a = dir.path().join("input-a.pdf");
        let input_b = dir.path().join("input-b.pdf");
        let output = dir.path().join("out.pdf");
        let json_marker = dir.path().join("json-called");
        fs::write(&input_a, b"not-a-real-pdf").expect("first input file should be created");
        fs::write(&input_b, b"not-a-real-pdf").expect("second input file should be created");

        let qpdf = QpdfRunner::new(create_invalid_pdf_qpdf_probe(dir.path(), &json_marker));
        let common = CommonOptions {
            qpdf: None,
            pdftoppm: None,
            strict: true,
            quiet: false,
            verbose: false,
        };
        let args = MergeArgs {
            inputs: vec![input_a.clone(), input_b],
            output,
            overwrite: false,
        };

        assert!(matches!(
            run(args, &common, &qpdf),
            Err(AppError::InputPdfInvalid { path, .. }) if path == input_a
        ));
        assert!(
            !json_marker.exists(),
            "strict inspection should not run first"
        );
    }
}
