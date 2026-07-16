import { useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
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
    help: "Reserved for the AI assistant, which is not built yet.",
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
    <div className="flex flex-col gap-3">
      <p className="px-1 text-footnote" style={{ color: "var(--label-secondary)" }}>
        These work anywhere in Windows, even when Audis is in the background. Click Change and press
        the keys you want, or Escape to cancel.
      </p>

      {SHORTCUTS.map((shortcut) => {
        const value = settings.shortcuts[shortcut.key];
        const isRecording = recording === shortcut.key;

        return (
          <div
            key={shortcut.key}
            className="flex items-center justify-between gap-4 p-3"
            style={{
              background: "var(--surface-content)",
              borderRadius: "var(--radius-card)",
              boxShadow: "var(--shadow-card)",
            }}
          >
            <div className="flex min-w-0 flex-col gap-0.5">
              <span className="text-subheadline">{shortcut.label}</span>
              <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
                {shortcut.help}
              </span>
            </div>

            <div className="flex shrink-0 items-center gap-2">
              {isRecording ? (
                <span
                  className="rounded-[6px] px-2.5 py-1 text-footnote font-medium"
                  style={{
                    color: "var(--color-accent)",
                    border: "1px solid var(--color-accent)",
                  }}
                >
                  Press keys…
                </span>
              ) : (
                <Keycap value={value} />
              )}
              <Button onClick={() => setRecording(shortcut.key)}>Change</Button>
              {value ? (
                <Button onClick={() => set(shortcut.key, null)} variant="standard">
                  Clear
                </Button>
              ) : null}
            </div>
          </div>
        );
      })}
    </div>
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
    <span
      className="rounded-[6px] px-2.5 py-1 text-footnote font-medium tabular-nums"
      style={{
        background: "var(--surface-sunken)",
        color: "var(--label-primary)",
        border: "0.5px solid var(--border-control)",
      }}
    >
      {prettyAccelerator(value)}
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
