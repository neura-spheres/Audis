import { useCallback, useEffect, useState } from "react";

import { AUDIS_EVENTS, subscribe } from "@/services/events";
import { getSettings, updateSettings, AudisIpcError } from "@/services/ipc";
import { settingsSchema, type Settings, type UserFacingError } from "@/schemas/ipc";
import { applyTheme, resolveTheme } from "@/stores/theme";

interface UseSettingsResult {
  settings: Settings | undefined;
  error: UserFacingError | undefined;
  /** Apply a change and persist it. Rust owns the durable copy. */
  update: (change: (current: Settings) => Settings) => Promise<void>;
}

/** Load settings from Rust and write changes back. */
export function useSettings(): UseSettingsResult {
  const [settings, setSettings] = useState<Settings>();
  const [error, setError] = useState<UserFacingError>();

  useEffect(() => {
    let active = true;

    getSettings()
      .then((loaded) => {
        if (!active) return;
        setSettings(loaded);
        applyTheme(resolveTheme(loaded.general.theme));
      })
      .catch((cause: unknown) => {
        if (active) setError(toUserFacing(cause));
      });

    // Every copy of the settings follows the saved one. Home and a settings
    // pane can both be editing the same value, and the tray and overlays change
    // settings too; without this each would keep showing what it last loaded.
    const stop = subscribe(AUDIS_EVENTS.settingsChanged, (payload) => {
      const parsed = settingsSchema.safeParse(payload);
      if (active && parsed.success) setSettings(parsed.data);
    });

    return () => {
      active = false;
      stop();
    };
  }, []);

  const update = useCallback(
    async (change: (current: Settings) => Settings) => {
      if (!settings) return;

      const next = change(settings);
      const previous = settings;

      setSettings(next);
      applyTheme(resolveTheme(next.general.theme));

      try {
        const saved = await updateSettings(next);
        setSettings(saved);
        setError(undefined);
      } catch (cause) {
        setSettings(previous);
        applyTheme(resolveTheme(previous.general.theme));
        setError(toUserFacing(cause));
      }
    },
    [settings],
  );

  return { settings, error, update };
}

function toUserFacing(cause: unknown): UserFacingError {
  if (cause instanceof AudisIpcError) return cause.userFacing;
  return {
    title: "Audis could not load your settings",
    explanation: "Your settings could not be read. Your sessions were not affected.",
    dataPreserved: true,
    suggestedAction: "Restart Audis.",
    technicalDetails: String(cause),
    diagnosticCode: "UNEXPECTED",
  };
}
