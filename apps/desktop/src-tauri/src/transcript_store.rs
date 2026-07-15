//! Session transcript persistence.
//!
//! Transcripts are written as JSON Lines, appended one segment at a time and
//! flushed as they arrive. A meeting can run for an hour, and the failure that
//! matters is losing all of it: with an append-only file, a crash or a power
//! cut costs at most the sentence in flight. A single JSON document would have
//! to be rewritten whole and would be empty until the session ended.
//!
//! Which modes save is decided by [`audis_common::FeatureId::persists_transcript`],
//! not here. Live Caption promises that nothing is written to disk, and this
//! module is never constructed for it.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use audis_common::{AppPaths, AudisError, Language, Result, SessionMode, TranscriptSegment};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The first line of a transcript file: what this session was.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHeader {
    /// Session id, matching the directory name.
    pub id: Uuid,
    /// Which feature produced it.
    pub mode: SessionMode,
    /// The language recognised.
    pub language: Language,
    /// When it started, as RFC 3339.
    pub started_at: String,
    /// Schema version of this file.
    pub version: u32,
}

/// The last line: how it ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFooter {
    /// Where the last transcribed segment ended, in milliseconds from the start.
    pub elapsed_ms: u64,
    /// How many segments were written.
    pub segment_count: usize,
    /// When it ended, as RFC 3339.
    pub ended_at: String,
}

/// One line of a transcript file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TranscriptLine {
    /// Session metadata. Always the first line.
    Header(SessionHeader),
    /// One recognised segment.
    Segment(Box<TranscriptSegment>),
    /// Session summary. Absent if Audis was killed mid-session, which is how a
    /// reader can tell a transcript is truncated rather than complete.
    Footer(SessionFooter),
}

/// Schema version of a transcript file.
const TRANSCRIPT_VERSION: u32 = 1;

/// Appends a session's transcript to disk.
pub struct SessionWriter {
    file: BufWriter<File>,
    path: PathBuf,
    segment_count: usize,
    /// End of the last segment written, which is how far the transcript covers.
    last_end_ms: i64,
}

impl SessionWriter {
    /// Create the session directory and open its transcript.
    pub fn create(
        paths: &AppPaths,
        id: Uuid,
        mode: SessionMode,
        language: Language,
    ) -> Result<Self> {
        let dir = paths.session_dir(id);
        std::fs::create_dir_all(&dir).map_err(|source| AudisError::Io {
            path: dir.clone(),
            detail: "could not create the folder for this session".to_owned(),
            source,
        })?;

        let path = dir.join("transcript.jsonl");
        let file = File::create(&path).map_err(|source| AudisError::Io {
            path: path.clone(),
            detail: "could not create the transcript file".to_owned(),
            source,
        })?;

        let mut writer = Self {
            file: BufWriter::new(file),
            path,
            segment_count: 0,
            last_end_ms: 0,
        };

        writer.write_line(&TranscriptLine::Header(SessionHeader {
            id,
            mode,
            language,
            started_at: now(),
            version: TRANSCRIPT_VERSION,
        }))?;

        Ok(writer)
    }

    /// Append one recognised segment.
    pub fn append(&mut self, segment: &TranscriptSegment) -> Result<()> {
        self.write_line(&TranscriptLine::Segment(Box::new(segment.clone())))?;
        self.segment_count += 1;
        self.last_end_ms = self.last_end_ms.max(segment.end_ms);
        Ok(())
    }

    /// Close the transcript, recording how it ended.
    ///
    /// The elapsed time is taken from the audio actually transcribed rather
    /// than passed in: this runs on the recognise thread, which has no view of
    /// the session clock, and a number invented here would be worse than one
    /// derived from the transcript itself.
    pub fn finish(mut self) -> Result<PathBuf> {
        self.write_line(&TranscriptLine::Footer(SessionFooter {
            elapsed_ms: u64::try_from(self.last_end_ms).unwrap_or(0),
            segment_count: self.segment_count,
            ended_at: now(),
        }))?;

        self.file.flush().map_err(|source| AudisError::Io {
            path: self.path.clone(),
            detail: "could not finish writing the transcript".to_owned(),
            source,
        })?;

        Ok(self.path)
    }

