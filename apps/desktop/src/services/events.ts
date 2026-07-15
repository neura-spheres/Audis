import { listen } from "@tauri-apps/api/event";

/**
 * Audis event channel names.
 *
 * Mirrors `audis_common::ipc::events`. The Rust side owns the canonical list
 * and events.test.ts fails if these drift apart.
 */
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
  diagnosticWarning: "audis://diagnostic/warning",
  modelProgress: "audis://model/progress",
  settingsChanged: "audis://settings/changed",
} as const;

export type AudisEventName = (typeof AUDIS_EVENTS)[keyof typeof AUDIS_EVENTS];

/**
 * Subscribe to an Audis event for the lifetime of an effect.
 *
 * `listen` is async, so a component that unmounts before it resolves would
 * otherwise leak a live subscription. It can also reject — in a window whose
 * capability does not grant `core:event:listen`, for instance — and an
 * unhandled rejection there would surface as a crash far from the cause rather
 * than as a dead panel.
 *
 * Returns the cleanup function an effect should return.
 *
 * @example
 * useEffect(() => subscribe(AUDIS_EVENTS.sessionState, handle), [handle]);
 */
export function subscribe<TPayload>(
  event: AudisEventName,
  handler: (payload: TPayload) => void,
): () => void {
  let cancelled = false;
  let stop: (() => void) | undefined;

  void listen<TPayload>(event, (received) => handler(received.payload))
    .then((unlisten) => {
      // Unmounted while listen was still resolving: drop it immediately rather
      // than leaving a handler pointed at a dead component.
      if (cancelled) unlisten();
      else stop = unlisten;
    })
    .catch((cause: unknown) => {
      console.error(`Audis could not subscribe to ${event}`, cause);
    });

  return () => {
    cancelled = true;
    if (!stop) return;

    // `UnlistenFn` is typed as returning void, but it does async work and can
    // reject — during window teardown the event plugin may already be gone.
    // Nothing useful remains to do at that point, and letting the rejection
    // escape would crash a component that is already unmounting.
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
