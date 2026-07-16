import { useEffect, useState } from "react";

import { getAppInfo, AudisIpcError } from "@/services/ipc";
import type { AppInfo, UserFacingError } from "@/schemas/ipc";

/** Load state for {@link useAppInfo}. */
export type AppInfoState =
  | { status: "loading" }
  | { status: "ready"; info: AppInfo }
  | { status: "error"; error: UserFacingError };

/** Fetch identity and build information from the Rust core. */
export function useAppInfo(): AppInfoState {
  const [state, setState] = useState<AppInfoState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    getAppInfo()
      .then((info) => {
        if (active) setState({ status: "ready", info });
      })
      .catch((cause: unknown) => {
        if (!active) return;
        setState({
          status: "error",
          error:
            cause instanceof AudisIpcError
              ? cause.userFacing
              : {
                  title: "Audis could not load its details",
                  explanation:
                    "Audis could not read its own version information. Your data was not affected.",
                  dataPreserved: true,
                  suggestedAction: "Restart Audis.",
                  technicalDetails: String(cause),
                  diagnosticCode: "UNEXPECTED",
                },
        });
      });

    return () => {
      active = false;
    };
  }, []);

  return state;
}
