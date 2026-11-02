// Work that shouldn't repeat: wasm calls serialize their inputs, so a row with
// no labels makes none, and a re-render with the same data repeats none.
import { render } from "@testing-library/react";
import { beforeAll, beforeEach, expect, it, vi } from "vitest";
import type { Task } from "../bindings";
import * as core from "../core";
import * as fx from "../ipc/fixtures";
import { TaskNavProvider } from "../taskview/nav";
import { loadCoreForTests } from "../test/wasm";
import { TaskRow } from "./TaskRow";

vi.mock("../core", async (original) => {
  const actual = await original<typeof import("../core")>();
  return { ...actual, labelGroups: vi.fn(actual.labelGroups) };
});

beforeAll(() => {
  loadCoreForTests();
});

beforeEach(() => {
  vi.mocked(core.labelGroups).mockClear();
});

const base = fx.tasks[0] as Task;

// biome-ignore lint/style/useComponentExportOnlyModules: a test probe, never hot-reloaded.
function Row({ labels, tick }: { labels: string[]; tick: number }) {
  return (
    <TaskNavProvider all={fx.tasks}>
      <TaskRow
        row={{ task: { ...base, labels }, progress: null }}
        today={`2026-11-0${tick}`}
        location={{ project: "Home", section: null }}
        labels={fx.labels}
        reminders={[]}
        onChanged={() => {}}
      />
    </TaskNavProvider>
  );
}

it("a row with no labels doesn't ask keel-core to group them", () => {
  render(<Row labels={[]} tick={1} />);
  expect(core.labelGroups).not.toHaveBeenCalled();
});

it("a row groups its labels once across re-renders with the same labels", () => {
  const labels = ["errand"];
  const { rerender } = render(<Row labels={labels} tick={1} />);
  rerender(<Row labels={labels} tick={2} />);
  expect(core.labelGroups).toHaveBeenCalledTimes(1);
});
