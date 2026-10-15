import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { Collapse } from "@/components/Collapse";
import { GroupedList, Row } from "@/components/GroupedList";
import { IconBadge, ProviderMonogram } from "@/components/IconBadge";
import { ChipIcon, ExternalIcon, GlobeIcon } from "@/components/icons";
import { Section } from "@/components/Page";
import { Badge, Button, SegmentedControl, Select, TextField } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import { listProviders, listProviderModels, setProviderKey } from "@/services/ipc";
import type {
  InstalledModel,
  Language,
  ModelId,
  ProviderStatus,
  Settings,
  TranscriptionEngine,
} from "@/schemas/ipc";

/**
 * Which engine turns speech into text, and in which language.
 *
 * Takes the model list from its parent so installing a model on the same page
 * shows up here at once, rather than after leaving and coming back.
 */
export function EngineSection({
  models,
  onInstallModel,
}: {
  models: InstalledModel[] | undefined;
  /** Take the user to where a model can be installed. */
  onInstallModel: () => void;
}) {
  const { settings, update } = useSettings();
  const [providers, setProviders] = useState<ProviderStatus[]>();

  const loadProviders = useCallback(() => {
    listProviders()
      .then(setProviders)
      .catch(() => undefined);
  }, []);

  useEffect(loadProviders, [loadProviders]);

  if (!settings) return <div className="audis-skeleton h-[220px]" />;

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
    <>
      <Section
        title="Speech engine"
        footnote={
          engine.kind === "local"
            ? "Recognition runs entirely on this PC. Nothing you say is sent anywhere, and it works with no internet connection."
            : "While a provider is selected, your audio leaves this PC to be transcribed. Everything else about Audis stays local."
        }
      >
        <div role="radiogroup" aria-label="Speech engine" className="flex flex-col gap-2.5">
          <LocalEngineCard
            selected={engine.kind === "local"}
            installed={installed}
            selectedModel={transcription.model}
            onSelect={() => setEngine({ kind: "local", model: transcription.model })}
            onModel={(model) => setEngine({ kind: "local", model })}
            onInstallModel={onInstallModel}
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
      </Section>

      <LanguageGroup settings={settings} update={update} />
    </>
  );
}

function LanguageGroup({
  settings,
  update,
}: {
  settings: Settings;
  update: (change: (current: Settings) => Settings) => Promise<void>;
}) {
  return (
    <GroupedList
      title="Language"
      footnote="Choosing beats guessing, especially when English words are mixed into Indonesian."
    >
      <Row
        icon={
          <IconBadge tint="blue" size={30}>
            <GlobeIcon />
          </IconBadge>
        }
        label="Spoken language"
        description="Audis recognises Indonesian and English."
        value={
          <SegmentedControl<Language>
            label="Recognition language"
            value={settings.transcription.language}
            options={[
              { id: "indonesian", label: "Indonesian" },
              { id: "english", label: "English" },
            ]}
            onChange={(language) =>
              void update((current) => ({
                ...current,
                transcription: { ...current.transcription, language },
              }))
            }
          />
        }
      />
    </GroupedList>
  );
}

