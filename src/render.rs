use std::path::{Path, PathBuf};

use crate::cli::{CommonOptions, RenderArgs};
use crate::error::AppError;
use crate::io_support::{
    PendingOutput, dedup_output_paths, ensure_output_directory, input_stem,
    prepare_multiple_outputs, validate_input_pdf,
};
use crate::page_spec::{PageSpec, PageSpecRules};
use crate::pdftoppm_runner::PdftoppmRunner;
use crate::qpdf_runner::QpdfRunner;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderPlanEntry {
    output_path: PathBuf,
    page: u32,
}

pub fn run(
    args: RenderArgs,
    _common: &CommonOptions,
    qpdf: &QpdfRunner,
    pdftoppm: &PdftoppmRunner,
) -> Result<(), AppError> {
    validate_input_pdf(&args.input)?;
    let total_pages = qpdf.show_npages(&args.input)?;
    let plan = build_render_plan(&args, total_pages)?;
    ensure_output_directory(&args.output_dir)?;

    let output_paths = plan
        .iter()
        .map(|entry| entry.output_path.clone())
        .collect::<Vec<_>>();
    let pending_outputs = prepare_multiple_outputs(&output_paths, args.overwrite)?;

    for (entry, pending) in plan.iter().zip(pending_outputs) {
        render_entry(&args.input, args.dpi, entry, pending, pdftoppm)?;
    }

    Ok(())
}

fn render_entry(
    input: &Path,
    dpi: u32,
    entry: &RenderPlanEntry,
    pending: PendingOutput,
    pdftoppm: &PdftoppmRunner,
) -> Result<(), AppError> {
    pdftoppm.render_page_to_png(input, entry.page, dpi, pending.temp_path())?;
    pending.finalize()?;
    Ok(())
}

