import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { setProviderKey, stopAudioTest, openDataFile, getAppInfo, AudisIpcError } from "./ipc";
import { AUDIS_APP_INFO_MOCK } from "@/test/fixtures";

describe("IPC boundary", () => {
  afterEach(() => {
    clearMocks();
  });

  /**
   * Regression: Tauri serialises Rust's `()` as JSON `null`, and a `z.void()`
   * schema rejects `null`. Every command returning nothing therefore resolved
   * as a failure even though Rust had done the work: the API key was saved to
   * the keystore and the UI still reported an error and kept the key on screen.
   */
  describe("commands that return nothing", () => {
    it("treat a null result as success, because that is what Rust sends", async () => {
      mockIPC(() => null);

      await expect(setProviderKey("gemini", "sk-test")).resolves.toBeUndefined();
      await expect(stopAudioTest()).resolves.toBeUndefined();
      await expect(openDataFile("C:\\data\\x.log")).resolves.toBeUndefined();
    });

    it("also accept undefined, so a mock returning nothing still works", async () => {
      mockIPC(() => undefined);

      await expect(stopAudioTest()).resolves.toBeUndefined();
    });

    it("still reject a result that is neither", async () => {
      mockIPC(() => ({ unexpected: "shape" }));

      await expect(stopAudioTest()).rejects.toBeInstanceOf(AudisIpcError);
    });
  });

  describe("commands that return a value", () => {
    it("parse and return it", async () => {
      mockIPC((command) => {
        if (command === "get_app_info") return AUDIS_APP_INFO_MOCK;
        throw new Error("unexpected");
      });

      await expect(getAppInfo()).resolves.toMatchObject({ appName: "Audis" });
    });

    /// A Rust rename that the frontend has not caught up with must fail loudly
    /// here rather than letting undefined reach a component.
    it("reject a payload that does not match the schema", async () => {
      mockIPC(() => ({ appName: "Audis" }));

      await expect(getAppInfo()).rejects.toBeInstanceOf(AudisIpcError);
    });

    it("surface a UserFacingError from a rejected command", async () => {
      mockIPC(() =>
        Promise.reject({
          title: "Audis could not reach its storage folder",
          explanation: "The folder could not be opened.",
          dataPreserved: true,
          suggestedAction: "Check the folder.",
          technicalDetails: null,
          diagnosticCode: "STORAGE_UNAVAILABLE",
        }),
      );

      await expect(getAppInfo()).rejects.toMatchObject({
        userFacing: { diagnosticCode: "STORAGE_UNAVAILABLE", dataPreserved: true },
      });
    });
  });
});
