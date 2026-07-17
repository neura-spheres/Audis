import { z } from "zod";

/** Runtime schemas for everything crossing the Rust/React boundary. */

/** Mirrors `DiagnosticCode` in audis-common. */
export const diagnosticCodeSchema = z.enum([
  "CONFIG_INVALID",
  "STORAGE_UNAVAILABLE",
  "DATA_SERIALIZATION",
  "INVALID_REQUEST",
  "UNEXPECTED",
]);

/** Mirrors `UserFacingError`. Every failed command returns this shape. */
export const userFacingErrorSchema = z.object({
  title: z.string().min(1),
  explanation: z.string().min(1),
  dataPreserved: z.boolean(),
  suggestedAction: z.string().min(1),
  technicalDetails: z.string().nullable(),
  diagnosticCode: diagnosticCodeSchema,
});

/** Mirrors `AudioSourceKind`. */
export const audioSourceKindSchema = z.enum(["microphone", "computerAudio"]);

/** Mirrors `ProviderId`. The one list; everything else refers to this. */
export const providerIdSchema = z.enum([
  "gemini",
  "deepSeek",
  "groq",
  "anthropic",
  "deepgram",
  "openAiCompatible",
]);

/** Mirrors `AppInfo`. */
export const appInfoSchema = z.object({
  appName: z.string().min(1),
  company: z.string().min(1),
  publisher: z.string().min(1),
  tagline: z.string().min(1),
  version: z.string().min(1),
  bundleId: z.string().min(1),
  dataDir: z.string().min(1),
});

/** Mirrors `DiagnosticWarning`. */
export const diagnosticWarningSchema = z.object({
  kind: z.string().min(1),
  message: z.string(),
});

/** Mirrors `Settings` and its nested types in audis-common. */
export const themePreferenceSchema = z.enum(["light", "dark", "system"]);
export const startPageSchema = z.enum(["dashboard", "sessions"]);
export const closeBehaviorSchema = z.enum(["minimizeToTray", "quit"]);

export const generalSettingsSchema = z.object({
  theme: themePreferenceSchema,
  startPage: startPageSchema,
  closeBehavior: closeBehaviorSchema,
  showTrayIcon: z.boolean(),
});

export const audioSettingsSchema = z.object({
  microphoneId: z.string().nullable(),
  computerAudioId: z.string().nullable(),
});

/** Mirrors `TranscriptionEngine`. The one setting that decides where audio goes. */
export const transcriptionEngineSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("local"),
    model: z.enum(["whisperTiny", "whisperBase", "whisperSmall", "whisperMedium"]),
  }),
  z.object({
    kind: z.literal("cloud"),
    provider: providerIdSchema,
    model: z.string(),
  }),
]);

export const transcriptionSettingsSchema = z.object({
  engine: transcriptionEngineSchema,
  model: z.enum(["whisperTiny", "whisperBase", "whisperSmall", "whisperMedium"]),
  language: z.enum(["indonesian", "english"]),
  captureMicrophone: z.boolean(),
  captureComputerAudio: z.boolean(),
});

export const captionSettingsSchema = z.object({
  fontSize: z.number().int().positive(),
  maxLines: z.number().int().positive(),
  backgroundOpacity: z.number().int().min(0).max(100),
  showSourceLabels: z.boolean(),
  clickThrough: z.boolean(),
});

export const shortcutSettingsSchema = z.object({
  stopSession: z.string().nullable(),
  togglePause: z.string().nullable(),
  toggleCaptions: z.string().nullable(),
  askAssistant: z.string().nullable(),
});

/** Mirrors `ProviderConfig`. Holds a credential reference, never a key. */
export const providerConfigSchema = z.object({
  id: providerIdSchema,
  enabled: z.boolean(),
  model: z.string(),
  endpoint: z.string().nullable(),
  credentialRef: z.string(),
});

export const assistantContextSchema = z.enum([
  "general",
  "meeting",
  "interview",
  "quiz",
  "lecture",
]);

export const assistantSettingsSchema = z.object({
  enabled: z.boolean(),
  provider: providerIdSchema,
  model: z.string(),
  context: assistantContextSchema,
  notes: z.string(),
  answerOwnQuestions: z.boolean(),
});

/** Mirrors `UpdateChannel`. */
export const updateChannelSchema = z.enum(["stable", "beta"]);

/** Mirrors `UpdateSettings`. */
export const updateSettingsSchema = z.object({
  channel: updateChannelSchema,
  checkOnStartup: z.boolean(),
});

