import { describe, expect, it } from "vitest";
import { fitTitle } from "./titleFit";

// A stand-in for the browser's wrapping: chars per line shrink with the size.
function fitsIn(perLineAt34: number, lines = 2) {
  return (size: number, text: string) => {
    const perLine = Math.floor((perLineAt34 * 34) / size);
    let used = 0;
    let count = 1;
    for (const word of text.split(" ")) {
      if (word.length > perLine) return false;
      if (used > 0 && used + 1 + word.length > perLine) {
        count += 1;
        used = word.length;
      } else {
        used += (used > 0 ? 1 : 0) + word.length;
      }
    }
    return count <= lines;
  };
}

describe("fitTitle", () => {
  it("keeps the full size when the title fits", () => {
    expect(fitTitle("Today", 34, fitsIn(12))).toEqual({ size: 34, text: "Today" });
  });

  it("shrinks before it cuts anything", () => {
    const fit = fitTitle("Home improvement projects", 34, fitsIn(12));
    expect(fit.text).toBe("Home improvement projects");
    expect(fit.size).toBeLessThan(34);
    expect(fit.size).toBeGreaterThanOrEqual(22);
  });

  it("past the smallest size, cuts between words", () => {
    const name = "Home improvement projects for the spring and the summer and the autumn too";
    const fit = fitTitle(name, 34, fitsIn(12));
    expect(fit.size).toBe(22);
    expect(fit.text.endsWith("…")).toBe(true);
    expect(name.startsWith(fit.text.slice(0, -1))).toBe(true);
    expect(name[fit.text.length - 1]).toBe(" ");
  });

  it("a word too long for any line shrinks further rather than breaking", () => {
    const word = "Supercalifragilisticexpialidocious";
    const fit = fitTitle(word, 34, fitsIn(12));
    expect(fit.text).toBe(word);
    expect(fit.size).toBeLessThan(22);
  });

  it("gives up at the floor without breaking the word", () => {
    const word = "x".repeat(200);
    expect(fitTitle(word, 34, fitsIn(12))).toEqual({ size: 12, text: word });
  });
});
