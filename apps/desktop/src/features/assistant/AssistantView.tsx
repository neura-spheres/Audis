import { useEffect, useRef, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button, Switch } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import { askAssistant, listProviderModels, listProviders } from "@/services/ipc";
import type { AssistantContext, ProviderId, ProviderStatus } from "@/schemas/ipc";
import { addQuestion, dropEntry, failAnswer, resolveAnswer, useAssistantFeed } from "./feed";

const CONTEXTS: { id: AssistantContext; label: string; hint: string }[] = [
  { id: "general", label: "General", hint: "A normal conversation." },
  { id: "meeting", label: "Meeting", hint: "Concise, factual answers when questions come up." },
  { id: "interview", label: "Interview", hint: "Suggests answers for you, the candidate." },
  { id: "quiz", label: "Quiz", hint: "Gives the correct answer with a short reason." },
  { id: "lecture", label: "Lecture", hint: "Answers questions and clarifies concepts." },
];

export function AssistantView() {
  const { settings, error, update } = useSettings();
  const [providers, setProviders] = useState<ProviderStatus[]>();

  useEffect(() => {
    listProviders()
      .then(setProviders)
      .catch(() => undefined);
  }, []);

  if (error) return <ErrorNotice error={error} />;
  if (!settings) return null;

  const a = settings.assistant;
  const keyed = (providers ?? []).filter((p) => p.hasKey);
  const hasKeyForChosen = keyed.some((p) => p.info.id === a.provider);

  const set = (patch: Partial<typeof a>) =>
    update((current) => ({ ...current, assistant: { ...current.assistant, ...patch } }));

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-3">
        <div
          className="flex items-center justify-between gap-4 p-3"
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
          }}
        >
          <div className="flex min-w-0 flex-col gap-0.5">
            <span className="text-subheadline font-semibold">
              Answer questions during a session
            </span>
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              When on, Audis sends the recent transcript to your provider to answer questions as
              they come up. Off means nothing is sent.
            </span>
          </div>
          <Switch
            label="Enable the assistant"
            checked={a.enabled}
            onChange={(v) => set({ enabled: v })}
          />
        </div>

        {a.enabled && keyed.length === 0 ? (
          <p className="px-1 text-footnote" style={{ color: "var(--color-warning)" }}>
            No provider has a key saved. Open Providers and add one for Groq or Gemini — both have
            free tiers.
          </p>
        ) : null}
      </section>

      {a.enabled ? (
        <>
          <section className="flex flex-col gap-3">
            <h2 className="px-1 text-subheadline font-semibold">Session context</h2>
            <p className="px-1 text-footnote" style={{ color: "var(--label-secondary)" }}>
              Tell the assistant what this session is before you start it. This shapes how it
              answers.
            </p>

            <div className="flex flex-col gap-2">
              {CONTEXTS.map((c) => (
                <ContextCard
                  key={c.id}
                  selected={a.context === c.id}
                  label={c.label}
                  hint={c.hint}
                  onSelect={() => set({ context: c.id })}
                />
              ))}
            </div>

            <label className="flex flex-col gap-1 px-1">
              <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
                Notes (optional) — anything specific, e.g. "Senior Rust role at a fintech"
              </span>
              <textarea
                value={a.notes}
                onChange={(e) => set({ notes: e.target.value })}
                rows={2}
                placeholder="Describe the meeting, the role, the topic…"
                className="w-full resize-none px-2.5 py-2 text-footnote"
                style={inputStyle}
              />
            </label>
          </section>

          <section className="flex flex-col gap-3">
            <h2 className="px-1 text-subheadline font-semibold">Model</h2>
            <ProviderRow
              keyed={keyed}
              provider={a.provider}
              model={a.model}
              onProvider={(provider) => {
                const chosen = keyed.find((p) => p.info.id === provider);
                set({ provider, model: chosen?.info.defaultModel ?? "" });
              }}
              onModel={(model) => set({ model })}
            />
            <div
              className="flex items-center justify-between gap-4 p-3"
              style={{
                background: "var(--surface-content)",
                borderRadius: "var(--radius-card)",
                boxShadow: "var(--shadow-card)",
              }}
            >
              <div className="flex min-w-0 flex-col gap-0.5">
                <span className="text-subheadline">Answer my own questions too</span>
                <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
                  By default the assistant answers what other people say, not you.
                </span>
              </div>
              <Switch
                label="Answer my own questions"
                checked={a.answerOwnQuestions}
                onChange={(v) => set({ answerOwnQuestions: v })}
              />
            </div>
          </section>

          <AskBox disabled={!hasKeyForChosen} />
          <Feed />
        </>
      ) : null}
    </div>
  );
}

