import { Badge } from "@/components/controls";
import { Equalizer } from "@/components/Equalizer";
import { IconBadge } from "@/components/IconBadge";
import { ArrowRightIcon, PlayIcon } from "@/components/icons";
import type { Feature } from "@/schemas/ipc";

import { FEATURE_STYLE } from "./featureStyle";
import { describeBlocker } from "./blockers";

/** What a tile does when clicked, which is also how it is named. */
export type TileAction = "start" | "setUp" | "running" | "unavailable";

/**
 * One feature, startable in one click. A tile that cannot start says why and
 * takes the user to the fix instead.
 */
export function FeatureTile({
  feature,
  action,
  listening,
  starting,
  blockedByOther,
  onClick,
}: {
  feature: Feature;
  action: TileAction;
  /** The running session is hearing audio right now, rather than paused. */
  listening: boolean;
  /** This tile's session is being started. */
  starting: boolean;
  /** Another session is running or starting, so this one cannot. */
  blockedByOther: boolean;
  onClick: () => void;
}) {
  const style = FEATURE_STYLE[feature.id];
  const disabled = blockedByOther || action === "unavailable" || starting;

  const label =
    action === "running"
      ? `${feature.name} is running`
      : action === "setUp"
        ? `Set up ${feature.name}`
        : `Start ${feature.name}`;

  const hint = starting
    ? "Starting… the first start loads the speech model."
    : action === "setUp"
      ? describeBlocker(feature.blocker).short
      : blockedByOther
        ? "Another session is running."
        : feature.summary;

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={feature.blocker ?? undefined}
      className="audis-pressable audis-lift group flex min-h-[168px] flex-col gap-3 p-4 text-left"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-tile)",
        boxShadow:
          action === "running"
            ? "0 0 0 2px var(--color-listening), var(--shadow-card)"
            : "var(--shadow-card)",
        opacity: disabled && !starting ? 0.5 : 1,
      }}
    >
      <div className="flex w-full items-start justify-between gap-2">
        <IconBadge tint={style.tint} size={40}>
          {style.icon}
        </IconBadge>
        {action === "running" ? (
          <span style={{ color: "var(--color-listening)" }}>
            <Equalizer active={listening} size={16} />
          </span>
        ) : action === "setUp" ? (
          <Badge tone="warning">Needs setup</Badge>
        ) : null}
      </div>

      <div className="flex min-w-0 flex-col gap-1">
        <span className="text-body font-semibold">{feature.name}</span>
        <span className="line-clamp-2 text-footnote" style={{ color: "var(--label-secondary)" }}>
          {hint}
        </span>
      </div>

      <div className="mt-auto flex w-full items-center justify-between gap-2">
        <span
          className="text-caption1"
          style={{ color: "var(--label-tertiary)" }}
          title={
            feature.usesCloud ? "Sends transcript text to the AI provider you choose." : undefined
          }
        >
          {feature.id === "liveCaption" ? "Nothing is saved" : "Saves a transcript"}
          {feature.usesCloud ? " · Uses cloud AI" : ""}
        </span>
        <TileAffordance action={action} starting={starting} />
      </div>
    </button>
  );
}

/** The round control at the tile's corner: what clicking will do. */
function TileAffordance({ action, starting }: { action: TileAction; starting: boolean }) {
  if (action === "running") {
    return (
      <span className="text-footnote font-semibold" style={{ color: "var(--color-listening)" }}>
        Running
      </span>
    );
  }

  if (action === "setUp") {
    return (
      <span
        className="flex items-center gap-1 text-footnote font-semibold [&>svg]:h-3.5 [&>svg]:w-3.5"
        style={{ color: "var(--color-accent)" }}
      >
        Set up
        <ArrowRightIcon />
      </span>
    );
  }

  return (
    <span
      aria-hidden
      className="flex h-8 w-8 items-center justify-center rounded-full [&>svg]:h-3.5 [&>svg]:w-3.5"
      style={{
        background: "var(--color-accent)",
        color: "#ffffff",
        boxShadow: "0 2px 8px color-mix(in srgb, var(--color-accent) 40%, transparent)",
        transform: starting ? "scale(0.9)" : undefined,
        transition: "transform var(--spring-snappy-duration) var(--spring-snappy)",
      }}
    >
      {starting ? <Spinner /> : <PlayIcon />}
    </span>
  );
}

/** A small activity ring for a start that takes a moment. */
export function Spinner() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden className="animate-spin">
      <circle
        cx="7"
        cy="7"
        r="5.5"
        fill="none"
        stroke="currentColor"
        strokeOpacity="0.3"
        strokeWidth="2"
      />
      <path
        d="M7 1.5a5.5 5.5 0 0 1 5.5 5.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}
