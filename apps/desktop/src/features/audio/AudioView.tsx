import { useCallback, useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { ErrorNotice } from "@/components/ErrorNotice";
import { IconBadge } from "@/components/IconBadge";
import { MicIcon, SpeakerIcon } from "@/components/icons";
import { Page } from "@/components/Page";
import { Button, Select, Switch } from "@/components/controls";
import { useSettings } from "@/hooks/useSettings";
import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { LevelMeter } from "./LevelMeter";
import { useAudioLevels } from "./useAudioLevels";
import { listAudioDevices, startAudioTest, stopAudioTest, AudisIpcError } from "@/services/ipc";
import type { AudioDevice, AudioDevices, AudioTestStatus, UserFacingError } from "@/schemas/ipc";

/** Audio devices and the live capture test. */
export function AudioView() {
  const { settings, update } = useSettings();
  const [devices, setDevices] = useState<AudioDevices>();
  const [status, setStatus] = useState<AudioTestStatus>();
  const [error, setError] = useState<UserFacingError>();
  const [testing, setTesting] = useState(false);

  const levels = useAudioLevels(testing);

  const micId = settings?.audio.microphoneId ?? null;
  const outId = settings?.audio.computerAudioId ?? null;

  useEffect(() => {
    const load = () =>
      listAudioDevices()
        .then(setDevices)
        .catch((cause: unknown) => setError(toUserFacing(cause)));
    load();
    return subscribe(AUDIS_EVENTS.audioDeviceChange, load);
  }, []);

  useEffect(() => {
    return () => {
      void stopAudioTest().catch(() => undefined);
    };
  }, []);

  const start = useCallback(() => {
    startAudioTest(micId, outId)
      .then((result) => {
        setStatus(result);
        setTesting(true);
        setError(undefined);
      })
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, [micId, outId]);

  const stop = useCallback(() => {
    stopAudioTest()
      .then(() => {
        setTesting(false);
        setStatus(undefined);
      })
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  const pick = (kind: "mic" | "out", id: string | null) => {
    const nextMic = kind === "mic" ? id : micId;
    const nextOut = kind === "out" ? id : outId;

    update((current) => ({
      ...current,
      audio: { microphoneId: nextMic, computerAudioId: nextOut },
    }));

    if (testing) {
      startAudioTest(nextMic, nextOut)
        .then(setStatus)
        .catch((cause: unknown) => setError(toUserFacing(cause)));
    }
  };

  if (!settings) return null;

  const t = settings.transcription;

  return (
    <Page
      title="Audio"
      subtitle="Your microphone and the computer's sound are captured separately, so you are never mixed in with the people you are listening to."
      accessory={
        testing ? (
          <Button onClick={stop} variant="danger">
            Stop test
          </Button>
        ) : (
          <Button onClick={start} variant="accent">
            Test audio
          </Button>
        )
      }
    >
      {error ? <ErrorNotice error={error} /> : null}

      <GroupedList
        title="Listen to"
        footnote={
          !t.captureMicrophone && !t.captureComputerAudio ? (
            <span style={{ color: "var(--color-warning)" }}>
              Audis needs at least one of these switched on before a session can start.
            </span>
          ) : (
            "Used by every session you start."
          )
        }
      >
        <Row
          icon={
            <IconBadge tint="red" size={30}>
              <MicIcon />
            </IconBadge>
          }
          label="Your microphone"
          description="What you say. Always labelled as you, never guessed."
          value={
            <Switch
              label="Capture your microphone"
              checked={t.captureMicrophone}
              onChange={(captureMicrophone) =>
                update((current) => ({
                  ...current,
                  transcription: { ...current.transcription, captureMicrophone },
                }))
              }
            />
          }
        />
        <Row
          icon={
            <IconBadge tint="blue" size={30}>
              <SpeakerIcon />
            </IconBadge>
          }
          label="Computer audio"
          description="Everyone else in a call, and any video you play. No virtual cable needed."
          value={
            <Switch
              label="Capture computer audio"
              checked={t.captureComputerAudio}
              onChange={(captureComputerAudio) =>
                update((current) => ({
                  ...current,
                  transcription: { ...current.transcription, captureComputerAudio },
                }))
              }
            />
          }
        />
      </GroupedList>

      <GroupedList title="Microphone" footnote="Your choice is saved and used for every session.">
        <Row
          label="Device"
          stacked
          value={
            <DevicePicker
              label="Microphone device"
              devices={devices?.inputs ?? []}
              value={micId}
              onChange={(id) => pick("mic", id)}
            />
          }
        />
        <Row
          label="Level"
          description={
            status?.microphone
              ? `${status.microphone.deviceName} · ${status.microphone.sampleRate} Hz · ${channelLabel(status.microphone.channels)}`
              : testing
                ? "Not capturing."
                : "Test audio to see live levels. Nothing is recorded."
          }
          stacked
          value={<LevelMeter level={levels.microphone} />}
        />
        {status?.microphoneError ? (
          <Row
            label="Problem"
            description={status.microphoneError.explanation}
            value={<span style={{ color: "var(--color-danger)" }}>Failed</span>}
          />
        ) : null}
      </GroupedList>

      {testing && isSilent(levels.microphone?.silenceDurationMs) ? (
        <p className="-mt-4 px-3 text-footnote" style={{ color: "var(--color-warning)" }}>
          The microphone has been silent for a while. Check that the right device is selected and
          that microphone access is allowed in Windows privacy settings.
        </p>
      ) : null}

      <GroupedList title="Computer audio" footnote="Play something and the meter should move.">
        <Row
          label="Capture from"
          stacked
          value={
            <DevicePicker
              label="Computer audio device"
              devices={devices?.outputs ?? []}
              value={outId}
              onChange={(id) => pick("out", id)}
            />
          }
        />
        <Row
          label="Level"
          description={
            status?.computerAudio
              ? `${status.computerAudio.deviceName} · ${status.computerAudio.sampleRate} Hz · ${channelLabel(status.computerAudio.channels)}`
              : testing
                ? "Not capturing."
                : "Test audio to see live levels. Nothing is recorded."
          }
          stacked
          value={<LevelMeter level={levels.computerAudio} />}
        />
        {status?.computerAudioError ? (
          <Row
            label="Problem"
            description={status.computerAudioError.explanation}
            value={<span style={{ color: "var(--color-danger)" }}>Failed</span>}
          />
        ) : null}
      </GroupedList>
    </Page>
  );
}

function isSilent(silenceMs: number | undefined): boolean {
  return silenceMs !== undefined && silenceMs > 4000;
}

function channelLabel(channels: number): string {
  if (channels === 1) return "mono";
  if (channels === 2) return "stereo";
  return `${channels} channels`;
}

/** Device picker. */
function DevicePicker({
  label,
  devices,
  value,
  onChange,
}: {
  label: string;
  devices: readonly AudioDevice[];
  value: string | null;
  onChange: (id: string | null) => void;
}) {
  const fallback = devices.find((device) => device.isDefault);
  return (
    <Select
      wide
      label={label}
      value={value ?? ""}
      onChange={(next) => onChange(next === "" ? null : next)}
    >
      <option value="">
        {fallback ? `Windows default (${fallback.name})` : "Windows default"}
      </option>
      {devices.map((device) => (
        <option key={device.id} value={device.id}>
          {device.name} ({device.sampleRate} Hz, {channelLabel(device.channels)})
        </option>
      ))}
    </Select>
  );
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not reach your audio devices",
    explanation: "Something went wrong talking to Windows audio. Nothing was recorded.",
    dataPreserved: true,
    suggestedAction: "Try again.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
