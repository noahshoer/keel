import { act, renderHook } from "@testing-library/react";
import { beforeAll, expect, it } from "vitest";
import { loadCoreForTests } from "../test/wasm";
import { FILTER_HINT, useQueryCheck } from "./filterQuery";

beforeAll(() => {
  loadCoreForTests();
});

const names = { projects: [], sections: [], labels: [] };

it("reads the query live, without a round trip", () => {
  const { result } = renderHook(() => useQueryCheck("", names));
  expect(result.current.problem).toBeNull();
  act(() => result.current.setQuery("p1 &"));
  expect(result.current.problem).toBe("Expected a filter term");
  act(() => result.current.setQuery("today"));
  expect(result.current.problem).toBeNull();
  expect(result.current.segments.some((s) => s.kind !== null)).toBe(true);
});

it("unhighlight quotes the term ending at the caret", () => {
  const { result } = renderHook(() => useQueryCheck("call mom tomorrow", names));
  let done = false;
  act(() => {
    done = result.current.unhighlight(17);
  });
  expect(done).toBe(true);
  expect(result.current.query).toBe('call mom "tomorrow"');
});

it("the hint", () => {
  expect(FILTER_HINT).toContain("next friday");
});
