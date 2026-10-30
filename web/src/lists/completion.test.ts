import { afterEach, expect, it } from "vitest";
import { COMPLETION_ICONS, FILL_MS, nextOnTap, OFF_MS, savedIcon } from "./completion";

afterEach(() => {
  localStorage.clear();
});

it("the waterline is the default icon", () => {
  expect(savedIcon()).toBe("wave");
  localStorage.setItem("completion-icon", "nonsense");
  expect(savedIcon()).toBe("wave");
});

it("icons round-trip through their keys", () => {
  for (const icon of COMPLETION_ICONS) {
    localStorage.setItem("completion-icon", icon);
    expect(savedIcon()).toBe(icon);
  }
});

it("the switch-off follows a visible fill", () => {
  expect(FILL_MS).toBe(350);
  expect(OFF_MS).toBe(380);
  expect(FILL_MS >= 300 && OFF_MS >= 300).toBe(true);
});

it("a tap starts, cancels during the fill, and is ignored during the switch-off", () => {
  expect(nextOnTap(0)).toBe("start");
  expect(nextOnTap(1)).toBe("cancel");
  expect(nextOnTap(2)).toBe("ignore");
});
