import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { growIn, LEAVE_MS, markCollapse, nextPresence, useFreshIds, usePresence } from "./presence";

describe("nextPresence", () => {
  it("comes in collapsed, then opens", () => {
    expect(nextPresence("out", "show")).toBe("entering");
    expect(nextPresence("entering", "frame")).toBe("shown");
  });
  it("goes out by collapsing first", () => {
    expect(nextPresence("shown", "hide")).toBe("leaving");
    expect(nextPresence("leaving", "done")).toBe("out");
  });
  it("turns back from either way", () => {
    expect(nextPresence("leaving", "show")).toBe("shown");
    expect(nextPresence("entering", "hide")).toBe("out");
  });
  it("ignores what doesn't apply", () => {
    expect(nextPresence("shown", "show")).toBe("shown");
    expect(nextPresence("out", "hide")).toBe("out");
    expect(nextPresence("shown", "done")).toBe("shown");
    expect(nextPresence("out", "frame")).toBe("out");
  });
});

describe("usePresence", () => {
  beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "requestAnimationFrame"] }));
  afterEach(() => vi.useRealTimers());

  it("starts as it is, with no animation", () => {
    expect(renderHook(() => usePresence(true)).result.current).toBe("shown");
    expect(renderHook(() => usePresence(false)).result.current).toBe("out");
  });

  it("enters collapsed for a frame, then opens", () => {
    const { result, rerender } = renderHook(({ show }) => usePresence(show), {
      initialProps: { show: false },
    });
    rerender({ show: true });
    expect(result.current).toBe("entering");
    act(() => {
      vi.advanceTimersToNextFrame();
    });
    expect(result.current).toBe("shown");
  });

  it("stays while it collapses, then goes", () => {
    const { result, rerender } = renderHook(({ show }) => usePresence(show), {
      initialProps: { show: true },
    });
    rerender({ show: false });
    expect(result.current).toBe("leaving");
    act(() => {
      vi.advanceTimersByTime(LEAVE_MS);
    });
    expect(result.current).toBe("out");
  });
});

describe("useFreshIds", () => {
  it("has none on the first render, then the ids that weren't there before", () => {
    const { result, rerender } = renderHook(({ ids }) => useFreshIds(ids, () => 0), {
      initialProps: { ids: ["a"] },
    });
    expect([...result.current]).toEqual([]);
    rerender({ ids: ["a", "b"] });
    expect([...result.current]).toEqual(["b"]);
  });

  it("keeps an id fresh until its animation is over, across renders", () => {
    let now = 0;
    const { result, rerender } = renderHook(({ ids }) => useFreshIds(ids, () => now), {
      initialProps: { ids: [] as string[] },
    });
    rerender({ ids: ["a"] });
    now = 100;
    rerender({ ids: ["a"] });
    expect(result.current.has("a")).toBe(true);
    now = 1000;
    rerender({ ids: ["a"] });
    expect(result.current.has("a")).toBe(false);
  });

  it("an id that comes back is fresh again", () => {
    let now = 0;
    const { result, rerender } = renderHook(({ ids }) => useFreshIds(ids, () => now), {
      initialProps: { ids: ["a"] },
    });
    rerender({ ids: [] });
    now = 2000;
    rerender({ ids: ["a"] });
    expect(result.current.has("a")).toBe(true);
  });
});

describe("growIn", () => {
  const el = (height: number) => {
    const animate = vi.fn();
    const node = {
      animate,
      getBoundingClientRect: () => ({ height }),
      ownerDocument: { defaultView: { getComputedStyle: () => ({ marginBottom: "8px" }) } },
    } as unknown as HTMLElement;
    return { node, animate };
  };

  it("grows from nothing to its own height and margin while it fades in", () => {
    const { node, animate } = el(60);
    growIn(node, false);
    const [frames] = animate.mock.calls[0] ?? [];
    expect(frames[0]).toMatchObject({ height: "0px", marginBottom: "0px", opacity: 0 });
    expect(frames[1]).toMatchObject({ height: "60px", marginBottom: "8px", opacity: 1 });
  });

  it("only fades under reduced motion", () => {
    const { node, animate } = el(60);
    growIn(node, true);
    const [frames] = animate.mock.calls[0] ?? [];
    expect(frames).toEqual([{ opacity: 0 }, { opacity: 1 }]);
  });

  it("does nothing where elements can't animate", () => {
    expect(() =>
      growIn({ getBoundingClientRect: () => ({ height: 1 }) } as unknown as HTMLElement, false),
    ).not.toThrow();
  });
});

describe("markCollapse", () => {
  it("records the row's height and its card's, for the switch-off to collapse from", () => {
    const card = document.createElement("div");
    card.className = "task-card";
    const row = document.createElement("div");
    card.appendChild(row);
    Object.defineProperty(card, "offsetHeight", { value: 78 });
    Object.defineProperty(row, "offsetHeight", { value: 70 });
    markCollapse(row);
    expect(row.style.getPropertyValue("--collapse-from")).toBe("70px");
    expect(card.style.getPropertyValue("--collapse-from")).toBe("78px");
  });
});
