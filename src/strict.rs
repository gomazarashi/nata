use std::fmt;
use std::io::Read;
use std::path::Path;

use serde_json::Value;

use crate::error::AppError;
use crate::qpdf_runner::QpdfRunner;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrictFeature {
    Outlines,
    StructTree,
    AcroForm,
    PageLabels,
    Names,
    Attachments,
    Encrypted,
}

impl StrictFeature {
    fn description(self) -> &'static str {
        match self {
            Self::Outlines => "outlines/bookmarks",
            Self::StructTree => "tagged PDF logical structure",
            Self::AcroForm => "AcroForm/forms",
            Self::PageLabels => "page labels",
            Self::Names => "document-level name trees",
            Self::Attachments => "attachments/embedded files",
            Self::Encrypted => "encryption/password protection",
        }
    }
}

impl fmt::Display for StrictFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StrictInspection {
    features: Vec<StrictFeature>,
}

impl StrictInspection {
    pub fn from_qpdf_json_reader(reader: impl Read) -> Result<Self, String> {
        let root: Value = serde_json::from_reader(reader)
            .map_err(|source| format!("unexpected qpdf --json output: {source}"))?;

        Ok(Self {
            features: collect_features(&root),
        })
    }

    pub fn features(&self) -> &[StrictFeature] {
        &self.features
    }

    pub fn is_clean(&self) -> bool {
        self.features.is_empty()
    }
}

pub fn enforce_on_input(
    input: &Path,
    strict_enabled: bool,
    verbose: bool,
    qpdf: &QpdfRunner,
) -> Result<(), AppError> {
    if !strict_enabled {
        return Ok(());
    }

    if verbose {
        eprintln!(
            "info: strict inspection via qpdf --json for {}",
            input.display()
        );
    }

    let inspection = qpdf.inspect_strict_features(input)?;
    if inspection.is_clean() {
        return Ok(());
    }

    let detail = inspection
        .features()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    Err(AppError::StrictViolation {
        path: input.to_path_buf(),
        detail,
    })
}

fn collect_features(root: &Value) -> Vec<StrictFeature> {
    let mut summary = StrictSummary::default();
    walk_value(root, &mut summary);

    let mut features = Vec::new();

    if summary.encrypted {
        features.push(StrictFeature::Encrypted);
    }
    if summary.outlines {
        features.push(StrictFeature::Outlines);
    }
    if summary.acroform {
        features.push(StrictFeature::AcroForm);
    }
    if summary.page_labels {
        features.push(StrictFeature::PageLabels);
    }
    if summary.struct_tree {
        features.push(StrictFeature::StructTree);
    }
    if summary.attachments {
        features.push(StrictFeature::Attachments);
    }
    if summary.names {
        features.push(StrictFeature::Names);
    }

    features.sort_unstable();
    features
}

#[derive(Debug, Default)]
struct StrictSummary {
    outlines: bool,
    struct_tree: bool,
    acroform: bool,
    page_labels: bool,
    names: bool,
    attachments: bool,
    encrypted: bool,
}

fn walk_value(value: &Value, summary: &mut StrictSummary) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                visit_entry(key, value, summary);
                walk_value(value, summary);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_value(item, summary);
            }
        }
        _ => {}
    }
}

