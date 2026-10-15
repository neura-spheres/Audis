import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { surfaceStyle } from "@/components/GroupedList";
import { ProviderMonogram } from "@/components/IconBadge";
import { ExternalIcon } from "@/components/icons";
import { Page } from "@/components/Page";
import { showToast } from "@/components/Toast";
import { Badge, Button, Select, Switch, TextField } from "@/components/controls";
import {
  deleteProviderKey,
  listProviders,
  listProviderModels,
  setProviderKey,
  updateProvider,
  AudisIpcError,
} from "@/services/ipc";
import type { ProviderStatus, UserFacingError } from "@/schemas/ipc";

/** Connect AI providers. */
export function ProvidersView() {
  const [providers, setProviders] = useState<ProviderStatus[]>();
  const [error, setError] = useState<UserFacingError>();

  const refresh = useCallback(() => {
    listProviders()
      .then(setProviders)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  useEffect(refresh, [refresh]);

  return (
    <Page
      title="AI Providers"
      subtitle="The assistant needs one. Gemini and Groq have free tiers. Transcription on this PC needs none of this."
    >
      {error ? <ErrorNotice error={error} /> : null}

      <div className="audis-stagger flex flex-col gap-3">
        {providers === undefined
          ? [0, 1, 2].map((index) => (
              <div
                key={index}
                className="audis-skeleton h-[148px]"
                style={{ borderRadius: "var(--radius-card)" }}
              />
            ))
          : providers.map((provider) => (
              <ProviderCard
                key={provider.info.id}
                provider={provider}
                onChanged={refresh}
                onError={setError}
              />
            ))}
      </div>

      <p className="px-1 text-footnote" style={{ color: "var(--label-tertiary)" }}>
        Keys are stored in the Windows Credential Manager, protected by your Windows account. Audis
        never writes them to its own files, logs or exports, and cannot show a key once saved.
      </p>
    </Page>
  );
}

function ProviderCard({
  provider,
  onChanged,
  onError,
}: {
  provider: ProviderStatus;
  onChanged: () => void;
  onError: (error: UserFacingError) => void;
}) {
  const { info } = provider;
  const [keyInput, setKeyInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [endpoint, setEndpoint] = useState(provider.endpoint ?? "");
  const [liveModels, setLiveModels] = useState<string[]>();

  useEffect(() => {
    if (!provider.hasKey) return;
    let active = true;
    listProviderModels(info.id, "chat")
      .then((models) => {
        if (active && models.length > 0) setLiveModels(models);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [provider.hasKey, info.id]);

  const models = liveModels ?? info.models;

  const saveKey = () => {
    if (!keyInput.trim()) return;
    setSaving(true);
    setProviderKey(info.id, keyInput)
      .then(() => {
        setKeyInput("");
        showToast({ title: "API key saved", message: `${info.name} is ready to use.` });
        onChanged();
      })
      .catch((cause: unknown) => onError(toUserFacing(cause)))
      .finally(() => setSaving(false));
  };

  const removeKey = () => {
    deleteProviderKey(info.id)
      .then(onChanged)
      .catch((cause: unknown) => onError(toUserFacing(cause)));
  };

  const setEnabled = (enabled: boolean) => {
    updateProvider(info.id, enabled, provider.model, endpoint || null)
      .then(onChanged)
      .catch((cause: unknown) => onError(toUserFacing(cause)));
  };

  const setModel = (model: string) => {
    updateProvider(info.id, provider.enabled, model, endpoint || null)
      .then(onChanged)
      .catch((cause: unknown) => onError(toUserFacing(cause)));
  };

  return (
    <section className="flex flex-col overflow-hidden" style={surfaceStyle}>
      <div className="flex items-center gap-3 p-3.5">
        <ProviderMonogram name={info.name} />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <div className="flex flex-wrap items-center gap-1.5">
            <h2 className="text-subheadline font-semibold">{info.name}</h2>
            {info.freeTier ? <Badge tone="success">Free tier</Badge> : null}
            {provider.hasKey ? <Badge tone="accent">Key saved</Badge> : null}
          </div>
          <p className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {info.summary}
          </p>
        </div>
        <Switch label={`Enable ${info.name}`} checked={provider.enabled} onChange={setEnabled} />
      </div>

      <div
        className="flex flex-col gap-3 px-3.5 pt-3 pb-3.5"
        style={{ boxShadow: "inset 0 0.5px 0 var(--separator)" }}
      >
        {info.needsEndpoint ? (
          <label className="flex flex-col gap-1">
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              Endpoint
            </span>
            <TextField
              type="url"
              value={endpoint}
              onChange={setEndpoint}
              onBlur={() => setModel(provider.model)}
              placeholder="http://localhost:11434/v1"
            />
          </label>
        ) : null}

        {models.length > 0 ? (
          <label className="flex items-center justify-between gap-3">
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              Model
            </span>
            <Select value={provider.model} onChange={setModel}>
              {[...new Set([provider.model, ...models])].map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
              ))}
            </Select>
          </label>
        ) : null}

        <div className="flex flex-col gap-1">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            {provider.hasKey ? "Replace API key" : "API key"}
          </span>
          <div className="flex items-center gap-2">
            <TextField
              type="password"
              value={keyInput}
              onChange={setKeyInput}
              onEnter={saveKey}
              placeholder={
                provider.hasKey ? "A key is saved. Type a new one to replace it." : "Paste your key"
              }
              secret
            />
            <Button onClick={saveKey} disabled={!keyInput.trim() || saving} variant="accent">
              {saving ? "Saving…" : "Save"}
            </Button>
            {provider.hasKey ? (
              <Button
                onClick={removeKey}
                variant="danger"
                ariaLabel={`Delete the ${info.name} key`}
              >
                Delete
              </Button>
            ) : null}
          </div>
        </div>

        {info.consoleUrl ? (
          <a
            href={info.consoleUrl}
            target="_blank"
            rel="noreferrer noopener"
            className="flex items-center gap-1.5 self-start text-footnote"
            style={{ color: "var(--color-accent)" }}
          >
            <ExternalIcon />
            Get a key from {info.name}
          </a>
        ) : null}
      </div>
    </section>
  );
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not update that provider",
    explanation: "Something went wrong. Your saved keys were not affected.",
    dataPreserved: true,
    suggestedAction: "Try again.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
