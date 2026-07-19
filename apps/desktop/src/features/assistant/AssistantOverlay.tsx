import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";

import { useOverlayMenu, type OverlayMenuItem } from "@/components/OverlayMenu";
import {
  assistantResponseEventSchema,
  assistantStatusEventSchema,
  sessionStatusSchema,
  settingsSchema,
  type AssistantResponseEvent,
} from "@/schemas/ipc";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { openMainWindow } from "@/services/ipc";

/** How many recent answers the panel keeps on screen. */
const MAX_ANSWERS = 4;

/** The card's width, and the transparent room left around it for its shadow. */
const PANEL_WIDTH = 360;
const ROOM_X = 44;
const ROOM_TOP = 30;
const ROOM_BOTTOM = 56;
const WINDOW_WIDTH = PANEL_WIDTH + ROOM_X * 2;

/**
 * The floating answer panel beside the controller chip.
 *
 * It lives in its own always-on-top window and shows itself only while it has
 * something to say, so it appears when a question is detected and gets out of
 * the way otherwise. Everything it shows arrives on the `assistant/status` and
 * `assistant/response` events the backend broadcasts, so it never talks to a
 * provider itself.
 */
export function AssistantOverlay() {
  const [thinking, setThinking] = useState(false);
  const [answers, setAnswers] = useState<AssistantResponseEvent[]>([]);
  const hasContent = thinking || answers.length > 0;
  const cardRef = useRef<HTMLDivElement>(null);

  // Shrink the window to hug the card, so the transparent area around it never
  // sits over — and swallow clicks meant for — whatever is behind the panel.
  useEffect(() => {
    const card = cardRef.current;
    if (!card) return;

    const fit = () => {
      const height = Math.ceil(card.getBoundingClientRect().height) + ROOM_TOP + ROOM_BOTTOM;
      void getCurrentWindow().setSize(new LogicalSize(WINDOW_WIDTH, Math.max(96, height)));
    };

    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(card);
    return () => observer.disconnect();
  }, [hasContent]);

  useEffect(() => {
    const self = getCurrentWindow();
    // The panel owns its own visibility: it shows itself the moment it has
    // something to say and hides when there is nothing, so an empty transparent
    // window never sits over the user's other apps. Showing on every update
    // re-asserts it even if the backend hid the overlays in between.
    const reveal = () => void self.show();
    const dismiss = () => {
      setThinking(false);
      setAnswers([]);
      void self.hide();
    };

    void self.hide();

    const stopStatus = subscribe(AUDIS_EVENTS.assistantStatus, (payload) => {
      const parsed = assistantStatusEventSchema.safeParse(payload);
      if (!parsed.success) return;
      setThinking(parsed.data.thinking);
      if (parsed.data.thinking) reveal();
    });

    const stopResponse = subscribe(AUDIS_EVENTS.assistantResponse, (payload) => {
      const parsed = assistantResponseEventSchema.safeParse(payload);
      if (!parsed.success) return;
      setThinking(false);
      setAnswers((current) => [parsed.data, ...current].slice(0, MAX_ANSWERS));
      reveal();
    });

    // A finished session clears the panel and puts it away until the next one.
    const stopSession = subscribe(AUDIS_EVENTS.sessionState, (payload) => {
      const parsed = sessionStatusSchema.safeParse(payload);
      const state = parsed.success ? parsed.data.state : "idle";
      const active = state === "starting" || state === "listening" || state === "paused";
      if (!active) dismiss();
    });

    // Turning the assistant off should make the panel disappear at once.
    const stopSettings = subscribe(AUDIS_EVENTS.settingsChanged, (payload) => {
      const parsed = settingsSchema.safeParse(payload);
      if (parsed.success && !parsed.data.assistant.enabled) dismiss();
    });

    return () => {
      stopStatus();
      stopResponse();
      stopSession();
      stopSettings();
    };
  }, []);

  const menuItems: OverlayMenuItem[] = [
    {
      id: "open",
      label: "Open Audis",
      onSelect: () => void openMainWindow(),
    },
    {
      id: "clear",
      label: "Clear answers",
      separatorBefore: true,
      onSelect: () => {
        setThinking(false);
        setAnswers([]);
        void getCurrentWindow().hide();
      },
    },
  ];

  const { menu, onContextMenu } = useOverlayMenu(menuItems);

  if (!hasContent) return null;

  return (
    <div
      className="flex h-screen w-screen items-start justify-center"
      style={{
        paddingTop: ROOM_TOP,
        paddingBottom: ROOM_BOTTOM,
        paddingLeft: ROOM_X,
        paddingRight: ROOM_X,
      }}
      onContextMenu={onContextMenu}
    >
      <div
        ref={cardRef}
        className="flex w-full flex-col overflow-hidden"
        style={{
          background: "rgba(28, 28, 32, 0.9)",
          backdropFilter: "blur(28px) saturate(160%)",
          borderRadius: 18,
          border: "0.5px solid rgba(255, 255, 255, 0.14)",
          boxShadow: "0 12px 40px rgba(0, 0, 0, 0.55)",
        }}
      >
        <div data-tauri-drag-region className="flex items-center gap-2 px-3.5 pt-2.5 pb-1.5">
          <SparkGlyph />
          <span
            data-tauri-drag-region
            className="flex-1 text-footnote font-semibold"
            style={{ color: "rgba(255,255,255,0.92)", letterSpacing: "0.01em" }}
          >
            Assistant
          </span>
          {thinking ? <ThinkingDots /> : null}
        </div>

        <div className="flex max-h-[260px] flex-col gap-2 overflow-y-auto px-3.5 pb-3 pt-1">
          {thinking && answers.length === 0 ? (
            <span className="py-3 text-subheadline" style={{ color: "rgba(255,255,255,0.5)" }}>
              Thinking…
            </span>
          ) : null}

          {answers.map((entry) => (
            <div key={entry.id} className="flex flex-col gap-1">
              <span
                data-selectable
                className="text-footnote font-medium"
                style={{ color: "rgba(255,255,255,0.55)" }}
              >
                {entry.question}
              </span>
              <span
                data-selectable
                className="text-subheadline whitespace-pre-wrap"
                style={{ color: "rgba(255,255,255,0.96)", lineHeight: 1.4 }}
              >
                {entry.answer}
              </span>
            </div>
          ))}
        </div>
      </div>

      {menu}
    </div>
  );
}

function SparkGlyph() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden>
      <path
        d="M8 1.5l1.4 3.6 3.6 1.4-3.6 1.4L8 11.5 6.6 7.9 3 6.5l3.6-1.4L8 1.5z"
        fill="rgba(120,170,255,0.95)"
      />
      <circle cx="12.5" cy="12" r="1.4" fill="rgba(120,170,255,0.7)" />
    </svg>
  );
}

function ThinkingDots() {
  return (
    <span className="flex items-center gap-1" aria-hidden>
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          style={{
            width: 4,
            height: 4,
            borderRadius: "50%",
            background: "rgba(255,255,255,0.7)",
            animation: `audis-rec-pulse 1.2s ease-in-out ${i * 0.18}s infinite`,
          }}
        />
      ))}
    </span>
  );
}
