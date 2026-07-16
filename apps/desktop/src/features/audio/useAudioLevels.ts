import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { AUDIS_EVENTS } from "@/services/events";
import { audioLevelEventSchema, type AudioLevelEvent, type AudioSourceKind } from "@/schemas/ipc";

type Levels = Partial<Record<AudioSourceKind, AudioLevelEvent>>;

/** Subscribe to `audis://audio/level`. */
export function useAudioLevels(active: boolean): Levels {
  const [levels, setLevels] = useState<Levels>({});

  useEffect(() => {
    if (!active) {
      setLevels({});
      return;
    }

    let cancelled = false;
    const unlisten = listen(AUDIS_EVENTS.audioLevel, (event) => {
      const parsed = audioLevelEventSchema.safeParse(event.payload);
      if (!parsed.success) return;
      setLevels((current) => ({ ...current, [parsed.data.source]: parsed.data }));
    });

    return () => {
      cancelled = true;
      void unlisten.then((stop) => {
        if (cancelled) stop();
      });
    };
  }, [active]);

  return levels;
}
