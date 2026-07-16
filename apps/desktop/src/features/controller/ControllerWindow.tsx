import { useEffect, useState } from "react";

import { useOverlayMenu, type OverlayMenuItem } from "@/components/OverlayMenu";
import { useSession } from "@/hooks/useSession";
import { hideOverlay, openMainWindow } from "@/services/ipc";

/** The controller chip. */
export function ControllerWindow() {
  const { session, stop, setPaused } = useSession();
  const paused = session?.state === "paused";

  const menuItems: OverlayMenuItem[] = [
    {
      id: "pause",
      label: paused ? "Resume" : "Pause",
      onSelect: () => void setPaused(!paused),
    },
    {
      id: "open",
      label: "Open Audis",
      onSelect: () => void openMainWindow(),
    },
    {
      id: "hide-captions",
      label: "Hide captions",
      onSelect: () => void hideOverlay("captions"),
      separatorBefore: true,
    },
    {
      id: "hide-controller",
      label: "Hide this controller",
      onSelect: () => void hideOverlay("controller"),
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

  if (!session) return null;

  return (
    <div
      className="flex h-screen w-screen items-center justify-center p-2"
      onContextMenu={onContextMenu}
    >
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 py-1.5 pl-3 pr-1.5"
        style={{
          background: "rgba(28, 28, 32, 0.82)",
          backdropFilter: "blur(24px) saturate(160%)",
          borderRadius: 999,
          border: "0.5px solid rgba(255, 255, 255, 0.14)",
          boxShadow: "0 8px 30px rgba(0, 0, 0, 0.5)",
        }}
      >
        <StatusDot paused={paused} />
        <Timer session={session} paused={paused} />

        <div className="mx-0.5 h-5 w-px" style={{ background: "rgba(255,255,255,0.14)" }} />

        <ChipButton label={paused ? "Resume" : "Pause"} onClick={() => void setPaused(!paused)}>
          {paused ? <PlayGlyph /> : <PauseGlyph />}
        </ChipButton>
        <ChipButton label="Stop session" danger onClick={() => void stop()}>
          <StopGlyph />
        </ChipButton>
      </div>

      {menu}
    </div>
  );
}

function StatusDot({ paused }: { paused: boolean }) {
  return (
    <span
      data-tauri-drag-region
      aria-hidden
      className="relative flex h-2.5 w-2.5 shrink-0 items-center justify-center"
    >
      <span
        style={{
          width: 9,
          height: 9,
          borderRadius: "50%",
          background: paused ? "#f5a623" : "#ff4d4d",
          animation: paused ? undefined : "audis-rec-pulse 1.6s ease-in-out infinite",
        }}
      />
    </span>
  );
}

function Timer({ session, paused }: { session: { elapsedMs: number }; paused: boolean }) {
  const [display, setDisplay] = useState(session.elapsedMs);

  useEffect(() => {
    setDisplay(session.elapsedMs);
    if (paused) return;

    const startedAt = Date.now();
    const base = session.elapsedMs;
    const id = window.setInterval(() => setDisplay(base + (Date.now() - startedAt)), 200);
    return () => window.clearInterval(id);
  }, [session.elapsedMs, paused]);

  return (
    <span
      data-tauri-drag-region
      className="min-w-[42px] text-center text-footnote font-semibold tabular-nums"
      style={{ color: "rgba(255,255,255,0.95)", letterSpacing: "0.02em" }}
    >
      {formatClock(display)}
    </span>
  );
}

function ChipButton({
  label,
  danger,
  onClick,
  children,
}: {
  label: string;
  danger?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="audis-chip-button flex h-7 w-7 items-center justify-center rounded-full"
      style={{ color: danger ? "#ff6b6b" : "rgba(255,255,255,0.9)" }}
    >
      {children}
    </button>
  );
}

/** mm:ss, or h:mm:ss once a session passes an hour. */
function formatClock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const seconds = total % 60;
  const minutes = Math.floor(total / 60) % 60;
  const hours = Math.floor(total / 3600);
  const pad = (value: number) => value.toString().padStart(2, "0");
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${pad(minutes)}:${pad(seconds)}`;
}

function PauseGlyph() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden>
      <rect x="4" y="3" width="3" height="10" rx="1" />
      <rect x="9" y="3" width="3" height="10" rx="1" />
    </svg>
  );
}

function PlayGlyph() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden>
      <path d="M5 3.5v9a.5.5 0 0 0 .77.42l7-4.5a.5.5 0 0 0 0-.84l-7-4.5A.5.5 0 0 0 5 3.5z" />
    </svg>
  );
}

function StopGlyph() {
  return (
    <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden>
      <rect x="3.5" y="3.5" width="9" height="9" rx="2" />
    </svg>
  );
}
