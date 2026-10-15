import { Equalizer } from "@/components/Equalizer";
import { PauseIcon, PlayIcon, StopIcon } from "@/components/icons";
import { useSession } from "@/hooks/useSession";
import { formatClock } from "@/lib/format";

import { featureName } from "./featureStyle";
import { useElapsed } from "./useLiveSession";

/**
 * The running session, kept in view from every page, the way Music keeps what
 * is playing at the bottom of its sidebar. Pause and stop are always one click
 * away, wherever the user has wandered off to.
 */
export function MiniPlayer({ onOpen }: { onOpen: () => void }) {
  const { session, stop, setPaused } = useSession();
  const elapsed = useElapsed(session);

  if (!session) return null;

  const listening = session.state === "listening";
  const paused = session.state === "paused";
  const name = featureName(session.mode);

  return (
    <div className="shrink-0 px-3 pb-3">
      <div
        className="flex items-center gap-2.5 p-2.5"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-raised)",
          animation: "audis-pop-up var(--spring-bouncy-duration) var(--spring-bouncy) both",
        }}
      >
        <button
          type="button"
          onClick={onOpen}
          aria-label={`Open ${name}`}
          className="flex min-w-0 flex-1 items-center gap-2.5 text-left"
        >
          <span
            className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full"
            style={{
              background: paused
                ? "var(--surface-sunken)"
                : "color-mix(in srgb, var(--color-listening) 14%, transparent)",
              color: paused ? "var(--label-secondary)" : "var(--color-listening)",
            }}
          >
            <Equalizer active={listening} size={13} />
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-footnote font-semibold">{name}</span>
            <span
              className="text-caption1 tabular-nums"
              style={{ color: paused ? "var(--color-warning)" : "var(--label-secondary)" }}
            >
              {paused ? "Paused" : listening ? formatClock(elapsed) : "Starting…"}
            </span>
          </span>
        </button>

        <IconButton
          label={paused ? "Resume session" : "Pause session"}
          onClick={() => void setPaused(!paused)}
          disabled={!listening && !paused}
        >
          {paused ? <PlayIcon /> : <PauseIcon />}
        </IconButton>
        <IconButton label="Stop session" onClick={() => void stop()} tone="danger">
          <StopIcon />
        </IconButton>
      </div>
    </div>
  );
}

function IconButton({
  label,
  onClick,
  children,
  tone = "neutral",
  disabled = false,
}: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
  tone?: "neutral" | "danger";
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
      className="audis-pressable flex h-7 w-7 shrink-0 items-center justify-center rounded-full [&>svg]:h-3.5 [&>svg]:w-3.5"
      style={{
        background: "var(--surface-sunken)",
        color: tone === "danger" ? "var(--color-danger)" : "var(--label-primary)",
        opacity: disabled ? 0.4 : 1,
      }}
    >
      {children}
    </button>
  );
}