fn build_render_plan(
    args: &RenderArgs,
    total_pages: u32,
) -> Result<Vec<RenderPlanEntry>, AppError> {
    let spec = PageSpec::parse(&args.pages)?;
    let pages = spec.validate_rules(
        total_pages,
        PageSpecRules {
            allow_odd_even: true,
            require_single_page: false,
            allow_empty_result: false,
        },
    )?;
    let input_stem = input_stem(&args.input);

    let entries = pages
        .into_iter()
        .map(|page| RenderPlanEntry {
            output_path: args.output_dir.join(format!("{input_stem}-{page}.png")),
            page,
        })
        .collect::<Vec<_>>();
    let output_paths = entries
        .iter()
        .map(|entry| entry.output_path.clone())
        .collect::<Vec<_>>();
    let deduped_paths = dedup_output_paths(output_paths, "output", "png");

    Ok(entries
        .into_iter()
        .zip(deduped_paths)
        .map(|(mut entry, output_path)| {
            entry.output_path = output_path;
            entry
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::{build_render_plan, run};
    use crate::cli::{CommonOptions, RenderArgs};
    use crate::error::AppError;
    use crate::pdftoppm_runner::PdftoppmRunner;
    use crate::qpdf_runner::QpdfRunner;
    use crate::test_support::{
        create_extract_probe_with_log, create_invalid_pdf_qpdf_probe, create_pdftoppm_probe,
    };

    #[test]
    fn render_plan_defaults_to_page_numbered_pngs() {
        let dir = tempdir().expect("temp dir should exist");
        let args = RenderArgs {
            input: PathBuf::from("input.pdf"),
            pages: "1,3,last".into(),
            output_dir: dir.path().join("out"),
            dpi: 150,
            overwrite: false,
        };

        let plan = build_render_plan(&args, 5).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| {
                entry
                    .output_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        let pages = plan.iter().map(|entry| entry.page).collect::<Vec<_>>();

        assert_eq!(outputs, vec!["input-1.png", "input-3.png", "input-5.png"]);
        assert_eq!(pages, vec![1, 3, 5]);
    }

    #[test]
    fn render_plan_adds_suffixes_for_duplicate_pages() {
        let dir = tempdir().expect("temp dir should exist");
        let args = RenderArgs {
            input: PathBuf::from("input.pdf"),
            pages: "1,1,1".into(),
            output_dir: dir.path().join("out"),
            dpi: 150,
            overwrite: false,
        };

        let plan = build_render_plan(&args, 3).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| {
                entry
                    .output_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            outputs,
            vec!["input-1.png", "input-1-2.png", "input-1-3.png"]
        );
    }

    #[test]
    fn render_plan_produces_unique_output_paths() {
        let dir = tempdir().expect("temp dir should exist");
        let args = RenderArgs {
            input: PathBuf::from("input.pdf"),
            pages: "1,1,2,2".into(),
            output_dir: dir.path().join("out"),
            dpi: 150,
            overwrite: false,
        };

        let plan = build_render_plan(&args, 3).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.clone())
            .collect::<HashSet<_>>();

        assert_eq!(outputs.len(), plan.len());
    }

    #[test]
    fn render_creates_missing_output_directory_and_writes_pngs() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("nested").join("out");
        let log_path = dir.path().join("render.log");
        fs::write(&input, b"pdf").expect("input file should exist");

        let qpdf = QpdfRunner::new(create_extract_probe_with_log(
            dir.path(),
            4,
            &dir.path().join("unused.log"),
            true,
        ));
        let pdftoppm = PdftoppmRunner::new(create_pdftoppm_probe(
            dir.path(),
            true,
            false,
            Some(&log_path),
        ));
        let common = CommonOptions {
            qpdf: None,
            pdftoppm: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let args = RenderArgs {
            input,
            pages: "1,3,last".into(),
            output_dir: output_dir.clone(),
            dpi: 200,
            overwrite: false,
        };

        run(args, &common, &qpdf, &pdftoppm).expect("render should succeed");

        assert!(output_dir.is_dir());
        assert!(output_dir.join("input-1.png").exists());
        assert!(output_dir.join("input-3.png").exists());
        assert!(output_dir.join("input-4.png").exists());

        let log = fs::read_to_string(&log_path).expect("log should exist");
        assert!(log.contains("1,1,200"));
        assert!(log.contains("3,3,200"));
        assert!(log.contains("4,4,200"));
    }

    #[test]
    fn render_rejects_existing_outputs_without_overwrite() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("out");
        fs::write(&input, b"pdf").expect("input file should exist");
        fs::create_dir_all(&output_dir).expect("output dir should exist");
        fs::write(output_dir.join("input-1.png"), b"existing").expect("existing output");

        let qpdf = QpdfRunner::new(create_extract_probe_with_log(
            dir.path(),
            2,
            &dir.path().join("unused.log"),
            true,
        ));
        let pdftoppm = PdftoppmRunner::new(create_pdftoppm_probe(dir.path(), true, false, None));
        let common = CommonOptions {
            qpdf: None,
            pdftoppm: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let args = RenderArgs {
            input,
            pages: "1".into(),
            output_dir,
            dpi: 150,
            overwrite: false,
        };

        let error =
            run(args, &common, &qpdf, &pdftoppm).expect_err("existing output should be rejected");
        assert!(matches!(error, AppError::OutputAlreadyExists { .. }));
    }

    #[test]
    fn render_overwrites_existing_outputs_when_requested() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("out");
        fs::write(&input, b"pdf").expect("input file should exist");
        fs::create_dir_all(&output_dir).expect("output dir should exist");
        fs::write(output_dir.join("input-1.png"), b"existing").expect("existing output");

        let qpdf = QpdfRunner::new(create_extract_probe_with_log(
            dir.path(),
            1,
            &dir.path().join("unused.log"),
            true,
        ));
        let pdftoppm = PdftoppmRunner::new(create_pdftoppm_probe(dir.path(), true, false, None));
        let common = CommonOptions {
            qpdf: None,
            pdftoppm: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let args = RenderArgs {
            input,
            pages: "1".into(),
            output_dir: output_dir.clone(),
            dpi: 150,
            overwrite: true,
        };

        run(args, &common, &qpdf, &pdftoppm).expect("render should succeed");

        assert!(
            fs::read(output_dir.join("input-1.png"))
                .expect("output should exist")
                .starts_with(b"png")
        );
    }

    #[test]
    fn render_keeps_input_pdf_error_class() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("out");
        let json_marker = dir.path().join("json-called");
        fs::write(&input, b"not-a-real-pdf").expect("input file should be created");

        let qpdf = QpdfRunner::new(create_invalid_pdf_qpdf_probe(dir.path(), &json_marker));
        let pdftoppm = PdftoppmRunner::new(create_pdftoppm_probe(dir.path(), true, false, None));
        let common = CommonOptions {
            qpdf: None,
            pdftoppm: None,
            strict: true,
            quiet: false,
            verbose: false,
        };
        let args = RenderArgs {
            input: input.clone(),
            pages: "1".into(),
            output_dir,
            dpi: 150,
            overwrite: false,
        };

        assert!(matches!(
            run(args, &common, &qpdf, &pdftoppm),
            Err(AppError::InputPdfInvalid { path, .. }) if path == input
        ));
        assert!(!json_marker.exists(), "strict inspection should not run");
    }

    #[test]
    fn render_does_not_leave_output_on_pdftoppm_failure() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("out");
        fs::write(&input, b"pdf").expect("input file should exist");

        let qpdf = QpdfRunner::new(create_extract_probe_with_log(
            dir.path(),
            1,
            &dir.path().join("unused.log"),
            true,
        ));
        let pdftoppm = PdftoppmRunner::new(create_pdftoppm_probe(dir.path(), false, false, None));
        let common = CommonOptions {
            qpdf: None,
            pdftoppm: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let args = RenderArgs {
            input,
            pages: "1".into(),
            output_dir: output_dir.clone(),
            dpi: 150,
            overwrite: false,
        };

        let error = run(args, &common, &qpdf, &pdftoppm).expect_err("render should fail");
        assert!(matches!(error, AppError::PdftoppmCommandFailed { .. }));
        assert!(!output_dir.join("input-1.png").exists());
    }
}
