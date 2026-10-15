import { useEffect, useState } from "react";

import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { transcriptSegmentSchema, type SessionStatus, type TranscriptSegment } from "@/schemas/ipc";

/**
 * The session clock, ticking.
 *
 * The backend reports elapsed time only when the state changes, so between
 * reports the clock runs forward from the last one. While paused it holds,
 * which matches the backend: paused time is not captured time.
 */
export function useElapsed(session: SessionStatus | null): number {
  const [now, setNow] = useState(Date.now());
  const running = session?.state === "listening";

  useEffect(() => {
    if (!running) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 500);
    return () => clearInterval(timer);
  }, [running]);

  if (!session) return 0;
  const at = anchorFor(session);
  return session.elapsedMs + (running ? Math.max(0, now - at) : 0);
}

/**
 * When each reported state was first seen. Shared, so the clock on Home and
 * the one in the sidebar count from the same moment and never disagree by a
 * second because one heard about the session a beat later.
 */
const anchors = new Map<string, number>();

function anchorFor(session: SessionStatus): number {
  const key = `${session.id}:${session.state}:${session.elapsedMs}`;
  let at = anchors.get(key);
  if (at === undefined) {
    at = Date.now();
    anchors.set(key, at);
    if (anchors.size > 32) anchors.delete(anchors.keys().next().value!);
  }
  return at;
}

/** How many finished lines the live view keeps. */
const KEEP = 4;

export interface LiveTranscript {
  /** The most recent finished lines, oldest first. */
  lines: TranscriptSegment[];
  /** The sentence being spoken right now, if any. */
  partial: TranscriptSegment | undefined;
}

/**
 * The last few lines of the running session, as they arrive.
 *
 * A peek, not the transcript: the full one is saved and opens in Library.
 */
export function useLiveTranscript(sessionId: string | undefined): LiveTranscript {
  const [lines, setLines] = useState<TranscriptSegment[]>([]);
  const [partial, setPartial] = useState<TranscriptSegment>();

  useEffect(() => {
    setLines([]);
    setPartial(undefined);
    if (!sessionId) return;

    const parse = (payload: unknown) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      return parsed.success && parsed.data.sessionId === sessionId ? parsed.data : undefined;
    };

    const stopFinal = subscribe(AUDIS_EVENTS.transcriptFinal, (payload) => {
      const segment = parse(payload);
      if (!segment) return;
      setLines((current) => [...current, segment].slice(-KEEP));
      // The sentence that was in progress for this source is now this line.
      setPartial((current) => (current?.source === segment.source ? undefined : current));
    });

    const stopPartial = subscribe(AUDIS_EVENTS.transcriptPartial, (payload) => {
      const segment = parse(payload);
      if (segment) setPartial(segment);
    });

    return () => {
      stopFinal();
      stopPartial();
    };
  }, [sessionId]);

  return { lines, partial };
}
