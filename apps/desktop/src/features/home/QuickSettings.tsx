import type { ReactNode } from "react";

import type { Navigate } from "@/app/navigation";
import { SegmentedControl } from "@/components/controls";
import {
  AiIcon,
  GlobeIcon,
  MicIcon,
  RecordIcon,
  SpeakerIcon,
  WaveformIcon,
} from "@/components/icons";
import { useSettings } from "@/hooks/useSettings";
import type { Language, Settings } from "@/schemas/ipc";

const LOCAL_MODEL_NAMES: Record<string, string> = {
  whisperTiny: "Whisper Tiny",
  whisperBase: "Whisper Base",
  whisperSmall: "Whisper Small",
  whisperMedium: "Whisper Medium",
};

const PROVIDER_NAMES: Record<string, string> = {
  gemini: "Google Gemini",
  groq: "Groq",
  deepgram: "Deepgram",
  deepSeek: "DeepSeek",
  anthropic: "Anthropic",
  openAiCompatible: "Custom provider",
};

/**
 * The handful of things people change between sessions, in one place, as
 * Control Center gathers them: what to listen to, in which language, whether to
 * record, and whether the assistant is on. Everything else lives in Settings.
 */
export function QuickSettings({
  onNavigate,
  running,
}: {
  onNavigate: Navigate;
  /** A session is running, so changes wait for the next one. */
  running: boolean;
}) {
  const { settings, update } = useSettings();
  if (!settings) return <div className="audis-skeleton h-[188px]" />;

  const t = settings.transcription;
  const set = (change: (current: Settings) => Settings) => void update(change);
  const noSource = !t.captureMicrophone && !t.captureComputerAudio;

  return (
    <div className="flex flex-col gap-2">
      <div className="grid grid-cols-3 gap-2.5">
        <ToggleTile
          label="Microphone"
          icon={<MicIcon />}
          on={t.captureMicrophone}
          detail={t.captureMicrophone ? "Listening to you" : "Off"}
          onToggle={(captureMicrophone) =>
            set((c) => ({ ...c, transcription: { ...c.transcription, captureMicrophone } }))
          }
        />
        <ToggleTile
          label="Computer audio"
          icon={<SpeakerIcon />}
          on={t.captureComputerAudio}
          detail={t.captureComputerAudio ? "Calls and videos" : "Off"}
          onToggle={(captureComputerAudio) =>
            set((c) => ({ ...c, transcription: { ...c.transcription, captureComputerAudio } }))
          }
        />
        <ToggleTile
          label="Record audio"
          icon={<RecordIcon />}
          on={settings.recording.enabled}
          tone="danger"
          detail={settings.recording.enabled ? "Saved as Opus" : "Not recorded"}
          onToggle={(enabled) => set((c) => ({ ...c, recording: { ...c.recording, enabled } }))}
        />

        <Tile>
          <TileHeading icon={<GlobeIcon />} label="Language" />
          <SegmentedControl<Language>
            label="Recognition language"
            value={t.language}
            options={[
              { id: "indonesian", label: "Indonesian" },
              { id: "english", label: "English" },
            ]}
            onChange={(language) =>
              set((c) => ({ ...c, transcription: { ...c.transcription, language } }))
            }
          />
        </Tile>

        <button
          type="button"
          onClick={() => onNavigate("speech")}
          aria-label="Change the speech engine"
          className="audis-pressable flex flex-col items-start gap-2 p-3 text-left"
          style={tileStyle}
        >
          <TileHeading icon={<WaveformIcon />} label="Speech engine" />
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-footnote font-semibold">{engineName(settings)}</span>
            <span className="truncate text-caption1" style={{ color: "var(--label-secondary)" }}>
              {t.engine.kind === "local" ? "On this PC" : t.engine.model}
            </span>
          </span>
        </button>

        <ToggleTile
          label="AI assistant"
          icon={<AiIcon />}
          on={settings.assistant.enabled}
          detail={settings.assistant.enabled ? "Answers questions" : "Off"}
          onToggle={(enabled) => set((c) => ({ ...c, assistant: { ...c.assistant, enabled } }))}
        />
      </div>

      {noSource ? (
        <p className="px-1 text-footnote" style={{ color: "var(--color-warning)" }}>
          Turn on the microphone or computer audio: a session needs something to listen to.
        </p>
      ) : running ? (
        <p className="px-1 text-footnote" style={{ color: "var(--label-tertiary)" }}>
          Changes here apply to the next session you start.
        </p>
      ) : null}
    </div>
  );
}

function engineName(settings: Settings): string {
  const engine = settings.transcription.engine;
  if (engine.kind === "local") return LOCAL_MODEL_NAMES[engine.model] ?? "Whisper";
  return PROVIDER_NAMES[engine.provider] ?? "Cloud provider";
}

const tileStyle = {
  background: "var(--surface-content)",
  borderRadius: "var(--radius-tile)",
  boxShadow: "var(--shadow-card)",
} as const;

function Tile({ children }: { children: ReactNode }) {
  return (
    <div className="flex flex-col items-start gap-2 p-3" style={tileStyle}>
      {children}
    </div>
  );
}

function TileHeading({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <span
      className="flex items-center gap-1.5 text-caption1 font-semibold [&>svg]:h-3.5 [&>svg]:w-3.5"
      style={{ color: "var(--label-secondary)" }}
    >
      {icon}
      {label}
    </span>
  );
}

/** A Control Center module: the whole tile is the switch. */
function ToggleTile({
  label,
  icon,
  on,
  detail,
  onToggle,
  tone = "accent",
}: {
  label: string;
  icon: ReactNode;
  on: boolean;
  detail: string;
  onToggle: (next: boolean) => void;
  tone?: "accent" | "danger";
}) {
  const colour = tone === "danger" ? "var(--color-listening)" : "var(--color-accent)";

  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onToggle(!on)}
      className="audis-pressable flex items-center gap-2.5 p-3 text-left"
      style={tileStyle}
    >
      <span
        aria-hidden
        className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full [&>svg]:h-4 [&>svg]:w-4"
        style={{
          background: on ? colour : "var(--surface-sunken)",
          color: on ? "#ffffff" : "var(--label-secondary)",
          transform: on ? "scale(1)" : "scale(0.94)",
          transition: `background-color var(--duration-standard) var(--ease-standard), transform var(--spring-bouncy-duration) var(--spring-bouncy)`,
        }}
      >
        {icon}
      </span>
      <span className="flex min-w-0 flex-col">
        <span className="truncate text-footnote font-semibold">{label}</span>
        <span className="truncate text-caption1" style={{ color: "var(--label-secondary)" }}>
          {detail}
        </span>
      </span>
    </button>
  );
}
