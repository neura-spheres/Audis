import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { CheckIcon } from "@/components/icons";
import { listFeatures, AudisIpcError } from "@/services/ipc";
import { useSession } from "@/hooks/useSession";
import type { Feature, UserFacingError } from "@/schemas/ipc";
import type { ViewId } from "@/app/navigation";

/**
 * The launcher: everything Audis can do, and whether it can do it right now.
 *
 * Status comes from Rust, which checks what is actually on this machine, so a
 * feature is offered as Ready only when its model and keys genuinely exist. A
 * blocked feature says what is missing and links to the page that fixes it,
 * rather than failing after the user commits to starting a session.
 */
export function FeaturesView({ onNavigate }: { onNavigate: (id: ViewId) => void }) {
  const [features, setFeatures] = useState<Feature[]>();
  const [error, setError] = useState<UserFacingError>();
  const { session, starting, error: sessionError, start, stop } = useSession();

  const refresh = useCallback(() => {
    listFeatures()
      .then((result) => {
        setFeatures(result);
        setError(undefined);
      })
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  useEffect(refresh, [refresh]);

  if (error) return <ErrorNotice error={error} />;

  return (
    <div className="flex flex-col gap-5">
      {sessionError ? <ErrorNotice error={sessionError} /> : null}

      <p className="px-1 text-subheadline" style={{ color: "var(--label-secondary)" }}>
        Pick what you want Audis to do. Captions appear over your other windows while a session
        runs.
      </p>

      <div className="flex flex-col gap-3">
        {(features ?? []).map((feature) => (
          <FeatureCard
            key={feature.id}
            feature={feature}
            onNavigate={onNavigate}
            running={session?.mode === feature.id}
            // One session at a time: two would fight over the microphone.
            blockedByOther={session !== null && session.mode !== feature.id}
            starting={starting}
            onStart={() => void start(feature.id)}
            onStop={() => void stop()}
          />
        ))}
      </div>

      {features === undefined ? (
        <p className="px-1 text-footnote" style={{ color: "var(--label-tertiary)" }}>
          Checking what is ready…
        </p>
      ) : null}
    </div>
  );
}

function FeatureCard({
  feature,
  onNavigate,
  running,
  blockedByOther,
  starting,
  onStart,
  onStop,
}: {
  feature: Feature;
  onNavigate: (id: ViewId) => void;
  running: boolean;
  blockedByOther: boolean;
  starting: boolean;
  onStart: () => void;
  onStop: () => void;
}) {
  const ready = feature.status === "ready";

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
            <h2 className="text-body font-semibold">{feature.name}</h2>
            <StatusChip status={feature.status} />
            {feature.usesCloud ? (
              <span
                className="px-1.5 py-0.5 text-caption2"
                style={{
                  background: "var(--surface-sunken)",
                  borderRadius: "var(--radius-chip)",
                  color: "var(--label-secondary)",
                }}
                title="This feature sends transcript text to an AI provider you choose."
              >
                Uses cloud AI
              </span>
            ) : null}
          </div>
          <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
            {feature.summary}
          </p>
        </div>
      </div>

      <ul className="flex flex-col gap-1">
        {feature.details.map((detail) => (
          <li
            key={detail}
            className="flex items-start gap-2 text-footnote"
            style={{ color: "var(--label-secondary)" }}
          >
            <span
              aria-hidden
              className="mt-[3px] shrink-0"
              style={{ color: "var(--label-tertiary)" }}
            >
              <CheckIcon />
            </span>
            {detail}
          </li>
        ))}
      </ul>

      {feature.blocker ? (
        <div
          className="flex flex-col gap-2 p-2.5"
          style={{ background: "var(--surface-sunken)", borderRadius: "var(--radius-control)" }}
        >
          <p className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {feature.blocker}
          </p>
          <div className="flex gap-2">
            {/* Send the user to the page that actually resolves the blocker,
                rather than making them work out where to go. */}
            <Button onClick={() => onNavigate(blockerDestination(feature.blocker))}>
              {feature.blocker.includes("Models") ? "Open Models" : "Open Providers"}
            </Button>
          </div>
        </div>
      ) : null}

      <div className="flex items-center gap-3">
        {running ? (
          <Button variant="danger" onClick={onStop} ariaLabel={`Stop ${feature.name}`}>
            Stop {feature.name}
          </Button>
        ) : (
          <Button
            variant={ready ? "accent" : "standard"}
            disabled={!ready || starting || blockedByOther}
            onClick={onStart}
            // The tooltip explains why it is disabled, but the accessible name
            // must stay the action itself, or a screen reader announces the
            // blocker text as the button's name.
            title={
              blockedByOther
                ? "Stop the running session first."
                : ready
                  ? `Start ${feature.name}`
                  : (feature.blocker ?? "Not available yet")
            }
            ariaLabel={`Start ${feature.name}`}
          >
            {starting ? "Starting…" : `Start ${feature.name}`}
          </Button>
        )}

        {running ? (
          <span className="text-footnote" style={{ color: "var(--color-success)" }}>
            Running now
          </span>
        ) : starting ? (
          <span className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            Loading the speech model. This takes a few seconds the first time.
          </span>
        ) : blockedByOther ? (
          <span className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            Another session is running
          </span>
        ) : !ready ? (
          <span className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            Not ready yet
          </span>
        ) : null}
      </div>
    </section>
  );
}

/** Which page fixes this blocker. */
function blockerDestination(blocker: string | null): ViewId {
  return blocker?.includes("Models") ? "models" : "providers";
}

function StatusChip({ status }: { status: Feature["status"] }) {
  const palette: Record<Feature["status"], { label: string; colour: string }> = {
    ready: { label: "Ready", colour: "var(--color-success)" },
    needsSetup: { label: "Needs setup", colour: "var(--color-warning)" },
    notBuilt: { label: "Not built", colour: "var(--label-tertiary)" },
  };
  const { label, colour } = palette[status];

  return (
    // Text as well as colour, so the status still reads without colour vision.
    <span className="text-caption2 font-medium" style={{ color: colour }}>
      {label}
    </span>
  );
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not check what is available",
    explanation: "The feature list could not be loaded. Nothing was changed.",
    dataPreserved: true,
    suggestedAction: "Try again.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
