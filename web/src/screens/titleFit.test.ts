import { describe, expect, it } from "vitest";
import { fitTitle } from "./titleFit";

const fitsIn = (chars: number) => (text: string) => text.length <= chars;

describe("fitTitle", () => {
  it("keeps a title that fits", () => {
    expect(fitTitle("Today", fitsIn(19))).toBe("Today");
    expect(fitTitle("Exactly nineteen ch", fitsIn(19))).toBe("Exactly nineteen ch");
  });

  it("cuts between words, keeping as many as fit", () => {
    expect(fitTitle("Home improvement projects", fitsIn(19))).toBe("Home improvement…");
    expect(fitTitle("Home improvement projects for the spring", fitsIn(30))).toBe(
      "Home improvement projects for…",
    );
  });

  it("leaves a first word wider than the line to the CSS ellipsis", () => {
    const word = "Supercalifragilisticexpialidocious";
    expect(fitTitle(word, fitsIn(19))).toBe(word);
    expect(fitTitle(`${word} list`, fitsIn(19))).toBe(`${word} list`);
  });
});
