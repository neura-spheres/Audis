import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";

import {
  AUDIS_APP_INFO_MOCK,
  AUDIS_DIAGNOSTICS_MOCK,
  AUDIS_FILE_LISTING_MOCK,
  AUDIS_MODELS_MOCK,
  AUDIS_PROVIDERS_MOCK,
} from "@/test/fixtures";
import type {
  Feature,
  FeatureId,
  InstalledModel,
  ProviderStatus,
  SessionStatus,
  SessionSummary,
  Settings,
  TranscriptSegment,
} from "@/schemas/ipc";

/**
 * Fake IPC backend for browser development.
 *
 * Enough of Audis to walk every screen in a browser: sessions start, stream a
 * sample transcript and land in Library; models install with progress; keys
 * save. It is never loaded inside the real app, which has a Rust core.
 */
export function installDevIpcMock(): void {
  if ("__TAURI_INTERNALS__" in window) return;

  let settings: Settings = {
    version: 1,
    general: {
      theme: "system",
      startPage: "home",
      closeBehavior: "minimizeToTray",
      showTrayIcon: true,
    },
    audio: {
      microphoneId: null,
      computerAudioId: null,
    },
    transcription: {
      engine: { kind: "local", model: "whisperBase" },
      model: "whisperBase",
      language: "indonesian",
      captureMicrophone: true,
      captureComputerAudio: true,
    },
    speakers: {
      enabled: true,
      expectedSpeakers: 0,
    },
    recording: {
      enabled: false,
    },
    captions: {
      fontSize: 28,
      maxLines: 3,
      backgroundOpacity: 70,
      showSourceLabels: true,
      clickThrough: false,
    },
    shortcuts: {
      stopSession: "CmdOrCtrl+Shift+S",
      togglePause: "CmdOrCtrl+Shift+P",
      toggleCaptions: "CmdOrCtrl+Shift+C",
      askAssistant: "CmdOrCtrl+Shift+A",
    },
    assistant: {
      enabled: false,
      provider: "gemini",
      model: "gemini-3.8-flash",
      context: "general",
      notes: "",
      answerOwnQuestions: false,
    },
    updates: { channel: "stable", checkOnStartup: true },
    providers: [],
  };

  let models: InstalledModel[] = AUDIS_MODELS_MOCK.map((model) => ({ ...model }));
  let providers: ProviderStatus[] = AUDIS_PROVIDERS_MOCK.map((provider) => ({ ...provider }));
  let session: SessionStatus | null = null;
  // When the current run of listening began, so pausing reports real time.
  let runningSince = 0;
  let feed: ReturnType<typeof setInterval> | undefined;
  let library: SessionSummary[] = seedLibrary();

  const features = (): Feature[] => {
    const hasModel = models.some((model) => model.installed);
    const hasKey = providers.some((provider) => provider.hasKey);
    return FEATURES.map(([id, name, summary, usesCloud]) => {
      const blocker = !hasModel
        ? "Install a speech model first. Open Models and choose Whisper Base; it is free."
        : usesCloud && !hasKey
          ? "Connect an AI provider first. Open Providers; Gemini and Groq have free tiers."
          : null;
      return {
        id,
        name,
        summary,
        details: [],
        status: blocker ? "needsSetup" : "ready",
        blocker,
        usesCloud,
      };
    });
  };

  const publish = (next: SessionStatus | null, state?: SessionStatus["state"]) => {
    if (next) void emit("audis://session/state", { ...next, state: state ?? next.state });
  };

  const stopFeed = () => {
    if (feed) clearInterval(feed);
    feed = undefined;
  };

  const startFeed = (id: string) => {
    let line = 0;
    let at = 0;
    feed = setInterval(() => {
      if (!session || session.state !== "listening") return;
      const [source, text] = SAMPLE_LINES[line % SAMPLE_LINES.length]!;
      const segment: TranscriptSegment = {
        id: crypto.randomUUID(),
        sessionId: id,
        source,
        speaker: source === "microphone" ? "You" : "Speaker 1",
        startMs: at,
        endMs: at + 2400,
        text,
        language: settings.transcription.language,
        confidence: null,
        isFinal: false,
        engine: "dev-mock",
      };
      // A partial first, then the finished line, as a real engine sends them.
      void emit("audis://transcript/partial", {
        ...segment,
        text: text.split(" ").slice(0, 4).join(" "),
      });
      setTimeout(() => void emit("audis://transcript/final", { ...segment, isFinal: true }), 700);
      line += 1;
      at += 2600;
    }, 2200);
  };

  mockIPC(
    (command, args) => {
      const input = (args ?? {}) as Record<string, unknown>;
      switch (command) {
        case "get_app_info":
          return AUDIS_APP_INFO_MOCK;
        case "get_settings":
          return settings;
        case "update_settings":
          settings = input["settings"] as Settings;
          void emit("audis://settings/changed", settings);
          return settings;
        case "list_data_files":
          return AUDIS_FILE_LISTING_MOCK;
        case "get_diagnostics":
          return AUDIS_DIAGNOSTICS_MOCK;
        case "list_audio_devices":
          return {
            inputs: [
              {
                id: "mic-1",
                name: "Microphone Array (Realtek)",
                kind: "input",
                isDefault: true,
                sampleRate: 48000,
                channels: 2,
              },
            ],
            outputs: [
              {
                id: "out-1",
                name: "Headphones",
                kind: "output",
                isDefault: true,
                sampleRate: 48000,
                channels: 2,
              },
            ],
          };
        case "start_audio_test":
          return {
            microphone: null,
            computerAudio: null,
            microphoneError: null,
            computerAudioError: null,
          };
        case "list_features":
          return features();
        case "list_models":
          return models;
        case "is_model_downloading":
          return false;
        case "install_model": {
          const id = input["id"];
          const total = 148_000_000;
          let done = 0;
          const tick = setInterval(() => {
            done = Math.min(total, done + total / 8);
            const finished = done >= total;
            if (finished) {
              clearInterval(tick);
              models = models.map((model) =>
                model.info.id === id ? { ...model, installed: true, installedBytes: total } : model,
              );
            }
            void emit("audis://model/progress", {
              id,
              downloadedBytes: done,
              totalBytes: total,
              done: finished,
              error: null,
            });
          }, 350);
          return null;
        }
        case "remove_model":
          models = models.map((model) =>
            model.info.id === input["id"]
              ? { ...model, installed: false, installedBytes: null }
              : model,
          );
          return null;
        case "list_providers":
          return providers;
        case "list_provider_models":
          return [];
        case "set_provider_key":
          providers = providers.map((provider) =>
            provider.info.id === input["id"] ? { ...provider, hasKey: true } : provider,
          );
          return null;
        case "delete_provider_key":
          providers = providers.map((provider) =>
            provider.info.id === input["id"] ? { ...provider, hasKey: false } : provider,
          );
          return null;
        case "get_session_status":
          return session;
        case "start_session": {
          const mode = input["feature"] as FeatureId;
          session = {
            id: crypto.randomUUID(),
            mode,
            state: "listening",
            language: settings.transcription.language,
            elapsedMs: 0,
            microphone: settings.transcription.captureMicrophone,
            computerAudio: settings.transcription.captureComputerAudio,
            captionsVisible: true,
            assistantEnabled: false,
            error: null,
          };
          runningSince = Date.now();
          publish(session);
          startFeed(session.id);
          return session;
        }
        case "set_session_paused": {
          if (!session) throw notRunning();
          const pausing = Boolean(input["paused"]);
          const elapsedMs = pausing
            ? session.elapsedMs + (Date.now() - runningSince)
            : session.elapsedMs;
          if (!pausing) runningSince = Date.now();
          session = { ...session, state: pausing ? "paused" : "listening", elapsedMs };
          publish(session);
          return session;
        }
        case "stop_session": {
          if (!session) throw notRunning();
          stopFeed();
          const ended = session;
          session = null;
          if (ended.mode !== "liveCaption") {
            library = [
              {
                id: ended.id,
                mode: ended.mode,
                language: ended.language,
                startedAt: new Date().toISOString(),
                endedAt: new Date().toISOString(),
                segmentCount: 6,
                elapsedMs: 15_000,
                complete: true,
              },
              ...library,
            ];
          }
          publish(ended, "completed");
          return { ...ended, state: "completed" };
        }
        case "list_sessions":
          return library;
        case "get_session_transcript":
          return SAMPLE_LINES.map(([source, text], index) => ({
            id: `segment-${index}`,
            sessionId: input["id"],
            source,
            speaker: source === "microphone" ? "You" : "Speaker 1",
            startMs: index * 2600,
            endMs: index * 2600 + 2400,
            text,
            language: "indonesian",
            confidence: null,
            isFinal: true,
            engine: "dev-mock",
          }));
        case "revise_session_segment":
          return {
            id: input["segmentId"],
            sessionId: input["id"],
            source: "microphone",
            speaker: input["speaker"] ?? "You",
            startMs: 0,
            endMs: 2400,
            text: input["text"],
            language: "indonesian",
            confidence: null,
            isFinal: true,
            engine: "dev-mock",
          };
        case "export_session":
          return `${AUDIS_APP_INFO_MOCK.dataDir}\\exports\\session.${input["format"] === "srt" ? "srt" : input["format"] === "markdown" ? "md" : "txt"}`;
        case "generate_session_report":
          return new Promise((resolve) =>
            setTimeout(() => resolve(`${AUDIS_APP_INFO_MOCK.dataDir}\\exports\\report.pdf`), 1500),
          );
        case "delete_session":
          library = library.filter((item) => item.id !== input["id"]);
          return null;
        case "ask_assistant":
          return new Promise((resolve) =>
            setTimeout(() => resolve("A sample answer from the browser preview."), 900),
          );
        case "check_for_updates":
          return { currentVersion: AUDIS_APP_INFO_MOCK.version, update: null, channel: "stable" };
        case "stop_audio_test":
        case "cancel_model_download":
        case "update_provider":
        case "open_data_file":
        case "reveal_data_file":
        case "close_main_window":
        case "set_assistant_enabled":
          return null;
        default:
          throw new Error(`dev mock has no handler for command "${command}"`);
      }
    },
    { shouldMockEvents: true },
  );
}

