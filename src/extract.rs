use std::fs;

use crate::cli::CommonOptions;
use crate::cli::ExtractArgs;
use crate::error::AppError;
use crate::io_support::{prepare_single_output, validate_input_pdf};
use crate::page_spec::{PageSpec, PageSpecRules};
use crate::qpdf_runner::QpdfRunner;
use crate::strict;

pub fn run(args: ExtractArgs, common: &CommonOptions, qpdf: &QpdfRunner) -> Result<(), AppError> {
    validate_input_pdf(&args.input)?;
    strict::enforce_on_input(
        &args.input,
        common.strict,
        common.verbose && !common.quiet,
        qpdf,
    )?;
    ensure_output_differs_from_input(&args.output, &args.input)?;

    let total_pages = qpdf.show_npages(&args.input)?;
    let spec = PageSpec::parse(&args.pages)?;
    spec.validate_rules(
        total_pages,
        PageSpecRules {
            allow_odd_even: true,
            require_single_page: false,
            allow_empty_result: false,
        },
    )?;
    let page_range = spec.to_qpdf_range();

    let pending = prepare_single_output(&args.output, args.overwrite)?;
    qpdf.extract_pages(&args.input, &page_range, pending.temp_path())?;
    pending.finalize()?;

    Ok(())
}

fn ensure_output_differs_from_input(
    output: &std::path::Path,
    input: &std::path::Path,
) -> Result<(), AppError> {
    let output_path = fs::canonicalize(output).unwrap_or_else(|_| output.to_path_buf());
    let input_path = fs::canonicalize(input).map_err(|source| AppError::InputPdfInvalid {
        path: input.to_path_buf(),
        detail: format!("failed to resolve input path: {source}"),
    })?;

    if input_path == output_path {
        return Err(AppError::OutputPathMatchesInput {
            path: output.to_path_buf(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::ensure_output_differs_from_input;
    use crate::error::AppError;

    #[test]
    fn extract_output_must_differ_from_input() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        fs::write(&input, b"pdf").expect("input file should be created");

        assert!(matches!(
            ensure_output_differs_from_input(PathBuf::from(&input).as_path(), input.as_path()),
            Err(AppError::OutputPathMatchesInput { .. })
        ));
    }
}
