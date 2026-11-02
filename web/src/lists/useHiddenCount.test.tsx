import { act, render } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, expect, it, vi } from "vitest";
import { useHiddenCount } from "./useHiddenCount";

// biome-ignore lint/style/useComponentExportOnlyModules: a test probe, never hot-reloaded.
function Probe({ names }: { names: string[] }) {
  const ref = useRef<HTMLDivElement>(null);
  const hidden = useHiddenCount(ref, names.join("\n"));
  return (
    <>
      <div ref={ref} data-testid="box">
        {names.map((n) => (
          <span key={n} data-label="">
            {n}
          </span>
        ))}
      </div>
      <output>{hidden}</output>
    </>
  );
}

const rect = (left: number, width: number) => ({ left, width }) as DOMRect;
let resize: () => void = () => {};

/** Lays the labels out 40px apart in a box of `width`. */
function layout(width: number) {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    if (this.dataset["testid"] === "box") return rect(0, width);
    const index = [...(this.parentElement?.children ?? [])].indexOf(this);
    return rect(index * 40, 30);
  });
}

afterEach(() => {
  Reflect.deleteProperty(document, "fonts");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function stubObserver() {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: () => void) {
        resize = callback;
      }
      observe() {}
      disconnect() {}
    },
  );
}

const shown = (c: HTMLElement) => c.querySelector("output")?.textContent;

it("counts the labels past the visible edge", () => {
  stubObserver();
  layout(100);
  const { container } = render(<Probe names={["a", "b", "c", "d", "e"]} />);
  expect(shown(container)).toBe("2");
});

it("counts again when the row's width changes", () => {
  stubObserver();
  layout(100);
  const { container } = render(<Probe names={["a", "b", "c", "d", "e"]} />);
  layout(1000);
  act(() => resize());
  expect(shown(container)).toBe("0");
});

it("counts again when the labels change", () => {
  stubObserver();
  layout(100);
  const { container, rerender } = render(<Probe names={["a", "b", "c", "d", "e"]} />);
  rerender(<Probe names={["a", "b"]} />);
  expect(shown(container)).toBe("0");
});

it("counts again once the app's font has loaded", async () => {
  stubObserver();
  let loaded: () => void = () => {};
  Object.defineProperty(document, "fonts", {
    configurable: true,
    value: {
      ready: new Promise<void>((done) => {
        loaded = done;
      }),
    },
  });
  layout(100);
  const { container } = render(<Probe names={["a", "b", "c", "d", "e"]} />);
  layout(1000);
  await act(async () => {
    loaded();
    await Promise.resolve();
  });
  expect(shown(container)).toBe("0");
});

it("leaves the count alone before the row has a width", () => {
  stubObserver();
  layout(0);
  const { container } = render(<Probe names={["a", "b"]} />);
  expect(shown(container)).toBe("0");
});
