import { beforeAll, describe, expect, it } from "vitest";
import { makeTask } from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import { cards, dueLabel, progress, timeLabel } from ".";

beforeAll(() => {
  loadCoreForTests();
});

describe("keel-core over wasm", () => {
  it("due label matches Rust", () => {
    // Same cases as `due_label` in crates/core/src/views.rs.
    expect(dueLabel("2026-10-30", "2026-10-29")).toBe("Tomorrow");
    expect(dueLabel("2026-10-29", "2026-10-29")).toBe("Today");
    expect(dueLabel("2027-03-04", "2026-10-19")).toBe("4 Mar 2027");
    // A datetime due labels by its date.
    expect(dueLabel("2026-10-30T17:00:00", "2026-10-29")).toBe("Tomorrow");
  });

  it("due label rejects a malformed date", () => {
    expect(() => dueLabel("30/10/2026", "2026-10-29")).toThrow(/invalid date "30\/10\/2026"/);
  });

  it("time label matches Rust", () => {
    expect(timeLabel("17:00")).toBe("5 PM");
    expect(timeLabel("09:05")).toBe("9:05 AM");
    expect(timeLabel("17:30:00")).toBe("5:30 PM");
  });

  it("cards hide sub-tasks under their parent", () => {
    const parent = makeTask({ id: "t-parent", project_id: "p-home", content: "Pack" });
    const child = makeTask({ id: "t-child", project_id: "p-home", content: "Socks", parent_id: "t-parent" });
    const all = [parent, child];
    const out = cards(all, all, "TopLevel");
    expect(out).toHaveLength(1);
    expect(out[0]?.head.task.id).toBe("t-parent");
    expect(out[0]?.head.progress).toEqual({ done: 0, total: 1 });
    expect(out[0]?.rows).toEqual([]);
    // Round-trips the task unchanged (wire shape equals the binding).
    expect(out[0]?.head.task).toEqual(parent);
  });

  it("progress counts direct sub-tasks", () => {
    const parent = makeTask({ id: "a", project_id: "p", content: "A" });
    const done = makeTask({ id: "b", project_id: "p", content: "B", parent_id: "a", checked: true });
    const open = makeTask({ id: "c", project_id: "p", content: "C", parent_id: "a" });
    expect(progress("a", [parent, done, open])).toEqual({ done: 1, total: 2 });
    expect(progress("b", [parent, done, open])).toBeNull();
  });
});
