import { useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { Button, SegmentedControl, Switch } from "@/components/controls";
import { useAppInfo } from "@/hooks/useAppInfo";
import { useSettings } from "@/hooks/useSettings";
import { ErrorNotice } from "@/components/ErrorNotice";
import { Wordmark } from "@/components/Wordmark";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { checkForUpdates, installUpdate, openReleasePage } from "@/services/ipc";
import {
  updateCheckSchema,
  updateProgressEventSchema,
  type UpdateCheck,
  type UpdateChannel,
} from "@/schemas/ipc";

/** About Audis. Every value comes from the Rust core, never a frontend constant. */
export function AboutView() {
  const state = useAppInfo();

  if (state.status === "error") {
    return <ErrorNotice error={state.error} />;
  }

  const info = state.status === "ready" ? state.info : undefined;
  const placeholder = "Unknown";

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-col items-center gap-3 pt-6 pb-2">
        <Wordmark size={56} />
        <div className="flex flex-col items-center gap-1">
          <h1
            className="text-title1 font-semibold"
            style={{ letterSpacing: "var(--tracking-tighter)" }}
          >
            {info?.appName ?? "Audis"}
          </h1>
          <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
            {info?.tagline ?? placeholder}
          </p>
          <p className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
            Version {info?.version ?? placeholder}
          </p>
        </div>
      </header>

      <Updates />

      <GroupedList title="Product">
        <Row label="Publisher" value={info?.publisher ?? placeholder} />
        <Row label="Company" value={info?.company ?? placeholder} />
        <Row label="Bundle identifier" value={info?.bundleId ?? placeholder} />
      </GroupedList>

      <GroupedList
        title="Legal"
        footnote="Audis captures audio only when you ask it to. You are responsible for obtaining any consent required where you record."
      >
        <Row label="Licence" value="Proprietary" />
        <Row label="Copyright" value={`© ${new Date().getFullYear()} Neura Audis`} />
      </GroupedList>
    </div>
  );
}

/** Release channel, the update check, and whatever it found. */
function Updates() {
  const { settings, update } = useSettings();
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<UpdateCheck>();
  const [failed, setFailed] = useState<string>();
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<number>();

  // A check that ran at startup lands here too, so opening About shows it.
  useEffect(() => {
    const stopStatus = subscribe(AUDIS_EVENTS.updateStatus, (payload) => {
      const parsed = updateCheckSchema.safeParse(payload);
      if (parsed.success) setResult(parsed.data);
    });

    const stopProgress = subscribe(AUDIS_EVENTS.updateProgress, (payload) => {
      const parsed = updateProgressEventSchema.safeParse(payload);
      if (!parsed.success) return;
      const { downloaded, total } = parsed.data;
      // Without a content length there is no percentage to show, only motion.
      setProgress(total ? Math.min(100, Math.round((downloaded / total) * 100)) : undefined);
    });

    return () => {
      stopStatus();
      stopProgress();
    };
  }, []);

  const install = () => {
    setInstalling(true);
    setFailed(undefined);
    setProgress(0);
    // On success Audis restarts into the new version, so this never resolves.
    installUpdate().catch((error: unknown) => {
      setFailed(error instanceof Error ? error.message : String(error));
      setInstalling(false);
      setProgress(undefined);
    });
  };

  const updates = settings?.updates;

  const set = (change: Partial<NonNullable<typeof updates>>) =>
    update((current) => ({ ...current, updates: { ...current.updates, ...change } }));

  const check = () => {
    setChecking(true);
    setFailed(undefined);
    checkForUpdates()
      .then(setResult)
      .catch((error: unknown) => setFailed(error instanceof Error ? error.message : String(error)))
      .finally(() => setChecking(false));
  };

  if (!updates) return null;

  return (
    <section className="flex flex-col gap-3">
      <h2 className="px-1 text-subheadline font-semibold">Updates</h2>

      <div
        className="flex flex-col gap-3 p-3"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        <div className="flex items-center justify-between gap-4">
          <div className="flex min-w-0 flex-col gap-0.5">
            <span className="text-subheadline">Release channel</span>
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              Betas ship earlier and are less tested. You are moved back to a finished release as
              soon as one is newer than the beta you are on.
            </span>
          </div>
          <SegmentedControl<UpdateChannel>
            label="Release channel"
            value={updates.channel}
            options={[
              { id: "stable", label: "Stable" },
              { id: "beta", label: "Beta" },
            ]}
            onChange={(channel) => set({ channel })}
          />
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="flex min-w-0 flex-col gap-0.5">
            <span className="text-subheadline">Check when Audis starts</span>
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              Asks GitHub whether a newer version exists. Nothing is downloaded or installed without
              you.
            </span>
          </div>
          <Switch
            label="Check for updates on startup"
            checked={updates.checkOnStartup}
            onChange={(checkOnStartup) => set({ checkOnStartup })}
          />
        </div>

        <div className="flex items-center justify-between gap-4">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {checking
              ? "Checking…"
              : failed
                ? failed
                : result
                  ? result.update
                    ? `Version ${result.update.version} is available.`
                    : "Audis is up to date."
                  : "Not checked yet."}
          </span>
          <Button onClick={check} disabled={checking}>
            Check now
          </Button>
        </div>
      </div>

      {result?.update ? (
        <div
          className="flex flex-col gap-2 p-3"
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
            outline: "1.5px solid var(--color-accent)",
            outlineOffset: -1,
          }}
        >
          <div className="flex items-center justify-between gap-3">
            <span className="text-subheadline font-semibold">
              {result.update.version}
              {result.update.prerelease ? " (beta)" : ""}
            </span>
            <div className="flex shrink-0 items-center gap-2">
              <Button onClick={() => void openReleasePage(result.update!.url)}>
                Release notes
              </Button>
              {result.update.manifestUrl ? (
                <Button variant="accent" onClick={install} disabled={installing}>
                  {installing
                    ? progress === undefined
                      ? "Downloading…"
                      : `${progress}%`
                    : "Update and restart"}
                </Button>
              ) : null}
            </div>
          </div>

          {installing ? (
            <div
              aria-hidden
              className="h-1 w-full overflow-hidden"
              style={{ background: "var(--surface-elevated)", borderRadius: 999 }}
            >
              <div
                style={{
                  width: progress === undefined ? "100%" : `${progress}%`,
                  height: "100%",
                  background: "var(--color-accent)",
                  transition: "width 160ms ease",
                  opacity: progress === undefined ? 0.4 : 1,
                }}
              />
            </div>
          ) : null}

          {!result.update.manifestUrl ? (
            <p className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              This release has no signed installer, so Audis cannot install it for you. Open the
              release notes to download it yourself.
            </p>
          ) : null}
          {result.update.notes ? (
            <p
              data-selectable
              className="max-h-48 overflow-y-auto whitespace-pre-wrap text-footnote"
              style={{ color: "var(--label-secondary)" }}
            >
              {result.update.notes}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
