//! Session transcript persistence.

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
    /// A correction to an earlier segment, applied on read.
    Revision(Box<audis_common::SegmentRevision>),
    /// Session summary. Absent if Audis was killed mid-session, which is how a
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

/// A saved session, as the library lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    /// Session id.
    pub id: Uuid,
    /// Which feature produced it.
    pub mode: SessionMode,
    /// The language recognised.
    pub language: Language,
    /// When it started, RFC 3339.
    pub started_at: String,
    /// When it ended, or `None` if it was cut off before finishing.
    pub ended_at: Option<String>,
    /// How many segments it holds.
    pub segment_count: usize,
    /// Captured milliseconds.
    pub elapsed_ms: u64,
    /// Whether the file has a footer, i.e. the session ended cleanly.
    pub complete: bool,
}

/// The format an exported transcript is written in.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    /// Plain text.
    Text,
    /// Markdown, with speakers in bold.
    Markdown,
    /// SubRip subtitles, timed from each segment.
    Srt,
}

/// Every saved session, newest first.
pub fn list_summaries(paths: &AppPaths) -> Vec<SessionSummary> {
    let mut sessions = Vec::new();
    let Ok(entries) = std::fs::read_dir(paths.sessions_dir()) else {
        return sessions;
    };

    for entry in entries.flatten() {
        let file = entry.path().join("transcript.jsonl");
        if let Some(summary) = read_summary(&file) {
            sessions.push(summary);
        }
    }

    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    sessions
}

pub fn session_summary(paths: &AppPaths, id: Uuid) -> Option<SessionSummary> {
    let file = paths.session_dir(id).join("transcript.jsonl");
    read_summary(&file)
}

fn read_summary(path: &std::path::Path) -> Option<SessionSummary> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut header = None;
    let mut footer = None;
    let mut count = 0;

    for line in content.lines() {
        match serde_json::from_str::<TranscriptLine>(line) {
            Ok(TranscriptLine::Header(value)) => header = Some(value),
            Ok(TranscriptLine::Segment(_)) => count += 1,
            Ok(TranscriptLine::Footer(value)) => footer = Some(value),
            Ok(TranscriptLine::Revision(_)) => {}
            Err(_) => {}
        }
    }

    let header = header?;
    Some(SessionSummary {
        id: header.id,
        mode: header.mode,
        language: header.language,
        started_at: header.started_at,
        ended_at: footer.as_ref().map(|f| f.ended_at.clone()),
        segment_count: footer.as_ref().map_or(count, |f| f.segment_count),
        elapsed_ms: footer.as_ref().map_or(0, |f| f.elapsed_ms),
        complete: footer.is_some(),
    })
}

/// Every segment of one saved session.
pub fn read_segments(paths: &AppPaths, id: Uuid) -> Result<Vec<TranscriptSegment>> {
    let path = paths.session_dir(id).join("transcript.jsonl");
    let content = std::fs::read_to_string(&path).map_err(|source| AudisError::Io {
        path,
        detail: "could not read the transcript".to_owned(),
        source,
    })?;

    let mut segments = Vec::new();
    let mut revisions = Vec::new();
    for line in content.lines() {
        match serde_json::from_str(line) {
            Ok(TranscriptLine::Segment(segment)) => segments.push(*segment),
            Ok(TranscriptLine::Revision(revision)) => revisions.push(*revision),
            _ => {}
        }
    }

    for revision in revisions {
        if let Some(segment) = segments.iter_mut().find(|s| s.id == revision.id) {
            segment.text = revision.text;
            segment.speaker = revision.speaker;
        }
    }

    Ok(segments)
}

/// Append a correction to a saved session and return the corrected segment.
pub fn revise_segment(
    paths: &AppPaths,
    session_id: Uuid,
    revision: audis_common::SegmentRevision,
) -> Result<TranscriptSegment> {
    let path = paths.session_dir(session_id).join("transcript.jsonl");

    let json = serde_json::to_string(&TranscriptLine::Revision(Box::new(revision.clone())))
        .map_err(|source| AudisError::Serialization {
            context: "a transcript revision".to_owned(),
            source,
        })?;

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .map_err(|source| AudisError::Io {
            path: path.clone(),
            detail: "could not open the transcript to correct it".to_owned(),
            source,
        })?;
    writeln!(file, "{json}").map_err(|source| AudisError::Io {
        path: path.clone(),
        detail: "could not write the correction".to_owned(),
        source,
    })?;

    read_segments(paths, session_id)?
        .into_iter()
        .find(|segment| segment.id == revision.id)
        .ok_or(AudisError::InvalidArgument {
            field: "segmentId".to_owned(),
            detail: "no such segment in this session".to_owned(),
        })
}

