import { useEffect, useState } from "react";

import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { diagnosticWarningSchema } from "@/schemas/ipc";

/**
 * Says so when speech recognition is failing.
 *
 * A provider that is out of credit, rate limited, or holding a rejected key
 * refuses every chunk, and the only symptom is captions that never arrive. The
 * backend already knows exactly why; this is where the user finds out.
 */
export function AsrProblemBanner() {
  const [problem, setProblem] = useState<string>();

  useEffect(() => {
    const stopWarning = subscribe(AUDIS_EVENTS.diagnosticWarning, (payload) => {
      const parsed = diagnosticWarningSchema.safeParse(payload);
      if (parsed.success && parsed.data.kind.startsWith("asr.")) {
        setProblem(parsed.data.message);
      }
    });

    // Words arrived, so whatever was wrong is over.
    const stopTranscript = subscribe(AUDIS_EVENTS.transcriptFinal, () => setProblem(undefined));
    const stopSession = subscribe(AUDIS_EVENTS.sessionState, () => setProblem(undefined));

    return () => {
      stopWarning();
      stopTranscript();
      stopSession();
    };
  }, []);

  if (!problem) return null;

  return (
    <div
      role="status"
      className="flex shrink-0 items-start gap-2.5 px-6 py-2.5"
      style={{
        background: "color-mix(in srgb, var(--color-warning) 14%, transparent)",
        borderBottom: "0.5px solid var(--separator)",
      }}
    >
      <span aria-hidden style={{ color: "var(--color-warning)" }}>
        ⚠
      </span>
      <p data-selectable className="flex-1 text-footnote leading-[1.4]">
        {problem}
      </p>
      <button
        type="button"
        onClick={() => setProblem(undefined)}
        className="shrink-0 text-footnote"
        style={{ color: "var(--label-secondary)" }}
      >
        Dismiss
      </button>
    </div>
  );
}