function notRunning() {
  return {
    title: "No session is running",
    explanation: "There is nothing to pause or stop.",
    dataPreserved: true,
    suggestedAction: "Start a session from Home.",
    technicalDetails: null,
    diagnosticCode: "INVALID_ARGUMENT",
  };
}

const FEATURES: [FeatureId, string, string, boolean][] = [
  ["liveCaption", "Live Caption", "Captions on screen as people speak. Nothing is saved.", false],
  [
    "transcription",
    "Transcription",
    "Captions plus a saved transcript you can search and export.",
    false,
  ],
  [
    "meetingAssistant",
    "Meeting Assistant",
    "Transcription plus a rolling summary, decisions and action items.",
    true,
  ],
  [
    "interviewPractice",
    "Interview Practice",
    "Practice answering questions with the assistant helping.",
    true,
  ],
];

const SAMPLE_LINES: ["microphone" | "computerAudio", string][] = [
  ["computerAudio", "Selamat pagi semua, terima kasih sudah bergabung hari ini."],
  ["microphone", "Pagi, saya siap mulai kapan saja."],
  ["computerAudio", "Kita bahas progres rilis minggu ini dulu, ya."],
  ["computerAudio", "Desain baru sudah selesai dan siap diuji."],
  ["microphone", "Bagus, saya akan cek build-nya sore ini."],
  ["computerAudio", "Oke, kita lanjut ke agenda berikutnya."],
];

function seedLibrary(): SessionSummary[] {
  const at = (daysAgo: number, hour: number) => {
    const date = new Date();
    date.setDate(date.getDate() - daysAgo);
    date.setHours(hour, 12, 0, 0);
    return date.toISOString();
  };
  return [
    {
      id: "seed-1",
      mode: "meetingAssistant",
      language: "indonesian",
      startedAt: at(0, 9),
      endedAt: at(0, 10),
      segmentCount: 184,
      elapsedMs: 2_712_000,
      complete: true,
    },
    {
      id: "seed-2",
      mode: "transcription",
      language: "english",
      startedAt: at(1, 15),
      endedAt: at(1, 15),
      segmentCount: 42,
      elapsedMs: 604_000,
      complete: true,
    },
    {
      id: "seed-3",
      mode: "transcription",
      language: "indonesian",
      startedAt: at(9, 11),
      endedAt: null,
      segmentCount: 17,
      elapsedMs: 191_000,
      complete: false,
    },
  ];
}
