import { act, renderHook } from "@testing-library/react";
import { expect, it } from "vitest";
import { useHeldNames } from "./heldNames";
import type { Loaded } from "./load";

const lists = (project: string, label = "errand", filter = "Top") =>
  ({
    projects: [{ id: "p", name: project }],
    labels: [{ id: "l", name: label }],
    filters: [{ id: "f", name: filter }],
  }) as unknown as Loaded;

const setup = () =>
  renderHook(({ loaded }: { loaded: Loaded | null }) => useHeldNames(loaded), {
    initialProps: { loaded: lists("Home") as Loaded | null },
  });

it("changes nothing with nothing held", () => {
  const { result, rerender } = setup();
  const first = lists("Home");
  rerender({ loaded: first });
  expect(result.current.page).toBe(first);
  expect(result.current.sidebar).toBe(first);
});

it("shows a saved name at once, through a load that's still the old one", () => {
  const { result, rerender } = setup();
  act(() => result.current.holdName("p", "House", Promise.resolve()));
  expect(result.current.sidebar?.projects[0]?.name).toBe("House");
  expect(result.current.page?.projects[0]?.name).toBe("House");
  rerender({ loaded: lists("Home") });
  expect(result.current.sidebar?.projects[0]?.name).toBe("House");
});

it("lets go once a load has the name, so the next rename isn't masked", () => {
  const { result, rerender } = setup();
  act(() => result.current.holdName("p", "House", Promise.resolve()));
  rerender({ loaded: lists("House") });
  rerender({ loaded: lists("Elsewhere") });
  expect(result.current.sidebar?.projects[0]?.name).toBe("Elsewhere");
});

it("a label's name is held for the sidebar, not the page, which finds it by the old one", () => {
  const { result } = setup();
  act(() => result.current.holdName("l", "chores", Promise.resolve()));
  expect(result.current.sidebar?.labels[0]?.name).toBe("chores");
  expect(result.current.page?.labels[0]?.name).toBe("errand");
});

it("holds a filter's name", () => {
  const { result } = setup();
  act(() => result.current.holdName("f", "Soon", Promise.resolve()));
  expect(result.current.sidebar?.filters[0]?.name).toBe("Soon");
});

it("a failed save puts the old name back", async () => {
  const { result } = setup();
  await act(async () => {
    result.current.holdName("p", "House", Promise.reject(new Error("no")));
  });
  expect(result.current.sidebar?.projects[0]?.name).toBe("Home");
});

it("a failed save doesn't undo a newer one", async () => {
  const { result } = setup();
  await act(async () => {
    result.current.holdName("p", "House", Promise.reject(new Error("no")));
    result.current.holdName("p", "Hut", Promise.resolve());
  });
  expect(result.current.sidebar?.projects[0]?.name).toBe("Hut");
});
