import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest";
import type { Task } from "../bindings";
import { makeTask } from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import { TaskNavProvider } from "./nav";
import { useTaskNav } from "./nav-context";

beforeAll(() => {
  loadCoreForTests();
});

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

const task = (id: string, parent: string | null): Task =>
  makeTask({ id, project_id: "proj", content: id, parent_id: parent, child_order: 0 });

const all = [task("P", null), task("C", "P"), task("G", "C")];

function setup(tasks: Task[] = all) {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <TaskNavProvider all={tasks}>{children}</TaskNavProvider>
  );
  return renderHook(() => useTaskNav(), { wrapper });
}

it("open of a grandchild opens its chain with show-completed off", () => {
  const { result } = setup();
  act(() => result.current.setShowCompleted(true));
  act(() => result.current.open("G"));
  expect(result.current.stack).toEqual(["P", "C", "G"]);
  expect(result.current.showCompleted).toBe(false);
  expect(result.current.addSub).toBe(false);
});

it("openAdding also asks for the sub-task editor", () => {
  const { result } = setup();
  act(() => result.current.openAdding("C"));
  expect(result.current.stack).toEqual(["P", "C"]);
  expect(result.current.addSub).toBe(true);
  act(() => result.current.clearAddSub());
  expect(result.current.addSub).toBe(false);
});

it("push, back, completedHere and close walk the stack", () => {
  const { result } = setup();
  act(() => result.current.open("P"));
  act(() => result.current.push("C"));
  act(() => result.current.push("G"));
  expect(result.current.stack).toEqual(["P", "C", "G"]);
  act(() => result.current.back());
  expect(result.current.stack).toEqual(["P", "C"]);
  act(() => result.current.completedHere());
  expect(result.current.stack).toEqual(["P"]);
  act(() => result.current.close());
  expect(result.current.stack).toEqual([]);
});

it("askClose only asks while a view is open", () => {
  const { result } = setup();
  act(() => result.current.askClose());
  expect(result.current.closeAsked).toBe(false);
  act(() => result.current.open("P"));
  act(() => result.current.askClose());
  expect(result.current.closeAsked).toBe(true);
  act(() => result.current.clearCloseAsked());
  expect(result.current.closeAsked).toBe(false);
});

it("holdCircles holds until releaseCircles resolves 400 ms later", async () => {
  const { result } = setup();
  let first = false;
  let second = true;
  act(() => {
    first = result.current.holdCircles();
    // The second tap in the same tick sees it held.
    second = result.current.holdCircles();
  });
  expect(first).toBe(true);
  expect(second).toBe(false);
  let released = false;
  act(() => {
    void result.current.releaseCircles().then(() => {
      released = true;
    });
  });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(399);
  });
  expect(released).toBe(false);
  expect(result.current.holdCircles()).toBe(false);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(1);
  });
  expect(released).toBe(true);
  expect(result.current.holdCircles()).toBe(true);
});

it("prune cuts deleted tasks and keeps the same stack when nothing changed", () => {
  const { result } = setup();
  act(() => result.current.open("G"));
  const before = result.current.stack;
  act(() => result.current.prune(all));
  expect(result.current.stack).toBe(before);
  act(() => result.current.prune([task("P", null)]));
  expect(result.current.stack).toEqual(["P"]);
});

it("the functions stay the same across renders", () => {
  const { result } = setup();
  const { open, push, back, holdCircles, prune } = result.current;
  act(() => result.current.open("G"));
  expect(result.current.open).toBe(open);
  expect(result.current.push).toBe(push);
  expect(result.current.back).toBe(back);
  expect(result.current.holdCircles).toBe(holdCircles);
  expect(result.current.prune).toBe(prune);
});
