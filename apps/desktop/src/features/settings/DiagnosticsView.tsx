import { useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { Button } from "@/components/controls";
import { getDiagnostics, revealDataFile } from "@/services/ipc";
import type { Diagnostics } from "@/schemas/ipc";
import { formatBytes } from "@/lib/format";

/** Real environment information, read from the Rust core on mount. */
export function DiagnosticsSection() {
  const [diagnostics, setDiagnostics] = useState<Diagnostics>();

  // Secondary information on About: if it cannot be read, the rows say
  // "Unknown" rather than putting a second error on a page that has a primary
  // purpose of its own.
  useEffect(() => {
    let active = true;
    getDiagnostics()
      .then((result) => active && setDiagnostics(result))
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);

  const unknown = "Unknown";

  return (
    <div className="flex flex-col gap-7">
      <GroupedList title="This PC">
        <Row label="Operating system" value={diagnostics?.os ?? unknown} />
        <Row label="Architecture" value={diagnostics?.arch ?? unknown} />
        <Row label="Tauri" value={diagnostics?.tauriVersion ?? unknown} />
        <Row label="WebView2" value={diagnostics?.webviewVersion ?? unknown} />
      </GroupedList>

      <GroupedList
        title="Diagnostics"
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
