import { useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { useAppInfo } from "@/hooks/useAppInfo";
import { useSession } from "@/hooks/useSession";
import { getDiagnostics } from "@/services/ipc";
import type { Diagnostics } from "@/schemas/ipc";
import type { ViewId } from "@/app/navigation";
import { formatBytes } from "@/lib/format";

/** Dashboard. */
export function DashboardView({ onNavigate }: { onNavigate: (id: ViewId) => void }) {
  const state = useAppInfo();
  const { session } = useSession();
  const [diagnostics, setDiagnostics] = useState<Diagnostics>();

  useEffect(() => {
    let active = true;
    getDiagnostics()
      .then((result) => active && setDiagnostics(result))
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);

  if (state.status === "error") {
    return <ErrorNotice error={state.error} />;
  }

  const info = state.status === "ready" ? state.info : undefined;
  const listening = session?.state === "listening";
  const listeningLabel = session
    ? session.state === "paused"
      ? "Paused"
      : listening
        ? "Listening"
        : "Starting…"
    : "Idle";

  return (
    <div className="flex flex-col gap-8">
      <GroupedList
        title="Status"
        footnote="Recognition runs on this PC. Audis will never listen without showing you that it is."
      >
        <Row
          label={info?.appName ?? "Audis"}
          description={info?.tagline}
          value={info ? `Version ${info.version}` : "Loading…"}
        />
        <Row
          label="Listening"
          description={
            listening
              ? `${session?.mode ? modeName(session.mode) : "A session"} is running.`
              : "No session is running."
          }
          value={
            <span className="flex items-center gap-1.5 whitespace-nowrap">
              <span
                aria-hidden
                style={{ color: listening ? "var(--color-success)" : "var(--label-tertiary)" }}
              >
                ●
              </span>
              {listeningLabel}
            </span>
          }
        />
        <Row
          label="Microphone"
          description="What you say. Always labelled as you."
          value={<SourceState on={session?.microphone === true} live={listening} />}
        />
        <Row
          label="Computer audio"
          description="Everyone else in a call, and any video you play."
          value={<SourceState on={session?.computerAudio === true} live={listening} />}
        />
      </GroupedList>

      <GroupedList
        title="Storage"
        footnote="Recordings, models and logs are stored on this PC only. Nothing is uploaded."
      >
        <Row
          label="Space used"
          value={diagnostics ? formatBytes(diagnostics.storageBytes) : "Measuring…"}
        />
        <Row
          label="Files"
          description="Everything Audis has written so far."
          value={
            <div className="flex items-center gap-2.5">
              <span>{diagnostics ? diagnostics.fileCount : "…"}</span>
              <Button onClick={() => onNavigate("files")}>Browse</Button>
            </div>
          }
        />
      </GroupedList>

      <GroupedList title="Environment">
        <Row label="Windows" value={diagnostics?.os ?? "Loading…"} />
        <Row label="WebView2" value={diagnostics?.webviewVersion ?? "Unknown"} />
        <Row
          label="Diagnostics"
          description="Versions, paths and storage detail."
          value={<Button onClick={() => onNavigate("diagnostics")}>Open</Button>}
        />
      </GroupedList>
    </div>
  );
}

/** Whether a source is being captured by the running session. */
function SourceState({ on, live }: { on: boolean; live: boolean }) {
  if (!on) {
    return <span style={{ color: "var(--label-tertiary)" }}>Not capturing</span>;
  }

  return (
    <span style={{ color: live ? "var(--color-success)" : "var(--label-secondary)" }}>
      {live ? "Capturing" : "Held"}
    </span>
  );
}

/** The user-facing name of a running mode. */
function modeName(mode: string): string {
  const names: Record<string, string> = {
    liveCaption: "Live Caption",
    transcription: "Transcription",
    meetingAssistant: "Meeting Assistant",
    interviewPractice: "Interview Practice",
  };
  return names[mode] ?? "A session";
}
