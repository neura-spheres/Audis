import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { GroupedList, Row } from "@/components/GroupedList";
import { Button } from "@/components/controls";
import { ExternalIcon, RevealIcon } from "@/components/icons";
import { listDataFiles, openDataFile, revealDataFile, AudisIpcError } from "@/services/ipc";
import type { DataFileListing, DataFile, UserFacingError } from "@/schemas/ipc";
import { formatBytes, formatWhen } from "@/lib/format";

/** Every file Audis has written, grouped by category, with the ability to open */
export function FilesView() {
  const [listing, setListing] = useState<DataFileListing>();
  const [error, setError] = useState<UserFacingError>();
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    setLoading(true);
    listDataFiles()
      .then((result) => {
        setListing(result);
        setError(undefined);
      })
      .catch((cause: unknown) => setError(toUserFacing(cause)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(refresh, [refresh]);

  const act = (action: (path: string) => Promise<void>, path: string) => {
    action(path).catch((cause: unknown) => setError(toUserFacing(cause)));
  };

  if (error && !listing) {
    return <ErrorNotice error={error} />;
  }

  const groups = listing?.groups ?? [];
  const populated = groups.filter((group) => group.files.length > 0);
  const empty = groups.filter((group) => group.files.length === 0);

  return (
    <div className="flex flex-col gap-6">
      {error ? <ErrorNotice error={error} /> : null}

      <div className="flex items-center justify-between gap-4 px-1">
        <div className="flex min-w-0 flex-col">
          <h2 className="text-title3 font-semibold">Files</h2>
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {listing
              ? `${listing.totalFiles} files · ${formatBytes(listing.totalBytes)} on this PC`
              : "Reading files…"}
          </span>
        </div>
        <Button onClick={refresh} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </Button>
      </div>

      {listing && listing.totalFiles === 0 ? (
        <div
          className="flex flex-col items-center gap-1.5 px-6 py-10 text-center"
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
          }}
        >
          <h3 className="text-body font-semibold">No files yet</h3>
          <p className="max-w-[420px] text-subheadline" style={{ color: "var(--label-secondary)" }}>
            Recordings, transcripts and exports will appear here once you run a session. Audis has
            created the folders below and is ready for them.
          </p>
        </div>
      ) : null}

      {populated.map((group) => (
        <GroupedList
          key={group.category}
          title={group.label}
          accessory={`${group.files.length} · ${formatBytes(group.totalBytes)}`}
        >
          {group.files.map((file) => (
            <FileRow
              key={file.path}
              file={file}
              onOpen={() => act(openDataFile, file.path)}
              onReveal={() => act(revealDataFile, file.path)}
            />
          ))}
        </GroupedList>
      ))}

      {empty.length > 0 ? (
        <GroupedList title="Empty folders">
          {empty.map((group) => (
            <Row
              key={group.category}
              label={<span style={{ color: "var(--label-secondary)" }}>{group.label}</span>}
              value={<span className="text-caption1">Empty</span>}
            />
          ))}
        </GroupedList>
      ) : null}
    </div>
  );
}

function FileRow({
  file,
  onOpen,
  onReveal,
}: {
  file: DataFile;
  onOpen: () => void;
  onReveal: () => void;
}) {
  return (
    <div className="audis-row flex min-h-[52px] items-center gap-3 px-3.5 py-2">
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="truncate text-subheadline" title={file.name}>
          {file.name}
        </span>
        <span
          className="truncate text-caption1"
          style={{ color: "var(--label-tertiary)" }}
          title={file.relativePath}
        >
          {file.relativePath}
        </span>
      </div>

      <span
        className="shrink-0 text-caption1 tabular-nums"
        style={{ color: "var(--label-secondary)" }}
      >
        {formatBytes(file.sizeBytes)}
      </span>
      <span
        className="hidden shrink-0 text-caption1 sm:inline"
        style={{ color: "var(--label-tertiary)" }}
      >
        {formatWhen(file.modified)}
      </span>

      <div className="flex shrink-0 items-center gap-1.5">
        <Button onClick={onOpen} title={`Open ${file.name}`} ariaLabel={`Open ${file.name}`}>
          <ExternalIcon />
          Open
        </Button>
        <Button
          onClick={onReveal}
          title="Show in File Explorer"
          ariaLabel={`Show ${file.name} in File Explorer`}
        >
          <RevealIcon />
        </Button>
      </div>
    </div>
  );
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not read your files",
    explanation: "The file list could not be loaded. Nothing on disk was changed.",
    dataPreserved: true,
    suggestedAction: "Try refreshing.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
