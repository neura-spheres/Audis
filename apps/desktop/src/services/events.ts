import { listen } from "@tauri-apps/api/event";

/** Audis event channel names. */
export const AUDIS_EVENTS = {
  sessionState: "audis://session/state",
  audioLevel: "audis://audio/level",
  audioDeviceChange: "audis://audio/device-change",
  transcriptPartial: "audis://transcript/partial",
  transcriptFinal: "audis://transcript/final",
  transcriptRevision: "audis://transcript/revision",
  asrStatus: "audis://asr/status",
  speakerUpdate: "audis://speaker/update",
  assistantStatus: "audis://assistant/status",
  assistantResponse: "audis://assistant/response",
  meetingUpdate: "audis://meeting/update",
  updateStatus: "audis://update/status",
  updateProgress: "audis://update/progress",
  diagnosticWarning: "audis://diagnostic/warning",
  modelProgress: "audis://model/progress",
  settingsChanged: "audis://settings/changed",
} as const;

export type AudisEventName = (typeof AUDIS_EVENTS)[keyof typeof AUDIS_EVENTS];

/** Subscribe to an Audis event for the lifetime of an effect. */
export function subscribe<TPayload>(
  event: AudisEventName,
  handler: (payload: TPayload) => void,
): () => void {
  let cancelled = false;
  let stop: (() => void) | undefined;

  void listen<TPayload>(event, (received) => handler(received.payload))
    .then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    })
    .catch((cause: unknown) => {
      console.error(`Audis could not subscribe to ${event}`, cause);
    });

  return () => {
    cancelled = true;
    if (!stop) return;

    try {
      const pending = stop() as unknown;
      if (pending instanceof Promise) {
        pending.catch((cause: unknown) => {
          console.debug(`Audis could not unsubscribe from ${event}`, cause);
        });
      }
    } catch (cause) {
      console.debug(`Audis could not unsubscribe from ${event}`, cause);
    }
  };
}
