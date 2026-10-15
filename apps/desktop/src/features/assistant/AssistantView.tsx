import { useEffect, useRef, useState } from "react";

import type { Navigate } from "@/app/navigation";
import { Collapse } from "@/components/Collapse";
import { ErrorNotice } from "@/components/ErrorNotice";
import { GroupedList, Row } from "@/components/GroupedList";
import { IconBadge } from "@/components/IconBadge";
import { AiIcon, CheckIcon } from "@/components/icons";
import { Page, Section } from "@/components/Page";
import { Button, Select, Switch, TextField, fieldStyle } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import { askAssistant, listProviderModels, listProviders } from "@/services/ipc";
import type { AssistantContext, ProviderId, ProviderStatus } from "@/schemas/ipc";
import { addQuestion, dropEntry, failAnswer, resolveAnswer, useAssistantFeed } from "./feed";
import { useMeeting } from "./meeting";

const CONTEXTS: { id: AssistantContext; label: string; hint: string }[] = [
  { id: "general", label: "General", hint: "A normal conversation." },
  { id: "meeting", label: "Meeting", hint: "Concise, factual answers when questions come up." },
  { id: "interview", label: "Interview", hint: "Suggests answers for you, the candidate." },
  { id: "quiz", label: "Quiz", hint: "Gives the correct answer with a short reason." },
  { id: "lecture", label: "Lecture", hint: "Answers questions and clarifies concepts." },
];

export function AssistantView({ onNavigate }: { onNavigate: Navigate }) {
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
  // A key is not enough: a speech-only provider like Deepgram has no chat
  // endpoint, so offering it here would only produce a request that cannot land.
  const keyed = (providers ?? []).filter((p) => p.hasKey && p.info.chat !== null);
  const hasKeyForChosen = keyed.some((p) => p.info.id === a.provider);

  const set = (patch: Partial<typeof a>) =>
    update((current) => ({ ...current, assistant: { ...current.assistant, ...patch } }));

  return (
    <Page
      title="AI Assistant"
      subtitle="Answers questions as they come up in a session, using the AI provider you choose."
    >
      <GroupedList
        footnote={
          a.enabled && providers !== undefined && keyed.length === 0 ? (
            <span className="flex items-center justify-between gap-3">
              <span style={{ color: "var(--color-warning)" }}>
                No provider has a key yet. Groq and Gemini both have free tiers.
              </span>
              <Button variant="tinted" onClick={() => onNavigate("providers")}>
                Add a key
              </Button>
            </span>
          ) : (
            "Off means nothing is sent. When on, the recent transcript goes to your provider only when a question needs answering."
          )
        }
      >
        <Row
          icon={
            <IconBadge tint="indigo" size={30}>
              <AiIcon />
            </IconBadge>
          }
          label="Answer questions during a session"
          description="Sends the recent transcript to your provider to answer what people ask."
          value={
            <Switch
              label="Enable the assistant"
              checked={a.enabled}
              onChange={(v) => set({ enabled: v })}
            />
          }
        />
      </GroupedList>

      <Collapse open={a.enabled}>
        {a.enabled ? (
          <div className="flex flex-col gap-7">
            <GroupedList
              title="This session is a…"
              footnote="Set it before you start. It shapes how the assistant answers."
            >
              {CONTEXTS.map((c) => (
                <ContextRow
                  key={c.id}
                  selected={a.context === c.id}
                  label={c.label}
                  hint={c.hint}
                  onSelect={() => set({ context: c.id })}
                />
              ))}
            </GroupedList>

            <Section
              title="What should the AI know?"
              footnote="The topic, who is speaking, the role you are interviewing for, terms to use. Sent with every answer."
            >
              <textarea
                value={a.notes}
                onChange={(e) => set({ notes: e.target.value })}
                rows={4}
                aria-label="Notes for the assistant"
                placeholder={
                  'e.g. "This is a senior Rust interview at a fintech. I am the candidate. ' +
                  'Prefer concrete, technical answers."'
                }
                className="w-full resize-y px-3 py-2.5 text-footnote"
                style={{ ...fieldStyle, borderRadius: "var(--radius-card)" }}
              />
            </Section>

            <GroupedList title="Model">
              <ProviderRows
                keyed={keyed}
                provider={a.provider}
                model={a.model}
                onProvider={(provider) => {
                  const chosen = keyed.find((p) => p.info.id === provider);
                  set({ provider, model: chosen?.info.defaultModel ?? "" });
                }}
                onModel={(model) => set({ model })}
              />
              <Row
                label="Answer my own questions too"
                description="By default it answers what other people say, not you."
                value={
                  <Switch
                    label="Answer my own questions"
                    checked={a.answerOwnQuestions}
                    onChange={(v) => set({ answerOwnQuestions: v })}
                  />
                }
              />
            </GroupedList>

            <AskBox disabled={!hasKeyForChosen} />
            <MeetingNotes />
            <Feed />
          </div>
        ) : null}
      </Collapse>
    </Page>
  );
}

