import { beforeAll, describe, expect, it } from "vitest";
import type { Id, Task } from "../bindings";
import { makeTask } from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import { back, openChain, prune, push, top, under } from "./stack";

beforeAll(() => {
  loadCoreForTests();
});

const task = (id: string, parent: string | null, order: number): Task =>
  makeTask({ id, project_id: "proj", content: id, parent_id: parent, child_order: order });

describe("view stack", () => {
  it("push adds on top of the current task", () => {
    expect(push(["A"], "B")).toEqual(["A", "B"]);
  });

  it("open_chain replaces the stack with the chain", () => {
    const all = [task("P", null, 0), task("C", "P", 0), task("G", "C", 0)];
    expect(openChain("G", all)).toEqual(["P", "C", "G"]);
  });

  it("back_pops_then_closes", () => {
    let s: readonly Id[] = ["A", "B"];
    s = back(s);
    expect(s).toEqual(["A"]);
    s = back(s);
    expect(s).toEqual([]);
    s = back(s);
    expect(s).toEqual([]);
  });

  it("prune_cuts_at_deleted", () => {
    // B and its sub-task C are deleted, so neither is in `all`.
    expect(prune(["A", "B", "C"], [task("A", null, 0)])).toEqual(["A"]);
  });

  it("prune_cuts_even_if_later_ids_exist", () => {
    const all = [task("A", null, 0), task("C", "B", 0)];
    expect(prune(["A", "B", "C"], all)).toEqual(["A"]);
  });

  it("prune_everything_closes", () => {
    expect(prune(["A"], [])).toEqual([]);
  });

  it("completed_elsewhere_stays_open", () => {
    const all = [{ ...task("A", null, 0), checked: true }];
    expect(prune(["A"], all)).toEqual(["A"]);
  });

  it("prune returns the same array when nothing changed", () => {
    const s: readonly Id[] = ["A", "B"];
    expect(prune(s, [task("A", null, 0), task("B", "A", 0)])).toBe(s);
  });

  it("top and under", () => {
    expect(top(["A", "B"])).toBe("B");
    expect(top([])).toBeNull();
    expect(under(["A", "B"])).toBe("A");
    expect(under(["A"])).toBeNull();
    expect(under([])).toBeNull();
  });
});
