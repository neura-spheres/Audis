import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { ExternalIcon, RevealIcon } from "@/components/icons";
import { listDataFiles, openDataFile, revealDataFile, AudisIpcError } from "@/services/ipc";
import type { DataFileListing, DataFile, UserFacingError } from "@/schemas/ipc";
import { formatBytes, formatWhen } from "@/lib/format";

/**
 * Every file Audis has written, grouped by category, with the ability to open
 * each one or show it in File Explorer.
 *
 * This lists real files from disk. Empty categories are shown too, so the
 * storage layout is legible before anything has been recorded.
 */
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

      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-subheadline font-medium">
            {listing ? `${listing.totalFiles} files` : "Reading files…"}
          </span>
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {listing ? `${formatBytes(listing.totalBytes)} on this PC` : " "}
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
          <h2 className="text-body font-semibold">No files yet</h2>
          <p className="max-w-[420px] text-subheadline" style={{ color: "var(--label-secondary)" }}>
            Recordings, transcripts and exports will appear here once you run a session. Audis has
            created the folders below and is ready for them.
          </p>
        </div>
      ) : null}

      {populated.map((group) => (
        <section key={group.category} className="flex flex-col">
          <div className="mb-2 flex items-baseline justify-between gap-3 px-3">
            <h2 className="text-footnote font-medium" style={{ color: "var(--label-secondary)" }}>
              {group.label}
            </h2>
            <span className="text-caption1" style={{ color: "var(--label-tertiary)" }}>
              {group.files.length} · {formatBytes(group.totalBytes)}
            </span>
          </div>

          <div
            className="overflow-hidden"
            style={{
              background: "var(--surface-content)",
              borderRadius: "var(--radius-card)",
              boxShadow: "var(--shadow-card)",
            }}
          >
            {group.files.map((file) => (
              <FileRow
                key={file.path}
                file={file}
                onOpen={() => act(openDataFile, file.path)}
                onReveal={() => act(revealDataFile, file.path)}
              />
            ))}
          </div>
        </section>
      ))}

      {empty.length > 0 ? (
        <section className="flex flex-col">
          <h2
            className="mb-2 px-3 text-footnote font-medium"
            style={{ color: "var(--label-secondary)" }}
          >
            Empty folders
          </h2>
          <div
            className="overflow-hidden"
            style={{
              background: "var(--surface-content)",
              borderRadius: "var(--radius-card)",
              boxShadow: "var(--shadow-card)",
            }}
          >
            {empty.map((group) => (
              <div
                key={group.category}
                className="flex items-center justify-between gap-3 px-3 py-2.5 first:border-t-0"
                style={{ borderTop: "0.5px solid var(--separator)" }}
              >
                <span className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
                  {group.label}
                </span>
                <span className="text-caption1" style={{ color: "var(--label-tertiary)" }}>
                  Empty
                </span>
              </div>
            ))}
          </div>
        </section>
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
    <div
      className="group flex min-h-[52px] items-center gap-3 px-3 py-2 first:border-t-0"
      style={{ borderTop: "0.5px solid var(--separator)" }}
    >
      {/* min-w-0 is what allows the long relative path below to truncate
          instead of pushing the buttons out of the card. */}
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
