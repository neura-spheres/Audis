import { useEffect, useRef } from "react";

import { useSession } from "@/hooks/useSession";
import { useSettings } from "@/hooks/useSettings";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { askAssistant } from "@/services/ipc";
import { transcriptSegmentSchema } from "@/schemas/ipc";
import {
  addQuestion,
  clearFeed,
  dropEntry,
  failAnswer,
  looksLikeQuestion,
  resolveAnswer,
} from "./feed";

const CONTEXT_LINES = 12;

/**
 * Runs the assistant while a session is live and the assistant is on.
 *
 * Mounted once at the app shell so it keeps working whatever view is open. It
 * watches finished transcript lines, and when one looks like a question it asks
 * the model for an answer. One request at a time, so a burst of questions does
 * not fan out into a burst of billed calls.
 */
export function useAssistantEngine() {
  const { settings } = useSettings();
  const { session } = useSession();

  const transcript = useRef<string[]>([]);
  const busy = useRef(false);
  const enabled = settings?.assistant.enabled ?? false;
  const answerOwn = settings?.assistant.answerOwnQuestions ?? false;
  const running = session !== null;

  // Reset when a session ends, so the next one starts clean.
  useEffect(() => {
    if (!running) {
      transcript.current = [];
      clearFeed();
    }
  }, [running]);

  useEffect(() => {
    if (!enabled || !running) return;

    return subscribe(AUDIS_EVENTS.transcriptFinal, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (!parsed.success) return;
      const segment = parsed.data;

      const speaker =
        segment.speaker ?? (segment.source === "microphone" ? "You" : "Computer Audio");
      transcript.current = [...transcript.current, `${speaker}: ${segment.text}`].slice(-40);

      // The user's own questions are only answered if they asked for that; by
      // default the assistant answers what other people say (the interviewer,
      // the quiz master), not the user thinking aloud.
      const fromUser = segment.source === "microphone";
      if (fromUser && !answerOwn) return;
      if (!looksLikeQuestion(segment.text)) return;
      if (busy.current) return;

      busy.current = true;
      const id = addQuestion(segment.text);
      const context = transcript.current.slice(-CONTEXT_LINES);

      askAssistant(segment.text, context)
        .then((answer) => {
          if (answer.trim().length === 0) dropEntry(id);
          else resolveAnswer(id, answer);
        })
        .catch((error: unknown) => failAnswer(id, String(error)))
        .finally(() => {
          busy.current = false;
        });
    });
  }, [enabled, running, answerOwn]);
}
