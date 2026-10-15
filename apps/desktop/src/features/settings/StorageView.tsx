import { useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { Page } from "@/components/Page";
import { Button } from "@/components/controls";
import { FilesView } from "@/features/files/FilesView";
import { getDiagnostics, revealDataFile } from "@/services/ipc";
import type { Diagnostics } from "@/schemas/ipc";
import { formatBytes } from "@/lib/format";

/** Where Audis keeps things, how much space it uses, and every file in it. */
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
    <Page
      title="Storage"
      subtitle="Everything Audis records stays on this computer unless you choose a cloud engine."
    >
      <GroupedList title="On this PC">
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
        footnote="Automatic cleanup is not built yet. Delete sessions you no longer need from Library."
      >
        <Row
          label="Audio and transcripts"
          value={<span style={{ color: "var(--label-tertiary)" }}>Keep everything</span>}
        />
      </GroupedList>

      <FilesView />
    </Page>
  );
}