function MeetingNotes() {
  const meeting = useMeeting();
  if (!meeting) return null;

  const hasContent =
    meeting.summary.trim().length > 0 ||
    meeting.decisions.length > 0 ||
    meeting.actionItems.length > 0;
  if (!hasContent) return null;

  return (
    <section className="flex flex-col gap-2.5">
      <h2 className="px-1 text-title3 font-semibold">Meeting notes</h2>
      <div
        data-selectable
        className="flex flex-col gap-3 p-3"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        {meeting.summary.trim() ? (
          <p
            className="text-subheadline whitespace-pre-wrap"
            style={{ color: "var(--label-primary)" }}
          >
            {meeting.summary}
          </p>
        ) : null}
        {meeting.decisions.length > 0 ? (
          <NoteList title="Decisions" items={meeting.decisions} />
        ) : null}
        {meeting.actionItems.length > 0 ? (
          <NoteList title="Action items" items={meeting.actionItems} />
        ) : null}
      </div>
    </section>
  );
}

function NoteList({ title, items }: { title: string; items: string[] }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-footnote font-medium" style={{ color: "var(--label-secondary)" }}>
        {title}
      </span>
      <ul className="flex flex-col gap-1">
        {items.map((item, index) => (
          <li
            key={`${title}-${index}`}
            className="flex items-start gap-2 text-footnote"
            style={{ color: "var(--label-primary)" }}
          >
            <span aria-hidden className="mt-[3px]" style={{ color: "var(--label-tertiary)" }}>
              •
            </span>
            <span>{item}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** One choice in a list where exactly one is checked, as in System Settings. */
function ContextRow({
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
      role="radio"
      aria-checked={selected}
      onClick={onSelect}
      className="audis-row audis-side-item flex w-full items-center gap-3 px-3.5 py-2.5 text-left"
    >
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-subheadline">{label}</span>
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {hint}
        </span>
      </div>
      <span
        aria-hidden
        className="flex h-4 w-4 shrink-0 [&>svg]:h-full [&>svg]:w-full"
        style={{
          color: "var(--color-accent)",
          opacity: selected ? 1 : 0,
          transform: selected ? "scale(1)" : "scale(0.5)",
          transition:
            "opacity var(--duration-fast) var(--ease-standard), transform var(--spring-bouncy-duration) var(--spring-bouncy)",
        }}
      >
        <CheckIcon />
      </span>
    </button>
  );
}

function ProviderRows({
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
    <>
      <Row
        label="Provider"
        value={
          <Select
            value={provider}
            onChange={(value) => onProvider(value as ProviderId)}
            label="Provider"
          >
            {keyed.length === 0 ? <option value={provider}>{provider}</option> : null}
            {keyed.map((p) => (
              <option key={p.info.id} value={p.info.id}>
                {p.info.name}
              </option>
            ))}
          </Select>
        }
      />
      <Row
        label="Model"
        value={
          <Select value={model} onChange={onModel} label="Model">
            {options.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </Select>
        }
      />
    </>
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
    askAssistant(question, [], "")
      .then((answer) => (answer.trim() ? resolveAnswer(id, answer) : dropEntry(id)))
      .catch((error: unknown) => failAnswer(id, String(error)))
      .finally(() => {
        busy.current = false;
      });
  };

  return (
    <Section title="Ask anything">
      <div className="flex items-center gap-2">
        <TextField
          value={text}
          onChange={setText}
          onEnter={ask}
          label="Question for the assistant"
          placeholder={disabled ? "Add a key for this provider first" : "Type a question…"}
          disabled={disabled}
        />
        <Button onClick={ask} disabled={disabled || !text.trim()} variant="accent">
          Ask
        </Button>
      </div>
    </Section>
  );
}

function Feed() {
  const feed = useAssistantFeed();
  if (feed.length === 0) return null;

  return (
    <section className="flex flex-col gap-2.5">
      <h2 className="px-1 text-title3 font-semibold">Answers</h2>
      {[...feed].reverse().map((entry) => (
        <div
          key={entry.id}
          className="flex flex-col gap-1.5 p-3.5"
          data-selectable
          style={{
            background: "var(--surface-content)",
            borderRadius: "var(--radius-card)",
            boxShadow: "var(--shadow-card)",
            animation: "audis-rise var(--spring-smooth-duration) var(--spring-smooth) both",
          }}
        >
          <span className="text-footnote font-semibold" style={{ color: "var(--label-secondary)" }}>
            {entry.question}
          </span>
          {entry.pending ? (
            <span className="audis-skeleton h-4 w-2/3" />
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
