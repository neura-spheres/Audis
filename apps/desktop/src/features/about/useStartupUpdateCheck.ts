import { useEffect, useRef } from "react";

import { useSettings } from "@/hooks/useSettings";
import { checkForUpdates } from "@/services/ipc";

/**
 * Look for a new version once on launch, when the user asked us to.
 *
 * The answer is broadcast on `audis://update/status` rather than returned, so
 * whichever view cares can show it. A failed check is ignored on purpose: being
 * offline is not something to interrupt someone about.
 */
export function useStartupUpdateCheck() {
  const { settings } = useSettings();
  const checked = useRef(false);
  const enabled = settings?.updates.checkOnStartup ?? false;

  useEffect(() => {
    if (!enabled || checked.current) return;
    checked.current = true;
    checkForUpdates().catch(() => undefined);
  }, [enabled]);
}
