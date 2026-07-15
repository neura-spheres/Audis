import { render, screen, waitFor } from "@testing-library/react";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { AboutView } from "./AboutView";
import { AUDIS_APP_INFO_MOCK } from "@/test/fixtures";

/**
 * Exercises the whole identity path: a mocked get_app_info, through the
 * validating IPC layer and useAppInfo, to the DOM. If the Rust payload and the
 * frontend schema disagree, this fails at the boundary rather than rendering
 * undefined.
 */
describe("AboutView", () => {
  afterEach(() => {
    clearMocks();
  });

  it("shows the identity returned by the backend", async () => {
    mockIPC((command) => {
      if (command === "get_app_info") return AUDIS_APP_INFO_MOCK;
      throw new Error(`unexpected command ${command}`);
    });

    render(<AboutView />);

    expect(await screen.findByRole("heading", { name: "Audis" })).toBeInTheDocument();
    expect(screen.getByText("Hear more. Understand faster.")).toBeInTheDocument();
    expect(screen.getByText("ai.neura.audis")).toBeInTheDocument();
    // Publisher and company are both Neura Audis, so it appears twice.
    expect(screen.getAllByText("Neura Audis")).toHaveLength(2);
  });

  it("renders the error card when the command fails", async () => {
    mockIPC((command) => {
      if (command === "get_app_info") {
        return Promise.reject({
          title: "Audis could not reach its storage folder",
          explanation: "A folder Audis needs could not be opened.",
          dataPreserved: true,
          suggestedAction: "Check the storage folder and try again.",
          technicalDetails: null,
          diagnosticCode: "STORAGE_UNAVAILABLE",
        });
      }
      throw new Error(`unexpected command ${command}`);
    });

    render(<AboutView />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
    expect(screen.getByText("Your data was not affected.")).toBeInTheDocument();
  });
});
