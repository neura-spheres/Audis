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
    /// Good English, modest size. Weak at Indonesian.
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
    /// Whether this model can decode speech faster than it arrives.
    ///
    /// The property that decides whether a model is usable at all for live
    /// captions: below real time it keeps up, above it the captions fall
    /// further behind every sentence until they are worthless.
    pub keeps_up_live: bool,
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

    /// Where this model is downloaded from.
    ///
    /// Separate from `info` so that downloading, which has nothing to do with
    /// language, does not have to invent one to ask for.
    pub fn url(self) -> String {
        // ggerganov's official whisper.cpp model host.
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.file_name()
        )
    }

    /// The model to recommend for `language`.
    ///
    /// Whisper's training is overwhelmingly English, and the smaller models pay
    /// for that unevenly: Base is genuinely good at English and genuinely poor
    /// at Indonesian, where it misreads ordinary words. Recommending one model
    /// for both languages meant recommending the wrong one to every Indonesian
    /// speaker, which is exactly the group Audis is for.
    pub fn recommended_for(language: crate::language::Language) -> Self {
        // Base for both, and for Indonesian that is a compromise rather than a
        // happy answer. Small and Medium really are more accurate at
        // Indonesian, and neither can decode faster than people speak: measured
        // on a 12-core CPU, Medium runs at 4.85x real time and Small lands near
        // 2x, against Base's 0.61x. A model that falls permanently behind is
        // not a better transcript, it is no transcript. `keeps_up_live` is what
        // says so on the page rather than leaving the user to discover it.
        let _ = language;
        Self::WhisperBase
    }

    /// Whether this model decodes faster than speech arrives.
    ///
    /// Measured on a 12-core desktop CPU with a release build and greedy
    /// decoding. A slower machine will do worse, so this is optimistic by
    /// design: it must never promise real time to someone who will not get it.
    pub fn keeps_up_live(self) -> bool {
        match self {
            // 0.61x real time measured; Tiny is faster still.
            Self::WhisperTiny | Self::WhisperBase => true,
            // Small ~2x, Medium 4.85x measured. Both fall behind permanently.
            Self::WhisperSmall | Self::WhisperMedium => false,
        }
    }

    /// Catalogue entry, as described for someone recognising `language`.
    ///
    /// Sizes are the published sizes of the ggml builds. They are used for the
    /// progress bar and the "this will use N MB" warning, so they are close
    /// enough to be honest without being load-bearing: the real size comes from
    /// the response's Content-Length.
    pub fn info(self, language: crate::language::Language) -> ModelInfo {
        // Summaries say what each model is actually like in each language
        // rather than averaging them into a comfortable half-truth. Base used
        // to claim it "handles Indonesian and English well"; it does not.
        let (name, summary, size_bytes, requirement) = match self {
            Self::WhisperTiny => (
                "Whisper Tiny",
                "Fastest, and the least accurate. Usable for clear English. Not good enough for \
                 Indonesian.",
                77_700_000,
                "Runs on any PC.",
            ),
            Self::WhisperBase => (
                "Whisper Base",
                "Good for English, and light on your CPU. Understands Indonesian, but misreads \
                 ordinary words often enough to be frustrating.",
                148_000_000,
                "Runs comfortably on any modern PC.",
            ),
            Self::WhisperSmall => (
                "Whisper Small",
                "More accurate than Base, especially for Indonesian and with background noise, \
                 but too slow for live captions on most PCs: it decodes at roughly twice real \
                 time, so captions fall behind and never catch up.",
                488_000_000,
                "Cannot keep up with live speech on a typical desktop CPU.",
            ),
            Self::WhisperMedium => (
                "Whisper Medium",
                "The most accurate model Audis runs locally, and by far the slowest. Measured at \
                 nearly five times real time on a fast 12-core CPU, so it cannot caption live \
                 speech at all.",
                1_530_000_000,
                "Far too slow to keep up with live speech on any current CPU.",
            ),
        };

        ModelInfo {
            id: self,
            name: name.to_owned(),
            summary: summary.to_owned(),
            size_bytes,
            file_name: self.file_name().to_owned(),
            url: self.url(),
            requirement: requirement.to_owned(),
            recommended: self == Self::recommended_for(language),
            keeps_up_live: self.keeps_up_live(),
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
    use crate::language::Language;

    #[test]
    fn every_model_has_a_distinct_file_and_url() {
        let mut files = std::collections::HashSet::new();
        for id in ModelId::ALL {
            let info = id.info(Language::English);
            assert!(files.insert(info.file_name.clone()), "duplicate file name");
            assert!(info.url.ends_with(&info.file_name));
            assert!(info.url.starts_with("https://"), "model URLs must be HTTPS");
            assert!(info.size_bytes > 0);
            assert!(!info.summary.is_empty());
        }
    }

    /// Exactly one recommendation. Two would be no recommendation at all.
    #[test]
    fn exactly_one_model_is_recommended_per_language() {
        for language in [Language::English, Language::Indonesian] {
            let recommended: Vec<_> = ModelId::ALL
                .iter()
                .filter(|id| id.info(language).recommended)
                .collect();
            assert_eq!(
                recommended.len(),
                1,
                "{language:?} must have exactly one recommended model"
            );
        }
    }

    /// Audis must never claim Base is good at Indonesian. It is not: it misreads
    /// ordinary words, and the old summary promising it "handles Indonesian and
    /// English well" is what sent Indonesian speakers to a bad experience while
    /// telling them it was the right choice.
    ///
    /// It is still what gets recommended, because it is the only model that
    /// decodes faster than people speak. That is a compromise, not a claim, and
    /// the honest summary is what carries the difference.
    #[test]
    fn base_is_never_described_as_good_at_indonesian() {
        let base = ModelId::WhisperBase.info(Language::Indonesian);

        assert!(
            !base.summary.contains("Handles Indonesian and English well"),
            "the summary must not claim Base is good at Indonesian"
        );
        assert!(
            base.summary.contains("misreads"),
            "the summary must say plainly that Base misreads Indonesian"
        );
    }

    /// A model is only recommendable if it can keep up with live speech.
    ///
    /// Small and Medium are genuinely more accurate at Indonesian, and both
    /// decode slower than people talk (measured: Medium at 4.85x real time,
    /// Small near 2x, against Base's 0.61x). Recommending one would trade a
    /// mediocre transcript for captions that fall permanently behind, which is
    /// not a better transcript but no transcript.
    #[test]
    fn only_a_model_that_keeps_up_is_ever_recommended() {
        for language in [Language::English, Language::Indonesian] {
            let recommended = ModelId::recommended_for(language);
            assert!(
                recommended.keeps_up_live(),
                "{language:?} is recommended {recommended:?}, which cannot keep up with live speech"
            );
        }
    }

    /// The slow models must say so, or a user pays 1.5 GB to find out.
    #[test]
    fn a_model_that_cannot_keep_up_says_so_in_its_description() {
        for id in [ModelId::WhisperSmall, ModelId::WhisperMedium] {
            let info = id.info(Language::Indonesian);
            assert!(!info.keeps_up_live);
            assert!(
                info.summary.contains("slow") || info.summary.contains("real time"),
                "{} must warn that it cannot keep up",
                info.name
            );
            assert!(
                info.requirement.contains("keep up") || info.requirement.contains("slow"),
                "{} must warn in its requirement line too",
                info.name
            );
        }
    }

    #[test]
    fn catalogue_is_ordered_smallest_first() {
        let sizes: Vec<u64> = ModelId::ALL
            .iter()
            .map(|id| id.info(Language::English).size_bytes)
            .collect();
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
