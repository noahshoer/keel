import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, expect, it } from "vitest";
import { blankValues } from "../core";
import * as fx from "../ipc/fixtures";
import { loadCoreForTests } from "../test/wasm";
import { AddTask } from "./AddTask";

beforeAll(loadCoreForTests);
afterEach(cleanup);

function add(main: boolean) {
  return render(
    <div className="content">
      <AddTask
        main={main}
        defaults={blankValues("p-inbox", null, "", [])}
        projects={fx.projects}
        sections={fx.sections}
        labels={fx.labels}
        onChanged={() => {}}
      />
    </div>,
  );
}

const PLACEHOLDER = "Task name — try “Call mom tomorrow 5pm p1 #Home @phone”";

it("floats a round add button over the view's own list, and it opens the editor", () => {
  const { container } = add(true);
  const fab = screen.getByRole("button", { name: "New task" });
  expect(container.querySelector(".content > .fab")).toBe(fab);
  fireEvent.click(fab);
  expect(screen.getByPlaceholderText(PLACEHOLDER)).toBeTruthy();
  // The editor stands in for the button while it's open.
  expect(screen.queryByRole("button", { name: "New task" })).toBeNull();
});

it("has no floating button on a section's own row", () => {
  add(false);
  expect(screen.queryByRole("button", { name: "New task" })).toBeNull();
});
