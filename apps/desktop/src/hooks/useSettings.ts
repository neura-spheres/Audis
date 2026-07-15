import { useCallback, useEffect, useState } from "react";

import { getSettings, updateSettings, AudisIpcError } from "@/services/ipc";
import type { Settings, UserFacingError } from "@/schemas/ipc";
import { applyTheme, resolveTheme } from "@/stores/theme";

interface UseSettingsResult {
  settings: Settings | undefined;
  error: UserFacingError | undefined;
  /** Apply a change and persist it. Rust owns the durable copy. */
  update: (change: (current: Settings) => Settings) => Promise<void>;
}

/**
 * Load settings from Rust and write changes back.
 *
 * The Rust store is the source of truth. State here is a cache that is updated
 * optimistically for a responsive UI and rolled back if the save fails, so the
 * screen never shows a setting that was not actually written.
 */
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

    return () => {
      active = false;
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
        // The save failed, so the UI must not keep showing the new value.
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
