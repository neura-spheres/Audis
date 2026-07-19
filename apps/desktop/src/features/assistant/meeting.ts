import { useSyncExternalStore } from "react";

import type { MeetingUpdate } from "@/schemas/ipc";

let current: MeetingUpdate | null = null;
const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) listener();
}

export function setMeeting(update: MeetingUpdate) {
  current = update;
  emit();
}

export function clearMeeting() {
  if (current === null) return;
  current = null;
  emit();
}

export function useMeeting(): MeetingUpdate | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => current,
  );
}
