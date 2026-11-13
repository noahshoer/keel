import { afterEach, describe, expect, it, vi } from "vitest";
import { stubMedia } from "../test/media";
import { attachHeadScroll, type Followed, followStep, headCollapse, listFade, titleMove } from "./headScroll";

describe("headCollapse", () => {
  it("follows the list one to one until the title is in the top row", () => {
    expect(headCollapse(0)).toBe(0);
    expect(headCollapse(21.5)).toBe(0.5);
    expect(headCollapse(42)).toBe(1);
    expect(headCollapse(400)).toBe(1);
  });

  it("is fully open at a short list's resting position, its one pixel of bounce room", () => {
    expect(headCollapse(1)).toBe(0);
  });

  it("stays open while the list bounces past its top", () => {
    expect(headCollapse(-40)).toBe(0);
  });
});

describe("titleMove", () => {
  it("is done before the slow end of a spring-back, while the list's pace is still even", () => {
    expect(titleMove(1)).toBe(0);
    expect(titleMove(8)).toBe(0);
    expect(titleMove(25)).toBe(0.5);
    expect(titleMove(42)).toBe(1);
    expect(titleMove(400)).toBe(1);
  });

  it("stays home while the list bounces past its top", () => {
    expect(titleMove(-40)).toBe(0);
  });
});

describe("listFade", () => {
  it("without a collapsing header, grows from the first pixel of scroll", () => {
    expect(listFade(0, false)).toBe(0);
    expect(listFade(16, false)).toBe(0.5);
    expect(listFade(32, false)).toBe(1);
  });

  it("with a collapsing header, waits until the list starts going under it", () => {
    expect(listFade(42, true)).toBe(0);
    expect(listFade(58, true)).toBe(0.5);
    expect(listFade(74, true)).toBe(1);
  });

  it("is gone while the list bounces past its top", () => {
    expect(listFade(-40, false)).toBe(0);
    expect(listFade(-40, true)).toBe(0);
  });
});

describe("followStep", () => {
  const still = (value: number) => ({ value, speed: 0 });
  const run = (from: number, to: number, frames: number) => {
    const seen = [still(from)];
    for (let i = 0; i < frames; i++) seen.push(followStep(seen[seen.length - 1] as Followed, to, 17));
    return seen;
  };
  const steps = (seen: Followed[]) => seen.slice(1).map((f, i) => f.value - (seen[i] as Followed).value);

  it("eases off from still, with no kick in its first frame", () => {
    const moved = steps(run(0, 1, 6));
    expect(moved[0]).toBeGreaterThan(0);
    expect(moved[1]).toBeGreaterThan(moved[0] as number);
    expect(moved[0]).toBeLessThan(0.1);
  });

  it("arrives without overshooting", () => {
    for (const f of run(0, 1, 200)) expect(f.value).toBeLessThanOrEqual(1);
  });

  it("lands exactly and stops", () => {
    const last = run(0, 1, 200).at(-1);
    expect(last).toEqual({ value: 1, speed: 0 });
  });

  it("moves at the same pace whatever the frame lengths", () => {
    const two = followStep(followStep(still(0), 1, 10), 1, 10);
    const one = followStep(still(0), 1, 20);
    expect(two.value).toBeCloseTo(one.value, 10);
    expect(two.speed).toBeCloseTo(one.speed, 10);
  });

  it("steadies a target that wobbles from frame to frame", () => {
    let f = still(0);
    let target = 0;
    const seen = [f];
    for (let i = 0; i < 12; i++) {
      target += i % 2 ? 0.01 : 0.05;
      f = followStep(f, target, 17);
      seen.push(f);
    }
    const moved = steps(seen).slice(6);
    const changes = moved.slice(1).map((m, i) => Math.abs(m - (moved[i] as number)));
    expect(Math.max(...changes)).toBeLessThan(0.01);
  });
});

describe("attachHeadScroll", () => {
  afterEach(() => vi.unstubAllGlobals());

  const phone = (matches: boolean) => stubMedia({ coarse: matches });
  const value = (el: HTMLElement, name: string) => Number(el.style.getPropertyValue(name));

  /** A scroller and target, with frames run by hand. */
  const setup = () => {
    let frames: FrameRequestCallback[] = [];
    let now = 0;
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => frames.push(cb));
    vi.stubGlobal("performance", { now: () => now });
    const run = (count: number) => {
      for (let i = 0; i < count; i++) {
        now += 17;
        const due = frames;
        frames = [];
        for (const cb of due) cb(now);
      }
    };
    const scroller = document.createElement("div");
    const target = document.createElement("div");
    const scrollTo = (top: number) => {
      scroller.scrollTop = top;
      scroller.dispatchEvent(new Event("scroll"));
    };
    return { scroller, target, scrollTo, run, pending: () => frames.length };
  };

  it("starts exactly at the scroll position, with no frame needed", () => {
    phone(true);
    const { scroller, target } = setup();
    scroller.scrollTop = 58;
    attachHeadScroll(scroller, target);
    expect(value(target, "--head-collapse")).toBe(1);
    expect(value(target, "--title-move")).toBe(1);
    expect(value(target, "--list-fade")).toBe(0.5);
  });

  it("on the phone, the title holds still for the end of a spring-back while the header follows the list", () => {
    phone(true);
    const { scroller, target, scrollTo, run } = setup();
    attachHeadScroll(scroller, target);
    scrollTo(8);
    run(60);
    expect(value(target, "--head-collapse")).toBeGreaterThan(0);
    expect(value(target, "--title-move")).toBe(0);
  });

  it("on the phone, follows over several frames and lands exactly", () => {
    phone(true);
    const { scroller, target, scrollTo, run, pending } = setup();
    attachHeadScroll(scroller, target);
    scrollTo(58);
    expect(value(target, "--head-collapse")).toBe(0);
    run(1);
    const first = value(target, "--head-collapse");
    expect(first).toBeGreaterThan(0);
    expect(first).toBeLessThan(1);
    run(60);
    expect(value(target, "--head-collapse")).toBe(1);
    expect(value(target, "--list-fade")).toBe(0.5);
    expect(pending()).toBe(0);
  });

  it("a flick stretching a short list past its end still moves the title", () => {
    phone(true);
    const { scroller, target, scrollTo, run } = setup();
    attachHeadScroll(scroller, target);
    scrollTo(176);
    run(60);
    expect(value(target, "--head-collapse")).toBe(1);
    expect(value(target, "--list-fade")).toBe(1);
  });

  it("stops following once detached", () => {
    phone(true);
    const { scroller, target, scrollTo, run, pending } = setup();
    const detach = attachHeadScroll(scroller, target);
    scrollTo(58);
    detach();
    run(5);
    expect(value(target, "--head-collapse")).toBe(0);
    expect(pending()).toBe(0);
  });

  it("elsewhere, the header never collapses", () => {
    phone(false);
    const { scroller, target, scrollTo, run } = setup();
    attachHeadScroll(scroller, target);
    scrollTo(16);
    run(60);
    expect(value(target, "--head-collapse")).toBe(0);
    expect(value(target, "--title-move")).toBe(0);
    expect(value(target, "--list-fade")).toBe(0.5);
  });
});