/** Mirrors `ReleaseInfo`. */
export const releaseInfoSchema = z.object({
  version: z.string(),
  tag: z.string(),
  notes: z.string(),
  url: z.string(),
  prerelease: z.boolean(),
  publishedAt: z.string().nullable(),
  manifestUrl: z.string().nullable(),
});

/** Carried on `audis://update/progress` while a new version downloads. */
export const updateProgressEventSchema = z.object({
  downloaded: z.number().nonnegative(),
  total: z.number().nonnegative().nullable(),
});

/** Mirrors `UpdateCheck`, also carried on `audis://update/status`. */
export const updateCheckSchema = z.object({
  currentVersion: z.string(),
  update: releaseInfoSchema.nullable(),
  channel: updateChannelSchema,
});
export type UpdateChannel = z.infer<typeof updateChannelSchema>;
export type UpdateSettings = z.infer<typeof updateSettingsSchema>;
export type ReleaseInfo = z.infer<typeof releaseInfoSchema>;
export type UpdateCheck = z.infer<typeof updateCheckSchema>;

/** Carried on `audis://assistant/status`. */
export const assistantStatusEventSchema = z.object({ thinking: z.boolean() });

/** Carried on `audis://assistant/response`. */
export const assistantResponseEventSchema = z.object({
  id: z.string(),
  question: z.string(),
  answer: z.string(),
});
export type AssistantResponseEvent = z.infer<typeof assistantResponseEventSchema>;

/** Every field of Rust's `Settings` must appear here. */
export const settingsSchema = z.object({
  version: z.number().int(),
  general: generalSettingsSchema,
  audio: audioSettingsSchema,
  transcription: transcriptionSettingsSchema,
  captions: captionSettingsSchema,
  shortcuts: shortcutSettingsSchema,
  assistant: assistantSettingsSchema,
  updates: updateSettingsSchema,
  providers: z.array(providerConfigSchema),
});

export type AssistantContext = z.infer<typeof assistantContextSchema>;
export type AssistantSettings = z.infer<typeof assistantSettingsSchema>;

/** Mirrors `DataCategory`. */
export const dataCategorySchema = z.enum([
  "database",
  "sessions",
  "recordings",
  "models",
  "cache",
  "logs",
  "updates",
  "exports",
  "temp",
  "other",
]);

/** Mirrors `DataFile`. */
export const dataFileSchema = z.object({
  path: z.string().min(1),
  relativePath: z.string(),
  name: z.string().min(1),
  sizeBytes: z.number().int().nonnegative(),
  modified: z.string().nullable(),
  category: dataCategorySchema,
});

/** Mirrors `DataCategoryGroup`. */
export const dataCategoryGroupSchema = z.object({
  category: dataCategorySchema,
  label: z.string().min(1),
  path: z.string().min(1),
  files: z.array(dataFileSchema),
  totalBytes: z.number().int().nonnegative(),
});

/** Mirrors `DataFileListing`. */
export const dataFileListingSchema = z.object({
  root: z.string().min(1),
  groups: z.array(dataCategoryGroupSchema),
  totalBytes: z.number().int().nonnegative(),
  totalFiles: z.number().int().nonnegative(),
});

/** Mirrors `Diagnostics`. */
export const diagnosticsSchema = z.object({
  appVersion: z.string().min(1),
  os: z.string(),
  arch: z.string(),
  webviewVersion: z.string().nullable(),
  tauriVersion: z.string(),
  dataDir: z.string().min(1),
  logsDir: z.string().min(1),
  storageBytes: z.number().int().nonnegative(),
  fileCount: z.number().int().nonnegative(),
});

/** Mirrors `AudioDevice` in audis-audio. */
export const deviceKindSchema = z.enum(["input", "output"]);

export const audioDeviceSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  kind: deviceKindSchema,
  isDefault: z.boolean(),
  sampleRate: z.number().int().positive(),
  channels: z.number().int().nonnegative(),
});

export const audioDevicesSchema = z.object({
  inputs: z.array(audioDeviceSchema),
  outputs: z.array(audioDeviceSchema),
});

/** Mirrors `StreamStatus`. */
export const streamStatusSchema = z.object({
  deviceName: z.string(),
  sampleRate: z.number().int().nonnegative(),
  channels: z.number().int().nonnegative(),
});

/** Mirrors `AudioTestStatus`. */
export const audioTestStatusSchema = z.object({
  microphone: streamStatusSchema.nullable(),
  computerAudio: streamStatusSchema.nullable(),
  microphoneError: userFacingErrorSchema.nullable(),
  computerAudioError: userFacingErrorSchema.nullable(),
});

