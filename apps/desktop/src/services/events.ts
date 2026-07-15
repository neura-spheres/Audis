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
} as const;

export type AudisEventName = (typeof AUDIS_EVENTS)[keyof typeof AUDIS_EVENTS];
