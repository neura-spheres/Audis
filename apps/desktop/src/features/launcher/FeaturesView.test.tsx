import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FeaturesView } from "./FeaturesView";
import { AUDIS_FEATURES_MOCK, withAmbientIpc } from "@/test/fixtures";

describe("FeaturesView", () => {
  afterEach(() => {
    clearMocks();
  });

  function mockFeatures() {
    mockIPC(
      withAmbientIpc((command) => {
        if (command === "list_features") return AUDIS_FEATURES_MOCK;
        throw new Error(`unexpected ${command}`);
      }),
    );
  }

  it("lists every feature with its status", async () => {
    mockFeatures();
    render(<FeaturesView onNavigate={() => undefined} />);

    expect(await screen.findByText("Live Caption")).toBeInTheDocument();
    expect(screen.getByText("Meeting Assistant")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
    expect(screen.getByText("Needs setup")).toBeInTheDocument();
  });

  /// A blocked feature must not be startable: discovering the model is missing
  it("disables starting a feature that is not ready, and says why", async () => {
    mockFeatures();
    render(<FeaturesView onNavigate={() => undefined} />);
    await screen.findByText("Meeting Assistant");

    expect(screen.getByRole("button", { name: "Start Live Caption" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Start Meeting Assistant" })).toBeDisabled();
    expect(screen.getByText(/Connect an AI provider first/)).toBeInTheDocument();
  });

  it("sends the user to the page that resolves the blocker", async () => {
    mockFeatures();
    const navigate = vi.fn();
    render(<FeaturesView onNavigate={navigate} />);
    await screen.findByText("Meeting Assistant");

    await userEvent.click(screen.getByRole("button", { name: "Open Providers" }));

    expect(navigate).toHaveBeenCalledWith("providers");
  });

  /// The whole point of the launcher. A Start button that navigates instead of
  it("actually starts a session when Start is clicked", async () => {
    const started: unknown[] = [];
    mockIPC(
      withAmbientIpc((command, args) => {
        if (command === "list_features") return AUDIS_FEATURES_MOCK;
        if (command === "start_session") {
          started.push(args);
          return {
            id: "00000000-0000-0000-0000-000000000001",
            mode: "liveCaption",
            state: "listening",
            language: "english",
            elapsedMs: 0,
            microphone: true,
            computerAudio: true,
            captionsVisible: true,
            assistantEnabled: false,
            error: null,
          };
        }
        throw new Error(`unexpected ${command}`);
      }),
    );

    render(<FeaturesView onNavigate={() => undefined} />);
    await screen.findByText("Live Caption");

    await userEvent.click(screen.getByRole("button", { name: "Start Live Caption" }));

    await waitFor(() => {
      expect(started).toEqual([{ feature: "liveCaption" }]);
    });

    expect(await screen.findByRole("button", { name: "Stop Live Caption" })).toBeInTheDocument();
  });

  /// Two sessions would fight over the microphone.
  it("does not let a second session start while one is running", async () => {
    mockIPC(
      withAmbientIpc((command) => {
        if (command === "list_features") return AUDIS_FEATURES_MOCK;
        if (command === "start_session") {
          return {
            id: "00000000-0000-0000-0000-000000000001",
            mode: "liveCaption",
            state: "listening",
            language: "english",
            elapsedMs: 0,
            microphone: true,
            computerAudio: true,
            captionsVisible: true,
            assistantEnabled: false,
            error: null,
          };
        }
        throw new Error(`unexpected ${command}`);
      }),
    );

    render(<FeaturesView onNavigate={() => undefined} />);
    await screen.findByText("Live Caption");
    await userEvent.click(screen.getByRole("button", { name: "Start Live Caption" }));

    await screen.findByRole("button", { name: "Stop Live Caption" });
    expect(screen.getByRole("button", { name: "Start Meeting Assistant" })).toBeDisabled();
  });

  /// Users must be able to tell, before starting, whether text leaves the PC.
  it("marks which features send data to a cloud provider", async () => {
    mockFeatures();
    render(<FeaturesView onNavigate={() => undefined} />);
    await screen.findByText("Meeting Assistant");

    expect(screen.getAllByText("Uses cloud AI")).toHaveLength(1);
  });
});
