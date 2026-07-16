import { useCallback, useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { Button, Switch } from "@/components/controls";
import { ExternalIcon } from "@/components/icons";
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
    <div className="flex flex-col gap-5">
      {error ? <ErrorNotice error={error} /> : null}

      <p className="px-1 text-subheadline" style={{ color: "var(--label-secondary)" }}>
        The AI assistant needs a provider. Gemini and Groq have free tiers. Transcription does not
        need any of this: it runs on this PC for free.
      </p>

      <div className="flex flex-col gap-3">
        {(providers ?? []).map((provider) => (
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
    </div>
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
    <section
      className="flex flex-col gap-3 p-4"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="flex min-w-0 flex-col gap-1">
          <div className="flex items-center gap-2">
            <h2 className="text-body font-semibold">{info.name}</h2>
            {info.freeTier ? (
              <span className="text-caption2 font-medium" style={{ color: "var(--color-success)" }}>
                Free tier
              </span>
            ) : null}
            {provider.hasKey ? (
              <span className="text-caption2" style={{ color: "var(--label-secondary)" }}>
                Key saved
              </span>
            ) : null}
          </div>
          <p className="text-subheadline" style={{ color: "var(--label-secondary)" }}>
            {info.summary}
          </p>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Switch label={`Enable ${info.name}`} checked={provider.enabled} onChange={setEnabled} />
        </div>
      </div>

      {info.needsEndpoint ? (
        <label className="flex flex-col gap-1">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Endpoint
          </span>
          <input
            type="url"
            value={endpoint}
            onChange={(event) => setEndpoint(event.target.value)}
            onBlur={() => setModel(provider.model)}
            placeholder="http://localhost:11434/v1"
            className="w-full px-2.5 py-[5px] text-footnote"
            style={inputStyle}
          />
        </label>
      ) : null}

      {models.length > 0 ? (
        <label className="flex items-center justify-between gap-3">
          <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
            Model
          </span>
          <select
            value={provider.model}
            onChange={(event) => setModel(event.target.value)}
            className="px-2.5 py-[5px] text-footnote"
            style={inputStyle}
          >
            {[...new Set([provider.model, ...models])].map((model) => (
              <option key={model} value={model}>
                {model}
              </option>
            ))}
          </select>
        </label>
      ) : null}

      <div className="flex flex-col gap-2">
        <div className="flex items-end gap-2">
          <label className="flex min-w-0 flex-1 flex-col gap-1">
            <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
              {provider.hasKey ? "Replace API key" : "API key"}
            </span>
            <input
              type="password"
              value={keyInput}
              onChange={(event) => setKeyInput(event.target.value)}
              placeholder={
                provider.hasKey ? "A key is saved. Type a new one to replace it." : "Paste your key"
              }
              autoComplete="off"
              spellCheck={false}
              className="w-full px-2.5 py-[5px] text-footnote"
              style={inputStyle}
            />
          </label>
          <Button onClick={saveKey} disabled={!keyInput.trim() || saving} variant="accent">
            {saving ? "Saving…" : "Save"}
          </Button>
          {provider.hasKey ? (
            <Button onClick={removeKey} variant="danger" ariaLabel={`Delete the ${info.name} key`}>
              Delete
            </Button>
          ) : null}
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

const inputStyle = {
  background: "var(--surface-elevated)",
  color: "var(--label-primary)",
  border: "0.5px solid var(--border-control)",
  borderRadius: "var(--radius-control)",
} as const;

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
