import { useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { GroupedList, Row } from "@/components/GroupedList";
import { Page } from "@/components/Page";
import { Button } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import type { ShortcutSettings } from "@/schemas/ipc";

type ShortcutKey = keyof ShortcutSettings;

const SHORTCUTS: { key: ShortcutKey; label: string; help: string }[] = [
  { key: "stopSession", label: "Stop session", help: "End the running session from anywhere." },
  { key: "togglePause", label: "Pause or resume", help: "Hold the session without ending it." },
  { key: "toggleCaptions", label: "Show or hide captions", help: "Toggle the caption overlay." },
  {
    key: "askAssistant",
    label: "Ask the assistant",
    help: "Answer the latest line now, during a session with the assistant on.",
  },
];

export function ShortcutsView() {
  const { settings, error, update } = useSettings();
  const [recording, setRecording] = useState<ShortcutKey>();

  useEffect(() => {
    if (!recording) return;

    const onKey = (event: KeyboardEvent) => {
      event.preventDefault();
      if (event.key === "Escape") {
        setRecording(undefined);
        return;
      }
      const accelerator = toAccelerator(event);
      if (!accelerator) return;

      update((current) => ({
        ...current,
        shortcuts: { ...current.shortcuts, [recording]: accelerator },
      }));
      setRecording(undefined);
    };

    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, update]);

  if (error) return <ErrorNotice error={error} />;
  if (!settings) return null;

  const set = (key: ShortcutKey, value: string | null) =>
    update((current) => ({
      ...current,
      shortcuts: { ...current.shortcuts, [key]: value },
    }));

  return (
    <Page
      title="Shortcuts"
      subtitle="These work anywhere in Windows, even when Audis is in the background."
    >
      <GroupedList footnote="Click Change and press the keys you want, or Escape to cancel. A shortcut needs Ctrl, Alt, Shift or Win.">
        {SHORTCUTS.map((shortcut) => {
          const value = settings.shortcuts[shortcut.key];
          const isRecording = recording === shortcut.key;

          return (
            <Row
              key={shortcut.key}
              label={shortcut.label}
              description={shortcut.help}
              value={
                <div className="flex shrink-0 items-center gap-2">
                  {isRecording ? (
                    <span
                      className="rounded-[6px] px-2.5 py-1 text-footnote font-medium"
                      style={{
                        color: "var(--color-accent)",
                        boxShadow: "inset 0 0 0 1px var(--color-accent)",
                        animation: "audis-breathe 1.6s ease-in-out infinite",
                      }}
                    >
                      Press keys…
                    </span>
                  ) : (
                    <Keycap value={value} />
                  )}
                  <Button onClick={() => setRecording(shortcut.key)}>Change</Button>
                  {value ? (
                    <Button onClick={() => set(shortcut.key, null)} variant="plain">
                      Clear
                    </Button>
                  ) : null}
                </div>
              }
            />
          );
        })}
      </GroupedList>
    </Page>
  );
}

function Keycap({ value }: { value: string | null }) {
  if (!value) {
    return (
      <span className="text-footnote" style={{ color: "var(--label-tertiary)" }}>
        Not set
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1">
      {prettyAccelerator(value)
        .split(" ")
        .map((key, index) => (
          <kbd
            key={`${key}-${index}`}
            className="min-w-[22px] px-1.5 py-0.5 text-center text-caption1 font-semibold"
            style={{
              fontFamily: "var(--font-sans)",
              background: "var(--surface-elevated)",
              color: "var(--label-primary)",
              borderRadius: 5,
              boxShadow: "0 1px 0 var(--border-control), 0 0 0 0.5px var(--border-control)",
            }}
          >
            {key}
          </kbd>
        ))}
    </span>
  );
}

/** Build a Tauri accelerator string from a keydown, or null if unusable. */
function toAccelerator(event: KeyboardEvent): string | null {
  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (event.metaKey) modifiers.push("Super");

  const key = mainKey(event.code);
  if (!key || modifiers.length === 0) return null;

  return [...modifiers, key].join("+");
}

function mainKey(code: string): string | null {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  const named: Record<string, string> = {
    Space: "Space",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Enter: "Enter",
    Comma: ",",
    Period: ".",
    Slash: "/",
    Backquote: "`",
  };
  return named[code] ?? null;
}

/** "Ctrl+Shift+S" → the OS glyphs users expect. */
function prettyAccelerator(value: string): string {
  return value
    .split("+")
    .map((part) => {
      switch (part) {
        case "Ctrl":
        case "CmdOrCtrl":
        case "CommandOrControl":
          return "Ctrl";
        case "Alt":
          return "Alt";
        case "Shift":
          return "⇧";
        case "Super":
          return "Win";
        default:
          return part;
      }
    })
    .join(" ");
}
