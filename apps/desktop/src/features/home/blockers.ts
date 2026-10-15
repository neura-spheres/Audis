import type { Navigate } from "@/app/navigation";

/** Which page, and which part of it, resolves a blocker. */
export function fixFor(blocker: string | null): Parameters<Navigate> {
  return needsModel(blocker) ? ["speech", "models"] : ["providers"];
}

/** A blocker is either a missing speech model or a missing provider key. */
function needsModel(blocker: string | null): boolean {
  return Boolean(blocker && /model/i.test(blocker));
}

/**
 * How Home words a blocker. The backend's sentence names the page that fixes
 * it, and Home links to that page directly, so it says what is missing instead.
 */
export function describeBlocker(blocker: string | null): {
  title: string;
  detail: string;
  action: string;
  short: string;
} {
  return needsModel(blocker)
    ? {
        title: "Install a speech model",
        detail: "Whisper Base is free, runs on this PC, and downloads in about a minute.",
        action: "Choose model",
        short: "Needs a speech model first.",
      }
    : {
        title: "Connect an AI provider",
        detail: "Gemini and Groq have free tiers. Paste a key and it is ready.",
        action: "Add key",
        short: "Needs an AI provider key first.",
      };
}
