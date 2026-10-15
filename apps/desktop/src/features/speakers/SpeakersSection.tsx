import { useEffect, useState } from "react";

import { Collapse } from "@/components/Collapse";
import { GroupedList, Row } from "@/components/GroupedList";
import { IconBadge } from "@/components/IconBadge";
import { SpeakerIcon } from "@/components/icons";
import { SegmentedControl, Switch } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { sessionStatusSchema, speakerUpdateSchema, type SpeakerUpdate } from "@/schemas/ipc";
import type { SpeakerSettings } from "@/schemas/ipc";

const EXPECTED: { id: string; label: string; value: number }[] = [
  { id: "auto", label: "Auto", value: 0 },
  { id: "2", label: "2", value: 2 },
  { id: "3", label: "3", value: 3 },
  { id: "4", label: "4", value: 4 },
  { id: "6", label: "6", value: 6 },
];

/**
 * Telling the remote voices apart. Your microphone is already known to be you,
 * so only the computer's audio needs separating.
 */
export function SpeakersSection() {
  const { settings, update } = useSettings();
  if (!settings) return null;

  const { speakers } = settings;

  const set = (change: Partial<SpeakerSettings>) =>
    update((current) => ({ ...current, speakers: { ...current.speakers, ...change } }));

  const expected =
    EXPECTED.find((option) => option.value === speakers.expectedSpeakers)?.id ?? "auto";

  return (
    <div className="flex flex-col gap-3">
      <GroupedList
        title="Speakers"
        footnote="Audis never infers identity, age, gender or ethnicity from a voice. Live labels are provisional and can change as more audio arrives. Renaming, merging and saved voice profiles are not built yet."
      >
        <Row
          icon={
            <IconBadge tint="orange" size={30}>
              <SpeakerIcon />
            </IconBadge>
          }
          label="Separate remote speakers"
          description="Give each distinct voice in your computer's audio its own label."
          value={
            <Switch
              label="Separate remote speakers"
              checked={speakers.enabled}
              onChange={(enabled) => set({ enabled })}
            />
          }
        />
        {speakers.enabled ? (
          <Row
            label="Expected number of speakers"
            description="A hint, not a promise. Auto lets Audis decide as it listens."
            value={
              <SegmentedControl<string>
                label="Expected number of speakers"
                value={expected}
                options={EXPECTED.map((option) => ({ id: option.id, label: option.label }))}
                onChange={(id) =>
                  set({ expectedSpeakers: EXPECTED.find((o) => o.id === id)?.value ?? 0 })
                }
              />
            }
          />
        ) : null}
      </GroupedList>

      <Collapse open={speakers.enabled}>
        <LiveRoster />
      </Collapse>
    </div>
  );
}

function LiveRoster() {
  const [speakers, setSpeakers] = useState<SpeakerUpdate[]>([]);
  const [live, setLive] = useState(false);

  useEffect(() => {
    const stopSpeaker = subscribe(AUDIS_EVENTS.speakerUpdate, (payload) => {
      const parsed = speakerUpdateSchema.safeParse(payload);
      if (!parsed.success) return;
      setSpeakers((current) =>
        current.some((entry) => entry.id === parsed.data.id) ? current : [...current, parsed.data],
      );
    });

    const stopSession = subscribe(AUDIS_EVENTS.sessionState, (payload) => {
      const parsed = sessionStatusSchema.safeParse(payload);
      const state = parsed.success ? parsed.data.state : "idle";
      const active = state === "starting" || state === "listening" || state === "paused";
      setLive(active);
      if (!active) setSpeakers([]);
    });

    return () => {
      stopSpeaker();
      stopSession();
    };
  }, []);

  if (!live) {
    return (
      <div
        className="flex flex-col gap-1 p-3"
        style={{
          background: "var(--surface-content)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
        }}
      >
        <span className="text-subheadline">Speakers heard</span>
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          Start a session with your computer's audio to see the remote speakers appear here.
        </span>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col gap-2 p-3"
      style={{
        background: "var(--surface-content)",
        borderRadius: "var(--radius-card)",
        boxShadow: "var(--shadow-card)",
      }}
    >
      <span className="text-subheadline">Speakers heard</span>
      {speakers.length === 0 ? (
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          Listening… speakers appear as distinct voices are heard.
        </span>
      ) : (
        <ul className="flex flex-wrap gap-2">
          {speakers.map((speaker) => (
            <li
              key={speaker.id}
              className="px-2.5 py-1 text-footnote font-medium"
              style={{
                background: "var(--surface-sunken)",
                borderRadius: "var(--radius-chip)",
                color: "var(--label-primary)",
                animation: "audis-pop-up var(--spring-bouncy-duration) var(--spring-bouncy) both",
              }}
            >
              {speaker.label}
            </li>
          ))}
        </ul>
      )}
      <span className="text-caption2" style={{ color: "var(--label-tertiary)" }}>
        Provisional. These can change as more of the conversation is heard.
      </span>
    </div>
  );
}
