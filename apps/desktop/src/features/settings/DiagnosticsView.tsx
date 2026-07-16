import { useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { getDiagnostics, revealDataFile, AudisIpcError } from "@/services/ipc";
import type { Diagnostics, UserFacingError } from "@/schemas/ipc";
import { formatBytes } from "@/lib/format";

/** Real environment information, read from the Rust core on mount. */
export function DiagnosticsView() {
  const [diagnostics, setDiagnostics] = useState<Diagnostics>();
  const [error, setError] = useState<UserFacingError>();

  useEffect(() => {
    let active = true;
    getDiagnostics()
      .then((result) => active && setDiagnostics(result))
      .catch((cause: unknown) => {
        if (active) setError(cause instanceof AudisIpcError ? cause.userFacing : undefined);
      });
    return () => {
      active = false;
    };
  }, []);

  if (error) return <ErrorNotice error={error} />;

  const unknown = "Unknown";

  return (
    <div className="flex flex-col gap-8">
      <GroupedList title="Application">
        <Row label="Version" value={diagnostics?.appVersion ?? unknown} />
        <Row label="Tauri" value={diagnostics?.tauriVersion ?? unknown} />
        <Row label="WebView2" value={diagnostics?.webviewVersion ?? unknown} />
      </GroupedList>

      <GroupedList title="System">
        <Row label="Operating system" value={diagnostics?.os ?? unknown} />
        <Row label="Architecture" value={diagnostics?.arch ?? unknown} />
      </GroupedList>

      <GroupedList
        title="Storage"
        footnote="Logs never contain transcript text, audio, or API keys."
      >
        <Row
          label="Files written"
          value={
            diagnostics
              ? `${diagnostics.fileCount} · ${formatBytes(diagnostics.storageBytes)}`
              : unknown
          }
        />
        <Row label="Data folder" stacked value={<PathValue path={diagnostics?.dataDir} />} />
        <Row label="Logs" stacked value={<PathValue path={diagnostics?.logsDir} />} />
      </GroupedList>
    </div>
  );
}

/** A long filesystem path with a reveal action. */
function PathValue({ path }: { path: string | undefined }) {
  if (!path) return <span style={{ color: "var(--label-tertiary)" }}>Unknown</span>;

  return (
    <div className="flex min-w-0 items-start justify-between gap-3">
      <code
        data-selectable
        className="min-w-0 flex-1 text-caption1 break-all"
        style={{ fontFamily: "var(--font-mono)", color: "var(--label-secondary)" }}
      >
        {path}
      </code>
      <Button onClick={() => void revealDataFile(path).catch(() => undefined)}>Show</Button>
    </div>
  );
}
