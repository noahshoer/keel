import { afterEach, expect, it, vi } from "vitest";
import { installKeyboardPrime } from "./keyboardPrime";

let uninstall = () => {};
afterEach(() => {
  uninstall();
  document.body.innerHTML = "";
});

// jsdom's events are never trusted, so the tests say which ones count as a person's.
const install = (trusted = true) => {
  uninstall = installKeyboardPrime(document, { trusted: () => trusted });
};

it("on the first real tap, focuses a hidden field that shows no keyboard, then lets it go", () => {
  install();
  const focused: Element[] = [];
  document.addEventListener("focusin", (e) => focused.push(e.target as Element), true);
  document.body.innerHTML = `<button id="b" type="button">Home</button>`;
  const blurred = vi.fn();
  document.addEventListener("focusout", blurred, true);
  (document.getElementById("b") as HTMLElement).click();
  expect(focused).toHaveLength(1);
  const field = focused[0] as HTMLInputElement;
  expect(field.getAttribute("inputmode")).toBe("none");
  expect(field.getAttribute("aria-hidden")).toBe("true");
  expect(field.tabIndex).toBe(-1);
  // Let go at once, and gone from the page.
  expect(blurred).toHaveBeenCalledTimes(1);
  expect(field.isConnected).toBe(false);
  expect(document.activeElement).toBe(document.body);
});

it("primes once: later taps are left alone", () => {
  install();
  const focused = vi.fn();
  document.addEventListener("focusin", focused, true);
  document.body.click();
  document.body.click();
  expect(focused).toHaveBeenCalledTimes(1);
});

it("a click a script sends doesn't count", () => {
  install(false);
  const focused = vi.fn();
  document.addEventListener("focusin", focused, true);
  document.body.click();
  expect(focused).not.toHaveBeenCalled();
});

it("a tap on a field primes nothing: that focus is already the person's", () => {
  install();
  document.body.innerHTML = `<input id="f">`;
  const field = document.getElementById("f") as HTMLInputElement;
  field.focus();
  const focused = vi.fn();
  document.addEventListener("focusin", focused, true);
  field.click();
  expect(focused).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(field);
  // Done: a later tap elsewhere doesn't prime either.
  document.body.click();
  expect(focused).not.toHaveBeenCalled();
});