fn visit_entry(key: &str, value: &Value, summary: &mut StrictSummary) {
    if key.eq_ignore_ascii_case("outlines") {
        summary.outlines |= is_non_empty_container(value);
    } else if key.eq_ignore_ascii_case("acroform") {
        summary.acroform |= acroform_value_present(value);
    } else if key.eq_ignore_ascii_case("pagelabels") || key.eq_ignore_ascii_case("page_labels") {
        summary.page_labels |= is_non_empty_container(value);
    } else if key.eq_ignore_ascii_case("tagged") {
        summary.struct_tree |= value.as_bool() == Some(true);
    } else if key.eq_ignore_ascii_case("structtreeroot")
        || key.eq_ignore_ascii_case("struct_tree_root")
        || key.eq_ignore_ascii_case("/StructTreeRoot")
    {
        summary.struct_tree |= value.as_str().is_some();
    } else if key.eq_ignore_ascii_case("embeddedfiles")
        || key.eq_ignore_ascii_case("embedded_files")
        || key.eq_ignore_ascii_case("attachments")
    {
        summary.attachments |= is_non_empty_object(value);
    } else if key.eq_ignore_ascii_case("names") {
        summary.names |= is_non_empty_object(value);
    } else if key.eq_ignore_ascii_case("encrypted") {
        summary.encrypted |= value.as_bool() == Some(true);
    } else if key.eq_ignore_ascii_case("encrypt") || key.eq_ignore_ascii_case("encryption") {
        summary.encrypted |= encryption_value_present(value);
    }
}

fn acroform_value_present(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.get("hasacroform")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || map
                    .get("fields")
                    .and_then(Value::as_array)
                    .is_some_and(|fields| !fields.is_empty())
        }
        _ => false,
    }
}

fn encryption_value_present(value: &Value) -> bool {
    match value {
        Value::Object(map) => map
            .get("encrypted")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        _ => false,
    }
}

fn is_non_empty_container(value: &Value) -> bool {
    is_non_empty_object(value) || matches!(value, Value::Array(items) if !items.is_empty())
}

fn is_non_empty_object(value: &Value) -> bool {
    matches!(value, Value::Object(map) if !map.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{StrictFeature, StrictInspection};

    #[test]
    fn detects_outlines() {
        let json = br#"{"outlines":{"first":"1 0 R"}}"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::Outlines]);
    }

    #[test]
    fn detects_struct_tree_and_tagged_flag() {
        let json = br#"{"tagged":true,"qpdf":[{"jsonversion":2}]}"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::StructTree]);
    }

    #[test]
    fn detects_acroform_and_page_labels() {
        let json = br#"{"acroform":{"fields":[1]},"pagelabels":{"nums":[0]}}"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert_eq!(
            inspection.features(),
            &[StrictFeature::AcroForm, StrictFeature::PageLabels]
        );
    }

    #[test]
    fn detects_names_and_attachments() {
        let json = br#"{"names":{"EmbeddedFiles":{"Kids":[1]}},"attachments":{"items":[1]}}"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert_eq!(
            inspection.features(),
            &[StrictFeature::Names, StrictFeature::Attachments]
        );
    }

    #[test]
    fn detects_encryption() {
        let json = br#"{"encrypt":{"encrypted":true}}"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::Encrypted]);
    }

    #[test]
    fn clean_document_has_no_violations() {
        let json = br#"{"pages":[{"object":"3 0 R"}],"pdfversion":"1.7"}"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert!(inspection.is_clean());
    }

    #[test]
    fn handles_multiple_matches() {
        let json =
            br#"{"names":{"EmbeddedFiles":{"items":[1]}},"embeddedfiles":{"root":1},"tagged":true}"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert_eq!(
            inspection.features(),
            &[
                StrictFeature::StructTree,
                StrictFeature::Names,
                StrictFeature::Attachments
            ]
        );
    }

    #[test]
    fn ignores_empty_or_false_top_level_sections() {
        let json = br#"{
            "acroform": {"fields": [], "hasacroform": false, "needappearances": false},
            "attachments": {},
            "encrypt": {"encrypted": false},
            "outlines": [],
            "pagelabels": []
        }"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert!(inspection.is_clean());
    }

    #[test]
    fn detects_struct_tree_root_reference() {
        let json = br#"{
            "objects": {
                "obj:19 0 R": {
                    "value": {
                        "/StructTreeRoot": "18 0 R",
                        "/Type": "/Catalog"
                    }
                }
            }
        }"#;
        let inspection =
            StrictInspection::from_qpdf_json_reader(&json[..]).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::StructTree]);
    }
}
