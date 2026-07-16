import { useEffect, useState } from "react";

import { useOverlayMenu, type OverlayMenuItem } from "@/components/OverlayMenu";
import { useSession } from "@/hooks/useSession";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { getSettings, hideOverlay, openMainWindow } from "@/services/ipc";
import {
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
  const { session, stop, setPaused } = useSession();
  const paused = session?.state === "paused";

  const menuItems: OverlayMenuItem[] = [
    {
      id: "pause",
      label: paused ? "Resume" : "Pause",
      onSelect: () => void setPaused(!paused),
    },
    { id: "open", label: "Open Audis", onSelect: () => void openMainWindow() },
    {
      id: "hide",
      label: "Hide captions",
      separatorBefore: true,
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

  const maxLines = captions?.maxLines ?? 3;

  useEffect(() => {
    const stopTranscript = subscribe(AUDIS_EVENTS.transcriptFinal, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (!parsed.success) return;

      setLines((current) => [...current, parsed.data].slice(-maxLines));
      setPartial(undefined);
    });

    const stopPartial = subscribe(AUDIS_EVENTS.transcriptPartial, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (parsed.success) setPartial(parsed.data);
    });

    const stopSession = subscribe(AUDIS_EVENTS.sessionState, () => {
      setLines([]);
      setPartial(undefined);
    });

    return () => {
      stopTranscript();
      stopPartial();
      stopSession();
    };
  }, [maxLines]);

  if (!captions) return null;

  const opacity = captions.backgroundOpacity / 100;
  const hasPanel = opacity > 0.01;

  const visible = [...lines, ...(partial ? [partial] : [])].slice(-maxLines);

  return (
    <div
      data-tauri-drag-region
      className="flex h-screen w-screen items-end justify-center p-4"
      onContextMenu={onContextMenu}
    >
      <div
        data-tauri-drag-region
        className="flex w-full flex-col gap-2 transition-opacity"
        style={{
          maxWidth: "min(100%, 1100px)",
          padding: hasPanel ? "18px 24px" : "4px 8px",
          borderRadius: 18,
          background: hasPanel ? `rgba(14, 15, 17, ${opacity})` : "transparent",
          backdropFilter: hasPanel ? "blur(20px) saturate(140%)" : undefined,
          border: hasPanel ? `1px solid rgba(255, 255, 255, ${0.1 * opacity})` : undefined,
          boxShadow: hasPanel ? "0 8px 40px rgba(0, 0, 0, 0.45)" : undefined,
          opacity: visible.length > 0 ? 1 : 0,
          transitionDuration: "180ms",
        }}
      >
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
      className="flex items-baseline gap-2.5 leading-[1.35]"
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
        fontSize: Math.max(11, Math.round(size * 0.42)),
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
