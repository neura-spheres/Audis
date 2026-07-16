import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { formatWhen } from "@/lib/format";
import {
  deleteSession,
  exportSession,
  getSessionTranscript,
  listSessions,
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

  const doDelete = () => {
    setBusy(true);
    deleteSession(session.id)
      .then(onDeleted)
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setBusy(false));
  };

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
          <Button onClick={() => doExport("text")} disabled={busy}>
            Text
          </Button>
          <Button onClick={() => doExport("markdown")} disabled={busy}>
            Markdown
          </Button>
          <Button onClick={() => doExport("srt")} disabled={busy}>
            SRT
          </Button>
          <Button onClick={doDelete} disabled={busy} variant="danger">
            Delete
          </Button>
        </div>
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
            <p key={segment.id} className="flex flex-col gap-0.5">
              <span
                className="text-caption2 font-medium"
                style={{ color: sourceColour(segment.source) }}
              >
                {segment.speaker ?? (segment.source === "microphone" ? "You" : "Computer Audio")}
              </span>
              <span className="text-subheadline" style={{ color: "var(--label-primary)" }}>
                {segment.text}
              </span>
            </p>
          ))
        )}
      </div>
    </div>
  );
}

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
