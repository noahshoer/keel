import { afterEach, expect, it, vi } from "vitest";
import { installUnderPage } from "./underPage";

type Webkit = { messageHandlers: Record<string, { postMessage: (m: unknown) => void }> };
const win = window as unknown as { webkit?: Webkit };
let uninstall = () => {};

afterEach(() => {
  uninstall();
  delete win.webkit;
  document.documentElement.style.removeProperty("--page");
});

it("tells iOS the list's background, so what shows past the page's end matches it", () => {
  const postMessage = vi.fn();
  win.webkit = { messageHandlers: { keelUnderPage: { postMessage } } };
  document.documentElement.style.setProperty("--page", " #1a1a1a");
  uninstall = installUnderPage(document, window).uninstall;
  expect(postMessage).toHaveBeenLastCalledWith("#1a1a1a");
});

it("tells it again when asked (the theme changed), and only when the color did", () => {
  const postMessage = vi.fn();
  win.webkit = { messageHandlers: { keelUnderPage: { postMessage } } };
  document.documentElement.style.setProperty("--page", "#1a1a1a");
  const under = installUnderPage(document, window);
  uninstall = under.uninstall;
  under.sync();
  expect(postMessage).toHaveBeenCalledTimes(1);
  document.documentElement.style.setProperty("--page", "#f5f3f0");
  under.sync();
  expect(postMessage).toHaveBeenLastCalledWith("#f5f3f0");
  expect(postMessage).toHaveBeenCalledTimes(2);
});

it("does nothing where there's no iOS handler (desktop, the browser)", () => {
  document.documentElement.style.setProperty("--page", "#1a1a1a");
  expect(() => {
    const under = installUnderPage(document, window);
    under.sync();
    under.uninstall();
  }).not.toThrow();
});
