import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeAll, expect, it } from "vitest";
import { localToday } from "../clock";
import { loadCoreForTests } from "../test/wasm";
import { type Loaded, loadView, useLoaded } from "./load";
import type { View } from "./view";

beforeAll(() => {
  loadCoreForTests();
});

const today = localToday();

it("upcoming puts the overdue task first", async () => {
  const loaded = await loadView({ kind: "upcoming" }, false, today);
  expect(loaded.tasks[0]?.id).toBe("t-taxes");
  expect(loaded.tasks.map((t) => t.id)).toContain("t-groceries");
});

it("today counts today's tasks and lists the inbox", async () => {
  const loaded = await loadView({ kind: "today" }, false, today);
  expect(loaded.todayCount).toBe(loaded.tasks.length);
  expect(loaded.inbox.is_inbox).toBe(true);
  expect(loaded.reminders.length).toBeGreaterThan(0);
  expect(loaded.completed).toEqual([]);
});

it("show completed loads the view's completed tasks", async () => {
  const loaded = await loadView({ kind: "today" }, true, today);
  expect(loaded.completed.map((t) => t.id)).toContain("t-done");
});

it("a filter that can't run gives its problem and no tasks", async () => {
  const loaded = await loadView({ kind: "filter", id: "f-bad", name: "Broken", query: "p1 &" }, false, today);
  expect(loaded.filterError).toBe("expected a term after '&'");
  expect(loaded.tasks).toEqual([]);
});

it("a slow earlier load never replaces a newer one", async () => {
  const pending = new Map<string, (l: Loaded) => void>();
  const real = await loadView({ kind: "today" }, false, today);
  const load = (v: View) =>
    new Promise<Loaded>((resolve) => pending.set(v.kind, (l) => resolve({ ...l, view: v })));
  const { result, rerender } = renderHook(({ view }) => useLoaded(view, false, 0, load), {
    initialProps: { view: { kind: "today" } as View },
  });
  rerender({ view: { kind: "inbox" } });
  await act(async () => pending.get("inbox")?.(real));
  await waitFor(() => expect(result.current.loaded?.view.kind).toBe("inbox"));
  await act(async () => pending.get("today")?.(real));
  expect(result.current.loaded?.view.kind).toBe("inbox");
});
