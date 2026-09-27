//! Request and response shapes, and validation of what callers send.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const MAX_FILES_PER_VERSION: usize = 8;
pub const MAX_FILE_BYTES: i64 = 50 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Musicxml,
    Midi,
    Audio,
    Pdf,
    Other,
}

impl FileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Musicxml => "musicxml",
            Self::Midi => "midi",
            Self::Audio => "audio",
            Self::Pdf => "pdf",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "musicxml" => Ok(Self::Musicxml),
            "midi" => Ok(Self::Midi),
            "audio" => Ok(Self::Audio),
            "pdf" => Ok(Self::Pdf),
            "other" => Ok(Self::Other),
            other => Err(AppError::Database(format!("unknown file kind {other}"))),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NewVersionRequest {
    pub title: Option<String>,
    pub label: String,
    #[serde(default)]
    pub notes: String,
    pub source_ref: Option<String>,
    /// Number of the piece's latest version to replace instead of adding one.
    pub replaces: Option<i32>,
    pub files: Vec<NewFile>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewFile {
    pub filename: String,
    pub kind: FileKind,
    pub content_type: String,
    pub size_bytes: i64,
}

/// A validated publish request, ready for the store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewVersion {
    pub slug: String,
    pub title: Option<String>,
    pub label: String,
    pub notes: String,
    pub source: &'static str,
    pub source_ref: Option<String>,
    pub replaces: Option<i32>,
    pub files: Vec<PlannedFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedFile {
    pub id: Uuid,
    pub filename: String,
    pub kind: FileKind,
    pub content_type: String,
    pub size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileSummary {
    pub id: Uuid,
    pub kind: FileKind,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VersionSummary {
    pub id: Uuid,
    pub number: i32,
    pub label: String,
    pub notes: String,
    pub source: String,
    pub source_ref: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub files: Vec<FileSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PieceSummary {
    pub slug: String,
    pub title: String,
    pub version_count: i64,
    pub latest: Option<VersionSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PieceDetail {
    pub slug: String,
    pub title: String,
    pub versions: Vec<VersionSummary>,
}

pub fn validate_slug(slug: &str) -> AppResult<()> {
    let valid = !slug.is_empty()
        && slug.len() <= 64
        && slug.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if valid {
        Ok(())
    } else {
        Err(AppError::Validation(
            "piece slug must be lowercase letters, digits, '_' or '-' (max 64)".to_string(),
        ))
    }
}

pub fn validate_new_version(
    slug: &str,
    request: NewVersionRequest,
    source: &'static str,
) -> AppResult<NewVersion> {
    validate_slug(slug)?;
    let title = optional_text(request.title, "title", 200)?;
    let label = required_text(&request.label, "label", 200)?;
    let notes = bounded_text(&request.notes, "notes", 4000)?;
    let source_ref = optional_text(request.source_ref, "source_ref", 200)?;
    if request.replaces.is_some_and(|number| number <= 0) {
        return Err(AppError::Validation(
            "replaces must be a version number".to_string(),
        ));
    }
    let files = validate_files(request.files)?;
    Ok(NewVersion {
        slug: slug.to_string(),
        title,
        label,
        notes,
        source,
        source_ref,
        replaces: request.replaces,
        files,
    })
}

fn validate_files(files: Vec<NewFile>) -> AppResult<Vec<PlannedFile>> {
    if files.is_empty() || files.len() > MAX_FILES_PER_VERSION {
        return Err(AppError::Validation(format!(
            "a version needs 1 to {MAX_FILES_PER_VERSION} files"
        )));
    }
    let mut seen = HashSet::new();
    files
        .into_iter()
        .map(|file| {
            validate_file(&file)?;
            if !seen.insert(file.filename.clone()) {
                return Err(AppError::Validation(format!(
                    "duplicate filename {}",
                    file.filename
                )));
            }
            Ok(PlannedFile {
                id: Uuid::new_v4(),
                filename: file.filename,
                kind: file.kind,
                content_type: file.content_type,
                size_bytes: file.size_bytes,
            })
        })
        .collect()
}

fn validate_file(file: &NewFile) -> AppResult<()> {
    let name_ok = !file.filename.is_empty()
        && file.filename.len() <= 128
        && !file.filename.starts_with('.')
        && file
            .filename
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '));
    if !name_ok {
        return Err(AppError::Validation(format!(
            "invalid filename {:?}",
            file.filename
        )));
    }
    let content_type_ok = (3..=100).contains(&file.content_type.len())
        && file.content_type.split('/').count() == 2
        && !file.content_type.contains(char::is_whitespace);
    if !content_type_ok {
        return Err(AppError::Validation("invalid content_type".to_string()));
    }
    if file.size_bytes <= 0 || file.size_bytes > MAX_FILE_BYTES {
        return Err(AppError::Validation(format!(
            "size_bytes must be between 1 and {MAX_FILE_BYTES}"
        )));
    }
    Ok(())
}

fn required_text(value: &str, field: &str, max: usize) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{field} is required")));
    }
    bounded_text(trimmed, field, max)
}

fn bounded_text(value: &str, field: &str, max: usize) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.chars().count() > max {
        return Err(AppError::Validation(format!(
            "{field} is longer than {max}"
        )));
    }
    Ok(trimmed.to_string())
}

fn optional_text(value: Option<String>, field: &str, max: usize) -> AppResult<Option<String>> {
    match value.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(text) => bounded_text(text, field, max).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> NewFile {
        NewFile {
            filename: name.to_string(),
            kind: FileKind::Musicxml,
            content_type: "application/vnd.recordare.musicxml+xml".to_string(),
            size_bytes: 1024,
        }
    }

    fn request(files: Vec<NewFile>) -> NewVersionRequest {
        NewVersionRequest {
            title: Some(" Boreal Pocket ".to_string()),
            label: "  key ladders ".to_string(),
            notes: String::new(),
            source_ref: Some("66db377".to_string()),
            replaces: None,
            files,
        }
    }

    #[test]
    fn test_valid_request_is_trimmed_and_planned() {
        let version = validate_new_version(
            "boreal_pocket",
            request(vec![file("boreal_pocket.musicxml")]),
            "agent",
        )
        .unwrap();
        assert_eq!(version.label, "key ladders");
        assert_eq!(version.title.as_deref(), Some("Boreal Pocket"));
        assert_eq!(version.files.len(), 1);
    }

    #[test]
    fn test_slug_rules() {
        assert!(validate_slug("boreal_pocket").is_ok());
        for bad in ["", "Boreal", "-x", "a/b", &"a".repeat(65)] {
            assert!(validate_slug(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn test_replaces_must_be_a_positive_number() {
        let mut req = request(vec![file("a.mid")]);
        req.replaces = Some(0);
        assert!(validate_new_version("p", req, "agent").is_err());
        let mut req = request(vec![file("a.mid")]);
        req.replaces = Some(3);
        let version = validate_new_version("p", req, "agent").unwrap();
        assert_eq!(version.replaces, Some(3));
    }

    #[test]
    fn test_rejects_blank_label() {
        let mut req = request(vec![file("a.mid")]);
        req.label = "   ".to_string();
        assert!(validate_new_version("p", req, "agent").is_err());
    }

    #[test]
    fn test_rejects_duplicate_and_unsafe_filenames() {
        let dupes = request(vec![file("a.mid"), file("a.mid")]);
        assert!(validate_new_version("p", dupes, "agent").is_err());
        for bad in ["../x.mid", ".hidden", "a/b.mid", ""] {
            let req = request(vec![file(bad)]);
            assert!(validate_new_version("p", req, "agent").is_err(), "{bad}");
        }
    }

    #[test]
    fn test_rejects_file_count_and_size_limits() {
        assert!(validate_new_version("p", request(vec![]), "agent").is_err());
        let many = (0..=MAX_FILES_PER_VERSION)
            .map(|i| file(&format!("f{i}.mid")))
            .collect();
        assert!(validate_new_version("p", request(many), "agent").is_err());
        let mut big = file("big.wav");
        big.size_bytes = MAX_FILE_BYTES + 1;
        assert!(validate_new_version("p", request(vec![big]), "agent").is_err());
    }

    #[test]
    fn test_file_kind_round_trips() {
        for kind in [FileKind::Musicxml, FileKind::Midi, FileKind::Audio] {
            assert_eq!(FileKind::parse(kind.as_str()).unwrap(), kind);
        }
        assert!(FileKind::parse("exe").is_err());
    }
}
