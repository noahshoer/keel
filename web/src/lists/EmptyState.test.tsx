import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, expect, it, vi } from "vitest";
import type { Card } from "../bindings";
import { cards } from "../core";
import * as fx from "../ipc/fixtures";
import { TaskNavProvider } from "../taskview/nav";
import { loadCoreForTests } from "../test/wasm";
import { CardList } from "./CardList";
import { EmptyState } from "./EmptyState";
import { LEAVE_MS } from "./presence";

beforeAll(loadCoreForTests);

beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "requestAnimationFrame"] }));
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

it("a view that starts empty shows its message at once", () => {
  const { container } = render(<EmptyState show big="No tasks yet." small="Add one below." />);
  expect(screen.getByText("No tasks yet.")).toBeTruthy();
  expect(container.querySelector(".presence")?.className).toBe("presence");
});

it("collapses and fades the message when a task arrives, then removes it", () => {
  const { container, rerender } = render(<EmptyState show big="No tasks yet." small="Add one below." />);
  rerender(<EmptyState show={false} big="No tasks yet." small="Add one below." />);
  expect(container.querySelector(".presence")?.className).toBe("presence leaving");
  act(() => {
    vi.advanceTimersByTime(LEAVE_MS);
  });
  expect(container.querySelector(".presence")).toBeNull();
});

it("brings the message back collapsed, then opens it, when the last task goes", () => {
  const { container, rerender } = render(
    <EmptyState show={false} big="No tasks yet." small="Add one below." />,
  );
  expect(container.querySelector(".presence")).toBeNull();
  rerender(<EmptyState show big="No tasks yet." small="Add one below." />);
  expect(container.querySelector(".presence")?.className).toBe("presence entering");
  act(() => {
    vi.advanceTimersToNextFrame();
  });
  expect(container.querySelector(".presence")?.className).toBe("presence");
});

const card = (id: string): Card => {
  const task = fx.makeTask({ id, project_id: "p-inbox", content: `task ${id}` });
  return cards([task], [task], "TopLevel", fx.today)[0] as Card;
};

it("grows in a card that arrives, and not the ones already there", () => {
  const animate = vi.fn();
  Element.prototype.animate = animate as unknown as Element["animate"];
  const props = { today: "2026-11-06", locations: new Map(), labels: [], reminders: [], onChanged: () => {} };
  const list = (ids: string[]) => (
    <TaskNavProvider all={fx.tasks}>
      <CardList cards={ids.map(card)} {...props} />
    </TaskNavProvider>
  );
  const { rerender } = render(list(["a"]));
  expect(animate).not.toHaveBeenCalled();
  rerender(list(["a", "b"]));
  expect(animate).toHaveBeenCalledTimes(1);
  expect((animate.mock.contexts[0] as HTMLElement).textContent).toContain("task b");
});
