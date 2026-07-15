import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FeaturesView } from "./FeaturesView";
import { AUDIS_FEATURES_MOCK } from "@/test/fixtures";

describe("FeaturesView", () => {
  afterEach(() => {
    clearMocks();
  });

  function mockFeatures() {
    mockIPC((command) => {
      if (command === "list_features") return AUDIS_FEATURES_MOCK;
      throw new Error(`unexpected ${command}`);
    });
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
  /// after committing to a session is the failure this prevents.
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

  /// Users must be able to tell, before starting, whether text leaves the PC.
  it("marks which features send data to a cloud provider", async () => {
    mockFeatures();
    render(<FeaturesView onNavigate={() => undefined} />);
    await screen.findByText("Meeting Assistant");

    // Meeting Assistant uses cloud AI; Live Caption does not.
    expect(screen.getAllByText("Uses cloud AI")).toHaveLength(1);
  });
});
