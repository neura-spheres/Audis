import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FilesView } from "./FilesView";
import { AUDIS_FILE_LISTING_MOCK } from "@/test/fixtures";

describe("FilesView", () => {
  afterEach(() => {
    clearMocks();
  });

  it("lists real files with their size and offers to open them", async () => {
    mockIPC((command) => {
      if (command === "list_data_files") return AUDIS_FILE_LISTING_MOCK;
      throw new Error(`unexpected command ${command}`);
    });

    render(<FilesView />);

    expect(await screen.findByText("audis.log.2026-07-15")).toBeInTheDocument();
    expect(screen.getByText("logs\\audis.log.2026-07-15")).toBeInTheDocument();
    expect(screen.getByText("8 KB")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Open audis\.log/ })).toBeInTheDocument();
  });

  it("shows empty categories separately so the layout is still legible", async () => {
    mockIPC((command) => {
      if (command === "list_data_files") return AUDIS_FILE_LISTING_MOCK;
      throw new Error(`unexpected command ${command}`);
    });

    render(<FilesView />);

    expect(await screen.findByText("Empty folders")).toBeInTheDocument();
    expect(screen.getByText("Recordings")).toBeInTheDocument();
  });

  it("asks the backend to open the file that was clicked", async () => {
    const opened: string[] = [];
    mockIPC((command, args) => {
      if (command === "list_data_files") return AUDIS_FILE_LISTING_MOCK;
      if (command === "open_data_file") {
        opened.push((args as { path: string }).path);
        return null;
      }
      throw new Error(`unexpected command ${command}`);
    });

    render(<FilesView />);
    await screen.findByText("audis.log.2026-07-15");

    await userEvent.click(screen.getByRole("button", { name: /Open audis\.log/ }));

    await waitFor(() => {
      expect(opened).toEqual([AUDIS_FILE_LISTING_MOCK.groups[0]!.files[0]!.path]);
    });
  });

  it("surfaces a backend failure instead of rendering an empty list", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);

    mockIPC((command) => {
      if (command === "list_data_files") {
        return Promise.reject({
          title: "Audis could not reach its storage folder",
          explanation: "The folder could not be opened.",
          dataPreserved: true,
          suggestedAction: "Check the folder and try again.",
          technicalDetails: null,
          diagnosticCode: "STORAGE_UNAVAILABLE",
        });
      }
      throw new Error(`unexpected command ${command}`);
    });

    render(<FilesView />);

    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Your data was not affected.")).toBeInTheDocument();
  });
});
