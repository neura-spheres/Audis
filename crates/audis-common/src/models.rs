//! The catalogue of local speech models Audis can install.
//!
//! Models are downloaded on demand rather than bundled, so the installer stays
//! small and a user only pays for the one they actually use.

use serde::{Deserialize, Serialize};

/// A model Audis can download and run locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelId {
    /// Fastest, least accurate. Usable on any machine.
    WhisperTiny,
    /// The sensible default: good Indonesian and English, modest size.
    WhisperBase,
    /// Noticeably better, still real-time on a modern CPU.
    WhisperSmall,
    /// Best accuracy Audis offers locally. Wants a strong CPU.
    WhisperMedium,
}

/// Everything the UI needs to describe and install a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Stable identifier.
    pub id: ModelId,
    /// Name shown to the user.
    pub name: String,
    /// One line on what this trade-off buys.
    pub summary: String,
    /// Download size in bytes.
    pub size_bytes: u64,
    /// File name on disk.
    pub file_name: String,
    /// Where to fetch it.
    pub url: String,
    /// Rough guidance on hardware, in plain words.
    pub requirement: String,
    /// Whether Audis recommends this one.
    pub recommended: bool,
}

impl ModelId {
    /// Every model in the catalogue, smallest first.
    pub const ALL: [Self; 4] = [
        Self::WhisperTiny,
        Self::WhisperBase,
        Self::WhisperSmall,
        Self::WhisperMedium,
    ];

    /// The file this model is stored as.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::WhisperTiny => "ggml-tiny.bin",
            Self::WhisperBase => "ggml-base.bin",
            Self::WhisperSmall => "ggml-small.bin",
            Self::WhisperMedium => "ggml-medium.bin",
        }
    }

    /// Catalogue entry.
    ///
    /// Sizes are the published sizes of the ggml builds. They are used for the
    /// progress bar and the "this will use N MB" warning, so they are close
    /// enough to be honest without being load-bearing: the real size comes from
    /// the response's Content-Length.
    pub fn info(self) -> ModelInfo {
        let (name, summary, size_bytes, requirement, recommended) = match self {
            Self::WhisperTiny => (
                "Whisper Tiny",
                "Fastest. Fine for clear speech, but it will misread names and jargon.",
                77_700_000,
                "Runs on any PC.",
                false,
            ),
            Self::WhisperBase => (
                "Whisper Base",
                "The best balance for most people. Handles Indonesian and English well.",
                148_000_000,
                "Runs comfortably on any modern PC.",
                true,
            ),
            Self::WhisperSmall => (
                "Whisper Small",
                "Noticeably more accurate, especially with accents and background noise.",
                488_000_000,
                "Wants a recent 4-core CPU or better.",
                false,
            ),
            Self::WhisperMedium => (
                "Whisper Medium",
                "The most accurate model Audis runs locally.",
                1_530_000_000,
                "Wants a strong CPU. May not keep up live on older machines.",
                false,
            ),
        };

        ModelInfo {
            id: self,
            name: name.to_owned(),
            summary: summary.to_owned(),
            size_bytes,
            file_name: self.file_name().to_owned(),
            // ggerganov's official whisper.cpp model host.
            url: format!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
                self.file_name()
            ),
            requirement: requirement.to_owned(),
            recommended,
        }
    }
}

/// Whether a model is on disk, and how big it actually is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModel {
    /// Catalogue entry.
    pub info: ModelInfo,
    /// True when the file exists locally and is usable.
    pub installed: bool,
    /// Actual size on disk, when installed.
    pub installed_bytes: Option<u64>,
}

/// Progress of a download, carried on `audis://model/progress`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    /// Which model.
    pub id: ModelId,
    /// Bytes fetched so far.
    pub downloaded_bytes: u64,
    /// Total size, when the server reported one.
    pub total_bytes: Option<u64>,
    /// True once the file is verified and in place.
    pub done: bool,
    /// Set when the download failed.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_has_a_distinct_file_and_url() {
        let mut files = std::collections::HashSet::new();
        for id in ModelId::ALL {
            let info = id.info();
            assert!(files.insert(info.file_name.clone()), "duplicate file name");
            assert!(info.url.ends_with(&info.file_name));
            assert!(info.url.starts_with("https://"), "model URLs must be HTTPS");
            assert!(info.size_bytes > 0);
            assert!(!info.summary.is_empty());
        }
    }

    /// Exactly one recommendation. Two would be no recommendation at all.
    #[test]
    fn exactly_one_model_is_recommended() {
        let recommended: Vec<_> = ModelId::ALL
            .iter()
            .filter(|id| id.info().recommended)
            .collect();
        assert_eq!(recommended.len(), 1);
        assert_eq!(*recommended[0], ModelId::WhisperBase);
    }

    #[test]
    fn catalogue_is_ordered_smallest_first() {
        let sizes: Vec<u64> = ModelId::ALL.iter().map(|id| id.info().size_bytes).collect();
        let mut sorted = sizes.clone();
        sorted.sort_unstable();
        assert_eq!(sizes, sorted);
    }

    #[test]
    fn model_ids_serialise_as_camel_case() {
        let json = serde_json::to_string(&ModelId::WhisperBase).unwrap();
        assert_eq!(json, "\"whisperBase\"");
    }
}
