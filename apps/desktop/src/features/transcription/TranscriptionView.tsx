import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button, SegmentedControl, Switch } from "@/components/controls";
import { CheckIcon, ExternalIcon } from "@/components/icons";
import { useSettings } from "@/hooks/useSettings";
import { listModels, listProviders, listProviderModels, setProviderKey } from "@/services/ipc";
import type {
  InstalledModel,
  Language,
  ModelId,
  ProviderStatus,
  TranscriptionEngine,
} from "@/schemas/ipc";

/** Speech recognition settings. */
export function TranscriptionView() {
  const { settings, error, update } = useSettings();
  const [models, setModels] = useState<InstalledModel[]>();
  const [providers, setProviders] = useState<ProviderStatus[]>();

  const loadProviders = useCallback(() => {
    listProviders()
      .then(setProviders)
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    listModels()
      .then(setModels)
      .catch(() => undefined);
    loadProviders();
  }, [loadProviders]);

  if (error) return <ErrorNotice error={error} />;
  if (!settings) return null;

  const { transcription } = settings;
  const engine = transcription.engine;
  const installed = (models ?? []).filter((model) => model.installed);
  const speechProviders = (providers ?? []).filter((provider) => provider.info.speech !== null);

  const setEngine = (next: TranscriptionEngine) =>
    update((current) => ({
      ...current,
      transcription: {
        ...current.transcription,
        engine: next,
        model: next.kind === "local" ? next.model : current.transcription.model,
      },
    }));

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <SectionTitle>Speech engine</SectionTitle>
        <p className="px-1 text-footnote" style={{ color: "var(--label-secondary)" }}>
          What turns audio into text. Run it on this PC for free and offline, or send audio to a
          provider for accuracy this PC cannot match live.
        </p>

        <div role="radiogroup" aria-label="Speech engine" className="mt-1 flex flex-col gap-2.5">
          <LocalEngineCard
            selected={engine.kind === "local"}
            installed={installed}
            selectedModel={transcription.model}
            onSelect={() => setEngine({ kind: "local", model: transcription.model })}
            onModel={(model) => setEngine({ kind: "local", model })}
          />

          {speechProviders.map((provider) => (
            <ProviderEngineCard
              key={provider.info.id}
              provider={provider}
              engine={engine}
              onSelect={setEngine}
              onKeySaved={loadProviders}
            />
          ))}
        </div>
      </section>

      <section className="flex flex-col gap-3">
        <SectionTitle>Language</SectionTitle>
        <Row help="Audis recognises Indonesian and English. Tell it which you are speaking: choosing beats guessing, especially when you mix in English words.">
          <SegmentedControl<Language>
            label="Recognition language"
            value={transcription.language}
            options={[
              { id: "indonesian", label: "Indonesian" },
              { id: "english", label: "English" },
            ]}
            onChange={(language) =>
              update((current) => ({
                ...current,
                transcription: { ...current.transcription, language },
              }))
            }
          />
        </Row>
      </section>

      <section className="flex flex-col gap-3">
        <SectionTitle>What to listen to</SectionTitle>

        <Row label="Your microphone" help="What you say. Always labelled as you, never guessed.">
          <Switch
            label="Capture your microphone"
            checked={transcription.captureMicrophone}
            onChange={(captureMicrophone) =>
              update((current) => ({
                ...current,
                transcription: { ...current.transcription, captureMicrophone },
              }))
            }
          />
        </Row>

        <Row
          label="Computer audio"
          help="Everyone else in a call, and any video you play. Captured without a virtual cable."
        >
          <Switch
            label="Capture computer audio"
            checked={transcription.captureComputerAudio}
            onChange={(captureComputerAudio) =>
              update((current) => ({
                ...current,
                transcription: { ...current.transcription, captureComputerAudio },
              }))
            }
          />
        </Row>

        {!transcription.captureMicrophone && !transcription.captureComputerAudio ? (
          <p className="px-1 text-footnote" style={{ color: "var(--color-warning)" }}>
            Audis needs at least one of these switched on before a session can start.
          </p>
        ) : null}
      </section>

      <p className="px-1 text-footnote" style={{ color: "var(--label-tertiary)" }}>
        {engine.kind === "local"
          ? "Recognition runs entirely on this PC. Nothing you say is sent anywhere, and it works with no internet connection."
          : "While a provider is selected, your audio leaves this PC to be transcribed. Everything else about Audis stays local."}
      </p>
    </div>
  );
}

/** The local Whisper option, with its model picker when selected. */
function LocalEngineCard({
  selected,
  installed,
  selectedModel,
  onSelect,
  onModel,
}: {
  selected: boolean;
  installed: InstalledModel[];
  selectedModel: ModelId;
  onSelect: () => void;
  onModel: (model: ModelId) => void;
}) {
  const selectedIsInstalled = installed.some((model) => model.info.id === selectedModel);

  return (
    <EngineCard
      selected={selected}
      onSelect={onSelect}
      title="This PC"
      badge={{ label: "Free · Private", tone: "success" }}
      summary="Whisper runs on your computer. Works offline, and your voice never leaves the machine."
    >
      {installed.length === 0 ? (
        <p className="text-footnote" style={{ color: "var(--color-warning)" }}>
          No speech model is installed yet. Open Models and install Whisper Base — it is free.
        </p>
      ) : (
        <label className="flex items-center justify-between gap-3">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Model
          </span>
          <select
            value={selectedModel}
            onChange={(event) => onModel(event.target.value as ModelId)}
            className="px-2.5 py-[5px] text-footnote"
            style={inputStyle}
          >
            {installed.map((model) => (
              <option key={model.info.id} value={model.info.id}>
                {model.info.name}
              </option>
            ))}
          </select>
        </label>
      )}

      {installed.length > 0 && !selectedIsInstalled ? (
        <p className="text-footnote" style={{ color: "var(--color-warning)" }}>
          The model you chose is no longer installed. Pick one above, or reinstall it from Models.
        </p>
      ) : null}
    </EngineCard>
  );
}

