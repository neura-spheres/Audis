import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { ProvidersView } from "./ProvidersView";
import { AUDIS_PROVIDERS_MOCK, withAmbientIpc } from "@/test/fixtures";

describe("ProvidersView", () => {
  afterEach(() => {
    clearMocks();
  });

  it("offers a key field and links to where you get one", async () => {
    mockIPC(
      withAmbientIpc((command) => {
        if (command === "list_providers") return AUDIS_PROVIDERS_MOCK;
        throw new Error(`unexpected ${command}`);
      }),
    );

    render(<ProvidersView />);

    expect(await screen.findByText("Google Gemini")).toBeInTheDocument();
    expect(screen.getByText("Free tier")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Get a key/ })).toHaveAttribute(
      "href",
      "https://aistudio.google.com/apikey",
    );
  });

  it("sends the key to Rust and clears it from the field", async () => {
    const saved: { id: string; key: string }[] = [];
    mockIPC(
      withAmbientIpc((command, args) => {
        if (command === "list_providers") return AUDIS_PROVIDERS_MOCK;
        if (command === "set_provider_key") {
          saved.push(args as { id: string; key: string });
          return null;
        }
        throw new Error(`unexpected ${command}`);
      }),
    );

    render(<ProvidersView />);
    await screen.findByText("Google Gemini");

    const field = screen.getByPlaceholderText("Paste your key");
    await userEvent.type(field, "sk-secret-abc123");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(saved).toEqual([{ id: "gemini", key: "sk-secret-abc123" }]);
    });

    await waitFor(() => {
      expect(field).toHaveValue("");
    });
  });

  /// The core promise of the credential design: a saved key is never shown.
  it("never renders a saved key, not even partially", async () => {
    mockIPC(
      withAmbientIpc((command) => {
        if (command === "list_providers") {
          return [{ ...AUDIS_PROVIDERS_MOCK[0], hasKey: true }];
        }
        throw new Error(`unexpected ${command}`);
      }),
    );

    const { container } = render(<ProvidersView />);
    await screen.findByText("Google Gemini");

    expect(screen.getByText("Key saved")).toBeInTheDocument();

    expect(container.textContent).not.toMatch(/sk-|AIza|key-[a-z0-9]/i);

    const field = screen.getByPlaceholderText(/A key is saved/);
    expect(field).toHaveValue("");
    expect(field).toHaveAttribute("type", "password");
  });

  it("offers deleting the key only once one exists", async () => {
    mockIPC(
      withAmbientIpc((command) => {
        if (command === "list_providers") return AUDIS_PROVIDERS_MOCK;
        throw new Error(`unexpected ${command}`);
      }),
    );

    render(<ProvidersView />);
    await screen.findByText("Google Gemini");

    expect(screen.queryByRole("button", { name: /Delete the/ })).not.toBeInTheDocument();
  });
});
