use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::{CommonOptions, SplitArgs};
use crate::error::AppError;
use crate::io_support::{ensure_output_directory, prepare_multiple_outputs, validate_input_pdf};
use crate::page_spec::{PageSpec, PageSpecRules};
use crate::qpdf_runner::QpdfRunner;
use crate::strict;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SplitPlanEntry {
    output_path: PathBuf,
    page_range: String,
}

pub fn run(args: SplitArgs, common: &CommonOptions, qpdf: &QpdfRunner) -> Result<(), AppError> {
    validate_input_pdf(&args.input)?;
    let total_pages = qpdf.show_npages(&args.input)?;
    strict::enforce_on_input(
        &args.input,
        common.strict,
        common.verbose && !common.quiet,
        qpdf,
    )?;

    let plan = build_split_plan(&args, total_pages)?;
    ensure_output_directory(&args.output_dir)?;
    let output_paths = plan
        .iter()
        .map(|entry| entry.output_path.clone())
        .collect::<Vec<_>>();
    let pending_outputs = prepare_multiple_outputs(&output_paths, args.overwrite)?;

    for (entry, pending) in plan.iter().zip(pending_outputs) {
        qpdf.extract_pages(&args.input, &entry.page_range, pending.temp_path())?;
        pending.finalize()?;
    }

    Ok(())
}

fn build_split_plan(args: &SplitArgs, total_pages: u32) -> Result<Vec<SplitPlanEntry>, AppError> {
    let input_stem = input_stem(&args.input);
    let raw_entries = if !args.ranges.is_empty() {
        build_range_entries(&input_stem, &args.output_dir, &args.ranges, total_pages)?
    } else {
        let chunk_size = args.every.unwrap_or(1);
        build_chunk_entries(&input_stem, &args.output_dir, total_pages, chunk_size)
    };

    Ok(apply_duplicate_suffixes(raw_entries))
}

fn build_range_entries(
    input_stem: &str,
    output_dir: &Path,
    ranges: &[String],
    total_pages: u32,
) -> Result<Vec<SplitPlanEntry>, AppError> {
    ranges
        .iter()
        .map(|raw| {
            let spec = PageSpec::parse(raw)?;
            spec.validate_rules(
                total_pages,
                PageSpecRules {
                    allow_odd_even: false,
                    require_single_page: false,
                    allow_empty_result: false,
                },
            )?;

            Ok(SplitPlanEntry {
                output_path: output_dir.join(format!(
                    "{input_stem}-{}.pdf",
                    normalize_label(raw.trim())
                )),
                page_range: spec.to_qpdf_range(),
            })
        })
        .collect()
}

fn build_chunk_entries(
    input_stem: &str,
    output_dir: &Path,
    total_pages: u32,
    chunk_size: u32,
) -> Vec<SplitPlanEntry> {
    let mut entries = Vec::new();
    let mut start = 1;

    while start <= total_pages {
        let end = start.saturating_add(chunk_size - 1).min(total_pages);
        let label = range_label(start, end);
        entries.push(SplitPlanEntry {
            output_path: output_dir.join(format!("{input_stem}-{label}.pdf")),
            page_range: format!("{start}-{end}"),
        });
        start = end + 1;
    }

    entries
}

fn apply_duplicate_suffixes(entries: Vec<SplitPlanEntry>) -> Vec<SplitPlanEntry> {
    let mut used_paths = HashSet::<PathBuf>::new();

    entries
        .into_iter()
        .map(|mut entry| {
            let original_output_path = entry.output_path.clone();
            let mut suffix = 2;

            while !used_paths.insert(entry.output_path.clone()) {
                entry.output_path = with_duplicate_suffix(&original_output_path, suffix);
                suffix += 1;
            }

            entry
        })
        .collect()
}

fn with_duplicate_suffix(path: &Path, count: u32) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("input");
    let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("pdf");
    parent.join(format!("{stem}-{count}.{extension}"))
}