    /// How many segments have been written.
    pub fn segment_count(&self) -> usize {
        self.segment_count
    }

    fn write_line(&mut self, line: &TranscriptLine) -> Result<()> {
        let json = serde_json::to_string(line).map_err(|source| AudisError::Serialization {
            context: "a transcript line".to_owned(),
            source,
        })?;

        writeln!(self.file, "{json}").map_err(|source| AudisError::Io {
            path: self.path.clone(),
            detail: "could not write to the transcript".to_owned(),
            source,
        })?;

        // Flushed per segment so a crash loses at most the sentence in flight
        // rather than everything since the last buffer boundary. Segments are
        // seconds apart, so this costs nothing measurable.
        self.file.flush().map_err(|source| AudisError::Io {
            path: self.path.clone(),
            detail: "could not save the transcript".to_owned(),
            source,
        })
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use audis_common::AudioSourceKind;

    fn paths() -> (tempfile::TempDir, AppPaths) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths::rooted_at(dir.path());
        (dir, paths)
    }

    fn segment(session_id: Uuid, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: Uuid::new_v4(),
            session_id,
            source: AudioSourceKind::Microphone,
            speaker: Some("You".to_owned()),
            start_ms: 0,
            end_ms: 1_000,
            text: text.to_owned(),
            language: Language::English,
            confidence: Some(0.9),
            is_final: true,
            engine: "whisper".to_owned(),
        }
    }

    #[test]
    fn a_transcript_round_trips_through_the_file() {
        let (_dir, paths) = paths();
        let id = Uuid::new_v4();

        let mut writer =
            SessionWriter::create(&paths, id, SessionMode::Transcription, Language::English)
                .expect("create");
        writer.append(&segment(id, "hello there")).expect("append");
        writer.append(&segment(id, "second line")).expect("append");
        let path = writer.finish().expect("finish");

        let contents = std::fs::read_to_string(&path).expect("read");
        let lines: Vec<TranscriptLine> = contents
            .lines()
            .map(|line| serde_json::from_str(line).expect("parse"))
            .collect();

        assert_eq!(lines.len(), 4, "header, two segments, footer");
        assert!(matches!(lines[0], TranscriptLine::Header(_)));
        assert!(matches!(lines[3], TranscriptLine::Footer(_)));

        let TranscriptLine::Segment(first) = &lines[1] else {
            panic!("expected a segment");
        };
        assert_eq!(first.text, "hello there");
    }

    /// The reason for JSON Lines. A session killed mid-meeting must still yield
    /// every sentence written before the crash.
    #[test]
    fn segments_survive_a_session_that_never_finishes() {
        let (_dir, paths) = paths();
        let id = Uuid::new_v4();

        let mut writer =
            SessionWriter::create(&paths, id, SessionMode::Transcription, Language::English)
                .expect("create");
        writer
            .append(&segment(id, "before the crash"))
            .expect("append");

        // Dropped without finish(), which is what a power cut looks like.
        let path = paths.session_dir(id).join("transcript.jsonl");
        drop(writer);

        let contents = std::fs::read_to_string(&path).expect("read");
        assert!(contents.contains("before the crash"));

        // No footer: a reader can tell this transcript is truncated.
        assert!(!contents.contains("\"footer\""));
    }

    #[test]
    fn each_session_gets_its_own_folder() {
        let (_dir, paths) = paths();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();

        SessionWriter::create(&paths, first, SessionMode::Transcription, Language::English)
            .expect("first")
            .finish()
            .expect("finish");
        SessionWriter::create(
            &paths,
            second,
            SessionMode::Transcription,
            Language::Indonesian,
        )
        .expect("second")
        .finish()
        .expect("finish");

        assert!(paths.session_dir(first).join("transcript.jsonl").exists());
        assert!(paths.session_dir(second).join("transcript.jsonl").exists());
    }
}
