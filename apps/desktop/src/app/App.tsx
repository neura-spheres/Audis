import { useState, type ReactNode } from "react";

import { Sidebar } from "@/components/Sidebar";
import { findNavItem, type ViewId } from "@/app/navigation";
import { DashboardView } from "@/features/dashboard/DashboardView";
import { SessionsView } from "@/features/sessions/SessionsView";
import { FilesView } from "@/features/files/FilesView";
import { AboutView } from "@/features/about/AboutView";
import { GeneralView } from "@/features/settings/GeneralView";
import { StorageView } from "@/features/settings/StorageView";
import { PrivacyView } from "@/features/settings/PrivacyView";
import { DiagnosticsView } from "@/features/settings/DiagnosticsView";
import { AudioView } from "@/features/audio/AudioView";
import { FeaturesView } from "@/features/launcher/FeaturesView";
import { ModelsView } from "@/features/models/ModelsView";
import { ProvidersView } from "@/features/providers/ProvidersView";
import {
  AssistantView,
  CaptionsView,
  ShortcutsView,
  SpeakersView,
  TranscriptionView,
  UpdatesView,
} from "@/features/settings/PlannedViews";

/**
 * Main window shell: a source list on the left, a titled content pane on the
 * right. Navigation is local state rather than a router, since a desktop app
 * has no URLs and nothing here needs a history model.
 */
export function App() {
  const [activeId, setActiveId] = useState<ViewId>("dashboard");
  const active = findNavItem(activeId);

  return (
    <div className="flex h-full w-full overflow-hidden">
      <Sidebar activeId={activeId} onSelect={setActiveId} />

      <main
        className="flex h-full min-w-0 flex-1 flex-col"
        style={{ background: "var(--surface-window)" }}
      >
        <header
          data-tauri-drag-region
          className="flex h-11 shrink-0 items-center px-6"
          style={{ borderBottom: "0.5px solid var(--separator)" }}
        >
          <h1
            data-tauri-drag-region
            className="text-subheadline font-semibold"
            style={{ letterSpacing: "var(--tracking-tight)" }}
          >
            {active?.title ?? "Audis"}
          </h1>
        </header>

        {/* `key` remounts the pane on navigation, so each view refetches and
            no stale state bleeds between sections. */}
        <div key={activeId} className="flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[760px] px-6 py-6">
            {renderView(activeId, setActiveId)}
          </div>
        </div>
      </main>
    </div>
  );
}

function renderView(id: ViewId, navigate: (id: ViewId) => void): ReactNode {
  switch (id) {
    case "dashboard":
      return <DashboardView onNavigate={navigate} />;
    case "features":
      return <FeaturesView onNavigate={navigate} />;
    case "sessions":
      return <SessionsView />;
    case "files":
      return <FilesView />;
    case "general":
      return <GeneralView />;
    case "audio":
      return <AudioView />;
    case "transcription":
      return <TranscriptionView />;
    case "speakers":
      return <SpeakersView />;
    case "assistant":
      return <AssistantView />;
    case "captions":
      return <CaptionsView />;
    case "shortcuts":
      return <ShortcutsView />;
    case "providers":
      return <ProvidersView />;
    case "models":
      return <ModelsView />;
    case "storage":
      return <StorageView />;
    case "updates":
      return <UpdatesView />;
    case "privacy":
      return <PrivacyView />;
    case "diagnostics":
      return <DiagnosticsView />;
    case "about":
      return <AboutView />;
  }
}