fn normalize_label(input: &str) -> String {
    let sanitized = input
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .map(|ch| match ch {
            ',' => '_',
            '-' | '_' => ch,
            _ if ch.is_ascii_alphanumeric() => ch,
            _ => '_',
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "part".into()
    } else {
        sanitized
    }
}

fn input_stem(input: &Path) -> String {
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .unwrap_or_default();

    if stem.is_empty() {
        "input".into()
    } else {
        stem.into()
    }
}

fn range_label(start: u32, end: u32) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::{build_split_plan, run};
    use crate::cli::{CommonOptions, SplitArgs};
    use crate::error::AppError;
    use crate::qpdf_runner::QpdfRunner;
    use crate::test_support::create_invalid_pdf_qpdf_probe;

    #[test]
    fn each_page_plan_creates_one_output_per_page() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: Vec::new(),
            every: None,
            each_page: true,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let plan = build_split_plan(&args, 3).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(outputs, vec!["input-1.pdf", "input-2.pdf", "input-3.pdf"]);
    }

    #[test]
    fn every_plan_keeps_last_partial_chunk() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: Vec::new(),
            every: Some(2),
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let plan = build_split_plan(&args, 5).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let ranges = plan
            .iter()
            .map(|entry| entry.page_range.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            outputs,
            vec!["input-1-2.pdf", "input-3-4.pdf", "input-5.pdf"]
        );
        assert_eq!(ranges, vec!["1-2", "3-4", "5-5"]);
    }

    #[test]
    fn ranges_plan_uses_independent_outputs() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: vec!["1-2".into(), "3-last".into()],
            every: None,
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let plan = build_split_plan(&args, 5).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let ranges = plan
            .iter()
            .map(|entry| entry.page_range.clone())
            .collect::<Vec<_>>();

        assert_eq!(outputs, vec!["input-1-2.pdf", "input-3-last.pdf"]);
        assert_eq!(ranges, vec!["1-2", "3-z"]);
    }

    #[test]
    fn ranges_reject_odd_even_specs() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: vec!["odd".into()],
            every: None,
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let error = build_split_plan(&args, 5).expect_err("odd should be rejected");
        assert!(matches!(error, AppError::OddEvenNotAllowed));
    }

    #[test]
    fn duplicate_labels_receive_numeric_suffixes() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: vec!["1,2".into(), "1 , 2".into()],
            every: None,
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let plan = build_split_plan(&args, 3).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(outputs, vec!["input-1_2.pdf", "input-1_2-2.pdf"]);
    }

    #[test]
    fn duplicate_suffixes_avoid_colliding_with_independent_labels() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: vec!["1,2".into(), "1 , 2".into(), "1,2-2".into()],
            every: None,
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let plan = build_split_plan(&args, 3).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            outputs,
            vec!["input-1_2.pdf", "input-1_2-2.pdf", "input-1_2-2-2.pdf"]
        );
    }

    #[test]
    fn duplicate_suffixes_keep_incrementing_until_unused_name_is_found() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: vec![
                "1,2".into(),
                "1 , 2".into(),
                "1, 2".into(),
                "1,2 ".into(),
            ],
            every: None,
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let plan = build_split_plan(&args, 4).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.file_name().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            outputs,
            vec![
                "input-1_2.pdf",
                "input-1_2-2.pdf",
                "input-1_2-3.pdf",
                "input-1_2-4.pdf",
            ]
        );
    }

    #[test]
    fn split_plan_produces_unique_output_paths() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: vec!["1,2".into(), "1 , 2".into(), "1,2-2".into()],
            every: None,
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let plan = build_split_plan(&args, 3).expect("plan should build");
        let outputs = plan
            .iter()
            .map(|entry| entry.output_path.clone())
            .collect::<HashSet<_>>();

        assert_eq!(outputs.len(), plan.len());
    }

    #[test]
    fn split_creates_missing_output_directory() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("nested").join("out");
        fs::write(&input, b"pdf").expect("input file should exist");

        let qpdf = QpdfRunner::new(create_extract_probe(dir.path(), 3, true));
        let common = CommonOptions {
            qpdf: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let args = SplitArgs {
            input,
            ranges: Vec::new(),
            every: Some(2),
            each_page: false,
            output_dir: output_dir.clone(),
            overwrite: false,
        };

        run(args, &common, &qpdf).expect("split should succeed");

        assert!(output_dir.is_dir());
        assert!(output_dir.join("input-1-2.pdf").exists());
        assert!(output_dir.join("input-3.pdf").exists());
    }

    #[test]
    fn invalid_split_plan_does_not_create_output_directory() {
        let dir = tempdir().expect("temp dir should exist");
        let args = SplitArgs {
            input: PathBuf::from("input.pdf"),
            ranges: vec!["odd".into()],
            every: None,
            each_page: false,
            output_dir: dir.path().join("out"),
            overwrite: false,
        };

        let error = build_split_plan(&args, 5).expect_err("odd should be rejected");

        assert!(matches!(error, AppError::OddEvenNotAllowed));
        assert!(!args.output_dir.exists());
    }

    #[test]
    fn split_rejects_existing_outputs_without_overwrite() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("out");
        fs::write(&input, b"pdf").expect("input file should exist");
        fs::create_dir_all(&output_dir).expect("output dir should exist");
        fs::write(output_dir.join("input-1.pdf"), b"existing").expect("existing output");

        let qpdf = QpdfRunner::new(create_extract_probe(dir.path(), 2, true));
        let common = CommonOptions {
            qpdf: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let args = SplitArgs {
            input,
            ranges: Vec::new(),
            every: None,
            each_page: true,
            output_dir,
            overwrite: false,
        };

        let error = run(args, &common, &qpdf).expect_err("existing output should be rejected");
        assert!(matches!(error, AppError::OutputAlreadyExists { .. }));
    }

    #[test]
    fn split_keeps_input_pdf_error_class_in_strict_mode() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("out");
        let json_marker = dir.path().join("json-called");
        fs::write(&input, b"not-a-real-pdf").expect("input file should be created");

        let qpdf = QpdfRunner::new(create_invalid_pdf_qpdf_probe(dir.path(), &json_marker));
        let common = CommonOptions {
            qpdf: None,
            strict: true,
            quiet: false,
            verbose: false,
        };
        let args = SplitArgs {
            input: input.clone(),
            ranges: Vec::new(),
            every: None,
            each_page: true,
            output_dir,
            overwrite: false,
        };

        assert!(matches!(
            run(args, &common, &qpdf),
            Err(AppError::InputPdfInvalid { path, .. }) if path == input
        ));
        assert!(
            !json_marker.exists(),
            "strict inspection should not run first"
        );
    }

    #[test]
    fn split_passes_expected_ranges_to_qpdf() {
        let dir = tempdir().expect("temp dir should exist");
        let input = dir.path().join("input.pdf");
        let output_dir = dir.path().join("out");
        let log_path = dir.path().join("extract.log");
        fs::write(&input, b"pdf").expect("input file should exist");

        let qpdf = QpdfRunner::new(create_extract_probe_with_log(dir.path(), 5, &log_path, true));
        let common = CommonOptions {
            qpdf: None,
            strict: false,
            quiet: false,
            verbose: false,
        };
        let args = SplitArgs {
            input,
            ranges: vec!["1-2".into(), "3-last".into()],
            every: None,
            each_page: false,
            output_dir,
            overwrite: false,
        };

        run(args, &common, &qpdf).expect("split should succeed");

        let log = fs::read_to_string(&log_path).expect("log should exist");
        assert!(log.contains("1-2"));
        assert!(log.contains("3-z"));
    }

    fn create_extract_probe(dir: &Path, total_pages: u32, succeed: bool) -> PathBuf {
        create_extract_probe_with_log(dir, total_pages, &dir.join("unused.log"), succeed)
    }

    fn create_extract_probe_with_log(
        dir: &Path,
        total_pages: u32,
        log_path: &Path,
        succeed: bool,
    ) -> PathBuf {
        #[cfg(windows)]
        {
            let script_path = dir.join("qpdf.cmd");
            let script = if succeed {
                format!(
                    "@echo off\r\nif \"%1\"==\"--show-npages\" (\r\n  echo {total_pages}\r\n  exit /b 0\r\n)\r\nif \"%2\"==\"--pages\" (\r\n  >>\"{}\" echo %4\r\n  >\"%6\" echo pdf\r\n  exit /b 0\r\n)\r\n>&2 echo unexpected call\r\nexit /b 1\r\n",
                    log_path.display()
                )
            } else {
                "@echo off\r\n>&2 echo extract failure\r\nexit /b 2\r\n".to_string()
            };
            fs::write(&script_path, script).expect("probe script should be created");
            script_path
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let script_path = dir.join("qpdf");
            let script = if succeed {
                format!(
                    "#!/bin/sh\nif [ \"$1\" = \"--show-npages\" ]; then\n  echo {total_pages}\n  exit 0\nfi\nif [ \"$2\" = \"--pages\" ]; then\n  echo \"$4\" >> '{}'\n  printf 'pdf' > \"$6\"\n  exit 0\nfi\necho unexpected call >&2\nexit 1\n",
                    log_path.display()
                )
            } else {
                "#!/bin/sh\necho extract failure >&2\nexit 2\n".to_string()
            };
            fs::write(&script_path, script).expect("probe script should be created");
            let mut permissions = fs::metadata(&script_path)
                .expect("probe script should exist")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script_path, permissions)
                .expect("probe script should be executable");
            script_path
        }
    }
}
