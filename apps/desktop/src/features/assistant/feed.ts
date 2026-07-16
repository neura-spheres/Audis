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

const QUESTION_STARTERS = new Set([
  "what",
  "why",
  "how",
  "who",
  "when",
  "where",
  "which",
  "whose",
  "is",
  "are",
  "was",
  "were",
  "do",
  "does",
  "did",
  "can",
  "could",
  "would",
  "should",
  "will",
  "have",
  "has",
  "apa",
  "apakah",
  "kenapa",
  "mengapa",
  "bagaimana",
  "gimana",
  "siapa",
  "kapan",
  "dimana",
  "berapa",
  "kah",
]);

/** A cheap first pass: does this line look like a question worth answering? */
export function looksLikeQuestion(text: string): boolean {
  const trimmed = text.trim().toLowerCase();
  if (trimmed.length < 3) return false;
  if (trimmed.endsWith("?")) return true;
  const first = trimmed.split(/\s+/)[0]?.replace(/[^a-z]/g, "") ?? "";
  return QUESTION_STARTERS.has(first);
}
