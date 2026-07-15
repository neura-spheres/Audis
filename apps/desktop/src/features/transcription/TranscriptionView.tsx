import { useEffect, useState } from "react";

import { ErrorNotice } from "@/components/ErrorNotice";
import { SegmentedControl, Switch } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import { listModels, listProviders } from "@/services/ipc";
import type {
  InstalledModel,
  Language,
  ModelId,
  ProviderStatus,
  TranscriptionEngine,
} from "@/schemas/ipc";

/**
 * Speech recognition settings.
 *
 * Recognition runs on this PC, so everything here is about which model and
 * which language, not about an account or a quota.
 */
export function TranscriptionView() {
  const { settings, error, update } = useSettings();
  const [models, setModels] = useState<InstalledModel[]>();
  const [providers, setProviders] = useState<ProviderStatus[]>();

  useEffect(() => {
    listModels()
      .then(setModels)
      .catch(() => undefined);
    listProviders()
      .then(setProviders)
      .catch(() => undefined);
  }, []);

  if (error) return <ErrorNotice error={error} />;
  if (!settings) return null;

  const { transcription } = settings;
  const installed = (models ?? []).filter((model) => model.installed);
  const selectedIsInstalled = installed.some((model) => model.info.id === transcription.model);
  const engine = transcription.engine;

  // Only providers that can actually hear, and only ones with a key saved:
  // offering the rest would move the failure to the moment you start talking.
  const speechProviders = (providers ?? []).filter(
    (provider) => provider.info.speech !== null && provider.hasKey,
  );

  const setEngine = (next: TranscriptionEngine) =>
    update((current) => ({
      ...current,
      transcription: {
        ...current.transcription,
        engine: next,
        // Remember the local model while a provider is selected, so switching
        // back does not silently reset it.
        model: next.kind === "local" ? next.model : current.transcription.model,
      },
    }));

  return (
    <div className="flex flex-col gap-5">
      <section className="flex flex-col gap-3">
        <Row
          label="Recognise speech with"
          help={
            engine.kind === "local"
              ? "Whisper on this PC. Free, works offline, and your voice never leaves the machine."
              : "A provider over the internet. More accurate than anything this PC can run live, and your audio is sent to them."
          }
        >
          <SegmentedControl<string>
            label="Speech engine"
            value={engine.kind}
            options={[
              { id: "local", label: "This PC" },
              { id: "cloud", label: "Provider" },
            ]}
            onChange={(kind) => {
              if (kind === "local") {
                setEngine({ kind: "local", model: transcription.model });
                return;
              }
              const first = speechProviders[0];
              if (!first?.info.speech) return;
              setEngine({
                kind: "cloud",
                provider: first.info.id,
                model: first.info.speech.defaultModel,
              });
            }}
          />
        </Row>

        {engine.kind === "cloud" ? (
          <p
            className="px-3 py-2 text-footnote"
            style={{
              background: "color-mix(in srgb, var(--color-warning) 12%, transparent)",
              borderRadius: "var(--radius-control)",
              color: "var(--label-secondary)",
            }}
          >
            Your audio is uploaded to this provider to be transcribed. It needs an internet
            connection, and captions stop if it goes down. Switch back to This PC to keep everything
            offline.
          </p>
        ) : null}

        {engine.kind === "cloud" && speechProviders.length === 0 ? (
          <p className="px-1 text-footnote" style={{ color: "var(--color-warning)" }}>
            No provider with a saved key can transcribe speech. Open Providers and add a key for
            Groq or Gemini; both have free tiers.
          </p>
        ) : null}

        {engine.kind === "cloud" && speechProviders.length > 0 ? (
          <>
            <Row label="Provider" help={providerSummary(speechProviders, engine)}>
              <select
                value={engine.provider}
                onChange={(event) => {
                  const chosen = speechProviders.find(
                    (provider) => provider.info.id === event.target.value,
                  );
                  if (!chosen?.info.speech) return;
                  setEngine({
                    kind: "cloud",
                    provider: chosen.info.id,
                    model: chosen.info.speech.defaultModel,
                  });
                }}
                className="px-2.5 py-[5px] text-footnote"
                style={selectStyle}
              >
                {speechProviders.map((provider) => (
                  <option key={provider.info.id} value={provider.info.id}>
                    {provider.info.name}
                  </option>
                ))}
              </select>
            </Row>

            <Row label="Provider model" help="Which of the provider's speech models to use.">
              <select
                value={engine.model}
                onChange={(event) =>
                  setEngine({
                    kind: "cloud",
                    provider: engine.provider,
                    model: event.target.value,
                  })
                }
                className="px-2.5 py-[5px] text-footnote"
                style={selectStyle}
              >
                {(
                  speechProviders.find((provider) => provider.info.id === engine.provider)?.info
                    .speech?.models ?? []
                ).map((model) => (
                  <option key={model} value={model}>
                    {model}
                  </option>
                ))}
              </select>
            </Row>
          </>
        ) : null}
        <Row
          label="Language"
          help="Audis recognises Indonesian and English. Tell it which one you are speaking: choosing is more accurate than letting it guess, especially when you mix in English words."
        >
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

        {engine.kind === "local" ? (
          <Row
            label="Speech model"
            help="Bigger models are more accurate and slower. Base suits most people."
          >
            {installed.length === 0 ? (
              <span className="text-footnote" style={{ color: "var(--color-warning)" }}>
                No model installed. Open Models.
              </span>
            ) : (
              <select
                value={transcription.model}
                onChange={(event) =>
                  update((current) => ({
                    ...current,
                    transcription: {
                      ...current.transcription,
                      model: event.target.value as ModelId,
                    },
                  }))
                }
                className="px-2.5 py-[5px] text-footnote"
                style={{
                  background: "var(--surface-elevated)",
                  color: "var(--label-primary)",
                  border: "0.5px solid var(--border-control)",
                  borderRadius: "var(--radius-control)",
                }}
              >
                {installed.map((model) => (
                  <option key={model.info.id} value={model.info.id}>
                    {model.info.name}
                  </option>
                ))}
              </select>
            )}
          </Row>
        ) : null}

        {/* The selected model can be one that was since removed. Saying so beats
            letting the user find out when a session refuses to start. */}
        {engine.kind === "local" && installed.length > 0 && !selectedIsInstalled ? (
          <p className="px-1 text-footnote" style={{ color: "var(--color-warning)" }}>
            The model you chose is no longer installed. Pick one above, or reinstall it from Models.
          </p>
        ) : null}
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="px-1 text-subheadline font-semibold">What to listen to</h2>

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

        {/* A session with nothing to listen to cannot start. Say it here rather
            than failing at the point the user commits to a meeting. */}
        {!transcription.captureMicrophone && !transcription.captureComputerAudio ? (
          <p className="px-1 text-footnote" style={{ color: "var(--color-warning)" }}>
            Audis needs at least one of these switched on before a session can start.
          </p>
        ) : null}
      </section>

      {/* This claim used to be unconditional. It is only true for one of the
          two engines now, and printing it while uploading audio would be the
          worst kind of wrong. */}
      <p className="px-1 text-footnote" style={{ color: "var(--label-tertiary)" }}>
        {engine.kind === "local"
          ? "Recognition runs entirely on this PC. Nothing you say is sent anywhere, and it works with no internet connection."
          : "While a provider is selected, your audio leaves this PC to be transcribed. Everything else about Audis stays local."}
      </p>
    </div>
  );
}

function Row({
  label,
  help,
  children,
}: {
  label: string;
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
        <span className="text-subheadline">{label}</span>
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {help}
        </span>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

const selectStyle = {
  background: "var(--surface-elevated)",
  color: "var(--label-primary)",
  border: "0.5px solid var(--border-control)",
  borderRadius: "var(--radius-control)",
} as const;

/** What the selected provider is like, in the catalogue's own words. */
function providerSummary(
  providers: ProviderStatus[],
  engine: Extract<TranscriptionEngine, { kind: "cloud" }>,
): string {
  const provider = providers.find((candidate) => candidate.info.id === engine.provider);
  return provider?.info.speech?.summary ?? "Which provider transcribes your audio.";
}
