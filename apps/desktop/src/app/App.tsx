import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { AsrProblemBanner } from "@/app/AsrProblemBanner";
import { findNavItem, type Navigate, type ViewId } from "@/app/navigation";
import { Sidebar } from "@/components/Sidebar";
import { ToastHost } from "@/components/Toast";
import { AboutView } from "@/features/about/AboutView";
import { useStartupUpdateCheck } from "@/features/about/useStartupUpdateCheck";
import { AssistantView } from "@/features/assistant/AssistantView";
import { useAssistantEngine } from "@/features/assistant/useAssistantEngine";
import { AudioView } from "@/features/audio/AudioView";
import { CaptionsView } from "@/features/captions/CaptionsView";
import { HomeView } from "@/features/home/HomeView";
import { ProvidersView } from "@/features/providers/ProvidersView";
import { SessionsView } from "@/features/sessions/SessionsView";
import { GeneralView } from "@/features/settings/GeneralView";
import { PrivacyView } from "@/features/settings/PrivacyView";
import { StorageView } from "@/features/settings/StorageView";
import { ShortcutsView } from "@/features/shortcuts/ShortcutsView";
import { SpeechView } from "@/features/speech/SpeechView";
import { useSettings } from "@/hooks/useSettings";

/** How far the large title scrolls before the toolbar takes it over. */
const TITLE_HANDOFF_PX = 44;

/** Main window shell: a source list on the left, a titled content pane on the */
export function App() {
  const [activeId, setActiveId] = useState<ViewId>("home");
  // Where on the page to land: a group's anchor, or a session to open. The
  // sequence number makes following the same link twice still scroll.
  const [anchor, setAnchor] = useState<{ target: string | undefined; seq: number }>({
    target: undefined,
    seq: 0,
  });
  const [scrolled, setScrolled] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const active = findNavItem(activeId);

  // Runs the assistant whatever view is open, so answers keep coming while you
  // watch the transcript rather than only on the assistant screen.
  useAssistantEngine();
  useStartupUpdateCheck();

  const navigate = useCallback<Navigate>((id, target) => {
    setActiveId(id);
    setAnchor((current) => ({ target, seq: current.seq + 1 }));
  }, []);

  // Open on the page chosen in General, once settings arrive. Only the first
  // time: a later settings change must not yank the user off the page they are on.
  const { settings } = useSettings();
  const opened = useRef(false);
  useEffect(() => {
    if (!settings || opened.current) return;
    opened.current = true;
    setActiveId(settings.general.startPage);
  }, [settings]);

  // A new page starts at its top.
  useEffect(() => {
    if (scroller.current) scroller.current.scrollTop = 0;
    setScrolled(false);
  }, [activeId]);

  // Then glides to the group it was opened for, if any.
  useEffect(() => {
    const target = anchor.target;
    if (!target) return;
    const frame = requestAnimationFrame(() => {
      document.getElementById(target)?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
    return () => cancelAnimationFrame(frame);
  }, [anchor]);

  // Keyboard: the shortcuts a Mac app would have, with Ctrl for Command.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!event.ctrlKey || event.altKey || event.metaKey) return;
      const key = event.key.toLowerCase();
      if (key === "f" || key === "k") {
        event.preventDefault();
        search.current?.focus();
        search.current?.select();
      } else if (key === "1") {
        event.preventDefault();
        navigate("home");
      } else if (key === "2") {
        event.preventDefault();
        navigate("library");
      } else if (key === ",") {
        event.preventDefault();
        navigate("general");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate]);

  return (
    <div className="flex h-full w-full overflow-hidden">
      <Sidebar activeId={activeId} onSelect={(id) => navigate(id)} searchRef={search} />

      <main
        className="relative flex h-full min-w-0 flex-1 flex-col"
        style={{ background: "var(--surface-window)" }}
      >
        <header
          data-tauri-drag-region
          className="absolute inset-x-0 top-0 z-20 flex h-12 items-center justify-center px-6"
          style={{
            background: scrolled ? "var(--surface-toolbar)" : "transparent",
            backdropFilter: scrolled ? "blur(24px) saturate(180%)" : "none",
            boxShadow: scrolled ? "inset 0 -0.5px 0 var(--separator)" : "none",
            transition:
              "background-color var(--duration-standard) var(--ease-standard), box-shadow var(--duration-standard) var(--ease-standard)",
          }}
        >
          <span
            data-tauri-drag-region
            aria-hidden={!scrolled}
            className="text-subheadline font-semibold"
            style={{
              opacity: scrolled ? 1 : 0,
              transform: scrolled ? "none" : "translateY(6px)",
              transition:
                "opacity var(--duration-standard) var(--ease-standard), transform var(--spring-snappy-duration) var(--spring-snappy)",
            }}
          >
            {active?.label ?? "Audis"}
          </span>
        </header>

        <div
          ref={scroller}
          data-scroll-root
          className="flex-1 overflow-y-auto [overflow-anchor:none]"
          onScroll={(event) => setScrolled(event.currentTarget.scrollTop > TITLE_HANDOFF_PX)}
        >
          <div className="sticky top-12 z-10">
            <AsrProblemBanner />
          </div>
          <div key={activeId} className="mx-auto w-full max-w-[780px] px-8 pt-14 pb-16">
            {renderView(activeId, navigate, anchor.target)}
          </div>
        </div>

        <ToastHost />
      </main>
    </div>
  );
}

function renderView(id: ViewId, navigate: Navigate, anchor: string | undefined): ReactNode {
  switch (id) {
    case "home":
      return <HomeView onNavigate={navigate} />;
    case "library":
      return <SessionsView openId={anchor} />;
    case "general":
      return <GeneralView />;
    case "audio":
      return <AudioView />;
    case "speech":
      return <SpeechView onNavigate={navigate} />;
    case "captions":
      return <CaptionsView />;
    case "assistant":
      return <AssistantView onNavigate={navigate} />;
    case "providers":
      return <ProvidersView />;
    case "shortcuts":
      return <ShortcutsView />;
    case "storage":
      return <StorageView />;
    case "privacy":
      return <PrivacyView />;
    case "about":
      return <AboutView />;
  }
}
