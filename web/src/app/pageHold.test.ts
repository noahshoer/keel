import { afterEach, expect, it, vi } from "vitest";
import { mockPageScrollHeld } from "../ipc/mock";
import { holdPage } from "./pageHold";

const settled = () => vi.waitFor(() => Promise.resolve());

afterEach(async () => {
  await settled();
});

it("holds the page until the last holder lets go", async () => {
  const first = holdPage();
  const second = holdPage();
  await vi.waitFor(() => expect(mockPageScrollHeld()).toBe(true));
  first();
  await settled();
  expect(mockPageScrollHeld()).toBe(true);
  second();
  await vi.waitFor(() => expect(mockPageScrollHeld()).toBe(false));
});

it("letting go twice counts once", async () => {
  const first = holdPage();
  const second = holdPage();
  first();
  first();
  await settled();
  await vi.waitFor(() => expect(mockPageScrollHeld()).toBe(true));
  second();
  await vi.waitFor(() => expect(mockPageScrollHeld()).toBe(false));
});
