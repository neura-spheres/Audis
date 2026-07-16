import { useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { Button } from "@/components/controls";
import { getDiagnostics, revealDataFile } from "@/services/ipc";
import type { Diagnostics } from "@/schemas/ipc";
import { formatBytes } from "@/lib/format";

/** Where Audis keeps things and how much space it uses. */
export function StorageView() {
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

  return (
    <div className="flex flex-col gap-8">
      <GroupedList
        title="On this PC"
        footnote="Everything Audis records stays on this computer unless you choose a cloud engine."
      >
        <Row
          label="Space used"
          value={diagnostics ? formatBytes(diagnostics.storageBytes) : "Measuring…"}
        />
        <Row label="Files" value={diagnostics ? String(diagnostics.fileCount) : "Measuring…"} />
        <Row
          label="Location"
          stacked
          value={
            <div className="flex min-w-0 items-start justify-between gap-3">
              <code
                data-selectable
                className="min-w-0 flex-1 text-caption1 break-all"
                style={{ fontFamily: "var(--font-mono)", color: "var(--label-secondary)" }}
              >
                {diagnostics?.dataDir ?? "Unknown"}
              </code>
              {diagnostics ? (
                <Button
                  onClick={() => void revealDataFile(diagnostics.dataDir).catch(() => undefined)}
                >
                  Show
                </Button>
              ) : null}
            </div>
          }
        />
      </GroupedList>

      <GroupedList
        title="Retention"
        footnote="Automatic cleanup arrives with recording. Until then, nothing is being written that needs a retention rule."
      >
        <Row
          label="Audio and transcripts"
          description="Not available yet."
          value={<span style={{ color: "var(--label-tertiary)" }}>Keep everything</span>}
        />
      </GroupedList>
    </div>
  );
}
