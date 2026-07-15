import { mockIPC } from "@tauri-apps/api/mocks";

import {
  AUDIS_APP_INFO_MOCK,
  AUDIS_DIAGNOSTICS_MOCK,
  AUDIS_FEATURES_MOCK,
  AUDIS_FILE_LISTING_MOCK,
  AUDIS_MODELS_MOCK,
  AUDIS_PROVIDERS_MOCK,
} from "@/test/fixtures";
import type { Settings } from "@/schemas/ipc";

/**
 * Fake IPC backend for browser development.
 *
 * Under `vite dev` in a plain browser there is no Rust core to answer invoke,
 * so the UI would render nothing but error states. No-op inside the real app,
 * where Tauri injects __TAURI_INTERNALS__, and tree-shaken out of production
 * builds by the DEV guard at the call site.
 */
export function installDevIpcMock(): void {
  if ("__TAURI_INTERNALS__" in window) return;

  // Held in memory so settings changes stick while developing.
  let settings: Settings = {
    version: 1,
    general: {
      theme: "system",
      startPage: "dashboard",
      closeBehavior: "minimizeToTray",
      showTrayIcon: true,
    },
  };

  mockIPC((command, args) => {
    switch (command) {
      case "get_app_info":
        return AUDIS_APP_INFO_MOCK;
      case "get_settings":
        return settings;
      case "update_settings":
        settings = (args as { settings: Settings }).settings;
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
      case "stop_audio_test":
        return null;
      case "list_features":
        return AUDIS_FEATURES_MOCK;
      case "list_models":
        return AUDIS_MODELS_MOCK;
      case "list_providers":
        return AUDIS_PROVIDERS_MOCK;
      case "is_model_downloading":
        return false;
      case "install_model":
      case "remove_model":
      case "cancel_model_download":
      case "set_provider_key":
      case "delete_provider_key":
      case "update_provider":
        return null;
      case "open_data_file":
      case "reveal_data_file":
      case "close_main_window":
        return null;
      default:
        throw new Error(`dev mock has no handler for command "${command}"`);
    }
  });
}
