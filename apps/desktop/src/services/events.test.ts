import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { AUDIS_EVENTS } from "./events";

/** Contract test against the Rust source. */

const here = path.dirname(fileURLToPath(import.meta.url));
const rustIpcSource = path.resolve(here, "../../../../crates/audis-common/src/ipc.rs");

function eventNamesDeclaredInRust(): string[] {
  const source = readFileSync(rustIpcSource, "utf8");
  const constPattern = /pub const [A-Z_]+: &str = "(audis:\/\/[^"]+)";/g;
  return [...source.matchAll(constPattern)].map((match) => match[1] as string);
}

describe("Audis event contract", () => {
  it("declares every Rust event channel in TypeScript", () => {
    const rust = eventNamesDeclaredInRust();
    const typescript = Object.values(AUDIS_EVENTS) as string[];

    expect(rust.length).toBeGreaterThan(0);

    expect([...rust].sort()).toEqual([...typescript].sort());
  });

  it("prefixes every channel with audis://", () => {
    for (const name of Object.values(AUDIS_EVENTS)) {
      expect(name.startsWith("audis://")).toBe(true);
    }
  });

  it("has no duplicate channels", () => {
    const names = Object.values(AUDIS_EVENTS);
    expect(new Set(names).size).toBe(names.length);
  });
});
