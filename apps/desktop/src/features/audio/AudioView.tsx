import { useCallback, useEffect, useState } from "react";

import { GroupedList, Row } from "@/components/GroupedList";
import { ErrorNotice } from "@/components/ErrorNotice";
import { Button } from "@/components/controls";
import { LevelMeter } from "./LevelMeter";
import { useAudioLevels } from "./useAudioLevels";
import { listAudioDevices, startAudioTest, stopAudioTest, AudisIpcError } from "@/services/ipc";
import type { AudioDevice, AudioDevices, AudioTestStatus, UserFacingError } from "@/schemas/ipc";

/**
 * Audio devices and the live capture test.
 *
 * This is real: it enumerates Windows endpoints, opens the microphone and a
 * WASAPI loopback capture of the chosen output, and shows both live levels.
 */
export function AudioView() {
  const [devices, setDevices] = useState<AudioDevices>();
  const [micId, setMicId] = useState<string | null>(null);
  const [outId, setOutId] = useState<string | null>(null);
  const [status, setStatus] = useState<AudioTestStatus>();
  const [error, setError] = useState<UserFacingError>();
  const [testing, setTesting] = useState(false);

  const levels = useAudioLevels(testing);

  useEffect(() => {
    listAudioDevices()
      .then(setDevices)
      .catch((cause: unknown) => setError(toUserFacing(cause)));
  }, []);

  // Releasing the devices when the user navigates away matters: holding the
  // microphone open would keep the Windows "in use" indicator lit and could
  // block other apps.
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

  // Changing a device mid-test restarts capture on the new one, which is what
  // clicking a picker obviously ought to do.
  const pick = (kind: "mic" | "out", id: string | null) => {
    if (kind === "mic") setMicId(id);
    else setOutId(id);
    if (testing) {
      startAudioTest(kind === "mic" ? id : micId, kind === "out" ? id : outId)
        .then(setStatus)
        .catch((cause: unknown) => setError(toUserFacing(cause)));
    }
  };

  return (
    <div className="flex flex-col gap-8">
      {error ? <ErrorNotice error={error} /> : null}

      <GroupedList
        title="Microphone"
        footnote="Your microphone is captured as its own source, so you are never mixed in with the people you are listening to."
      >
        <Row
          label="Device"
          stacked
          value={
            <DevicePicker
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
                : "Start the test to see live levels."
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

      <GroupedList
        title="Computer audio"
        footnote="Audis listens to what this PC is playing. No virtual audio cable or extra driver is needed. Play something and the meter should move."
      >
        <Row
          label="Capture from"
          stacked
          value={
            <DevicePicker
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
                : "Start the test to see live levels."
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

      <div className="flex items-center gap-3">
        {testing ? (
          <Button onClick={stop} variant="danger">
            Stop test
          </Button>
        ) : (
          <Button onClick={start} variant="accent">
            Start audio test
          </Button>
        )}
        <span className="text-footnote" style={{ color: "var(--label-secondary)" }}>
          {testing
            ? "Audis is listening. Both devices are open."
            : "Nothing is being recorded. The test only shows levels."}
        </span>
      </div>

      {testing && isSilent(levels.microphone?.silenceDurationMs) ? (
        <p className="px-3 text-footnote" style={{ color: "var(--color-warning)" }}>
          The microphone has been silent for a while. Check that the right device is selected and
          that microphone access is allowed in Windows privacy settings.
        </p>
      ) : null}
    </div>
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

/**
 * Device picker.
 *
 * A native select rather than a custom menu: it inherits Windows keyboard
 * behaviour, screen-reader support and overflow handling for free, and this
 * list can be long.
 */
function DevicePicker({
  devices,
  value,
  onChange,
}: {
  devices: readonly AudioDevice[];
  value: string | null;
  onChange: (id: string | null) => void;
}) {
  return (
    <select
      value={value ?? ""}
      onChange={(event) => onChange(event.target.value === "" ? null : event.target.value)}
      className="w-full px-2.5 py-[5px] text-footnote"
      style={{
        background: "var(--surface-elevated)",
        color: "var(--label-primary)",
        border: "0.5px solid var(--border-control)",
        borderRadius: "var(--radius-control)",
      }}
    >
      <option value="">
        {devices.find((device) => device.isDefault)
          ? `Windows default (${devices.find((device) => device.isDefault)?.name})`
          : "Windows default"}
      </option>
      {devices.map((device) => (
        <option key={device.id} value={device.id}>
          {device.name} ({device.sampleRate} Hz, {channelLabel(device.channels)})
        </option>
      ))}
    </select>
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
