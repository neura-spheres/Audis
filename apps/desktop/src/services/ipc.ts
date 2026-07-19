import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

import {
  appInfoSchema,
  audioDevicesSchema,
  audioTestStatusSchema,
  dataFileListingSchema,
  diagnosticsSchema,
  featureSchema,
  installedModelSchema,
  providerStatusSchema,
  sessionStatusSchema,
  sessionSummarySchema,
  updateCheckSchema,
  transcriptSegmentSchema,
  settingsSchema,
  userFacingErrorSchema,
  type AppInfo,
  type AudioDevices,
  type AudioTestStatus,
  type DataFileListing,
  type Diagnostics,
  type Feature,
  type InstalledModel,
  type ModelId,
  type ProviderId,
  type FeatureId,
  type ProviderStatus,
  type ExportFormat,
  type SessionStatus,
  type SessionSummary,
  type UpdateCheck,
  type TranscriptSegment,
  type Settings,
  type UserFacingError,
} from "@/schemas/ipc";

/** The typed IPC boundary. */

/** Thrown by {@link callCommand} when a command fails. */
export class AudisIpcError extends Error {
  readonly userFacing: UserFacingError;

  constructor(userFacing: UserFacingError) {
    super(userFacing.title);
    this.name = "AudisIpcError";
    this.userFacing = userFacing;
  }
}

