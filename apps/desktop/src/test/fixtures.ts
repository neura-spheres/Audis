import type { InvokeArgs } from "@tauri-apps/api/core";

import type {
  AppInfo,
  DataFileListing,
  Diagnostics,
  Feature,
  InstalledModel,
  ProviderStatus,
} from "@/schemas/ipc";

/** Fixtures for the dev mock and component tests. Kept identical in shape to */

export const AUDIS_APP_INFO_MOCK: AppInfo = {
  appName: "Audis",
  company: "Neura Audis",
  publisher: "Neura Audis",
  tagline: "Hear more. Understand faster.",
  version: "0.1.0-dev",
  bundleId: "ai.neura.audis",
  dataDir: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis",
};

export const AUDIS_DIAGNOSTICS_MOCK: Diagnostics = {
  appVersion: "0.1.0-dev",
  os: "windows 11",
  arch: "x86_64",
  webviewVersion: "150.0.4078.65",
  tauriVersion: "2.11.5",
  dataDir: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis",
  logsDir: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis\\logs",
  storageBytes: 5_368_709,
  fileCount: 3,
};

export const AUDIS_FILE_LISTING_MOCK: DataFileListing = {
  root: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis",
  totalBytes: 5_368_709,
  totalFiles: 3,
  groups: [
    {
      category: "logs",
      label: "Logs",
      path: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis\\logs",
      totalBytes: 8_192,
      files: [
        {
          path: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis\\logs\\audis.log.2026-07-15",
          relativePath: "logs\\audis.log.2026-07-15",
          name: "audis.log.2026-07-15",
          sizeBytes: 8_192,
          modified: "2026-07-15T10:00:00Z",
          category: "logs",
        },
      ],
    },
    {
      category: "sessions",
      label: "Sessions",
      path: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis\\sessions",
      totalBytes: 0,
      files: [],
    },
    {
      category: "recordings",
      label: "Recordings",
      path: "C:\\Users\\you\\AppData\\Local\\NeuraAudis\\Audis\\recordings",
      totalBytes: 0,
      files: [],
    },
  ],
};

export const AUDIS_FEATURES_MOCK: Feature[] = [
  {
    id: "liveCaption",
    name: "Live Caption",
    summary: "Captions on screen as people speak. Nothing is saved.",
    details: ["Lowest latency of any mode", "Captions float above your other windows"],
    status: "ready",
    blocker: null,
    usesCloud: false,
  },
  {
    id: "meetingAssistant",
    name: "Meeting Assistant",
    summary: "Transcription plus a rolling summary, decisions and action items.",
    details: ["Speaker separation", "Rolling summary as the meeting goes"],
    status: "needsSetup",
    blocker: "Connect an AI provider first. Open Providers; Gemini and Groq have free tiers.",
    usesCloud: true,
  },
];

export const AUDIS_MODELS_MOCK: InstalledModel[] = [
  {
    info: {
      id: "whisperBase",
      name: "Whisper Base",
      summary: "The best balance for most people.",
      sizeBytes: 148_000_000,
      fileName: "ggml-base.bin",
      url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
      requirement: "Runs comfortably on any modern PC.",
      recommended: true,
      keepsUpLive: true,
    },
    installed: false,
    installedBytes: null,
  },
];

export const AUDIS_PROVIDERS_MOCK: ProviderStatus[] = [
  {
    info: {
      id: "gemini",
      name: "Google Gemini",
      summary: "Free tier that is generous enough for everyday use.",
      consoleUrl: "https://aistudio.google.com/apikey",
      freeTier: true,
      defaultModel: "gemini-2.0-flash",
      models: ["gemini-2.0-flash", "gemini-2.0-flash-lite"],
      needsEndpoint: false,
      speech: {
        api: "geminiInline",
        baseUrl: "https://generativelanguage.googleapis.com/v1beta",
        defaultModel: "gemini-2.0-flash",
        models: ["gemini-2.0-flash"],
        summary: "Free tier. Good at Indonesian.",
      },
    },
    hasKey: false,
    enabled: false,
    model: "gemini-2.0-flash",
    endpoint: null,
  },
];

/** Wrap an IPC handler so event subscriptions and session polling succeed. */
export function withAmbientIpc(
  handler: (command: string, args?: InvokeArgs) => unknown,
): (command: string, args?: InvokeArgs) => unknown {
  return (command, args) => {
    if (command === "plugin:event|listen") return 1;
    if (command === "plugin:event|unlisten") return null;
    if (command === "get_session_status") return null;
    if (command === "list_provider_models") return [];

    return handler(command, args);
  };
}
