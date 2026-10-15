import { Button } from "@/components/controls";
import { Equalizer } from "@/components/Equalizer";
import { PauseIcon, PlayIcon, StopIcon } from "@/components/icons";
import { formatClock } from "@/lib/format";
import type { SessionStatus, TranscriptSegment } from "@/schemas/ipc";

import { featureName } from "./featureStyle";
import { useElapsed, useLiveTranscript } from "./useLiveSession";

/**
 * The running session, front and centre: what is running, for how long, the
 * last few things said, and the two controls that matter.
 */
export function NowListening({
  session,
  onPause,
  onStop,
}: {
  session: SessionStatus;
  onPause: (paused: boolean) => void;
  onStop: () => void;
}) {
  const elapsed = useElapsed(session);
  const { lines, partial } = useLiveTranscript(session.id);
  const listening = session.state === "listening";
  const paused = session.state === "paused";
  const name = featureName(session.mode);

  return (
    <section
      aria-label={`${name} session`}
      className="relative overflow-hidden p-5"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-tile)",
        boxShadow: "var(--shadow-raised)",
        animation: "audis-pop-up var(--spring-bouncy-duration) var(--spring-bouncy) both",
      }}
    >
      {/* An ambient glow that breathes while Audis listens and fades when it
          stops: the card should feel alive without anything to watch. */}
      <span
        aria-hidden
        className="pointer-events-none absolute -top-24 -right-16 h-64 w-64 rounded-full"
        style={{
          background:
            "radial-gradient(closest-side, color-mix(in srgb, var(--color-listening) 22%, transparent), transparent)",
          opacity: listening ? 1 : 0,
          animation: listening ? "audis-breathe 4.5s ease-in-out infinite" : "none",
          transition: "opacity var(--spring-gentle-duration) var(--spring-gentle)",
        }}
      />

      <div className="relative flex items-center gap-4">
        <span
          className="flex h-12 w-12 shrink-0 items-center justify-center rounded-full"
          style={{
            background: paused
              ? "var(--surface-sunken)"
              : "color-mix(in srgb, var(--color-listening) 14%, transparent)",
            color: paused ? "var(--label-secondary)" : "var(--color-listening)",
            transition: "background-color var(--duration-standard) var(--ease-standard)",
          }}
        >
          <Equalizer active={listening} size={20} />
        </span>

        <div className="flex min-w-0 flex-1 flex-col">
          <span className="text-title3 font-semibold">{name}</span>
          <span
            className="text-footnote"
            style={{ color: paused ? "var(--color-warning)" : "var(--label-secondary)" }}
          >
            {paused ? "Paused" : listening ? "Listening" : "Starting…"} ·{" "}
            {session.language === "indonesian" ? "Indonesian" : "English"}
          </span>
        </div>

        <span
          aria-label="Elapsed time"
          className="text-title1 font-light tabular-nums"
          style={{
            letterSpacing: "var(--tracking-tight)",
            color: paused ? "var(--label-tertiary)" : "var(--label-primary)",
          }}
        >
          {formatClock(elapsed)}
        </span>
      </div>

      <div className="relative mt-3 flex flex-wrap gap-1.5">
        <SourcePill on={session.microphone} live={listening} label="Microphone" />
        <SourcePill on={session.computerAudio} live={listening} label="Computer audio" />
      </div>

      <div
        data-selectable
        aria-live="polite"
        className="relative mt-4 flex min-h-[112px] flex-col justify-end gap-2 p-3.5"
        style={{ background: "var(--surface-window)", borderRadius: "var(--radius-card)" }}
      >
        {lines.length === 0 && !partial ? (
          <p className="m-auto text-footnote" style={{ color: "var(--label-tertiary)" }}>
            {paused ? "Paused. Nothing is being heard." : "Waiting for someone to speak…"}
          </p>
        ) : null}
        {lines.map((line) => (
          <Line key={line.id} segment={line} />
        ))}
        {partial ? <Line segment={partial} interim /> : null}
      </div>

      <div className="relative mt-4 flex items-center gap-2">
        <Button
          size="large"
          variant="tinted"
          onClick={() => onPause(!paused)}
          disabled={!listening && !paused}
          ariaLabel={paused ? "Resume" : "Pause"}
        >
          {paused ? <PlayIcon /> : <PauseIcon />}
          {paused ? "Resume" : "Pause"}
        </Button>
        <Button size="large" variant="danger" onClick={onStop} ariaLabel={`Stop ${name}`}>
          <StopIcon />
          Stop
        </Button>
        <span className="ml-auto text-caption1" style={{ color: "var(--label-tertiary)" }}>
          Captions are showing over your other windows.
        </span>
      </div>
    </section>
  );
}

function SourcePill({ on, live, label }: { on: boolean; live: boolean; label: string }) {
  return (
    <span
      className="inline-flex items-center gap-1.5 px-2.5 py-1 text-caption1 font-medium"
      style={{
        background: "var(--surface-window)",
        borderRadius: "var(--radius-chip)",
        color: on ? "var(--label-primary)" : "var(--label-tertiary)",
      }}
    >
      <span
        aria-hidden
        className="h-1.5 w-1.5 rounded-full"
        style={{
          background: on
            ? live
              ? "var(--color-success)"
              : "var(--color-warning)"
            : "var(--label-tertiary)",
        }}
      />
      {label}
      <span className="sr-only">{on ? "captured" : "not captured"}</span>
    </span>
  );
}

function Line({ segment, interim = false }: { segment: TranscriptSegment; interim?: boolean }) {
  const mine = segment.source === "microphone";
  const speaker = segment.speaker ?? (mine ? "You" : "Computer audio");

  return (
    <p
      className="flex items-baseline gap-2 text-subheadline"
      style={{
        animation: interim ? undefined : "audis-caption-in 260ms var(--ease-standard) both",
      }}
    >
      <span
        className="shrink-0 text-caption1 font-semibold"
        style={{ color: mine ? "var(--color-success)" : "var(--color-accent)" }}
      >
        {speaker}
      </span>
      <span style={{ color: interim ? "var(--label-secondary)" : "var(--label-primary)" }}>
        {segment.text}
      </span>
    </p>
  );
}
