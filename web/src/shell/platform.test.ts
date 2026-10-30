import { expect, it } from "vitest";
import { detect, searchHint, searchTooltip } from "./platform";

it("reads the platform from the navigator", () => {
  expect(detect("iPhone", 5, "")).toBe("touch");
  // iPadOS reports a Mac; its touch points give it away.
  expect(detect("MacIntel", 5, "")).toBe("touch");
  expect(detect("Linux armv8l", 5, "Mozilla/5.0 (Linux; Android 15)")).toBe("touch");
  expect(detect("MacIntel", 0, "")).toBe("mac");
  expect(detect("Win32", 0, "")).toBe("other");
  expect(detect("Linux x86_64", 0, "")).toBe("other");
});

it("the search hint names this platform's keys", () => {
  expect(searchHint("touch")).toBe("Every word must match, in any order.");
  expect(searchHint("mac")).toBe("Every word must match, in any order. Press / or ⌘K anytime to search.");
  expect(searchHint("other")).toBe(
    "Every word must match, in any order. Press / or Ctrl+K anytime to search.",
  );
  expect(searchTooltip("touch")).toBe("Search");
  expect(searchTooltip("mac")).toBe("Search (/ or ⌘K)");
  expect(searchTooltip("other")).toBe("Search (/ or Ctrl+K)");
});
