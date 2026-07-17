import { describe, expect, it } from "vitest";

import { looksComplete, looksLikeQuestion } from "./feed";

describe("looksLikeQuestion", () => {
  it("catches a plain question mark", () => {
    expect(looksLikeQuestion("What is your name?")).toBe(true);
  });

  it("catches a 5W1H word that does not start the line", () => {
    // Speech is cut into chunks, so a question rarely starts one cleanly.
    expect(looksLikeQuestion("So I wanted to ask you, what is your")).toBe(true);
    expect(looksLikeQuestion("and then tell me how you would approach")).toBe(true);
    expect(looksLikeQuestion("okay so why did the team choose that")).toBe(true);
  });

  it("catches a question chunk with no punctuation at all", () => {
    expect(looksLikeQuestion("where do you see yourself in five years")).toBe(true);
    expect(looksLikeQuestion("who was responsible for the migration")).toBe(true);
    expect(looksLikeQuestion("when did that ship")).toBe(true);
  });

  it("catches a yes/no question opened by an auxiliary", () => {
    expect(looksLikeQuestion("can you walk me through it")).toBe(true);
    expect(looksLikeQuestion("did the deploy finish")).toBe(true);
  });

  it("catches Indonesian question words anywhere", () => {
    expect(looksLikeQuestion("jadi menurut kamu bagaimana caranya")).toBe(true);
    expect(looksLikeQuestion("terus kenapa timnya pilih itu")).toBe(true);
    expect(looksLikeQuestion("kamu tahu siapa yang buat ini")).toBe(true);
  });

  it("catches the Indonesian -kah suffix", () => {
    expect(looksLikeQuestion("bisakah kamu jelaskan lagi")).toBe(true);
    expect(looksLikeQuestion("sudahkah kamu coba")).toBe(true);
  });

  it("ignores lines too short to carry a question", () => {
    expect(looksLikeQuestion("ok")).toBe(false);
    expect(looksLikeQuestion("  ")).toBe(false);
  });

  it("ignores a plain statement", () => {
    expect(looksLikeQuestion("The deploy finished about an hour ago.")).toBe(false);
    expect(looksLikeQuestion("I pushed the branch and it went green.")).toBe(false);
  });
});

describe("looksComplete", () => {
  it("treats terminal punctuation as a finished sentence", () => {
    expect(looksComplete("What is your name?")).toBe(true);
    expect(looksComplete("That shipped last week.")).toBe(true);
    expect(looksComplete('He said "we are done."')).toBe(true);
  });

  it("treats a line stopping mid-thought as unfinished", () => {
    // A chunk cut by the cloud time limit: the rest is still coming.
    expect(looksComplete("So I wanted to ask you, what is your")).toBe(false);
    expect(looksComplete("and then tell me how you would")).toBe(false);
  });
});
