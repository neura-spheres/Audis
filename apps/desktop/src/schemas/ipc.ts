import { z } from "zod";

/**
 * Runtime schemas for everything crossing the Rust/React boundary.
 *
 * TypeScript types are erased at runtime, so an annotation proves nothing about
 * what Rust actually sent. These schemas do. Without them a rename on the Rust
 * side surfaces as undefined deep inside a component instead of as an error at
 * the boundary.
 */

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

export const settingsSchema = z.object({
  version: z.number().int(),
  general: generalSettingsSchema,
});

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
export const providerIdSchema = z.enum([
  "gemini",
  "deepSeek",
  "groq",
  "anthropic",
  "openAiCompatible",
]);

/** Mirrors `ProviderInfo`. */
export const providerInfoSchema = z.object({
  id: providerIdSchema,
  name: z.string().min(1),
  summary: z.string().min(1),
  consoleUrl: z.string(),
  freeTier: z.boolean(),
  defaultModel: z.string(),
  models: z.array(z.string()),
  needsEndpoint: z.boolean(),
});

/**
 * Mirrors `ProviderStatus`.
 *
 * Note there is no key field, by design: Rust reports only whether one exists.
 */
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

export type DiagnosticCode = z.infer<typeof diagnosticCodeSchema>;
export type UserFacingError = z.infer<typeof userFacingErrorSchema>;
export type AudioSourceKind = z.infer<typeof audioSourceKindSchema>;
export type AppInfo = z.infer<typeof appInfoSchema>;
export type DiagnosticWarning = z.infer<typeof diagnosticWarningSchema>;
export type ThemePreference = z.infer<typeof themePreferenceSchema>;
export type StartPage = z.infer<typeof startPageSchema>;
export type CloseBehavior = z.infer<typeof closeBehaviorSchema>;
export type GeneralSettings = z.infer<typeof generalSettingsSchema>;
export type Settings = z.infer<typeof settingsSchema>;
export type DataCategory = z.infer<typeof dataCategorySchema>;
export type DataFile = z.infer<typeof dataFileSchema>;
export type DataCategoryGroup = z.infer<typeof dataCategoryGroupSchema>;
export type DataFileListing = z.infer<typeof dataFileListingSchema>;
export type Diagnostics = z.infer<typeof diagnosticsSchema>;
