/**
 * German through the page's path: the wasm calls Speak and Sing make, and the words Speak shows
 * at the mouth.
 */
import { describe, expect, test } from "bun:test";
import { initEngine } from "./harness.ts";

await initEngine();
const { germanPhrases, lyricsToSyllables, planPhrases } = await import("../src/wasm/engine.ts");
const { wordSpans } = await import("../src/ui/words.ts");
const male = { female: 0, child: 0 };

describe("German", () => {
  test("text becomes phrases with German sounds, stress and intonation", () => {
    const phrases = germanPhrases("Guten Tag! Kommst du morgen?");
    expect(phrases.length).toBe(2);
    expect(phrases[0].segments.map((s) => s.ipa).join("")).toBe("gutəntak");
    expect(phrases[0].segments.find((s) => s.stress === 1)?.ipa).toBe("u");
    expect(phrases[1].intonation).toBe("question");
  });

  test("each word gets its span in the plan", () => {
    const phrases = germanPhrases("Heute ist das Wetter schön.");
    const plan = planPhrases(phrases, { body: male, t0: 0 });
    const words = wordSpans(phrases, plan);
    expect(words.map((w) => w.text)).toEqual(["heute", "ist", "das", "wetter", "schön"]);
    for (let i = 1; i < words.length; i++) expect(words[i].t0).toBeGreaterThanOrEqual(words[i - 1].t1 - 1e-9);
  });

  test("lyrics in DE become German syllables; other scripts still go to their own front ends", () => {
    const syl = lyricsToSyllables("Stille Nacht 빵 дом", "de");
    expect(syl.map((s) => s.text)).toEqual(["Sti", "lle", "Nacht", "빵", "дом"]);
    expect(syl[2].coda.map((s) => s.ipa)).toEqual(["x", "t"]);
  });

  test("lyrics are English unless DE is chosen", () => {
    expect(lyricsToSyllables("schön grüß").every((s) => /^[ -~]*$/.test(s.text))).toBe(true);
  });
});
