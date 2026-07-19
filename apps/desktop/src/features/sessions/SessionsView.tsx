import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { formatWhen } from "@/lib/format";
import {
  deleteSession,
  exportSession,
  generateSessionReport,
  getSessionTranscript,
  listSessions,
  reviseSessionSegment,
  AudisIpcError,
} from "@/services/ipc";
import type {
  ExportFormat,
  SessionSummary,
  TranscriptSegment,
  UserFacingError,
} from "@/schemas/ipc";

export function SessionsView() {
  const [sessions, setSessions] = useState<SessionSummary[]>();
  const [open, setOpen] = useState<SessionSummary>();
  const [error, setError] = useState<UserFacingError>();

  const refresh = useCallback(() => {
    listSessions()
      .then(setSessions)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  useEffect(refresh, [refresh]);

  if (error) return <ErrorNotice error={error} />;

  if (open) {
    return (
      <SessionDetail
        session={open}
        onBack={() => setOpen(undefined)}
        onDeleted={() => {
          setOpen(undefined);
          refresh();
        }}
        onError={setError}
      />
    );
  }

  if (sessions === undefined) {
    return (
      <p className="px-1 text-footnote" style={{ color: "var(--label-tertiary)" }}>
        Loading your sessions…
      </p>
    );
  }

  if (sessions.length === 0) {
    return (
      <div
        className="flex flex-col items-center gap-2 px-6 py-16 text-center"
        style={{ color: "var(--label-secondary)" }}
      >
        <p className="text-body font-semibold" style={{ color: "var(--label-primary)" }}>
          No saved sessions yet
        </p>
        <p className="text-footnote">
          Transcription and Meeting Assistant save a transcript you can read here. Live Caption
          saves nothing.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2.5">
      {sessions.map((session) => (
        <button
          key={session.id}
          type="button"
          onClick={() => setOpen(session)}
          className="flex items-center justify-between gap-4 p-4 text-left"
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
          }}
        >
          <div className="flex min-w-0 flex-col gap-0.5">
            <div className="flex items-center gap-2">
              <span className="text-body font-semibold">{modeName(session.mode)}</span>
              {!session.complete ? (
                <span className="text-caption2" style={{ color: "var(--color-warning)" }}>
                  Incomplete
                </span>
              ) : null}
            </div>
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              {formatWhen(session.startedAt)} · {formatDuration(session.elapsedMs)} ·{" "}
              {session.segmentCount} lines
            </span>
          </div>
          <span aria-hidden style={{ color: "var(--label-tertiary)" }}>
            ›
          </span>
        </button>
      ))}
    </div>
  );
}

function SessionDetail({
  session,
  onBack,
  onDeleted,
  onError,
}: {
  session: SessionSummary;
  onBack: () => void;
  onDeleted: () => void;
  onError: (error: UserFacingError) => void;
}) {
  const [segments, setSegments] = useState<TranscriptSegment[]>();
  const [busy, setBusy] = useState(false);
  const [reporting, setReporting] = useState(false);
  const [reportSaved, setReportSaved] = useState(false);

  useEffect(() => {
    getSessionTranscript(session.id)
      .then(setSegments)
      .catch((cause: unknown) => onError(toUserFacing(cause)));
  }, [session.id, onError]);

  const doExport = (format: ExportFormat) => {
    setBusy(true);
    exportSession(session.id, format)
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setBusy(false));
  };

  const doReport = () => {
    setReporting(true);
    setReportSaved(false);
    generateSessionReport(session.id)
      .then(() => setReportSaved(true))
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setReporting(false));
  };

  const doDelete = () => {
    setBusy(true);
    deleteSession(session.id)
      .then(onDeleted)
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setBusy(false));
  };

  const locked = busy || reporting;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-3">
        <button
          type="button"
          onClick={onBack}
          className="text-footnote"
          style={{ color: "var(--color-accent)" }}
        >
          ‹ All sessions
        </button>
        <div className="flex items-center gap-2">
          <Button onClick={() => doExport("text")} disabled={locked}>
            Text
          </Button>
          <Button onClick={() => doExport("markdown")} disabled={locked}>
            Markdown
          </Button>
          <Button onClick={() => doExport("srt")} disabled={locked}>
            SRT
          </Button>
          <Button onClick={doDelete} disabled={locked} variant="danger">
            Delete
          </Button>
        </div>
      </div>

      <div
        className="flex items-center justify-between gap-4 p-3.5"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-subheadline font-semibold">Professional report</span>
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {reporting
              ? "Generating a structured report from this transcript…"
              : reportSaved
                ? "PDF report saved to your exports folder and opened."
                : "Summarise this session into a structured, professional PDF report with your AI provider."}
          </span>
        </div>
        <Button onClick={doReport} disabled={locked} variant="accent" ariaLabel="Generate report">
          {reporting ? "Generating…" : "Generate report"}
        </Button>
      </div>

      <div className="flex flex-col gap-0.5 px-1">
        <span className="text-body font-semibold">{modeName(session.mode)}</span>
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {formatWhen(session.startedAt)} · {formatDuration(session.elapsedMs)} ·{" "}
          {session.language === "indonesian" ? "Indonesian" : "English"}
        </span>
      </div>

      <div
        className="flex flex-col gap-3 p-4"
        data-selectable
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        {segments === undefined ? (
          <span className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            Loading transcript…
          </span>
        ) : segments.length === 0 ? (
          <span className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            This session has no transcribed lines.
          </span>
        ) : (
          segments.map((segment) => (
            <SegmentRow
              key={segment.id}
              segment={segment}
              sessionId={session.id}
              onRevised={(updated) =>
                setSegments((current) =>
                  current?.map((item) => (item.id === updated.id ? updated : item)),
                )
              }
              onError={onError}
            />
          ))
        )}
      </div>
    </div>
  );
}

