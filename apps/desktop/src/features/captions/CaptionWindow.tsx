import { useEffect, useRef, useState } from "react";

import { useOverlayMenu, type OverlayMenuItem } from "@/components/OverlayMenu";
import { useSession } from "@/hooks/useSession";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import {
  beginCaptionDrag,
  endCaptionDrag,
  getSettings,
  hideOverlay,
  openMainWindow,
  resetCaptionPosition,
  setCaptionClickThrough,
  setCaptionHotRects,
  type HotRect,
} from "@/services/ipc";
import {
  diagnosticWarningSchema,
  settingsSchema,
  transcriptSegmentSchema,
  type CaptionSettings,
  type TranscriptSegment,
} from "@/schemas/ipc";

export function CaptionWindow() {
  const [lines, setLines] = useState<TranscriptSegment[]>([]);
  const [partial, setPartial] = useState<TranscriptSegment>();
  const [captions, setCaptions] = useState<CaptionSettings>();
  const [active, setActive] = useState(false);
  const [locked, setLocked] = useState(false);
  const [problem, setProblem] = useState<string>();
  const { session, stop, setPaused } = useSession();
  const paused = session?.state === "paused";

  const panelRef = useRef<HTMLDivElement>(null);
  const gripRef = useRef<HTMLButtonElement>(null);

  const clickThrough = captions?.clickThrough ?? false;
  const draggable = !locked && !clickThrough;

  const menuItems: OverlayMenuItem[] = [
    {
      id: "pause",
      label: paused ? "Resume" : "Pause",
      onSelect: () => void setPaused(!paused),
    },
    { id: "open", label: "Open Audis", onSelect: () => void openMainWindow() },
    {
      id: "lock",
      label: locked ? "Unlock position" : "Lock position",
      separatorBefore: true,
      onSelect: () => setLocked((value) => !value),
    },
    {
      id: "click-through",
      label: clickThrough ? "Make captions clickable" : "Let clicks pass through",
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

  const { menu, onContextMenu, isOpen: menuOpen } = useOverlayMenu(menuItems);

  useEffect(() => {
    getSettings()
      .then((settings) => setCaptions(settings.captions))
      .catch(() => undefined);

    const stopSettings = subscribe(AUDIS_EVENTS.settingsChanged, (payload) => {
      const parsed = settingsSchema.safeParse(payload);
      if (parsed.success) setCaptions(parsed.data.captions);
    });

    const stopActive = subscribe<boolean>(AUDIS_EVENTS.captionActive, (payload) => {
      if (typeof payload === "boolean") setActive(payload);
    });

    return () => {
      stopSettings();
      stopActive();
    };
  }, []);

  const maxLines = captions?.maxLines ?? 2;

  useEffect(() => {
    const stopTranscript = subscribe(AUDIS_EVENTS.transcriptFinal, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (!parsed.success) return;
      setLines((current) => [...current, parsed.data].slice(-maxLines));
      setPartial(undefined);
      setProblem(undefined);
    });

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

  useEffect(() => {
    if (!captions) return;

    const report = () => {
      let rects: HotRect[] = [];
      if (menuOpen) {
        rects = [{ x: 0, y: 0, w: window.innerWidth, h: window.innerHeight }];
      } else if (clickThrough) {
        rects = rectOf(gripRef.current);
      } else {
        rects = rectOf(panelRef.current);
      }
      void setCaptionHotRects(rects);
    };

    report();

    const observer = new ResizeObserver(report);
    if (panelRef.current) observer.observe(panelRef.current);
    if (gripRef.current) observer.observe(gripRef.current);
    window.addEventListener("resize", report);

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", report);
    };
  }, [captions, clickThrough, menuOpen]);

  if (!captions) return null;

  const baseOpacity = captions.backgroundOpacity / 100;
  const opacity = active ? Math.max(0.9, baseOpacity) : baseOpacity;
  const hasPanel = opacity > 0.01;
  const showAffordance = active && draggable;

  const visible = [...lines, ...(partial ? [partial] : [])].slice(-maxLines);
  const showing = visible.length > 0 || problem !== undefined;

  const startDrag = (event: React.PointerEvent) => {
    if (!draggable || event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    void beginCaptionDrag();
  };

  const stopDrag = (event: React.PointerEvent) => {
    try {
      event.currentTarget.releasePointerCapture(event.pointerId);
    } catch {
      void 0;
    }
    void endCaptionDrag();
  };

  return (
    <div
      className="flex h-screen w-screen items-end justify-center p-4"
      onContextMenu={onContextMenu}
    >
      <div
        ref={panelRef}
        onPointerDown={startDrag}
        onPointerUp={stopDrag}
        onLostPointerCapture={stopDrag}
        className="relative flex w-fit max-w-full flex-col gap-1.5"
        style={{
          padding: hasPanel ? "10px 16px" : "4px 8px",
          borderRadius: 16,
          cursor: draggable ? "grab" : "default",
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
          opacity: showing || active ? 1 : 0,
          transition: "border-color 140ms ease, box-shadow 140ms ease, opacity 180ms ease",
        }}
      >
        {clickThrough ? (
          <GripButton gripRef={gripRef} active={active} onOpen={onContextMenu} />
        ) : (
          <DragHandle visible={showAffordance} />
        )}

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

function rectOf(element: Element | null): HotRect[] {
  if (!element) return [];
  const rect = element.getBoundingClientRect();
  return [{ x: rect.left, y: rect.top, w: rect.width, h: rect.height }];
}

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

function DragHandle({ visible }: { visible: boolean }) {
  return (
    <span
      aria-hidden
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

function GripButton({
  gripRef,
  active,
  onOpen,
}: {
  gripRef: React.RefObject<HTMLButtonElement | null>;
  active: boolean;
  onOpen: (event: React.MouseEvent) => void;
}) {
  return (
    <button
      ref={gripRef}
      type="button"
      aria-label="Caption options"
      title="Caption options — clicks pass through the rest"
      onClick={onOpen}
      onContextMenu={onOpen}
      className="absolute -top-7 left-1/2 flex -translate-x-1/2 items-center justify-center"
      style={{
        width: 42,
        height: 22,
        borderRadius: 999,
        background: "rgba(28, 28, 32, 0.92)",
        border: "0.5px solid rgba(255, 255, 255, 0.22)",
        color: "rgba(255, 255, 255, 0.75)",
        boxShadow: "0 6px 20px rgba(0, 0, 0, 0.5)",
        opacity: active ? 1 : 0.55,
        transition: "opacity 140ms ease",
        cursor: "pointer",
      }}
    >
      <svg width="16" height="4" viewBox="0 0 16 4" fill="currentColor" aria-hidden>
        <circle cx="2" cy="2" r="1.6" />
        <circle cx="8" cy="2" r="1.6" />
        <circle cx="14" cy="2" r="1.6" />
      </svg>
    </button>
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

function sourceColour(source: TranscriptSegment["source"]): string {
  return source === "microphone" ? "#4ade80" : "#60a5fa";
}