/** Mirrors `AudioLevelEvent`, carried on `audis://audio/level`. */
export const audioLevelEventSchema = z.object({
  source: audioSourceKindSchema,
  peak: z.number(),
  rms: z.number(),
  clipping: z.boolean(),
  silenceDurationMs: z.number().int().nonnegative(),
});

export type DeviceKind = z.infer<typeof deviceKindSchema>;
export type AudioDevice = z.infer<typeof audioDeviceSchema>;
export type AudioDevices = z.infer<typeof audioDevicesSchema>;
export type StreamStatus = z.infer<typeof streamStatusSchema>;
export type AudioTestStatus = z.infer<typeof audioTestStatusSchema>;
export type AudioLevelEvent = z.infer<typeof audioLevelEventSchema>;

/** Mirrors `Language`. Audis supports exactly these two. */
export const languageSchema = z.enum(["indonesian", "english"]);

/** Mirrors `ModelId`. */
export const modelIdSchema = z.enum([
  "whisperTiny",
  "whisperBase",
  "whisperSmall",
  "whisperMedium",
]);

/** Mirrors `ModelInfo`. */
export const modelInfoSchema = z.object({
  id: modelIdSchema,
  name: z.string().min(1),
  summary: z.string().min(1),
  sizeBytes: z.number().int().nonnegative(),
  fileName: z.string().min(1),
  url: z.string().min(1),
  requirement: z.string(),
  recommended: z.boolean(),
  keepsUpLive: z.boolean(),
});

/** Mirrors `InstalledModel`. */
export const installedModelSchema = z.object({
  info: modelInfoSchema,
  installed: z.boolean(),
  installedBytes: z.number().int().nonnegative().nullable(),
});

/** Mirrors `ModelDownloadProgress`, carried on `audis://model/progress`. */
export const modelProgressSchema = z.object({
  id: modelIdSchema,
  downloadedBytes: z.number().int().nonnegative(),
  totalBytes: z.number().int().nonnegative().nullable(),
  done: z.boolean(),
  error: z.string().nullable(),
});

/** Mirrors `ProviderId`. */
/** Mirrors `ProviderInfo`. */
/** Mirrors `SpeechSupport`. Absent when a provider cannot transcribe at all. */
export const speechSupportSchema = z.object({
  api: z.enum(["openAiTranscriptions", "geminiInline", "deepgramListen"]),
  baseUrl: z.string().nullable(),
  defaultModel: z.string(),
  models: z.array(z.string()),
  summary: z.string(),
});

/** Mirrors `ChatSupport`. Absent when a provider cannot converse at all. */
export const chatSupportSchema = z.object({
  api: z.enum(["openAiChat", "geminiGenerate", "anthropicMessages"]),
  baseUrl: z.string().nullable(),
  defaultModel: z.string(),
  models: z.array(z.string()),
});

export const providerInfoSchema = z.object({
  id: providerIdSchema,
  name: z.string().min(1),
  summary: z.string().min(1),
  consoleUrl: z.string(),
  freeTier: z.boolean(),
  defaultModel: z.string(),
  models: z.array(z.string()),
  needsEndpoint: z.boolean(),
  speech: speechSupportSchema.nullable(),
  chat: chatSupportSchema.nullable(),
});

/** Mirrors `ProviderStatus`. */
export const providerStatusSchema = z.object({
  info: providerInfoSchema,
  hasKey: z.boolean(),
  enabled: z.boolean(),
  model: z.string(),
  endpoint: z.string().nullable(),
});

/** Mirrors `FeatureId`. */
export const featureIdSchema = z.enum([
  "liveCaption",
  "transcription",
  "meetingAssistant",
  "interviewPractice",
]);

/** Mirrors `FeatureStatus`. */
export const featureStatusSchema = z.enum(["ready", "needsSetup", "notBuilt"]);

/** Mirrors `Feature`. */
export const featureSchema = z.object({
  id: featureIdSchema,
  name: z.string().min(1),
  summary: z.string().min(1),
  details: z.array(z.string()),
  status: featureStatusSchema,
  blocker: z.string().nullable(),
  usesCloud: z.boolean(),
});

