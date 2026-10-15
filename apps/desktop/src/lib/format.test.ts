import { describe, expect, it, vi, afterEach } from "vitest";

import { dayGroup, formatBytes, formatClock, formatDuration, formatWhen } from "./format";

describe("formatBytes", () => {
  it("formats each unit the way File Explorer does", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1024 * 1024)).toBe("1 MB");
    expect(formatBytes(5.5 * 1024 * 1024)).toBe("5.5 MB");
    expect(formatBytes(1024 ** 3)).toBe("1 GB");
  });

  it("does not blow past the largest unit", () => {
    expect(formatBytes(1024 ** 6)).toContain("TB");
  });

  it("survives nonsense input rather than rendering NaN", () => {
    expect(formatBytes(-1)).toBe("0 B");
    expect(formatBytes(Number.NaN)).toBe("0 B");
  });
});

describe("formatWhen", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("describes recent times relatively", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-15T12:00:00Z"));

    expect(formatWhen("2026-07-15T11:59:40Z")).toBe("just now");
    expect(formatWhen("2026-07-15T11:30:00Z")).toBe("30m ago");
    expect(formatWhen("2026-07-15T09:00:00Z")).toBe("3h ago");
    expect(formatWhen("2026-07-13T12:00:00Z")).toBe("2d ago");
  });

  it("falls back to a date once relative time stops being useful", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-15T12:00:00Z"));

    expect(formatWhen("2026-05-15T12:00:00Z")).toMatch(/2026/);
  });

  it("returns nothing for a missing or unparseable stamp", () => {
    expect(formatWhen(null)).toBe("");
    expect(formatWhen("not a date")).toBe("");
  });
});

describe("formatClock", () => {
  it("reads like a stopwatch and grows an hour field only when needed", () => {
    expect(formatClock(0)).toBe("0:00");
    expect(formatClock(7_400)).toBe("0:07");
    expect(formatClock(272_000)).toBe("4:32");
    expect(formatClock(3_729_000)).toBe("1:02:09");
    expect(formatClock(-5)).toBe("0:00");
  });
});

describe("formatDuration", () => {
  it("keeps short sessions short and long ones readable", () => {
    expect(formatDuration(45_000)).toBe("45s");
    expect(formatDuration(724_000)).toBe("12m 04s");
    expect(formatDuration(3_720_000)).toBe("1h 02m");
  });
});

describe("dayGroup", () => {
  const now = new Date(2026, 9, 15, 12, 0, 0);

  it("groups by calendar day, not by 24-hour windows", () => {
    expect(dayGroup(new Date(2026, 9, 15, 0, 5).toISOString(), now)).toBe("Today");
    expect(dayGroup(new Date(2026, 9, 14, 23, 55).toISOString(), now)).toBe("Yesterday");
    expect(dayGroup(new Date(2026, 9, 11, 9, 0).toISOString(), now)).toBe("This Week");
    expect(dayGroup(new Date(2026, 7, 2, 9, 0).toISOString(), now)).toContain("2026");
    expect(dayGroup("not a date", now)).toBe("Earlier");
  });
});
