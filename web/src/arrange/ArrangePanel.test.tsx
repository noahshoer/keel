import { render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ArrangePanel } from "./ArrangePanel";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

const narrow = () =>
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));

const panel = () => (
  <ArrangePanel
    title="Filter"
    action={{ label: "Reset", run: () => {} }}
    anchor={{ current: null }}
    onClose={() => {}}
  >
    <p>body</p>
  </ArrangePanel>
);

it("the phone's sheet is modal and has no grabber", () => {
  narrow();
  render(panel());
  const sheet = document.querySelector<HTMLElement>(".arrange-sheet") as HTMLElement;
  expect(sheet.getAttribute("aria-modal")).toBe("true");
  expect(document.querySelector(".tv-grabber")).toBeNull();
});

it("the popover is not modal", () => {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: true,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
  render(panel());
  expect(document.querySelector(".arrange-popover")?.getAttribute("aria-modal")).toBeNull();
});