/** Fallback for a rejection that is not shaped like a UserFacingError: a panic, */
function unrecognisedFailure(cause: unknown): UserFacingError {
  return {
    title: "Audis hit an unexpected problem",
    explanation:
      "Something went wrong inside Audis. Your saved sessions and recordings were not affected.",
    dataPreserved: true,
    suggestedAction:
      "Try again. If this keeps happening, restart Audis and export a diagnostic bundle.",
    technicalDetails: cause instanceof Error ? cause.message : String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}

/** Invoke a Rust command and validate its result. */
export async function callCommand<TSchema extends z.ZodType>(
  command: string,
  schema: TSchema,
  args?: Record<string, unknown>,
): Promise<z.infer<TSchema>> {
  let raw: unknown;
  try {
    raw = await invoke(command, args);
  } catch (rejection) {
    const parsed = userFacingErrorSchema.safeParse(rejection);
    throw new AudisIpcError(parsed.success ? parsed.data : unrecognisedFailure(rejection));
  }

  const result = schema.safeParse(raw);
  if (!result.success) {
    throw new AudisIpcError({
      ...unrecognisedFailure(new Error(`"${command}" returned an unexpected shape`)),
      technicalDetails: result.error.message,
      diagnosticCode: "DATA_SERIALIZATION",
    });
  }

  return result.data;
}

/** The result of a command that returns nothing. */
const voidResult = z.union([z.null(), z.undefined()]).transform(() => undefined as void);

/** Identity and build information for the About page and diagnostics. */
export function getAppInfo(): Promise<AppInfo> {
  return callCommand("get_app_info", appInfoSchema);
}

/** Current user settings. */
export function getSettings(): Promise<Settings> {
  return callCommand("get_settings", settingsSchema);
}

/** Persist settings and return what was actually saved. */
export function updateSettings(settings: Settings): Promise<Settings> {
  return callCommand("update_settings", settingsSchema, { settings });
}

/** Every file Audis has written, grouped by category. */
export function listDataFiles(): Promise<DataFileListing> {
  return callCommand("list_data_files", dataFileListingSchema);
}

/** Open a file with its default application. */
export function openDataFile(path: string): Promise<void> {
  return callCommand("open_data_file", voidResult, { path });
}

/** Show a file in File Explorer with it selected. */
export function revealDataFile(path: string): Promise<void> {
  return callCommand("reveal_data_file", voidResult, { path });
}

/** Environment information for the diagnostics page. */
export function getDiagnostics(): Promise<Diagnostics> {
  return callCommand("get_diagnostics", diagnosticsSchema);
}

/** Look for a newer Audis on the user's chosen release channel. */
export function checkForUpdates(): Promise<UpdateCheck> {
  return callCommand("check_for_updates", updateCheckSchema);
}

/** Download, verify and install the newest release, then restart into it. */
export function installUpdate(): Promise<void> {
  return callCommand("install_update", voidResult);
}

/** Open a release page in the browser. Refused unless it is an Audis release. */
export function openReleasePage(url: string): Promise<void> {
  return callCommand("open_release_page", voidResult, { url });
}

/** Every microphone and output endpoint on this machine. */
export function listAudioDevices(): Promise<AudioDevices> {
  return callCommand("list_audio_devices", audioDevicesSchema);
}

/** Open both captures and start streaming levels on `audis://audio/level`. */
export function startAudioTest(
  microphoneId: string | null,
  computerAudioId: string | null,
): Promise<AudioTestStatus> {
  return callCommand("start_audio_test", audioTestStatusSchema, {
    microphoneId,
    computerAudioId,
  });
}

/** Stop the audio test and release both devices. */
export function stopAudioTest(): Promise<void> {
  return callCommand("stop_audio_test", voidResult);
}

/** Every feature, with whether it can actually be started right now. */
export function listFeatures(): Promise<Feature[]> {
  return callCommand("list_features", z.array(featureSchema));
}

/** Every speech model, with whether it is installed. */
export function listModels(): Promise<InstalledModel[]> {
  return callCommand("list_models", z.array(installedModelSchema));
}

/** Download a model. Progress arrives on `audis://model/progress`. */
export function installModel(id: ModelId): Promise<void> {
  return callCommand("install_model", voidResult, { id });
}

/** Stop the running download. */
export function cancelModelDownload(): Promise<void> {
  return callCommand("cancel_model_download", voidResult);
}

/** Whether a download is running, so the UI can restore state after navigation. */
export function isModelDownloading(): Promise<boolean> {
  return callCommand("is_model_downloading", z.boolean());
}

/** Delete an installed model. */
export function removeModel(id: ModelId): Promise<void> {
  return callCommand("remove_model", voidResult, { id });
}

/** Every AI provider and whether a key is saved. Never returns the key itself. */
export function listProviders(): Promise<ProviderStatus[]> {
  return callCommand("list_providers", z.array(providerStatusSchema));
}

/** Save an API key to the OS credential store. */
export function setProviderKey(id: ProviderId, key: string): Promise<void> {
  return callCommand("set_provider_key", voidResult, { id, key });
}

/** Delete a provider's API key. */
export function deleteProviderKey(id: ProviderId): Promise<void> {
  return callCommand("delete_provider_key", voidResult, { id });
}

/** The provider's current models for a purpose, fetched from its API. */
export function listProviderModels(id: ProviderId, purpose: "speech" | "chat"): Promise<string[]> {
  return callCommand("list_provider_models", z.array(z.string()), { provider: id, purpose });
}

/** Ask the assistant to answer a question. Empty means "not a real question". */
export function askAssistant(
  question: string,
  transcript: string[],
  summary: string,
): Promise<string> {
  return callCommand("ask_assistant", z.string(), { question, transcript, summary });
}

/** Fold new transcript lines into the running session summary. */
export function assistantSummarize(previous: string, lines: string[]): Promise<string> {
  return callCommand("assistant_summarize", z.string(), { previous, lines });
}

/** Turn the assistant on or off, updating settings and the answer panel. */
export function setAssistantEnabled(enabled: boolean): Promise<void> {
  return callCommand("set_assistant_enabled", voidResult, { enabled });
}

/** Enable or configure a provider. */
export function updateProvider(
  id: ProviderId,
  enabled: boolean,
  model: string,
  endpoint: string | null,
): Promise<void> {
  return callCommand("update_provider", voidResult, { id, enabled, model, endpoint });
}

/** Start a live session for `feature`. */
export function startSession(feature: FeatureId): Promise<SessionStatus> {
  return callCommand("start_session", sessionStatusSchema, { feature });
}

/** Stop the running session and release every device. */
export function stopSession(): Promise<SessionStatus> {
  return callCommand("stop_session", sessionStatusSchema);
}

/** Pause or resume. Devices stay open while paused, so resuming is instant. */
export function setSessionPaused(paused: boolean): Promise<SessionStatus> {
  return callCommand("set_session_paused", sessionStatusSchema, { paused });
}

/** The running session, if there is one. */
export function getSessionStatus(): Promise<SessionStatus | null> {
  return callCommand("get_session_status", sessionStatusSchema.nullable());
}

/** Hide a floating overlay without ending the session. */
export function hideOverlay(overlay: "captions" | "controller" | "assistant"): Promise<void> {
  return callCommand("hide_overlay", voidResult, { overlay });
}

/** Recentre the captions along the bottom of the screen. */
export function resetCaptionPosition(): Promise<void> {
  return callCommand("reset_caption_position", voidResult);
}

/** Turn caption click-through on or off and apply it immediately. */
export function setCaptionClickThrough(clickThrough: boolean): Promise<void> {
  return callCommand("set_caption_click_through", voidResult, { clickThrough });
}

/** Bring the main window to the front, restoring session overlays with it. */
export function openMainWindow(): Promise<void> {
  return callCommand("open_main_window", voidResult);
}

/** Every saved session, newest first. */
export function listSessions(): Promise<SessionSummary[]> {
  return callCommand("list_sessions", z.array(sessionSummarySchema));
}

/** Every segment of one saved session. */
export function getSessionTranscript(id: string): Promise<TranscriptSegment[]> {
  return callCommand("get_session_transcript", z.array(transcriptSegmentSchema), { id });
}

/** Delete a saved session. */
export function deleteSession(id: string): Promise<void> {
  return callCommand("delete_session", voidResult, { id });
}

/** Export a session's transcript and reveal the file. Returns its path. */
export function exportSession(id: string, format: ExportFormat): Promise<string> {
  return callCommand("export_session", z.string(), { id, format });
}

export function generateSessionReport(id: string): Promise<string> {
  return callCommand("generate_session_report", z.string(), { id });
}
