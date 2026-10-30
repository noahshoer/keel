import { afterEach, describe, expect, it, vi } from "vitest";
import { weekStart } from "./locale";

afterEach(() => {
  vi.unstubAllGlobals();
});

function stubLocale(info: { firstDay: number } | undefined) {
  vi.stubGlobal(
    "Intl",
    Object.assign({}, Intl, {
      Locale: class {
        getWeekInfo() {
          return info;
        }
      },
    }),
  );
}

describe("weekStart", () => {
  it("reads the locale's first day", () => {
    stubLocale({ firstDay: 7 });
    expect(weekStart()).toBe(7);
    stubLocale({ firstDay: 6 });
    expect(weekStart()).toBe(6);
    stubLocale({ firstDay: 1 });
    expect(weekStart()).toBe(1);
  });

  it("is Sunday without week info, as the Dioxus app reads it", () => {
    stubLocale(undefined);
    expect(weekStart()).toBe(7);
  });
});
