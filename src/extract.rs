use crate::cli::ExtractArgs;
use crate::error::AppError;
use crate::io_support::{prepare_single_output, validate_input_pdf};
use crate::page_spec::{PageSpec, PageSpecRules};
use crate::qpdf_runner::QpdfRunner;

pub fn run(args: ExtractArgs, qpdf: &QpdfRunner) -> Result<(), AppError> {
    validate_input_pdf(&args.input)?;

    let total_pages = qpdf.show_npages(&args.input)?;
    let spec = PageSpec::parse(&args.pages)?;
    let pages = spec.validate_rules(
        total_pages,
        PageSpecRules {
            allow_odd_even: true,
            require_single_page: false,
            allow_empty_result: false,
        },
    )?;

    let pending = prepare_single_output(&args.output, args.overwrite)?;
    qpdf.extract_pages(&args.input, &pages, pending.temp_path())?;
    pending.finalize()?;

    Ok(())
}
