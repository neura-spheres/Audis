import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { DownloadIcon, TrashIcon } from "@/components/icons";
import { AUDIS_EVENTS } from "@/services/events";
import {
  cancelModelDownload,
  installModel,
  isModelDownloading,
  listModels,
  removeModel,
  AudisIpcError,
} from "@/services/ipc";
import {
  modelProgressSchema,
  type InstalledModel,
  type ModelId,
  type ModelProgress,
  type UserFacingError,
} from "@/schemas/ipc";
import { formatBytes } from "@/lib/format";

/**
 * Install and remove local speech models.
 *
 * Models are downloaded rather than bundled, so the installer stays small and a
 * user only pays for the one they use. Everything here is free and runs on this
 * PC with no account.
 */
export function ModelsView() {
  const [models, setModels] = useState<InstalledModel[]>();
  const [progress, setProgress] = useState<ModelProgress>();
  const [error, setError] = useState<UserFacingError>();

  const refresh = useCallback(() => {
    listModels()
      .then(setModels)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  useEffect(refresh, [refresh]);

  // A download outlives the view that started it, so on mount we ask whether
  // one is already running rather than assuming it is not.
  useEffect(() => {
    isModelDownloading()
      .then((running) => {
        if (running && !progress) {
          setProgress({
            id: "whisperBase",
            downloadedBytes: 0,
            totalBytes: null,
            done: false,
            error: null,
          });
        }
      })
      .catch(() => undefined);
    // Intentionally on mount only: this is a one-time state restore.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let cancelled = false;
    const unlisten = listen(AUDIS_EVENTS.modelProgress, (event) => {
      const parsed = modelProgressSchema.safeParse(event.payload);
      if (!parsed.success) return;

      if (parsed.data.done || parsed.data.error) {
        setProgress(undefined);
        refresh();
        if (parsed.data.error) {
          setError({
            title: "The model could not be installed",
            explanation: parsed.data.error,
            dataPreserved: true,
            suggestedAction: "Check your connection and try again.",
            technicalDetails: null,
            diagnosticCode: "UNEXPECTED",
          });
        }
        return;
      }

      setProgress(parsed.data);
    });

    return () => {
      cancelled = true;
      void unlisten.then((stop) => {
        if (cancelled) stop();
      });
    };
  }, [refresh]);

  const install = (id: ModelId) => {
    setError(undefined);
    setProgress({ id, downloadedBytes: 0, totalBytes: null, done: false, error: null });
    installModel(id)
      .catch((cause: unknown) => {
        setProgress(undefined);
        setError(toUserFacing(cause));
      })
      .finally(refresh);
  };

  const remove = (id: ModelId) => {
    removeModel(id)
      .then(refresh)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  };

  return (
    <div className="flex flex-col gap-5">
      {error ? <ErrorNotice error={error} /> : null}

      <p className="px-1 text-subheadline" style={{ color: "var(--label-secondary)" }}>
        Speech models run on this PC. They are free, work offline, and need no account. Audis
        recognises Indonesian and English.
      </p>

      <div className="flex flex-col gap-3">
        {(models ?? []).map((model) => (
          <ModelCard
            key={model.info.id}
            model={model}
            progress={progress?.id === model.info.id ? progress : undefined}
            busy={progress !== undefined}
            onInstall={() => install(model.info.id)}
            onRemove={() => remove(model.info.id)}
            onCancel={() => void cancelModelDownload().catch(() => undefined)}
          />
        ))}
      </div>
    </div>
  );
}

function ModelCard({
  model,
  progress,
  busy,
  onInstall,
  onRemove,
  onCancel,
}: {
  model: InstalledModel;
  progress: ModelProgress | undefined;
  busy: boolean;
  onInstall: () => void;
  onRemove: () => void;
  onCancel: () => void;
}) {
  const { info } = model;
  const downloading = progress !== undefined;

  return (
    <section
      className="flex flex-col gap-3 p-4"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="flex min-w-0 flex-col gap-1">
          <div className="flex items-center gap-2">
            <h2 className="text-body font-semibold">{info.name}</h2>
            {info.recommended ? (
              <span
                className="px-1.5 py-0.5 text-caption2 font-medium"
                style={{
                  background: "color-mix(in srgb, var(--color-accent) 15%, transparent)",
                  borderRadius: "var(--radius-chip)",
                  color: "var(--color-accent)",
                }}
              >
                Recommended
              </span>
            ) : null}
            {model.installed ? (
              <span className="text-caption2 font-medium" style={{ color: "var(--color-success)" }}>
                Installed
              </span>
            ) : null}
          </div>
          <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
            {info.summary}
          </p>
          <p className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            {formatBytes(model.installedBytes ?? info.sizeBytes)} · {info.requirement}
          </p>
        </div>

        <div className="flex shrink-0 gap-2">
          {model.installed ? (
            <Button onClick={onRemove} variant="danger" ariaLabel={`Remove ${info.name}`}>
              <TrashIcon />
              Remove
            </Button>
          ) : downloading ? (
            <Button onClick={onCancel} variant="standard">
              Cancel
            </Button>
          ) : (
            <Button
              onClick={onInstall}
              variant={info.recommended ? "accent" : "standard"}
              // One download at a time: two large models at once would starve
              // each other and make both progress bars meaningless.
              disabled={busy}
              ariaLabel={`Install ${info.name}`}
            >
              <DownloadIcon />
              Install
            </Button>
          )}
        </div>
      </div>

      {downloading ? <DownloadProgress progress={progress} fallbackTotal={info.sizeBytes} /> : null}
    </section>
  );
}

function DownloadProgress({
  progress,
  fallbackTotal,
}: {
  progress: ModelProgress;
  fallbackTotal: number;
}) {
  // The server's length when it gave one, otherwise the catalogue figure, so
  // the bar still moves rather than sitting at zero.
  const total = progress.totalBytes ?? fallbackTotal;
  const fraction = total > 0 ? Math.min(1, progress.downloadedBytes / total) : 0;

  return (
    <div className="flex flex-col gap-1.5">
      <div
        className="h-1.5 w-full overflow-hidden"
        style={{ background: "var(--surface-sunken)", borderRadius: "var(--radius-chip)" }}
        role="progressbar"
        aria-valuenow={Math.round(fraction * 100)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label="Download progress"
      >
        <div
          className="h-full transition-[width]"
          style={{
            width: `${fraction * 100}%`,
            background: "var(--color-accent)",
            transitionDuration: "var(--duration-standard)",
          }}
        />
      </div>
      <p className="text-caption1 tabular-nums" style={{ color: "var(--label-secondary)" }}>
        {progress.downloadedBytes === 0
          ? "Starting…"
          : `${formatBytes(progress.downloadedBytes)} of ${formatBytes(total)}`}
      </p>
    </div>
  );
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not manage your models",
    explanation: "Something went wrong. Your installed models were not affected.",
    dataPreserved: true,
    suggestedAction: "Try again.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
