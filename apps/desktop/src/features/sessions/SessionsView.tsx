import { useCallback, useEffect, useRef, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { GroupedList, Row } from "@/components/GroupedList";
import { IconBadge } from "@/components/IconBadge";
import { AiIcon, ChevronLeftIcon, LibraryIcon, ShareIcon, TrashIcon } from "@/components/icons";
import { Page } from "@/components/Page";
import { ConfirmSheet } from "@/components/Sheet";
import { showToast } from "@/components/Toast";
import { Badge, Button, fieldStyle } from "@/components/controls";
import { FEATURE_STYLE, featureName } from "@/features/home/featureStyle";
import { Spinner } from "@/features/home/FeatureTile";
import { dayGroup, formatClock, formatDuration, formatTime, formatWhen } from "@/lib/format";
import {
  deleteSession,
  exportSession,
  generateSessionReport,
  getSessionTranscript,
  listSessions,
  revealDataFile,
  reviseSessionSegment,
  AudisIpcError,
} from "@/services/ipc";
import type {
  ExportFormat,
  SessionSummary,
  TranscriptSegment,
  UserFacingError,
} from "@/schemas/ipc";

/** Library: every saved session, newest first, grouped by day. */
export function SessionsView({ openId }: { openId?: string | undefined }) {
  const [sessions, setSessions] = useState<SessionSummary[]>();
  const [open, setOpen] = useState<SessionSummary>();
  // Which way the last move went, so coming back slides the other way.
  const [direction, setDirection] = useState<"in" | "out">();
  const [error, setError] = useState<UserFacingError>();

  const refresh = useCallback(() => {
    listSessions()
      .then(setSessions)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  useEffect(refresh, [refresh]);

  // Opened from Home's recent list: go straight to that session.
  useEffect(() => {
    if (!openId || !sessions) return;
    const target = sessions.find((session) => session.id === openId);
    if (target) setOpen(target);
  }, [openId, sessions]);

  if (error) return <ErrorNotice error={error} />;

  if (open) {
    return (
      <div className="audis-push-in">
        <SessionDetail
          session={open}
          onBack={() => {
            setDirection("out");
            setOpen(undefined);
          }}
          onDeleted={() => {
            setDirection("out");
            setOpen(undefined);
            refresh();
          }}
          onError={setError}
        />
      </div>
    );
  }

  const groups = groupByDay(sessions ?? []);

  return (
    <div className={direction === "out" ? "audis-pop-in" : undefined}>
      <Page
        title="Library"
        subtitle={
          sessions === undefined
            ? "Loading your sessions…"
            : sessions.length === 0
              ? "Transcripts you save will appear here."
              : `${sessions.length} saved ${sessions.length === 1 ? "session" : "sessions"}`
        }
      >
        {sessions === undefined ? (
          <div className="flex flex-col gap-2">
            {[0, 1, 2].map((index) => (
              <div key={index} className="audis-skeleton h-[52px]" />
            ))}
          </div>
        ) : sessions.length === 0 ? (
          <EmptyLibrary />
        ) : (
          groups.map(([day, items]) => (
            <GroupedList key={day} title={day}>
              {items.map((session) => {
                const style = FEATURE_STYLE[session.mode];
                return (
                  <Row
                    key={session.id}
                    icon={
                      <IconBadge tint={style.tint} size={30}>
                        {style.icon}
                      </IconBadge>
                    }
                    label={featureName(session.mode)}
                    description={`${formatTime(session.startedAt)} · ${formatDuration(session.elapsedMs)} · ${session.segmentCount} lines`}
                    value={!session.complete ? <Badge tone="warning">Incomplete</Badge> : undefined}
                    onClick={() => {
                      setDirection("in");
                      setOpen(session);
                    }}
                  />
                );
              })}
            </GroupedList>
          ))
        )}
      </Page>
    </div>
  );
}

function EmptyLibrary() {
  return (
    <div
      className="flex flex-col items-center gap-3 px-6 py-14 text-center"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-tile)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <IconBadge tint="orange" size={52}>
        <LibraryIcon />
      </IconBadge>
      <div className="flex max-w-[400px] flex-col gap-1">
        <p className="text-body font-semibold">No saved sessions yet</p>
        <p className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          Transcription and Meeting Assistant save a transcript you can read, edit and export here.
          Live Caption saves nothing.
        </p>
      </div>
    </div>
  );
}

/** Sessions in day groups, keeping the order they arrived in. */
function groupByDay(sessions: SessionSummary[]): [string, SessionSummary[]][] {
  const groups = new Map<string, SessionSummary[]>();
  for (const session of sessions) {
    const day = dayGroup(session.startedAt);
    groups.set(day, [...(groups.get(day) ?? []), session]);
  }
  return [...groups.entries()];
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
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    getSessionTranscript(session.id)
      .then(setSegments)
      .catch((cause: unknown) => onError(toUserFacing(cause)));
  }, [session.id, onError]);

  const doExport = (format: ExportFormat) => {
    setBusy(true);
    exportSession(session.id, format)
      .then((path) =>
        showToast({
          title: `Exported as ${FORMAT_NAMES[format]}`,
          message: "Saved to your exports folder.",
          action: path
            ? { label: "Show", run: () => void revealDataFile(path).catch(() => undefined) }
            : undefined,
        }),
      )
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setBusy(false));
  };

  const doReport = () => {
    setReporting(true);
    setReportSaved(false);
    generateSessionReport(session.id)
      .then(() => {
        setReportSaved(true);
        showToast({
          title: "Report ready",
          message: "The PDF opened, and is in your exports folder.",
        });
      })
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setReporting(false));
  };

  const doDelete = () => {
    setConfirmDelete(false);
    setBusy(true);
    deleteSession(session.id)
      .then(() => {
        showToast({ title: "Session deleted", tone: "neutral" });
        onDeleted();
      })
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setBusy(false));
  };

  const locked = busy || reporting;
  const style = FEATURE_STYLE[session.mode];

  return (
    <div className="flex flex-col gap-6">
      <button
        type="button"
        onClick={onBack}
        className="audis-pressable -ml-1.5 flex items-center gap-0.5 self-start pr-2 text-subheadline [&>svg]:h-4 [&>svg]:w-4"
        style={{ color: "var(--color-accent)" }}
      >
        <ChevronLeftIcon />
        Library
      </button>

      <header className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3.5">
          <IconBadge tint={style.tint} size={44}>
            {style.icon}
          </IconBadge>
          <div className="flex min-w-0 flex-col">
            <h1
              className="text-title2 font-bold"
              style={{ letterSpacing: "var(--tracking-tighter)" }}
            >
              {featureName(session.mode)}
            </h1>
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              {formatWhen(session.startedAt)} · {formatDuration(session.elapsedMs)} ·{" "}
              {session.language === "indonesian" ? "Indonesian" : "English"}
            </span>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <ExportMenu disabled={locked} onExport={doExport} />
          <Button
            onClick={() => setConfirmDelete(true)}
            disabled={locked}
            variant="danger"
            ariaLabel="Delete"
          >
            <TrashIcon />
            Delete
          </Button>
        </div>
      </header>

      <div
        className="flex items-center justify-between gap-4 p-4"
        style={{
          background:
            "linear-gradient(135deg, color-mix(in srgb, var(--tint-indigo) 10%, var(--surface-content)), var(--surface-content) 70%)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        <div className="flex min-w-0 items-center gap-3">
          <IconBadge tint="indigo" size={34}>
            <AiIcon />
          </IconBadge>
          <div className="flex min-w-0 flex-col gap-0.5">
            <span className="text-subheadline font-semibold">Professional report</span>
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              {reporting
                ? "Writing a structured report from this transcript…"
                : reportSaved
                  ? "PDF report saved to your exports folder and opened."
                  : "Summarise this session into a structured PDF with your AI provider."}
            </span>
          </div>
        </div>
        <Button onClick={doReport} disabled={locked} variant="accent" ariaLabel="Generate report">
          {reporting ? <Spinner /> : null}
          {reporting ? "Generating…" : "Generate report"}
        </Button>
      </div>

      <section className="flex flex-col gap-2">
        <h2
          className="px-1 text-footnote font-semibold"
          style={{ color: "var(--label-secondary)" }}
        >
          Transcript
        </h2>
        <div
          className="flex flex-col py-1.5"
          data-selectable
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
          }}
        >
          {segments === undefined ? (
            <div className="flex flex-col gap-2 p-3">
              {[0, 1, 2].map((index) => (
                <div key={index} className="audis-skeleton h-10" />
              ))}
            </div>
          ) : segments.length === 0 ? (
            <span className="p-4 text-footnote" style={{ color: "var(--label-tertiary)" }}>
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
      </section>

      <ConfirmSheet
        open={confirmDelete}
        title="Delete this session?"
        message="Its transcript and any recording will be removed from this PC. This cannot be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={doDelete}
        onCancel={() => setConfirmDelete(false)}
      />
    </div>
  );
}

const FORMAT_NAMES: Record<ExportFormat, string> = {
  text: "Text",
  markdown: "Markdown",
  srt: "SRT subtitles",
};

/** A pop-up menu of export formats, the way a Share button opens one. */
function ExportMenu({
  disabled,
  onExport,
}: {
  disabled: boolean;
  onExport: (format: ExportFormat) => void;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (event: Event) => {
      if (event instanceof KeyboardEvent && event.key !== "Escape") return;
      if (event instanceof MouseEvent && root.current?.contains(event.target as Node)) return;
      setOpen(false);
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", close);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", close);
    };
  }, [open]);

  return (
    <div ref={root} className="relative">
      <Button onClick={() => setOpen((shown) => !shown)} disabled={disabled} ariaLabel="Export">
        <ShareIcon />
        Export
      </Button>
      {open ? (
        <div
          role="menu"
          className="absolute top-full right-0 z-30 mt-1.5 flex min-w-[180px] flex-col p-1"
          style={{
            background: "var(--surface-elevated)",
            borderRadius: 10,
            boxShadow: "var(--shadow-sheet)",
            transformOrigin: "top right",
            animation: "audis-sheet-in var(--spring-snappy-duration) var(--spring-snappy) both",
          }}
        >
          {(Object.keys(FORMAT_NAMES) as ExportFormat[]).map((format) => (
            <button
              key={format}
              type="button"
              role="menuitem"
              onClick={() => {
                setOpen(false);
                onExport(format);
              }}
              className="audis-side-item px-2.5 py-1.5 text-left text-footnote"
              style={{ borderRadius: 6 }}
            >
              {FORMAT_NAMES[format]}
            </button>
          ))}
        </div>
      ) : null}
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

  const mine = segment.source === "microphone";
  const label = segment.speaker ?? (mine ? "You" : "Computer Audio");

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
      <div
        className="mx-1.5 my-1 flex flex-col gap-2 p-3"
        style={{ background: "var(--surface-window)", borderRadius: "var(--radius-control)" }}
      >
        <input
          value={speaker}
          onChange={(event) => setSpeaker(event.target.value)}
          placeholder="Speaker"
          aria-label="Speaker"
          className="w-48 px-2.5 py-[5px] text-caption1"
          style={fieldStyle}
        />
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          aria-label="Line"
          rows={2}
          className="w-full resize-y px-2.5 py-2 text-subheadline"
          style={fieldStyle}
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
    <div className="group flex items-start gap-3 px-4 py-2">
      <span
        className="w-10 shrink-0 pt-[3px] text-caption1 tabular-nums"
        style={{ color: "var(--label-tertiary)" }}
      >
        {formatClock(segment.startMs)}
      </span>
      <p className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span
          className="text-caption1 font-semibold"
          style={{ color: mine ? "var(--color-success)" : "var(--color-accent)" }}
        >
          {label}
        </span>
        <span className="text-subheadline" style={{ color: "var(--label-primary)" }}>
          {segment.text}
        </span>
      </p>
      <button
        type="button"
        onClick={() => setEditing(true)}
        aria-label={`Edit line at ${formatClock(segment.startMs)}`}
        className="shrink-0 px-1.5 py-0.5 text-caption1 font-medium opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        style={{ color: "var(--color-accent)", transitionDuration: "var(--duration-fast)" }}
      >
        Edit
      </button>
    </div>
  );
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
