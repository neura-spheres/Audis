import { describe, expect, it } from "vitest";

import { settingsSchema } from "./ipc";

/**
 * `useSettings` reads settings, applies a change, and sends the whole object
 * back. Zod strips keys it does not know about, so any field missing from
 * `settingsSchema` is dropped on the way out and reset to its default by
 * serde's `default` on the way in.
 *
 * The damage is invisible and severe: toggling the theme would reset the user's
 * speech model and wipe their provider list, orphaning the API keys already in
 * the Windows Credential Manager. These tests pin the round-trip.
 */
describe("settings round-trip", () => {
  /** What Rust's `Settings::default()` serialises to, with values changed away
   *  from the defaults so a reset is visible rather than coincidentally equal. */
  const fromRust = {
    version: 1,
    general: {
      theme: "dark",
      startPage: "sessions",
      closeBehavior: "quit",
      showTrayIcon: false,
    },
    audio: {
      microphoneId: "mic-2",
      computerAudioId: null,
    },
    transcription: {
      engine: { kind: "local", model: "whisperSmall" },
      model: "whisperSmall",
      language: "indonesian",
      captureMicrophone: true,
      captureComputerAudio: false,
    },
    captions: {
      fontSize: 40,
      maxLines: 5,
      backgroundOpacity: 20,
      showSourceLabels: false,
      clickThrough: true,
    },
    shortcuts: {
      stopSession: "CmdOrCtrl+Shift+S",
      togglePause: null,
      toggleCaptions: null,
      askAssistant: null,
    },
    providers: [
      {
        id: "gemini",
        enabled: true,
        model: "gemini-2.0-flash",
        endpoint: null,
        credentialRef: "provider/gemini/default",
      },
    ],
  };

  it("keeps every field Rust sent", () => {
    const parsed = settingsSchema.parse(fromRust);
    expect(parsed).toEqual(fromRust);
  });

  /// The exact bug: a theme change must not reset transcription or providers.
  it("does not drop the model or providers when only the theme changes", () => {
    const parsed = settingsSchema.parse(fromRust);
    const next = { ...parsed, general: { ...parsed.general, theme: "light" as const } };

    // This is what would be sent back to Rust.
    const outbound = settingsSchema.parse(next);

    expect(outbound.transcription.model).toBe("whisperSmall");
    expect(outbound.providers).toHaveLength(1);
    expect(outbound.providers[0]?.credentialRef).toBe("provider/gemini/default");
    expect(outbound.audio.microphoneId).toBe("mic-2");
    expect(outbound.captions.fontSize).toBe(40);
  });

  /// The engine decides whether audio leaves the PC. If zod dropped it, a
  /// theme change would silently reset a deliberate provider choice back to
  /// local — or, far worse in the other direction, fail to preserve the user's
  /// intent about where their voice goes.
  it("preserves the speech engine, including a cloud one", () => {
    const cloud = {
      ...fromRust,
      transcription: {
        ...fromRust.transcription,
        engine: { kind: "cloud", provider: "groq", model: "whisper-large-v3" },
      },
    };

    const parsed = settingsSchema.parse(cloud);
    expect(parsed.transcription.engine).toEqual({
      kind: "cloud",
      provider: "groq",
      model: "whisper-large-v3",
    });
  });

  it("rejects settings that are missing a section", () => {
    const { transcription: _dropped, ...incomplete } = fromRust;
    expect(settingsSchema.safeParse(incomplete).success).toBe(false);
  });
});
