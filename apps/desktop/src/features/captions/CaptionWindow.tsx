import { useEffect, useState } from "react";

import { useOverlayMenu, type OverlayMenuItem } from "@/components/OverlayMenu";
import { useSession } from "@/hooks/useSession";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import {
  getSettings,
  hideOverlay,
  openMainWindow,
  resetCaptionPosition,
  setCaptionClickThrough,
} from "@/services/ipc";
import {
  diagnosticWarningSchema,
  settingsSchema,
  transcriptSegmentSchema,
  type CaptionSettings,
  type TranscriptSegment,
} from "@/schemas/ipc";

/** The caption overlay. */
export function CaptionWindow() {
  const [lines, setLines] = useState<TranscriptSegment[]>([]);
  /// The sentence being spoken right now, before it is finished and replaced.
  const [partial, setPartial] = useState<TranscriptSegment>();
  const [captions, setCaptions] = useState<CaptionSettings>();
  const [hovered, setHovered] = useState(false);
  /// Why no captions are appearing, when something is wrong.
  const [problem, setProblem] = useState<string>();
  const { session, stop, setPaused } = useSession();
  const paused = session?.state === "paused";

  const clickThrough = captions?.clickThrough ?? false;

  const menuItems: OverlayMenuItem[] = [
    {
      id: "pause",
      label: paused ? "Resume" : "Pause",
      onSelect: () => void setPaused(!paused),
    },
    { id: "open", label: "Open Audis", onSelect: () => void openMainWindow() },
    {
      id: "click-through",
      label: clickThrough ? "Make captions clickable" : "Let clicks pass through",
      separatorBefore: true,
      onSelect: () => void setCaptionClickThrough(!clickThrough),
    },
    {
      id: "recenter",
      label: "Recentre captions",
      onSelect: () => void resetCaptionPosition(),
    },
    {
      id: "hide",
      label: "Hide captions",
      onSelect: () => void hideOverlay("captions"),
    },
    {
      id: "stop",
      label: "Stop session",
      danger: true,
      separatorBefore: true,
      onSelect: () => void stop(),
    },
  ];

  const { menu, onContextMenu } = useOverlayMenu(menuItems);

  useEffect(() => {
    const load = () => {
      getSettings()
        .then((settings) => setCaptions(settings.captions))
        .catch(() => undefined);
    };
    load();

    return subscribe(AUDIS_EVENTS.settingsChanged, (payload) => {
      const parsed = settingsSchema.safeParse(payload);
      if (parsed.success) setCaptions(parsed.data.captions);
    });
  }, []);

  const maxLines = captions?.maxLines ?? 2;

  useEffect(() => {
    const stopTranscript = subscribe(AUDIS_EVENTS.transcriptFinal, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (!parsed.success) return;

      setLines((current) => [...current, parsed.data].slice(-maxLines));
      setPartial(undefined);
      // Words arrived, so whatever was wrong is over.
      setProblem(undefined);
    });

    // Recognition is failing. Without this the captions simply never appear and
    // there is nothing on screen to say why.
    const stopWarning = subscribe(AUDIS_EVENTS.diagnosticWarning, (payload) => {
      const parsed = diagnosticWarningSchema.safeParse(payload);
      if (parsed.success && parsed.data.kind.startsWith("asr.")) {
        setProblem(parsed.data.message);
      }
    });

    const stopPartial = subscribe(AUDIS_EVENTS.transcriptPartial, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (parsed.success) setPartial(parsed.data);
    });

    const stopSession = subscribe(AUDIS_EVENTS.sessionState, () => {
      setLines([]);
      setPartial(undefined);
      setProblem(undefined);
    });

    return () => {
      stopTranscript();
      stopPartial();
      stopWarning();
      stopSession();
    };
  }, [maxLines]);

  if (!captions) return null;

  const opacity = captions.backgroundOpacity / 100;
  const hasPanel = opacity > 0.01;
  const showAffordance = hovered && !clickThrough;

  const visible = [...lines, ...(partial ? [partial] : [])].slice(-maxLines);
  const showing = visible.length > 0 || problem !== undefined;

  return (
    <div
      className="flex h-screen w-screen items-end justify-center p-4"
      onContextMenu={onContextMenu}
    >
      <div
        data-tauri-drag-region
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        className="relative flex w-fit max-w-full flex-col gap-1.5"
        style={{
          padding: hasPanel ? "10px 16px" : "4px 8px",
          borderRadius: 16,
          cursor: clickThrough ? "default" : "grab",
          background: hasPanel ? `rgba(14, 15, 17, ${opacity})` : "transparent",
          backdropFilter: hasPanel
            ? `blur(${Math.round(opacity * 22)}px) saturate(140%)`
            : undefined,
          border: showAffordance
            ? "1px solid rgba(255, 255, 255, 0.35)"
            : hasPanel
              ? `1px solid rgba(255, 255, 255, ${0.1 * opacity})`
              : "1px solid transparent",
          boxShadow: showAffordance
            ? "0 10px 44px rgba(0, 0, 0, 0.5), 0 0 0 3px rgba(120, 170, 255, 0.28)"
            : hasPanel
              ? `0 8px 40px rgba(0, 0, 0, ${0.45 * opacity})`
              : undefined,
          opacity: showing ? 1 : 0,
          transition: "border-color 140ms ease, box-shadow 140ms ease, opacity 180ms ease",
        }}
      >
        <DragHandle visible={showAffordance} />
        {problem ? <Problem message={problem} /> : null}
        {visible.map((line, index) => (
          <CaptionLine
            key={line.isFinal ? line.id : `interim-${line.source}`}
            line={line}
            settings={captions}
            hasPanel={hasPanel}
            faded={index < visible.length - 1}
          />
        ))}
      </div>

      {menu}
    </div>
  );
}

