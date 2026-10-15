import { useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { IconBadge } from "@/components/IconBadge";
import { Section } from "@/components/Page";
import { Badge, Button } from "@/components/controls";
import { ChipIcon, DownloadIcon, TrashIcon } from "@/components/icons";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import {
  cancelModelDownload,
  installModel,
  isModelDownloading,
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
 * The list belongs to the Speech page, which shares it with the engine picker,
 * so `onChanged` is how an install or removal reaches both.
 */
export function ModelsSection({
  models,
  onChanged,
}: {
  models: InstalledModel[] | undefined;
  onChanged: () => void;
}) {
  const [progress, setProgress] = useState<ModelProgress>();
  const [error, setError] = useState<UserFacingError>();

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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    return subscribe(AUDIS_EVENTS.modelProgress, (payload) => {
      const parsed = modelProgressSchema.safeParse(payload);
      if (!parsed.success) return;

      if (parsed.data.done || parsed.data.error) {
        setProgress(undefined);
        onChanged();
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
  }, [onChanged]);

  const install = (id: ModelId) => {
    setError(undefined);
    setProgress({ id, downloadedBytes: 0, totalBytes: null, done: false, error: null });
    installModel(id)
      .catch((cause: unknown) => {
        setProgress(undefined);
        setError(toUserFacing(cause));
      })
      .finally(onChanged);
  };

  const remove = (id: ModelId) => {
    removeModel(id)
      .then(onChanged)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  };

  return (
    <Section
      id="models"
      title="On-device models"
      footnote="Free, offline, and no account needed. Bigger models are more accurate and need a faster PC."
    >
      {error ? <ErrorNotice error={error} /> : null}

      <div
        className="flex flex-col overflow-hidden"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        {models === undefined
          ? [0, 1].map((index) => (
              <div key={index} className="p-3.5">
                <div className="audis-skeleton h-10" />
              </div>
            ))
          : models.map((model) => (
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
    </Section>
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
    <div className="audis-row flex flex-col gap-2.5 px-3.5 py-3" data-inset="icon">
      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <IconBadge tint={model.installed ? "purple" : "gray"} size={30}>
            <ChipIcon />
          </IconBadge>
          <div className="flex min-w-0 flex-col gap-0.5">
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-subheadline font-semibold">{info.name}</span>
              {info.recommended ? <Badge tone="accent">Recommended</Badge> : null}
              {model.installed ? <Badge tone="success">Installed</Badge> : null}
              {!info.keepsUpLive ? (
                <Badge
                  tone="warning"
                  title="This model decodes slower than people speak, so live captions fall behind and never catch up."
                >
                  Too slow for live
                </Badge>
              ) : null}
            </div>
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              {info.summary}
            </span>
            <span className="text-caption1" style={{ color: "var(--label-tertiary)" }}>
              {formatBytes(model.installedBytes ?? info.sizeBytes)} · {info.requirement}
            </span>
          </div>
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
    </div>
  );
}

function DownloadProgress({
  progress,
  fallbackTotal,
}: {
  progress: ModelProgress;
  fallbackTotal: number;
}) {
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
          className="h-full"
          style={{
            width: `${fraction * 100}%`,
            background: "var(--color-accent)",
            borderRadius: "var(--radius-chip)",
            transition: "width var(--spring-smooth-duration) var(--spring-smooth)",
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
