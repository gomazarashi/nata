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

    let pending = prepare_single_output(&args.output, args.overwrite)?;
    qpdf.merge_pdfs(&args.inputs, pending.temp_path())?;
    pending.finalize()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::MergeArgs;
    use crate::error::AppError;

    use super::run;
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
}
