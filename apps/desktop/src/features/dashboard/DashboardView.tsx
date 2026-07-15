import { useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { useAppInfo } from "@/hooks/useAppInfo";
import { getDiagnostics } from "@/services/ipc";
import type { Diagnostics } from "@/schemas/ipc";
import type { ViewId } from "@/app/navigation";
import { formatBytes } from "@/lib/format";

/**
 * Dashboard.
 *
 * Reports only what Audis can genuinely determine right now: build identity,
 * listening state, and measured storage. Session controls appear once there is
 * a session engine behind them.
 */
export function DashboardView({ onNavigate }: { onNavigate: (id: ViewId) => void }) {
  const state = useAppInfo();
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

  return (
    <div className="flex flex-col gap-8">
      <GroupedList
        title="Status"
        footnote="Audio capture, transcription and the assistant are not built yet. Audis will never listen without showing you that it is."
      >
        <Row
          label={info?.appName ?? "Audis"}
          description={info?.tagline}
          value={info ? `Version ${info.version}` : "Loading…"}
        />
        <Row
          label="Listening"
          description="No session is running."
          value={
            <span className="flex items-center gap-1.5 whitespace-nowrap">
              <span aria-hidden style={{ color: "var(--label-tertiary)" }}>
                ●
              </span>
              Idle
            </span>
          }
        />
        <Row
          label="Microphone"
          description="Capture is not implemented."
          value={<span style={{ color: "var(--label-tertiary)" }}>Unavailable</span>}
        />
        <Row
          label="Computer audio"
          description="Capture is not implemented."
          value={<span style={{ color: "var(--label-tertiary)" }}>Unavailable</span>}
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