/// Delete a saved session and everything in its folder.
pub fn delete(paths: &AppPaths, id: Uuid) -> Result<()> {
    let dir = paths.session_dir(id);
    std::fs::remove_dir_all(&dir).map_err(|source| AudisError::Io {
        path: dir,
        detail: "could not delete the session".to_owned(),
        source,
    })
}

/// Write a session's transcript to the exports folder and return its path.
pub fn export(paths: &AppPaths, id: Uuid, format: ExportFormat) -> Result<std::path::PathBuf> {
    let segments = read_segments(paths, id)?;
    let body = render(&segments, format);

    let dir = paths.exports_dir();
    std::fs::create_dir_all(&dir).map_err(|source| AudisError::Io {
        path: dir.clone(),
        detail: "could not create the exports folder".to_owned(),
        source,
    })?;

    let extension = match format {
        ExportFormat::Text => "txt",
        ExportFormat::Markdown => "md",
        ExportFormat::Srt => "srt",
    };
    let path = dir.join(format!("session-{id}.{extension}"));
    std::fs::write(&path, body).map_err(|source| AudisError::Io {
        path: path.clone(),
        detail: "could not write the export".to_owned(),
        source,
    })?;
    Ok(path)
}

pub fn write_report(paths: &AppPaths, id: Uuid, pdf: &[u8]) -> Result<std::path::PathBuf> {
    let dir = paths.exports_dir();
    std::fs::create_dir_all(&dir).map_err(|source| AudisError::Io {
        path: dir.clone(),
        detail: "could not create the exports folder".to_owned(),
        source,
    })?;

    let path = dir.join(format!("session-{id}-report.pdf"));
    std::fs::write(&path, pdf).map_err(|source| AudisError::Io {
        path: path.clone(),
        detail: "could not write the report".to_owned(),
        source,
    })?;
    Ok(path)
}

pub fn plain_transcript(segments: &[TranscriptSegment]) -> String {
    render(segments, ExportFormat::Text)
}

fn render(segments: &[TranscriptSegment], format: ExportFormat) -> String {
    let speaker = |segment: &TranscriptSegment| {
        segment
            .speaker
            .clone()
            .unwrap_or_else(|| segment.source.default_label().to_owned())
    };

    match format {
        ExportFormat::Text => segments
            .iter()
            .map(|segment| format!("{}: {}", speaker(segment), segment.text))
            .collect::<Vec<_>>()
            .join("\n"),
        ExportFormat::Markdown => {
            let mut out = String::from("# Transcript\n\n");
            for segment in segments {
                out.push_str(&format!("**{}** {}\n\n", speaker(segment), segment.text));
            }
            out
        }
        ExportFormat::Srt => {
            let mut out = String::new();
            for (index, segment) in segments.iter().enumerate() {
                out.push_str(&format!(
                    "{}\n{} --> {}\n{}: {}\n\n",
                    index + 1,
                    srt_time(segment.start_ms),
                    srt_time(segment.end_ms),
                    speaker(segment),
                    segment.text
                ));
            }
            out
        }
    }
}

fn srt_time(ms: i64) -> String {
    let ms = ms.max(0);
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
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

        let path = paths.session_dir(id).join("transcript.jsonl");
        drop(writer);

        let contents = std::fs::read_to_string(&path).expect("read");
        assert!(contents.contains("before the crash"));

        assert!(!contents.contains("\"footer\""));
    }

    #[test]
    fn a_revision_corrects_a_segment_on_read() {
        let (_dir, paths) = paths();
        let id = Uuid::new_v4();

        let mut writer =
            SessionWriter::create(&paths, id, SessionMode::Transcription, Language::English)
                .expect("create");
        let original = segment(id, "helo wrold");
        let segment_id = original.id;
        writer.append(&original).expect("append");
        writer.finish().expect("finish");

        let updated = revise_segment(
            &paths,
            id,
            audis_common::SegmentRevision {
                id: segment_id,
                text: "hello world".to_owned(),
                speaker: Some("Alice".to_owned()),
            },
        )
        .expect("revise");

        assert_eq!(updated.text, "hello world");
        assert_eq!(updated.speaker.as_deref(), Some("Alice"));

        let segments = read_segments(&paths, id).expect("read");
        assert_eq!(segments.len(), 1, "a revision must not add a segment");
        assert_eq!(segments[0].text, "hello world");
        assert_eq!(segments[0].speaker.as_deref(), Some("Alice"));
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
