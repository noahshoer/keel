import { render } from "@testing-library/react";
import { beforeAll, expect, it } from "vitest";
import type { ArrangedGroup } from "../bindings";
import { tasks } from "../ipc/fixtures";
import { TaskNavProvider } from "../taskview/nav";
import { loadCoreForTests } from "../test/wasm";
import { ArrangedList } from "./ArrangedList";

beforeAll(() => {
  loadCoreForTests();
});

const group = (g: Partial<ArrangedGroup> & Pick<ArrangedGroup, "key">): ArrangedGroup => ({
  title: null,
  starts_title: false,
  title_count: 0,
  sub: null,
  tasks: [],
  ...g,
});

it("draws a title line only where a run starts, and tints a label sub-heading", () => {
  const laundry = tasks.filter((t) => t.id === "t-laundry");
  const { container } = render(
    <TaskNavProvider all={tasks}>
      <ArrangedList
        groups={[
          group({
            key: "a/b",
            title: "Home › Chores",
            starts_title: true,
            title_count: 1,
            sub: { text: "errand", label_color: "orange" },
            tasks: laundry,
          }),
          group({
            key: "a/c",
            title: "Home › Chores",
            sub: { text: "No label", label_color: null },
            tasks: [],
          }),
        ]}
        all={tasks}
        mode="TopLevel"
        today="2026-01-01"
        locations={new Map()}
        labels={[]}
        reminders={[]}
        onChanged={() => {}}
      />
    </TaskNavProvider>,
  );
  expect(container.querySelectorAll(".section-title")).toHaveLength(1);
  const subs = container.querySelectorAll<HTMLElement>(".arrange-sub");
  expect(subs[0]?.style.getPropertyValue("--tint")).toBe("var(--label-orange)");
  expect(subs[0]?.querySelector("svg")).not.toBeNull();
  expect(subs[1]?.querySelector("svg")).toBeNull();
});
