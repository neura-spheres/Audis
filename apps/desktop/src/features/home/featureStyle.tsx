import type { ReactNode } from "react";

import type { Tint } from "@/components/IconBadge";
import { AiIcon, CaptionIcon, DocumentIcon, SpeakerIcon } from "@/components/icons";
import type { FeatureId } from "@/schemas/ipc";

/** How each feature looks wherever it appears: tiles, the library, the mini player. */
export const FEATURE_STYLE: Record<FeatureId, { name: string; tint: Tint; icon: ReactNode }> = {
  liveCaption: { name: "Live Caption", tint: "blue", icon: <CaptionIcon /> },
  transcription: { name: "Transcription", tint: "orange", icon: <DocumentIcon /> },
  meetingAssistant: { name: "Meeting Assistant", tint: "indigo", icon: <AiIcon /> },
  interviewPractice: { name: "Interview Practice", tint: "pink", icon: <SpeakerIcon /> },
};

/** The user-facing name of a feature, for anything that only has its id. */
export function featureName(id: string): string {
  return FEATURE_STYLE[id as FeatureId]?.name ?? "Session";
}
