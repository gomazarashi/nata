use std::fmt;
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
    pub fn from_qpdf_json(raw: &[u8]) -> Result<Self, String> {
        let root: Value = serde_json::from_slice(raw)
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
    let mut features = Vec::new();

    if encryption_present(root) {
        features.push(StrictFeature::Encrypted);
    }
    if non_empty_array_key_present(root, "outlines")
        || non_empty_object_key_present(root, "outlines")
    {
        features.push(StrictFeature::Outlines);
    }
    if acroform_present(root) {
        features.push(StrictFeature::AcroForm);
    }
    if non_empty_array_key_present(root, "pagelabels")
        || non_empty_array_key_present(root, "page_labels")
        || non_empty_object_key_present(root, "pagelabels")
        || non_empty_object_key_present(root, "page_labels")
    {
        features.push(StrictFeature::PageLabels);
    }
    if struct_tree_present(root) {
        features.push(StrictFeature::StructTree);
    }
    if attachments_present(root) {
        features.push(StrictFeature::Attachments);
    }
    if non_empty_object_key_present(root, "names") {
        features.push(StrictFeature::Names);
    }

    features.sort_unstable();
    features.dedup();
    features
}

fn acroform_present(root: &Value) -> bool {
    find_value(root, &mut |key, value| {
        if !key.eq_ignore_ascii_case("acroform") {
            return false;
        }

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
    })
}

fn encryption_present(root: &Value) -> bool {
    bool_key_is_true(root, "encrypted")
        || find_value(root, &mut |key, value| {
            if !(key.eq_ignore_ascii_case("encrypt") || key.eq_ignore_ascii_case("encryption")) {
                return false;
            }

            match value {
                Value::Object(map) => map
                    .get("encrypted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                _ => false,
            }
        })
}

fn struct_tree_present(root: &Value) -> bool {
    bool_key_is_true(root, "tagged")
        || string_key_present(root, "structtreeroot")
        || string_key_present(root, "struct_tree_root")
        || string_key_present(root, "/StructTreeRoot")
}

fn attachments_present(root: &Value) -> bool {
    non_empty_object_key_present(root, "embeddedfiles")
        || non_empty_object_key_present(root, "embedded_files")
        || non_empty_object_key_present(root, "attachments")
}

fn non_empty_object_key_present(root: &Value, needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    find_value(root, &mut |key, value| {
        key.eq_ignore_ascii_case(&needle) && matches!(value, Value::Object(map) if !map.is_empty())
    })
}

fn non_empty_array_key_present(root: &Value, needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    find_value(root, &mut |key, value| {
        key.eq_ignore_ascii_case(&needle)
            && matches!(value, Value::Array(items) if !items.is_empty())
    })
}

fn bool_key_is_true(root: &Value, needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    find_value(root, &mut |key, value| {
        key.eq_ignore_ascii_case(&needle) && value.as_bool() == Some(true)
    })
}

fn string_key_present(root: &Value, needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    find_value(root, &mut |key, value| {
        key.eq_ignore_ascii_case(&needle) && value.as_str().is_some()
    })
}

fn find_value(value: &Value, predicate: &mut impl FnMut(&str, &Value) -> bool) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| predicate(key, value) || find_value(value, predicate)),
        Value::Array(items) => items.iter().any(|item| find_value(item, predicate)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{StrictFeature, StrictInspection};

    #[test]
    fn detects_outlines() {
        let json = br#"{"outlines":{"first":"1 0 R"}}"#;
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::Outlines]);
    }

    #[test]
    fn detects_struct_tree_and_tagged_flag() {
        let json = br#"{"tagged":true,"qpdf":[{"jsonversion":2}]}"#;
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::StructTree]);
    }

    #[test]
    fn detects_acroform_and_page_labels() {
        let json = br#"{"acroform":{"fields":[1]},"pagelabels":{"nums":[0]}}"#;
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
        assert_eq!(
            inspection.features(),
            &[StrictFeature::AcroForm, StrictFeature::PageLabels]
        );
    }

    #[test]
    fn detects_names_and_attachments() {
        let json = br#"{"names":{"EmbeddedFiles":{"Kids":[1]}},"attachments":{"items":[1]}}"#;
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
        assert_eq!(
            inspection.features(),
            &[StrictFeature::Names, StrictFeature::Attachments]
        );
    }

    #[test]
    fn detects_encryption() {
        let json = br#"{"encrypt":{"encrypted":true}}"#;
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::Encrypted]);
    }

    #[test]
    fn clean_document_has_no_violations() {
        let json = br#"{"pages":[{"object":"3 0 R"}],"pdfversion":"1.7"}"#;
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
        assert!(inspection.is_clean());
    }

    #[test]
    fn deduplicates_multiple_matches() {
        let json =
            br#"{"names":{"EmbeddedFiles":{"items":[1]}},"embeddedfiles":{"root":1},"tagged":true}"#;
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
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
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
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
        let inspection = StrictInspection::from_qpdf_json(json).expect("json should parse");
        assert_eq!(inspection.features(), &[StrictFeature::StructTree]);
    }
}
