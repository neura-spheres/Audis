//! Types for browsing the files Audis creates.

use serde::{Deserialize, Serialize};

/// Which part of the data directory a file belongs to.
///
/// Derived from the file's location rather than its extension, so the UI can
/// group files without guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataCategory {
    /// The SQLite database and its sidecars.
    Database,
    /// Per-session transcripts and journals.
    Sessions,
    /// Captured audio.
    Recordings,
    /// Local inference models.
    Models,
    /// Regenerable cache.
    Cache,
    /// Rolling logs.
    Logs,
    /// Staged updater artifacts.
    Updates,
    /// User-facing exports.
    Exports,
    /// Scratch space.
    Temp,
    /// Anything directly in the data root, such as settings.json.
    Other,
}

impl DataCategory {
    /// The directory name this category lives in.
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Database => "database",
            Self::Sessions => "sessions",
            Self::Recordings => "recordings",
            Self::Models => "models",
            Self::Cache => "cache",
            Self::Logs => "logs",
            Self::Updates => "updates",
            Self::Exports => "exports",
            Self::Temp => "temp",
            Self::Other => "",
        }
    }

    /// Label shown in the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Database => "Database",
            Self::Sessions => "Sessions",
            Self::Recordings => "Recordings",
            Self::Models => "Models",
            Self::Cache => "Cache",
            Self::Logs => "Logs",
            Self::Updates => "Updates",
            Self::Exports => "Exports",
            Self::Temp => "Temporary",
            Self::Other => "Other",
        }
    }

    /// Map a top-level directory name onto a category.
    pub fn from_dir_name(name: &str) -> Self {
        match name {
            "database" => Self::Database,
            "sessions" => Self::Sessions,
            "recordings" => Self::Recordings,
            "models" => Self::Models,
            "cache" => Self::Cache,
            "logs" => Self::Logs,
            "updates" => Self::Updates,
            "exports" => Self::Exports,
            "temp" => Self::Temp,
            _ => Self::Other,
        }
    }

    /// Every category, in the order the UI lists them.
    pub const ALL: [Self; 9] = [
        Self::Sessions,
        Self::Recordings,
        Self::Exports,
        Self::Models,
        Self::Database,
        Self::Logs,
        Self::Cache,
        Self::Updates,
        Self::Temp,
    ];
}

/// One file Audis created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFile {
    /// Absolute path. The frontend passes this back to open or reveal the file,
    /// and Rust re-checks that it is inside the data root before acting.
    pub path: String,
    /// Path relative to the data root, for display.
    pub relative_path: String,
    /// File name only.
    pub name: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Last modified time, RFC 3339. `None` when the platform does not report it.
    pub modified: Option<String>,
    /// Which part of the data directory this belongs to.
    pub category: DataCategory,
}

/// Files in one category, with a total so the UI does not have to sum them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataCategoryGroup {
    /// The category.
    pub category: DataCategory,
    /// Display label.
    pub label: String,
    /// Absolute path of the category directory.
    pub path: String,
    /// Files inside, newest first.
    pub files: Vec<DataFile>,
    /// Sum of `size_bytes` across `files`.
    pub total_bytes: u64,
}

/// Everything Audis has written, grouped by category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFileListing {
    /// Absolute path of the data root.
    pub root: String,
    /// One entry per category, including empty ones so the UI can show the
    /// full shape of the storage layout.
    pub groups: Vec<DataCategoryGroup>,
    /// Total bytes across every category.
    pub total_bytes: u64,
    /// Total number of files.
    pub total_files: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_names_round_trip_through_categories() {
        for category in DataCategory::ALL {
            assert_eq!(
                DataCategory::from_dir_name(category.dir_name()),
                category,
                "{category:?} did not round-trip"
            );
        }
    }

    #[test]
    fn unknown_directories_map_to_other() {
        assert_eq!(DataCategory::from_dir_name("nonsense"), DataCategory::Other);
        assert_eq!(DataCategory::from_dir_name(""), DataCategory::Other);
    }

    #[test]
    fn every_listed_category_has_a_label() {
        for category in DataCategory::ALL {
            assert!(!category.label().is_empty());
        }
    }
}
