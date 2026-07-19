import { useEffect, useRef } from "react";

import { useSession } from "@/hooks/useSession";
import { useSettings } from "@/hooks/useSettings";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { askAssistant, assistantSummarize } from "@/services/ipc";
import { transcriptSegmentSchema } from "@/schemas/ipc";
import {
  addQuestion,
  clearFeed,
  dropEntry,
  failAnswer,
  looksComplete,
  looksLikeQuestion,
  resolveAnswer,
} from "./feed";

/** Sentences kept before the question for local context. */
const LINES_BEFORE = 8;

/** Wait after a line that reads as a finished sentence. */
const SETTLE_COMPLETE_MS = 900;

/**
 * Wait after a line that stops mid-thought.
 *
 * A cloud engine cuts run-on speech at a few seconds, so the rest of a question
 * can be a whole chunk behind. Answering the half we have would answer the wrong
 * question, so an unfinished line buys the remainder more time to arrive.
 */
const SETTLE_CUT_MS = 3500;

/** Hard cap on how long to wait for trailing context after a question. */
const MAX_WAIT_MS = 8000;

/** Lines kept verbatim after the summary; only older lines get folded in. */
const RECENT_WINDOW = 10;

/** How many un-summarised older lines trigger a summary refresh. */
const SUMMARY_BATCH = 12;

interface Pending {
  /** Index of the question line in the transcript. */
  questionIndex: number;
  /** Feed entry showing "Thinking…" for this question. */
  feedId: string;
  /** Timer that fires once the question has settled. */
  timer: number;
  /** Latest moment we will still wait for trailing context. */
  deadline: number;
}

/**
 * Runs the assistant while a session is live and the assistant is on.
 *
 * Mounted once at the app shell so it keeps working whatever view is open. When
 * a line looks like a question it waits a short beat to gather the sentences
 * that follow — the rest of the question, a clarification — then answers with
 * the five lines before it, the question and its trailing context, plus a
 * running summary of everything earlier in the call. The summary is refreshed in
 * compact batches so the model can follow a long call without being sent the
 * whole transcript each time.
 */
export function useAssistantEngine() {
  const { settings } = useSettings();
  const { session } = useSession();

  const transcript = useRef<string[]>([]);
  const summary = useRef("");
  const summarizedCount = useRef(0);
  const summarizing = useRef(false);
  const busy = useRef(false);
  const pending = useRef<Pending | null>(null);

  const enabled = settings?.assistant.enabled ?? false;
  const answerOwn = settings?.assistant.answerOwnQuestions ?? false;
  const running = session !== null;

  // Reset when a session ends, so the next one starts clean.
  useEffect(() => {
    if (!running) {
      if (pending.current) window.clearTimeout(pending.current.timer);
      pending.current = null;
      transcript.current = [];
      summary.current = "";
      summarizedCount.current = 0;
      busy.current = false;
      clearFeed();
    }
  }, [running]);

  useEffect(() => {
    if (!enabled || !running) return;

    const maybeSummarize = () => {
      if (summarizing.current) return;
      const foldTo = transcript.current.length - RECENT_WINDOW;
      if (foldTo - summarizedCount.current < SUMMARY_BATCH) return;

      const batch = transcript.current.slice(summarizedCount.current, foldTo);
      summarizing.current = true;
      assistantSummarize(summary.current, batch)
        .then((next) => {
          summary.current = next;
          summarizedCount.current = foldTo;
        })
        .catch(() => undefined)
        .finally(() => {
          summarizing.current = false;
        });
    };

    const answerPending = () => {
      const current = pending.current;
      if (!current) return;
      if (busy.current) {
        // An earlier answer is still running; check back shortly.
        current.timer = window.setTimeout(answerPending, 250);
        return;
      }

      pending.current = null;
      busy.current = true;

      const from = Math.max(0, current.questionIndex - LINES_BEFORE);
      const context = transcript.current.slice(from);
      const question = transcript.current.slice(current.questionIndex).join(" ");
      const { feedId } = current;

      askAssistant(question, context, summary.current)
        .then((answer) => {
          if (answer.trim().length === 0) dropEntry(feedId);
          else resolveAnswer(feedId, answer);
        })
        .catch((error: unknown) => failAnswer(feedId, String(error)))
        .finally(() => {
          busy.current = false;
        });
    };

    const scheduleSettle = () => {
      const current = pending.current;
      if (!current) return;
      window.clearTimeout(current.timer);

      const last = transcript.current[transcript.current.length - 1] ?? "";
      const settle = looksComplete(last) ? SETTLE_COMPLETE_MS : SETTLE_CUT_MS;
      const wait = Math.min(settle, Math.max(0, current.deadline - Date.now()));
      current.timer = window.setTimeout(answerPending, wait);
    };

    const askNow = () => {
      if (busy.current || pending.current) return;
      const lastIndex = transcript.current.length - 1;
      if (lastIndex < 0) return;

      pending.current = {
        questionIndex: lastIndex,
        feedId: addQuestion(transcript.current[lastIndex] ?? ""),
        timer: 0,
        deadline: Date.now(),
      };
      answerPending();
    };

    const stopAsk = subscribe(AUDIS_EVENTS.assistantAsk, askNow);

    const stopFinal = subscribe(AUDIS_EVENTS.transcriptFinal, (payload) => {
      const parsed = transcriptSegmentSchema.safeParse(payload);
      if (!parsed.success) return;
      const segment = parsed.data;

      const speaker =
        segment.speaker ?? (segment.source === "microphone" ? "You" : "Computer Audio");
      transcript.current = [...transcript.current, `${speaker}: ${segment.text}`];

      maybeSummarize();

      // While a question is settling, every new line extends its trailing
      // context and pushes the answer back a little (up to the hard cap).
      if (pending.current) {
        scheduleSettle();
        return;
      }

      // The user's own questions are only answered if they asked for that; by
      // default the assistant answers what other people say (the interviewer,
      // the quiz master), not the user thinking aloud.
      const fromUser = segment.source === "microphone";
      if (fromUser && !answerOwn) return;
      if (!looksLikeQuestion(segment.text)) return;
      if (busy.current) return;

      pending.current = {
        questionIndex: transcript.current.length - 1,
        feedId: addQuestion(segment.text),
        timer: 0,
        deadline: Date.now() + MAX_WAIT_MS,
      };
      scheduleSettle();
    });

    return () => {
      stopAsk();
      stopFinal();
    };
  }, [enabled, running, answerOwn]);
}