/** The local Whisper option, with its model picker when selected. */
function LocalEngineCard({
  selected,
  installed,
  selectedModel,
  onSelect,
  onModel,
  onInstallModel,
}: {
  selected: boolean;
  installed: InstalledModel[];
  selectedModel: ModelId;
  onSelect: () => void;
  onModel: (model: ModelId) => void;
  onInstallModel: () => void;
}) {
  const selectedIsInstalled = installed.some((model) => model.info.id === selectedModel);

  return (
    <EngineCard
      selected={selected}
      onSelect={onSelect}
      icon={
        <IconBadge tint="gray" size={34}>
          <ChipIcon />
        </IconBadge>
      }
      title="This PC"
      badge={<Badge tone="success">Free · Private</Badge>}
      summary="Whisper runs on your computer. Works offline, and your voice never leaves the machine."
    >
      {installed.length === 0 ? (
        <div className="flex items-center justify-between gap-3">
          <p className="text-footnote" style={{ color: "var(--color-warning)" }}>
            No speech model is installed yet. Whisper Base is free and takes a minute.
          </p>
          <Button variant="tinted" onClick={onInstallModel}>
            Install a model
          </Button>
        </div>
      ) : (
        <label className="flex items-center justify-between gap-3">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Model
          </span>
          <Select value={selectedModel} onChange={(value) => onModel(value as ModelId)}>
            {installed.map((model) => (
              <option key={model.info.id} value={model.info.id}>
                {model.info.name}
              </option>
            ))}
          </Select>
        </label>
      )}

      {installed.length > 0 && !selectedIsInstalled ? (
        <p className="text-footnote" style={{ color: "var(--color-warning)" }}>
          The model you chose is no longer installed. Pick one above, or reinstall it below.
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

  const models = liveModels ?? speech?.models ?? [];
  const current = selected ? engine.model : (speech?.defaultModel ?? "");

  // A saved model Audis no longer offers — one dropped from the list, or one
  // chosen before the list narrowed — would leave the dropdown blank while the
  // session quietly ran on it anyway. Fall back to the default and write that
  // back, so what is shown is what will run.
  const stale = Boolean(speech) && selected && models.length > 0 && !models.includes(current);

  // Guarded by a ref, not just by `stale`: this writes settings from a render
  // effect, and `onSelect` is rebuilt every render. Correcting once is the whole
  // intent, and a failed write must not become a loop of retries.
  const corrected = useRef(false);
  useEffect(() => {
    if (!stale || !speech || corrected.current) return;
    corrected.current = true;
    onSelect({ kind: "cloud", provider: provider.info.id, model: speech.defaultModel });
  }, [stale, speech, provider.info.id, onSelect]);

  if (!speech) return null;

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
      icon={<ProviderMonogram name={provider.info.name} />}
      title={provider.info.name}
      badge={provider.info.freeTier ? <Badge tone="success">Free tier</Badge> : undefined}
      summary={speech.summary}
    >
      {provider.hasKey ? (
        <label className="flex items-center justify-between gap-3">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Model
          </span>
          <Select
            value={stale ? speech.defaultModel : current}
            onChange={(model) => onSelect({ kind: "cloud", provider: provider.info.id, model })}
          >
            {models.map((model) => (
              <option key={model} value={model}>
                {model}
              </option>
            ))}
          </Select>
        </label>
      ) : (
        <div className="flex flex-col gap-2">
          <p className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Add an API key to use {provider.info.name}. It is stored in the Windows Credential
            Manager, never in a file.
          </p>
          <div className="flex items-center gap-2">
            <TextField
              type="password"
              value={keyInput}
              onChange={setKeyInput}
              placeholder="Paste your API key"
              label={`${provider.info.name} API key`}
              onEnter={saveKey}
              secret
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

/** A selectable engine card, radio-style. Its body opens when selected. */
function EngineCard({
  selected,
  onSelect,
  icon,
  title,
  badge,
  summary,
  children,
}: {
  selected: boolean;
  onSelect: () => void;
  icon: ReactNode;
  title: string;
  badge?: ReactNode;
  summary: string;
  children: ReactNode;
}) {
  return (
    <div
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: selected
          ? "0 0 0 2px var(--color-accent), var(--shadow-card)"
          : "var(--shadow-card)",
        transition: "box-shadow var(--duration-standard) var(--ease-standard)",
      }}
    >
      <button
        type="button"
        role="radio"
        aria-checked={selected}
        onClick={onSelect}
        className="flex w-full items-center gap-3 p-3.5 text-left"
      >
        {icon}
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <span className="text-subheadline font-semibold">{title}</span>
            {badge}
          </div>
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {summary}
          </span>
        </div>
        <Radio selected={selected} />
      </button>

      <Collapse open={selected}>
        <div
          className="flex flex-col gap-2 px-3.5 pt-3 pb-3.5"
          style={{ boxShadow: "inset 0 0.5px 0 var(--separator)" }}
        >
          {children}
        </div>
      </Collapse>
    </div>
  );
}

function Radio({ selected }: { selected: boolean }) {
  return (
    <span
      aria-hidden
      className="flex h-[18px] w-[18px] shrink-0 items-center justify-center rounded-full"
      style={{
        boxShadow: selected ? "none" : "inset 0 0 0 1.5px var(--border-control)",
        background: selected ? "var(--color-accent)" : "transparent",
        transition: "background-color var(--duration-fast) var(--ease-standard)",
      }}
    >
      <span
        className="h-[7px] w-[7px] rounded-full"
        style={{
          background: "#ffffff",
          transform: selected ? "scale(1)" : "scale(0)",
          transition: "transform var(--spring-bouncy-duration) var(--spring-bouncy)",
        }}
      />
    </span>
  );
}
