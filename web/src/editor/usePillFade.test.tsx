import { act, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { usePillFade } from "./pillFade";

// biome-ignore lint/style/useComponentExportOnlyModules: a test probe, never hot-reloaded.
function Probe({ content, show = true }: { content: unknown; show?: boolean }) {
  const pills = usePillFade(content);
  return (
    <>
      {show ? <div ref={pills.ref} data-testid="row" onScroll={pills.onScroll} /> : null}
      <output>{pills.fadeClass}</output>
    </>
  );
}

let resize: () => void = () => {};
const observed: Element[] = [];

function stubObserver() {
  observed.length = 0;
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: () => void) {
        resize = callback;
      }
      observe(el: Element) {
        observed.push(el);
      }
      disconnect() {}
    },
  );
}

function size(clientWidth: number, scrollWidth: number) {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(clientWidth);
  vi.spyOn(HTMLElement.prototype, "scrollWidth", "get").mockReturnValue(scrollWidth);
}

afterEach(() => {
  Reflect.deleteProperty(document, "fonts");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const classes = (c: HTMLElement) => c.querySelector("output")?.textContent;

it("has no fades when the pills all fit", () => {
  stubObserver();
  size(300, 300);
  const { container } = render(<Probe content={1} />);
  expect(classes(container)).toBe("");
});

it("fades the end while pills run past it, on first draw", () => {
  stubObserver();
  size(300, 700);
  const { container } = render(<Probe content={1} />);
  expect(classes(container)).toBe("pills-run-on");
});

it("follows scrolling on both sides", () => {
  stubObserver();
  size(300, 700);
  const { container } = render(<Probe content={1} />);
  const row = container.querySelector('[data-testid="row"]') as HTMLElement;
  row.scrollLeft = 100;
  fireEvent.scroll(row);
  expect(classes(container)).toBe("pills-run-on pills-run-back");
  row.scrollLeft = 400;
  fireEvent.scroll(row);
  expect(classes(container)).toBe("pills-run-back");
});

it("measures again when the pills' content or the row's width changes", () => {
  stubObserver();
  size(300, 300);
  const { container, rerender } = render(<Probe content={1} />);
  size(300, 700);
  rerender(<Probe content={2} />);
  expect(classes(container)).toBe("pills-run-on");
  size(900, 700);
  act(() => resize());
  expect(classes(container)).toBe("");
});

it("watches a row that only appears later", () => {
  stubObserver();
  size(300, 300);
  const { container, rerender } = render(<Probe content={0} show={false} />);
  expect(observed).toHaveLength(0);
  rerender(<Probe content={1} />);
  expect(observed).toContain(container.querySelector('[data-testid="row"]'));
});

it("measures again once the app's font has loaded", async () => {
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
  size(300, 300);
  const { container } = render(<Probe content={1} />);
  size(300, 700);
  await act(async () => {
    loaded();
    await Promise.resolve();
  });
  expect(classes(container)).toBe("pills-run-on");
});
