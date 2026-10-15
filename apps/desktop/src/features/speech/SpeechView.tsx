import { useCallback, useEffect, useState } from "react";

import type { Navigate } from "@/app/navigation";
import { Page } from "@/components/Page";
import { ModelsSection } from "@/features/models/ModelsSection";
import { SpeakersSection } from "@/features/speakers/SpeakersSection";
import { EngineSection } from "@/features/transcription/TranscriptionView";
import { listModels } from "@/services/ipc";
import type { InstalledModel } from "@/schemas/ipc";

/**
 * Everything about turning speech into text: the engine, the language, the
 * models that run on this PC, and telling voices apart.
 *
 * The model list lives here, once, so the engine picker and the model list
 * can never disagree about what is installed.
 */
export function SpeechView({ onNavigate }: { onNavigate: Navigate }) {
  const [models, setModels] = useState<InstalledModel[]>();

  const refresh = useCallback(() => {
    listModels()
      .then(setModels)
      .catch(() => setModels([]));
  }, []);

  useEffect(refresh, [refresh]);

  return (
    <Page
      title="Speech Recognition"
      subtitle="What turns audio into text: free and private on this PC, or a provider for more accuracy."
    >
      <EngineSection models={models} onInstallModel={() => onNavigate("speech", "models")} />
      <ModelsSection models={models} onChanged={refresh} />
      <SpeakersSection />
    </Page>
  );
}