export type Language = z.infer<typeof languageSchema>;
export type ModelId = z.infer<typeof modelIdSchema>;
export type ModelInfo = z.infer<typeof modelInfoSchema>;
export type InstalledModel = z.infer<typeof installedModelSchema>;
export type ModelProgress = z.infer<typeof modelProgressSchema>;
export type ProviderId = z.infer<typeof providerIdSchema>;
export type ProviderInfo = z.infer<typeof providerInfoSchema>;
export type ProviderStatus = z.infer<typeof providerStatusSchema>;
export type FeatureId = z.infer<typeof featureIdSchema>;
export type FeatureStatus = z.infer<typeof featureStatusSchema>;
export type Feature = z.infer<typeof featureSchema>;

/** Mirrors `SessionState`. */
export const sessionStateSchema = z.enum([
  "idle",
  "starting",
  "listening",
  "paused",
  "stopping",
  "completed",
  "failed",
]);

/** Mirrors `SessionStatus`, carried on `audis://session/state`. */
export const sessionStatusSchema = z.object({
  id: z.string(),
  mode: featureIdSchema,
  state: sessionStateSchema,
  language: languageSchema,
  elapsedMs: z.number().int().nonnegative(),
  microphone: z.boolean(),
  computerAudio: z.boolean(),
  captionsVisible: z.boolean(),
  assistantEnabled: z.boolean(),
  error: z.string().nullable(),
});

/** Mirrors `TranscriptSegment`, carried on `audis://transcript/final`. */
export const transcriptSegmentSchema = z.object({
  id: z.string(),
  sessionId: z.string(),
  source: audioSourceKindSchema,
  speaker: z.string().nullable(),
  startMs: z.number().int(),
  endMs: z.number().int(),
  text: z.string(),
  language: languageSchema,
  confidence: z.number().nullable(),
  isFinal: z.boolean(),
  engine: z.string(),
});

/** Mirrors `AsrState`. */
export const asrStateSchema = z.enum([
  "starting",
  "listening",
  "recognising",
  "reconnecting",
  "stopped",
  "failed",
]);

/** Mirrors `AsrStatus`, carried on `audis://asr/status`. */
export const asrStatusSchema = z.object({
  source: audioSourceKindSchema,
  state: asrStateSchema,
  engine: z.string(),
  error: z.string().nullable(),
});

export type SessionState = z.infer<typeof sessionStateSchema>;
export type SessionStatus = z.infer<typeof sessionStatusSchema>;
export type TranscriptSegment = z.infer<typeof transcriptSegmentSchema>;
export type AsrState = z.infer<typeof asrStateSchema>;
export type AsrStatus = z.infer<typeof asrStatusSchema>;

/** Mirrors `SessionSummary`. */
export const sessionSummarySchema = z.object({
  id: z.string(),
  mode: featureIdSchema,
  language: languageSchema,
  startedAt: z.string(),
  endedAt: z.string().nullable(),
  segmentCount: z.number().int().nonnegative(),
  elapsedMs: z.number().int().nonnegative(),
  complete: z.boolean(),
});

export type SessionSummary = z.infer<typeof sessionSummarySchema>;
export type ExportFormat = "text" | "markdown" | "srt";

export type DiagnosticCode = z.infer<typeof diagnosticCodeSchema>;
export type UserFacingError = z.infer<typeof userFacingErrorSchema>;
export type AudioSourceKind = z.infer<typeof audioSourceKindSchema>;
export type AppInfo = z.infer<typeof appInfoSchema>;
export type DiagnosticWarning = z.infer<typeof diagnosticWarningSchema>;
export type ThemePreference = z.infer<typeof themePreferenceSchema>;
export type StartPage = z.infer<typeof startPageSchema>;
export type CloseBehavior = z.infer<typeof closeBehaviorSchema>;
export type GeneralSettings = z.infer<typeof generalSettingsSchema>;
export type TranscriptionEngine = z.infer<typeof transcriptionEngineSchema>;
export type SpeechSupport = z.infer<typeof speechSupportSchema>;
export type AudioSettings = z.infer<typeof audioSettingsSchema>;
export type TranscriptionSettings = z.infer<typeof transcriptionSettingsSchema>;
export type CaptionSettings = z.infer<typeof captionSettingsSchema>;
export type ShortcutSettings = z.infer<typeof shortcutSettingsSchema>;
export type ProviderConfig = z.infer<typeof providerConfigSchema>;
export type Settings = z.infer<typeof settingsSchema>;
export type DataCategory = z.infer<typeof dataCategorySchema>;
export type DataFile = z.infer<typeof dataFileSchema>;
export type DataCategoryGroup = z.infer<typeof dataCategoryGroupSchema>;
export type DataFileListing = z.infer<typeof dataFileListingSchema>;
export type Diagnostics = z.infer<typeof diagnosticsSchema>;
