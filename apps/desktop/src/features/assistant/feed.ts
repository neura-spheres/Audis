import { useSyncExternalStore } from "react";

export interface AssistantEntry {
  id: string;
  question: string;
  answer: string;
  pending: boolean;
  error?: string;
}

let entries: AssistantEntry[] = [];
const listeners = new Set<() => void>();

function emit() {
  entries = [...entries];
  for (const listener of listeners) listener();
}

export function addQuestion(question: string): string {
  const id = crypto.randomUUID();
  entries.push({ id, question, answer: "", pending: true });
  emit();
  return id;
}

export function resolveAnswer(id: string, answer: string) {
  const entry = entries.find((item) => item.id === id);
  if (!entry) return;
  entry.answer = answer;
  entry.pending = false;
  emit();
}

export function failAnswer(id: string, error: string) {
  const entry = entries.find((item) => item.id === id);
  if (!entry) return;
  entry.pending = false;
  entry.error = error;
  emit();
}

/** Drop an entry whose answer turned out to be empty ("not a real question"). */
export function dropEntry(id: string) {
  entries = entries.filter((item) => item.id !== id);
  emit();
}

export function clearFeed() {
  entries = [];
  emit();
}

export function useAssistantFeed(): AssistantEntry[] {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => entries,
  );
}

/**
 * The 5W1H words, and their Indonesian equivalents.
 *
 * Matched anywhere in the line, not just at the front. Speech arrives in chunks
 * cut on pauses or a time limit, so a question rarely starts one cleanly: "so I
 * wanted to ask you, what is your" is a question whose first word is "so".
 */
const QUESTION_WORDS = new Set([
  "what",
  "where",
  "when",
  "who",
  "whom",
  "whose",
  "why",
  "how",
  "which",
  "apa",
  "apakah",
  "kenapa",
  "mengapa",
  "bagaimana",
  "gimana",
  "siapa",
  "kapan",
  "dimana",
  "mana",
  "berapa",
]);

/** Auxiliaries that open a yes/no question ("can you…", "did they…"). */
const QUESTION_OPENERS = new Set([
  "is",
  "are",
  "was",
  "were",
  "am",
  "do",
  "does",
  "did",
  "can",
  "could",
  "would",
  "should",
  "shall",
  "will",
  "have",
  "has",
  "had",
  "may",
  "might",
  "bisakah",
  "bolehkah",
  "sudahkah",
  "adakah",
]);

const REQUEST_VERBS = new Set([
  "tell",
  "explain",
  "describe",
  "define",
  "compare",
  "list",
  "summarize",
  "summarise",
  "walk",
  "give",
  "share",
  "elaborate",
  "clarify",
  "name",
  "outline",
  "suggest",
  "recommend",
  "jelaskan",
  "sebutkan",
  "ceritakan",
  "bandingkan",
  "uraikan",
  "terangkan",
]);

const REQUEST_PHRASES = [
  "tell me",
  "let me know",
  "walk me through",
  "i wanted to ask",
  "i want to ask",
  "my question is",
  "do you know",
  "any thoughts",
  "what about",
  "how about",
  "your thoughts on",
  "your take on",
];

const TAG_ENDINGS =
  /\b(right|correct|isn't it|aren't they|don't you|wouldn't you|betul|kan)\s*[?.!]*$/;

/**
 * A cheap first pass: could this line be a question worth answering?
 *
 * Deliberately generous. It is only a pre-filter to decide what is worth asking
 * the model about; the model itself is told to reply NONE when a line turns out
 * not to be a real question, so a false positive costs one request rather than a
 * wrong answer. Missing a real question, by contrast, is not recoverable.
 */
export function looksLikeQuestion(text: string): boolean {
  const trimmed = text.trim().toLowerCase();
  if (trimmed.length < 3) return false;
  if (trimmed.includes("?")) return true;
  if (TAG_ENDINGS.test(trimmed)) return true;
  if (REQUEST_PHRASES.some((phrase) => trimmed.includes(phrase))) return true;

  const words = trimmed.split(/[^\p{L}]+/u).filter(Boolean);
  if (words.length === 0) return false;

  if (words.some((word) => QUESTION_WORDS.has(word))) return true;
  if (words.some((word) => word.length > 3 && word.endsWith("kah"))) return true;

  const opener = words[0] ?? "";
  const secondary = words[1] ?? "";
  if (QUESTION_OPENERS.has(opener)) return true;
  return REQUEST_VERBS.has(opener) || REQUEST_VERBS.has(secondary);
}

/**
 * Does this line read as a finished sentence?
 *
 * A chunk cut by the time limit stops mid-thought and has no closing
 * punctuation, which is the signal that the rest of the question is still
 * coming and is worth waiting for.
 */
export function looksComplete(text: string): boolean {
  return /[.!?…]["')\]]*$/.test(text.trim());
}
