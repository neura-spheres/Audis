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
  type ProviderStatus,
  type Settings,
  type UserFacingError,
} from "@/schemas/ipc";

/**
 * The typed IPC boundary.
 *
 * Everything arriving from Rust is parsed with a schema before React sees it,
 * and every error leaving this file is a UserFacingError, so components never
 * have to interpret a raw Tauri rejection.
 */

/** Thrown by {@link callCommand} when a command fails. */
export class AudisIpcError extends Error {
  readonly userFacing: UserFacingError;

  constructor(userFacing: UserFacingError) {
    super(userFacing.title);
    this.name = "AudisIpcError";
    this.userFacing = userFacing;
  }
}

/**
 * Fallback for a rejection that is not shaped like a UserFacingError: a panic,
 * or a failure before Rust could build a proper error. We still owe the user a
 * coherent message.
 */
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

/**
 * Invoke a Rust command and validate its result.
 *
 * @throws {AudisIpcError} if the command fails or returns an unexpected shape.
 */
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
    // The two halves of the app disagree about a shape. Fail loudly here rather
    // than letting undefined propagate into a component.
    throw new AudisIpcError({
      ...unrecognisedFailure(new Error(`"${command}" returned an unexpected shape`)),
      technicalDetails: result.error.message,
      diagnosticCode: "DATA_SERIALIZATION",
    });
  }

  return result.data;
}

/**
 * The result of a command that returns nothing.
 *
 * Tauri serialises Rust's `()` as JSON `null`, not `undefined`. A schema that
 * only accepts `undefined` therefore rejects it, and the command appears to
 * fail after having actually succeeded: the key gets saved, the file gets
 * opened, and the UI still shows an error. Accept both and normalise.
 */
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

/** Every microphone and output endpoint on this machine. */
export function listAudioDevices(): Promise<AudioDevices> {
  return callCommand("list_audio_devices", audioDevicesSchema);
}

/**
 * Open both captures and start streaming levels on `audis://audio/level`.
 *
 * Passing `null` for a device means "use the Windows default".
 */
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

/**
 * Download a model. Progress arrives on `audis://model/progress`.
 *
 * Resolves only when the download finishes, which can be minutes.
 */
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

/**
 * Save an API key to the OS credential store.
 *
 * The key leaves the frontend here and never comes back: there is deliberately
 * no command to read one.
 */
export function setProviderKey(id: ProviderId, key: string): Promise<void> {
  return callCommand("set_provider_key", voidResult, { id, key });
}

/** Delete a provider's API key. */
export function deleteProviderKey(id: ProviderId): Promise<void> {
  return callCommand("delete_provider_key", voidResult, { id });
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