function SegmentRow({
  segment,
  sessionId,
  onRevised,
  onError,
}: {
  segment: TranscriptSegment;
  sessionId: string;
  onRevised: (updated: TranscriptSegment) => void;
  onError: (error: UserFacingError) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(segment.text);
  const [speaker, setSpeaker] = useState(segment.speaker ?? "");
  const [busy, setBusy] = useState(false);

  const label = segment.speaker ?? (segment.source === "microphone" ? "You" : "Computer Audio");

  const save = () => {
    if (!text.trim()) return;
    setBusy(true);
    reviseSessionSegment(sessionId, segment.id, text.trim(), speaker.trim() || null)
      .then((updated) => {
        onRevised(updated);
        setEditing(false);
      })
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setBusy(false));
  };

  const cancel = () => {
    setText(segment.text);
    setSpeaker(segment.speaker ?? "");
    setEditing(false);
  };

  if (editing) {
    return (
      <div className="flex flex-col gap-1.5">
        <input
          value={speaker}
          onChange={(event) => setSpeaker(event.target.value)}
          placeholder="Speaker"
          className="w-full px-2.5 py-[5px] text-caption2"
          style={editInputStyle}
        />
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          rows={2}
          className="w-full resize-y px-2.5 py-2 text-subheadline"
          style={editInputStyle}
        />
        <div className="flex gap-2">
          <Button onClick={save} disabled={busy || !text.trim()} variant="accent">
            Save
          </Button>
          <Button onClick={cancel} disabled={busy}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <p className="flex flex-col gap-0.5">
      <span className="text-caption2 font-medium" style={{ color: sourceColour(segment.source) }}>
        {label}
      </span>
      <span className="text-subheadline" style={{ color: "var(--label-primary)" }}>
        {segment.text}
        <button
          type="button"
          onClick={() => setEditing(true)}
          className="ml-2 align-baseline text-caption2"
          style={{ color: "var(--color-accent)" }}
        >
          Edit
        </button>
      </span>
    </p>
  );
}

const editInputStyle = {
  background: "var(--surface-elevated)",
  color: "var(--label-primary)",
  border: "0.5px solid var(--border-control)",
  borderRadius: "var(--radius-control)",
} as const;

function modeName(mode: string): string {
  const names: Record<string, string> = {
    liveCaption: "Live Caption",
    transcription: "Transcription",
    meetingAssistant: "Meeting Assistant",
    interviewPractice: "Interview Practice",
  };
  return names[mode] ?? "Session";
}

function sourceColour(source: TranscriptSegment["source"]): string {
  return source === "microphone" ? "var(--color-success)" : "var(--color-accent)";
}

function formatDuration(ms: number): string {
  const total = Math.floor(ms / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  if (minutes === 0) return `${seconds}s`;
  return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not load your sessions",
    explanation: "Something went wrong. Your saved sessions were not changed.",
    dataPreserved: true,
    suggestedAction: "Try again.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
