import { useEffect, useState, type ReactNode } from "react";

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
            <div
              data-selectable
              className="max-h-48 overflow-y-auto text-footnote"
              style={{ color: "var(--label-secondary)" }}
            >
              <ReleaseNotes notes={result.update.notes} />
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

export function ReleaseNotes({ notes }: { notes: string }) {
  const lines = notes.replace(/\r\n/g, "\n").split("\n");
  const blocks: ReactNode[] = [];
  let list: string[] = [];
  let key = 0;

  const flushList = () => {
    if (list.length === 0) return;
    const items = list;
    list = [];
    blocks.push(
      <ul key={key++} className="flex flex-col gap-1">
        {items.map((item, index) => (
          <li key={index} className="flex items-start gap-2">
            <span aria-hidden className="mt-[2px]" style={{ color: "var(--label-tertiary)" }}>
              •
            </span>
            <span>{renderInline(item)}</span>
          </li>
        ))}
      </ul>,
    );
  };

  for (const raw of lines) {
    const line = raw.trim();
    if (line === "") {
      flushList();
      continue;
    }

    const bullet = /^[-*]\s+(.*)$/.exec(line);
    if (bullet) {
      list.push(bullet[1] ?? "");
      continue;
    }

    flushList();

    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (heading) {
      blocks.push(
        <p
          key={key++}
          className="font-semibold"
          style={{
            color: "var(--label-primary)",
            marginTop: blocks.length > 0 ? 4 : 0,
          }}
        >
          {renderInline(heading[2] ?? "")}
        </p>,
      );
      continue;
    }

    blocks.push(
      <p key={key++} className="leading-[1.45]">
        {renderInline(line)}
      </p>,
    );
  }
  flushList();

  return <div className="flex flex-col gap-1.5">{blocks}</div>;
}

function renderInline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /\*\*(.+?)\*\*|`(.+?)`/g;
  let last = 0;
  let key = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > last) nodes.push(text.slice(last, match.index));
    if (match[1] !== undefined) {
      nodes.push(
        <strong key={key++} style={{ color: "var(--label-primary)", fontWeight: 600 }}>
          {match[1]}
        </strong>,
      );
    } else if (match[2] !== undefined) {
      nodes.push(
        <code
          key={key++}
          style={{
            fontFamily: "var(--font-mono, monospace)",
            fontSize: "0.92em",
            padding: "0 3px",
            borderRadius: 4,
            background: "var(--surface-sunken)",
          }}
        >
          {match[2]}
        </code>,
      );
    }
    last = pattern.lastIndex;
  }
  if (last < text.length) nodes.push(text.slice(last));

  return nodes;
}
