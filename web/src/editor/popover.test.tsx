import { act, render } from "@testing-library/react";
import { useRef } from "react";
import { describe, expect, it } from "vitest";
import { placePopover, type Rect, usePlacedPopover } from "./popover";

const rect = (left: number, top: number, width: number, height: number): Rect => ({
  left,
  top,
  width,
  height,
  bottom: top + height,
});

describe("placePopover", () => {
  const pop = rect(0, 0, 280, 300);

  it("sits 6px below its chip", () => {
    expect(placePopover(rect(20, 100, 80, 30), pop, 1200, 900)).toEqual({ top: 136, left: 20 });
  });

  it("flips above when there's no room below", () => {
    expect(placePopover(rect(20, 700, 80, 30), pop, 1200, 900)).toEqual({ top: 394, left: 20 });
  });

  it("shifts left at the right edge of a phone", () => {
    expect(placePopover(rect(300, 100, 60, 30), pop, 375, 812).left).toBe(87);
  });

  it("clamps to the margin when neither side fits", () => {
    expect(placePopover(rect(20, 200, 80, 30), rect(0, 0, 280, 600), 375, 500).top).toBe(8);
  });
});

export function Picker() {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return <div ref={ref} className="popover" />;
}

describe("usePlacedPopover on the keyboard", () => {
  it("follows the task card when the card moves", async () => {
    const { container } = render(
      <form className="on-keyboard">
        <Picker />
      </form>,
    );
    const card = container.querySelector("form") as HTMLElement;
    const pop = container.querySelector(".popover") as HTMLElement;
    let cardTop = 300;
    card.getBoundingClientRect = () => rect(0, cardTop, 402, 136) as DOMRect;
    pop.getBoundingClientRect = () => rect(0, 0, 386, 100) as DOMRect;
    card.style.bottom = "1px";
    await act(async () => {
      await Promise.resolve();
    });
    expect(pop.style.top).toBe("192px");
    cardTop = 700;
    card.style.bottom = "0px";
    await act(async () => {
      await Promise.resolve();
    });
    expect(pop.style.top).toBe("592px");
  });
});
