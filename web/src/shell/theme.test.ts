import { afterEach, expect, it } from "vitest";
import { applyTheme, nextTheme, savedTheme, themeLabel } from "./theme";

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset["theme"];
});

it("defaults to the system theme", () => {
  expect(savedTheme()).toBe("system");
  localStorage.setItem("theme", "purple");
  expect(savedTheme()).toBe("system");
});

it("cycles system, light, dark and back", () => {
  expect(nextTheme("system")).toBe("light");
  expect(nextTheme("light")).toBe("dark");
  expect(nextTheme("dark")).toBe("system");
});

it("applies and remembers a theme on <html>", () => {
  applyTheme("dark");
  expect(document.documentElement.dataset["theme"]).toBe("dark");
  expect(savedTheme()).toBe("dark");
  applyTheme("system");
  expect(document.documentElement.dataset["theme"]).toBeUndefined();
  expect(savedTheme()).toBe("system");
});

it("labels say what a click does next", () => {
  expect(themeLabel("system")).toBe("Theme: System (click for Light)");
  expect(themeLabel("light")).toBe("Theme: Light (click for Dark)");
  expect(themeLabel("dark")).toBe("Theme: Dark (click for System)");
});