/** One cloud provider as a speech engine. */
function ProviderEngineCard({
  provider,
  engine,
  onSelect,
  onKeySaved,
}: {
  provider: ProviderStatus;
  engine: TranscriptionEngine;
  onSelect: (engine: TranscriptionEngine) => void;
  onKeySaved: () => void;
}) {
  const speech = provider.info.speech;
  const selected = engine.kind === "cloud" && engine.provider === provider.info.id;
  const [keyInput, setKeyInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [liveModels, setLiveModels] = useState<string[]>();

  useEffect(() => {
    if (!provider.hasKey) return;
    let active = true;
    listProviderModels(provider.info.id, "speech")
      .then((models) => {
        if (active && models.length > 0) setLiveModels(models);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [provider.hasKey, provider.info.id]);

  if (!speech) return null;

  const models = liveModels ?? speech.models;

  const select = () =>
    onSelect({ kind: "cloud", provider: provider.info.id, model: speech.defaultModel });

  const saveKey = () => {
    if (!keyInput.trim()) return;
    setSaving(true);
    setProviderKey(provider.info.id, keyInput)
      .then(() => {
        setKeyInput("");
        onKeySaved();
      })
      .catch(() => undefined)
      .finally(() => setSaving(false));
  };

  return (
    <EngineCard
      selected={selected}
      onSelect={select}
      title={provider.info.name}
      badge={provider.info.freeTier ? { label: "Free tier", tone: "success" } : undefined}
      summary={speech.summary}
    >
      {provider.hasKey ? (
        <label className="flex items-center justify-between gap-3">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Model
          </span>
          <select
            value={selected ? engine.model : speech.defaultModel}
            onChange={(event) =>
              onSelect({
                kind: "cloud",
                provider: provider.info.id,
                model: event.target.value,
              })
            }
            className="px-2.5 py-[5px] text-footnote"
            style={inputStyle}
          >
            {models.map((model) => (
              <option key={model} value={model}>
                {model}
              </option>
            ))}
          </select>
        </label>
      ) : (
        <div className="flex flex-col gap-2">
          <p className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Add an API key to use {provider.info.name}. It is stored in the Windows Credential
            Manager, never in a file.
          </p>
          <div className="flex items-end gap-2">
            <input
              type="password"
              value={keyInput}
              onChange={(event) => setKeyInput(event.target.value)}
              placeholder="Paste your API key"
              autoComplete="off"
              spellCheck={false}
              className="w-full px-2.5 py-[5px] text-footnote"
              style={inputStyle}
              onKeyDown={(event) => {
                if (event.key === "Enter") saveKey();
              }}
            />
            <Button onClick={saveKey} disabled={!keyInput.trim() || saving} variant="accent">
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
          {provider.info.consoleUrl ? (
            <a
              href={provider.info.consoleUrl}
              target="_blank"
              rel="noreferrer noopener"
              className="flex items-center gap-1.5 self-start text-footnote"
              style={{ color: "var(--color-accent)" }}
            >
              <ExternalIcon />
              Get a free key from {provider.info.name}
            </a>
          ) : null}
        </div>
      )}
    </EngineCard>
  );
}

/** A selectable engine card, radio-style. Its body shows only when selected. */
function EngineCard({
  selected,
  onSelect,
  title,
  badge,
  summary,
  children,
}: {
  selected: boolean;
  onSelect: () => void;
  title: string;
  badge?: { label: string; tone: "success" | "neutral" } | undefined;
  summary: string;
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
        outline: selected ? "1.5px solid var(--color-accent)" : "1.5px solid transparent",
        outlineOffset: -1,
      }}
    >
      <button
        type="button"
        role="radio"
        aria-checked={selected}
        onClick={onSelect}
        className="flex w-full items-start gap-3 p-4 text-left"
      >
        <Radio selected={selected} />
        <div className="flex min-w-0 flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="text-body font-semibold">{title}</span>
            {badge ? (
              <span
                className="px-1.5 py-0.5 text-caption2 font-medium"
                style={{
                  color:
                    badge.tone === "success" ? "var(--color-success)" : "var(--label-secondary)",
                }}
              >
                {badge.label}
              </span>
            ) : null}
          </div>
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {summary}
          </span>
        </div>
      </button>

      {selected ? (
        <div
          className="flex flex-col gap-2 border-t px-4 py-3"
          style={{ borderColor: "var(--separator)" }}
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}

function Radio({ selected }: { selected: boolean }) {
  return (
    <span
      aria-hidden
      className="mt-0.5 flex h-[18px] w-[18px] shrink-0 items-center justify-center rounded-full"
      style={{
        border: selected ? "none" : "1.5px solid var(--border-control)",
        background: selected ? "var(--color-accent)" : "transparent",
        color: "#ffffff",
      }}
    >
      {selected ? <CheckIcon /> : null}
    </span>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h2 className="px-1 text-subheadline font-semibold">{children}</h2>;
}

function Row({
  label,
  help,
  children,
}: {
  label?: string;
  help: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className="flex items-center justify-between gap-4 p-3"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <div className="flex min-w-0 flex-col gap-0.5">
        {label ? <span className="text-subheadline">{label}</span> : null}
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {help}
        </span>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

const inputStyle = {
  background: "var(--surface-elevated)",
  color: "var(--label-primary)",
  border: "0.5px solid var(--border-control)",
  borderRadius: "var(--radius-control)",
} as const;