function ContextCard({
  selected,
  label,
  hint,
  onSelect,
}: {
  selected: boolean;
  label: string;
  hint: string;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className="flex items-center gap-3 p-3 text-left"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
        outline: selected ? "1.5px solid var(--color-accent)" : "1.5px solid transparent",
        outlineOffset: -1,
      }}
    >
      <span
        aria-hidden
        className="h-[16px] w-[16px] shrink-0 rounded-full"
        style={{
          border: selected ? "5px solid var(--color-accent)" : "1.5px solid var(--border-control)",
        }}
      />
      <div className="flex min-w-0 flex-col">
        <span className="text-subheadline">{label}</span>
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {hint}
        </span>
      </div>
    </button>
  );
}

function ProviderRow({
  keyed,
  provider,
  model,
  onProvider,
  onModel,
}: {
  keyed: ProviderStatus[];
  provider: ProviderId;
  model: string;
  onProvider: (provider: ProviderId) => void;
  onModel: (model: string) => void;
}) {
  const [live, setLive] = useState<string[]>();
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    setLive(undefined);
    setLoaded(false);
    if (!keyed.some((p) => p.info.id === provider)) return;
    let active = true;
    listProviderModels(provider, "chat")
      .then((list) => {
        if (active && list.length > 0) setLive(list);
      })
      .catch(() => undefined)
      .finally(() => {
        if (active) setLoaded(true);
      });
    return () => {
      active = false;
    };
  }, [provider, keyed]);

  // The provider's own known models are the base, so the list is never empty or
  // stale while a live fetch is in flight or has failed.
  const staticModels = keyed.find((p) => p.info.id === provider)?.info.models ?? [];
  const valid = [...new Set([...(live ?? []), ...staticModels])];

  // Snap a model that does not belong to this provider onto a real one. Fixes
  // a saved model left over from a different provider, so the dropdown never
  // shows another provider's model.
  useEffect(() => {
    if (!loaded || valid.length === 0) return;
    if (!model || !valid.includes(model)) onModel(valid[0]!);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loaded, live]);

  const options =
    model && valid.includes(model) ? valid : [...new Set([model, ...valid])].filter(Boolean);

  return (
    <div
      className="flex flex-col gap-3 p-3"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <label className="flex items-center justify-between gap-3">
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          Provider
        </span>
        <select
          value={provider}
          onChange={(e) => onProvider(e.target.value as ProviderId)}
          className="px-2.5 py-[5px] text-footnote"
          style={inputStyle}
        >
          {keyed.length === 0 ? <option value={provider}>{provider}</option> : null}
          {keyed.map((p) => (
            <option key={p.info.id} value={p.info.id}>
              {p.info.name}
            </option>
          ))}
        </select>
      </label>
      <label className="flex items-center justify-between gap-3">
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          Model
        </span>
        <select
          value={model}
          onChange={(e) => onModel(e.target.value)}
          className="px-2.5 py-[5px] text-footnote"
          style={inputStyle}
        >
          {options.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}

function AskBox({ disabled }: { disabled: boolean }) {
  const [text, setText] = useState("");
  const busy = useRef(false);

  const ask = () => {
    const question = text.trim();
    if (!question || busy.current) return;
    busy.current = true;
    setText("");
    const id = addQuestion(question);
    askAssistant(question, [])
      .then((answer) => (answer.trim() ? resolveAnswer(id, answer) : dropEntry(id)))
      .catch((error: unknown) => failAnswer(id, String(error)))
      .finally(() => {
        busy.current = false;
      });
  };

  return (
    <section className="flex flex-col gap-2">
      <h2 className="px-1 text-subheadline font-semibold">Ask anything</h2>
      <div className="flex items-center gap-2">
        <input
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") ask();
          }}
          placeholder={disabled ? "Add a key for this provider first" : "Type a question…"}
          disabled={disabled}
          className="w-full px-2.5 py-[7px] text-footnote"
          style={inputStyle}
        />
        <Button onClick={ask} disabled={disabled || !text.trim()} variant="accent">
          Ask
        </Button>
      </div>
    </section>
  );
}

function Feed() {
  const feed = useAssistantFeed();
  if (feed.length === 0) return null;

  return (
    <section className="flex flex-col gap-2">
      <h2 className="px-1 text-subheadline font-semibold">Answers</h2>
      {[...feed].reverse().map((entry) => (
        <div
          key={entry.id}
          className="flex flex-col gap-1.5 p-3"
          data-selectable
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
          }}
        >
          <span className="text-footnote font-medium" style={{ color: "var(--label-secondary)" }}>
            {entry.question}
          </span>
          {entry.pending ? (
            <span className="text-subheadline" style={{ color: "var(--label-tertiary)" }}>
              Thinking…
            </span>
          ) : entry.error ? (
            <span className="text-subheadline" style={{ color: "var(--color-danger)" }}>
              Could not answer.
            </span>
          ) : (
            <span
              className="text-subheadline whitespace-pre-wrap"
              style={{ color: "var(--label-primary)" }}
            >
              {entry.answer}
            </span>
          )}
        </div>
      ))}
    </section>
  );
}

const inputStyle = {
  background: "var(--surface-elevated)",
  color: "var(--label-primary)",
  border: "0.5px solid var(--border-control)",
  borderRadius: "var(--radius-control)",
} as const;
