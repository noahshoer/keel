import { act, render } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { rect } from "../test/dom";
import { stubMedia } from "../test/media";
import { placePopover, usePlacedPopover } from "./popover";

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

export function ChipPicker() {
  const ref = useRef<HTMLDivElement>(null);
  usePlacedPopover(ref);
  return (
    <div>
      <button type="button">chip</button>
      <div ref={ref} className="popover" />
    </div>
  );
}

describe("usePlacedPopover by its chip", () => {
  it("places again when a step makes it taller, so it stays on screen", async () => {
    const { container } = render(<ChipPicker />);
    const chip = container.querySelector("button") as HTMLElement;
    const pop = container.querySelector(".popover") as HTMLElement;
    chip.getBoundingClientRect = () => rect(20, 500, 80, 30) as DOMRect;
    pop.getBoundingClientRect = () => rect(0, 0, 280, 400) as DOMRect;
    pop.appendChild(document.createElement("div"));
    await act(async () => {
      await Promise.resolve();
    });
    // jsdom's window is 768 tall: 536 + 400 won't fit below, so it flips above.
    expect(pop.style.top).toBe("94px");
  });

  it("scrolls when it's taller than the window", async () => {
    const { container } = render(<ChipPicker />);
    const pop = container.querySelector(".popover") as HTMLElement;
    expect(pop.style.maxHeight).toBe(`${innerHeight - 16}px`);
    expect(pop.style.overflowY).toBe("auto");
  });
});

describe("usePlacedPopover by its chip, when the keyboard comes up", () => {
  const setup = (coarse: boolean) => {
    const viewport = Object.assign(new EventTarget(), { offsetTop: 0, height: 874, scale: 1 });
    vi.stubGlobal("visualViewport", viewport);
    stubMedia({ coarse });
    vi.spyOn(document.documentElement, "clientHeight", "get").mockReturnValue(874);
    const { container } = render(<ChipPicker />);
    const chip = container.querySelector("button") as HTMLElement;
    const pop = container.querySelector(".popover") as HTMLElement;
    chip.getBoundingClientRect = () => rect(20, 400, 80, 30) as DOMRect;
    pop.getBoundingClientRect = () => rect(0, 0, 386, 200) as DOMRect;
    const move = (h: number) => {
      viewport.height = h;
      act(() => {
        viewport.dispatchEvent(new Event("resize"));
      });
    };
    return { pop, move };
  };

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("sits just above the keyboard, and goes back to its chip when it leaves", () => {
    const { pop, move } = setup(true);
    move(560);
    expect(pop.style.top).toBe("352px");
    expect(pop.style.maxHeight).toBe("544px");
    move(874);
    expect(pop.style.top).toBe("436px");
  });

  it("stays by its chip with a mouse, whatever the viewport does", () => {
    const { pop, move } = setup(false);
    const before = pop.style.top;
    move(560);
    expect(pop.style.top).toBe(before);
  });
});

describe("usePlacedPopover on the keyboard", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("is placed by its own size, not the smaller one its entrance's scale draws", async () => {
    const { container } = render(
      <form className="on-keyboard">
        <Picker />
      </form>,
    );
    const card = container.querySelector("form") as HTMLElement;
    const pop = container.querySelector(".popover") as HTMLElement;
    card.getBoundingClientRect = () => rect(0, 500, 402, 136) as DOMRect;
    // Drawn at 98% while it comes in; laid out 386 x 200.
    pop.getBoundingClientRect = () => rect(0, 0, 378.3, 196) as DOMRect;
    Object.defineProperty(pop, "offsetWidth", { value: 386 });
    Object.defineProperty(pop, "offsetHeight", { value: 200 });
    pop.appendChild(document.createElement("div"));
    await act(async () => {
      await Promise.resolve();
    });
    // Centered in jsdom's 1024 px window by its laid-out width.
    expect(pop.style.left).toBe("319px");
    expect(pop.style.top).toBe("-208px");
  });

  // The card is the picker's containing block (its transform), so the
  // picker's `top` is from the card's top: it rides along when the card moves.
  it("sits above the card, in the card's own coordinates, whatever WebKit scrolled", async () => {
    vi.stubGlobal("visualViewport", Object.assign(new EventTarget(), { offsetTop: 396, height: 478 }));
    const { container } = render(
      <form className="on-keyboard">
        <Picker />
      </form>,
    );
    const card = container.querySelector("form") as HTMLElement;
    const pop = container.querySelector(".popover") as HTMLElement;
    card.getBoundingClientRect = () => rect(0, 339, 402, 139) as DOMRect;
    pop.getBoundingClientRect = () => rect(0, 0, 386, 100) as DOMRect;
    Object.defineProperty(pop, "offsetWidth", { value: 386 });
    Object.defineProperty(pop, "offsetHeight", { value: 100 });
    card.style.bottom = "0px";
    await act(async () => {
      await Promise.resolve();
    });
    // 231 on the screen, 108 above the card's top.
    expect(pop.style.top).toBe("-108px");
    // Centered in jsdom's 1024 px window: 319 from the card's left edge (at 0 here).
    expect(pop.style.left).toBe("319px");
    expect(pop.style.maxHeight).toBe("323px");
  });

  it("places again when a step changes what's inside", async () => {
    const { container } = render(
      <form className="on-keyboard">
        <Picker />
      </form>,
    );
    const card = container.querySelector("form") as HTMLElement;
    const pop = container.querySelector(".popover") as HTMLElement;
    card.getBoundingClientRect = () => rect(0, 500, 402, 136) as DOMRect;
    pop.getBoundingClientRect = () => rect(0, 0, 386, 200) as DOMRect;
    Object.defineProperty(pop, "offsetWidth", { value: 386 });
    Object.defineProperty(pop, "offsetHeight", { value: 200 });
    pop.appendChild(document.createElement("div"));
    await act(async () => {
      await Promise.resolve();
    });
    expect(pop.style.top).toBe("-208px");
  });

  it("places against where the layout puts the card, not where its ride has it now", async () => {
    const { container } = render(
      <form className="on-keyboard">
        <Picker />
      </form>,
    );
    const card = container.querySelector("form") as HTMLElement;
    const pop = container.querySelector(".popover") as HTMLElement;
    // Laid out with its top at 600, drawn 294 higher as it starts down with the keyboard.
    card.getBoundingClientRect = () => rect(0, 306, 402, 136) as DOMRect;
    pop.getBoundingClientRect = () => rect(0, 0, 386, 400) as DOMRect;
    Object.defineProperty(pop, "offsetWidth", { value: 386 });
    Object.defineProperty(pop, "offsetHeight", { value: 400 });
    card.style.transform = "matrix(1, 0, 0, 1, 0, -294)";
    await act(async () => {
      await Promise.resolve();
    });
    // 192 on the screen once the card is down: 408 above its top.
    expect(pop.style.top).toBe("-408px");
    expect(pop.style.maxHeight).toBe("584px");
  });
});