/** Why the captions stopped, said plainly and in their place. */
function Problem({ message }: { message: string }) {
  return (
    <p
      data-selectable
      className="flex max-w-[560px] items-start gap-2 text-footnote leading-[1.4]"
      style={{ color: "#ffd7d7" }}
    >
      <span aria-hidden style={{ opacity: 0.9 }}>
        ⚠
      </span>
      <span>{message}</span>
    </p>
  );
}

/** A small grab handle that fades in when the captions are hovered. */
function DragHandle({ visible }: { visible: boolean }) {
  return (
    <span
      aria-hidden
      data-tauri-drag-region
      className="pointer-events-none absolute left-1/2 top-1 -translate-x-1/2"
      style={{
        width: 30,
        height: 4,
        borderRadius: 999,
        background: "rgba(255, 255, 255, 0.55)",
        opacity: visible ? 1 : 0,
        transition: "opacity 140ms ease",
      }}
    />
  );
}

function CaptionLine({
  line,
  settings,
  hasPanel,
  faded,
}: {
  line: TranscriptSegment;
  settings: CaptionSettings;
  hasPanel: boolean;
  faded: boolean;
}) {
  const size = settings.fontSize;

  return (
    <p
      data-tauri-drag-region
      className="flex items-baseline gap-2.5 leading-[1.3]"
      style={{
        fontSize: size,
        fontWeight: 600,
        color: "#ffffff",
        opacity: faded ? 0.55 : 1,
        textShadow: hasPanel
          ? "0 1px 2px rgba(0, 0, 0, 0.5)"
          : "0 0 3px rgba(0,0,0,0.95), 0 0 8px rgba(0,0,0,0.8), 0 2px 4px rgba(0,0,0,0.9)",
        animation: line.isFinal ? "audis-caption-in 160ms ease-out" : undefined,
      }}
    >
      {settings.showSourceLabels ? <SourceLabel line={line} size={size} /> : null}
      <span className="min-w-0">{line.text}</span>
    </p>
  );
}

/** Who said it. */
function SourceLabel({ line, size }: { line: TranscriptSegment; size: number }) {
  const colour = sourceColour(line.source);

  return (
    <span
      className="flex shrink-0 items-center gap-1.5 whitespace-nowrap"
      style={{
        fontSize: Math.max(10, Math.round(size * 0.42)),
        fontWeight: 600,
        letterSpacing: "0.04em",
        color: "rgba(255, 255, 255, 0.62)",
        textShadow: "0 1px 3px rgba(0, 0, 0, 0.9)",
        transform: "translateY(-0.08em)",
      }}
    >
      <span
        aria-hidden
        style={{
          width: Math.max(5, Math.round(size * 0.16)),
          height: Math.max(5, Math.round(size * 0.16)),
          borderRadius: "50%",
          background: colour,
          boxShadow: `0 0 8px ${colour}`,
        }}
      />
      {line.speaker}
    </span>
  );
}

/** Microphone and computer audio get distinct hues. */
function sourceColour(source: TranscriptSegment["source"]): string {
  return source === "microphone" ? "#4ade80" : "#60a5fa";
}
