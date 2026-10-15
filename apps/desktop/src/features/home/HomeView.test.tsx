import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";

import { HomeView } from "./HomeView";
import { AUDIS_FEATURES_MOCK, withAmbientIpc } from "@/test/fixtures";

const LISTENING = {
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

describe("HomeView", () => {
  afterEach(() => {
    clearMocks();
  });

  function mockFeatures(extra?: (command: string) => unknown) {
    mockIPC(
      withAmbientIpc((command) => {
        if (command === "list_features") return AUDIS_FEATURES_MOCK;
        if (command === "list_sessions") return [];
        const answer = extra?.(command);
        if (answer !== undefined) return answer;
        throw new Error(`unexpected ${command}`);
      }),
    );
  }

  it("shows every feature with what clicking it will do", async () => {
    mockFeatures();
    render(<HomeView onNavigate={() => undefined} />);

    expect(await screen.findByRole("button", { name: "Start Live Caption" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Set up Meeting Assistant" })).toBeEnabled();
    expect(screen.getByText("Needs setup")).toBeInTheDocument();
  });

  /// Users must be able to tell, before starting, whether text leaves the PC.
  it("marks which features send data to a cloud provider", async () => {
    mockFeatures();
    render(<HomeView onNavigate={() => undefined} />);
    await screen.findByRole("button", { name: "Start Live Caption" });

    expect(screen.getAllByText(/Uses cloud AI/)).toHaveLength(1);
  });

  /// A blocked feature is never a dead end: the tile and the checklist both
  /// lead to the page that fixes it.
  it("sends a feature that needs setup to the page that resolves it", async () => {
    mockFeatures();
    const navigate = vi.fn();
    render(<HomeView onNavigate={navigate} />);

    await userEvent.click(await screen.findByRole("button", { name: "Set up Meeting Assistant" }));
    expect(navigate).toHaveBeenLastCalledWith("providers");

    await userEvent.click(screen.getByRole("button", { name: "Add key" }));
    expect(navigate).toHaveBeenLastCalledWith("providers");
  });

  /// The whole point of Home. A Start button that navigates instead of
  /// starting would quietly break the main flow.
  it("starts a session in one click, shows it live, and stops it", async () => {
    const calls: string[] = [];
    mockFeatures((command) => {
      if (command === "start_session") {
        calls.push(command);
        return LISTENING;
      }
      if (command === "stop_session") {
        calls.push(command);
        return { ...LISTENING, state: "completed" };
      }
      return undefined;
    });

    render(<HomeView onNavigate={() => undefined} />);
    await userEvent.click(await screen.findByRole("button", { name: "Start Live Caption" }));

    expect(await screen.findByRole("region", { name: "Live Caption session" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Live Caption is running" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Stop Live Caption" }));
    await waitFor(() => expect(calls).toEqual(["start_session", "stop_session"]));
    await waitFor(() =>
      expect(
        screen.queryByRole("region", { name: "Live Caption session" }),
      ).not.toBeInTheDocument(),
    );
  });

  /// Two sessions would fight over the microphone.
  it("does not let a second session start while one is running", async () => {
    mockFeatures((command) => (command === "start_session" ? LISTENING : undefined));

    render(<HomeView onNavigate={() => undefined} />);
    await userEvent.click(await screen.findByRole("button", { name: "Start Live Caption" }));

    await screen.findByRole("button", { name: "Live Caption is running" });
    expect(screen.getByRole("button", { name: "Set up Meeting Assistant" })).toBeDisabled();
  });
});
