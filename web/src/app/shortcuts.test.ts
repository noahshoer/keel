import { expect, it } from "vitest";
import { isSearchShortcut } from "./shortcuts";

const key = (k: string, mods: Partial<KeyboardEventInit> = {}) =>
  new KeyboardEvent("keydown", { key: k, ...mods });

it("/ opens search outside text fields, with no modifier", () => {
  expect(isSearchShortcut(key("/"), false)).toBe(true);
  expect(isSearchShortcut(key("/"), true)).toBe(false);
  expect(isSearchShortcut(key("/", { ctrlKey: true }), false)).toBe(false);
  expect(isSearchShortcut(key("/", { altKey: true }), false)).toBe(false);
});

it("Ctrl+K or Cmd+K open search anywhere", () => {
  expect(isSearchShortcut(key("k", { metaKey: true }), true)).toBe(true);
  expect(isSearchShortcut(key("K", { ctrlKey: true }), false)).toBe(true);
  expect(isSearchShortcut(key("k"), false)).toBe(false);
});
