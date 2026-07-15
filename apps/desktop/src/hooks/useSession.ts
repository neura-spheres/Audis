import { useCallback, useEffect, useState } from "react";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import {
  getSessionStatus,
  setSessionPaused,
  startSession,
  stopSession,
  AudisIpcError,
} from "@/services/ipc";
import {
  sessionStatusSchema,
  type FeatureId,
  type SessionStatus,
  type UserFacingError,
} from "@/schemas/ipc";

/**
 * The running session.
 *
 * Rust owns the truth and announces it on `audis://session/state`; this hook
 * mirrors it. Nothing here decides what state the session is in, so the window
 * cannot disagree with the audio device that is actually open.
 */
export function useSession() {
  const [session, setSession] = useState<SessionStatus | null>(null);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<UserFacingError>();

  // A session outlives any one view, and the window can be reopened from the
  // tray, so the current state is asked for rather than assumed to be idle.
  useEffect(() => {
    let active = true;
    getSessionStatus()
      .then((status) => {
        if (active) setSession(status);
      })
      .catch(() => undefined);

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    return subscribe(AUDIS_EVENTS.sessionState, (payload) => {
      const parsed = sessionStatusSchema.safeParse(payload);
      if (!parsed.success) return;

      const status = parsed.data;
      // A finished session is not a running one. Keeping it would leave the UI
      // showing a stop button for something already stopped.
      setSession(status.state === "completed" || status.state === "failed" ? null : status);

      if (status.state === "failed" && status.error) {
        setError({
          title: "The session stopped unexpectedly",
          explanation: status.error,
          dataPreserved: true,
          suggestedAction: "Check your audio devices and try again.",
          technicalDetails: null,
          diagnosticCode: "UNEXPECTED",
        });
      }
    });
  }, []);

  const start = useCallback(async (feature: FeatureId) => {
    setError(undefined);
    setStarting(true);
    try {
      // Loading the model takes seconds on first start, so this resolves well
      // after the click.
      setSession(await startSession(feature));
    } catch (cause) {
      setError(toUserFacing(cause));
    } finally {
      setStarting(false);
    }
  }, []);

  const stop = useCallback(async () => {
    try {
      await stopSession();
      setSession(null);
    } catch (cause) {
      setError(toUserFacing(cause));
    }
  }, []);

  const setPaused = useCallback(async (paused: boolean) => {
    try {
      setSession(await setSessionPaused(paused));
    } catch (cause) {
      setError(toUserFacing(cause));
    }
  }, []);

  return {
    session,
    starting,
    error,
    start,
    stop,
    setPaused,
    dismissError: () => setError(undefined),
  };
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "The session could not be started",
    explanation: "Something went wrong. Nothing was recorded.",
    dataPreserved: true,
    suggestedAction: "Try again.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
