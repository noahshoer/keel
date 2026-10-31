import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { currentPlatform } from "../shell/platform";
import { setWidth, touch } from "../test/touch";
import { installSheet, slideClose } from "./sheet";

vi.mock("../shell/platform", () => ({ currentPlatform: vi.fn(() => "touch") }));

let uninstall: () => void;
let onClose: ReturnType<typeof vi.fn<() => void>>;

beforeEach(() => {
  vi.useFakeTimers();
  vi.mocked(currentPlatform).mockReturnValue("touch");
  setWidth(390);
  document.body.innerHTML = `
    <div class="tv-scrim" id="scrim"></div>
    <div class="tv-sheet" id="sheet" data-detent="medium">
      <div class="tv-stack">
        <div class="task-view tv-under" id="under"><div class="tv-body" id="under-body">Parent</div></div>
        <div class="task-view" id="top">
          <div class="tv-head"><button class="tv-back" id="back">‹</button></div>
          <div class="tv-body" id="body"><div class="tv-scroll" id="scroll">Sub-task</div></div>
        </div>
      </div>
    </div>`;
  onClose = vi.fn<() => void>();
  uninstall = installSheet(document, window, onClose);
});

afterEach(() => {
  uninstall();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const el = (id: string) => document.getElementById(id) as HTMLElement;

/** A vertical drag on the sheet's content: down by `dy`, 100 ms per step. */
function dragDown(dy: number) {
  const steps = [10, dy / 2, dy];
  touch("touchstart", el("scroll"), 200, 300, 0);
  steps.forEach((s, i) => {
    touch("touchmove", el("scroll"), 200, 300 + s, (i + 1) * 100);
  });
  touch("touchend", el("scroll"), 200, 300 + dy, 400);
}

/** A swipe in from x 6, `steps` px to the right at 50 ms each, lifted 10 ms later. */
function edgeSwipe(steps: number[]) {
  touch("touchstart", el("scroll"), 6, 300, 0);
  steps.forEach((s, i) => {
    touch("touchmove", el("scroll"), 6 + s, 300, i * 50);
  });
  touch("touchend", el("scroll"), 6 + (steps.at(-1) ?? 0), 300, (steps.length - 1) * 50 + 10);
}

describe("the vertical drag", () => {
  it("a 100 px drag down from medium goes large while dragged and closes after 300 ms", () => {
    touch("touchstart", el("scroll"), 200, 300, 0);
    touch("touchmove", el("scroll"), 200, 310, 100);
    touch("touchmove", el("scroll"), 200, 350, 200);
    expect(el("sheet").classList.contains("tv-large")).toBe(true);
    touch("touchmove", el("scroll"), 200, 400, 300);
    touch("touchend", el("scroll"), 200, 400, 400);
    vi.advanceTimersByTime(299);
    expect(onClose).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("a 30 px drag settles back at medium", () => {
    dragDown(30);
    vi.advanceTimersByTime(300);
    expect(onClose).not.toHaveBeenCalled();
    expect(el("sheet").dataset["detent"]).toBe("medium");
    expect(el("sheet").classList.contains("tv-large")).toBe(false);
    expect(el("sheet").style.transform).toBe("");
  });

  it("the drag cancels the browser's scroll", () => {
    touch("touchstart", el("scroll"), 200, 300, 0);
    const first = touch("touchmove", el("scroll"), 200, 320, 100);
    expect(first.defaultPrevented).toBe(true);
  });

  it("nothing moves on the desktop panel", () => {
    setWidth(1280);
    dragDown(100);
    vi.advanceTimersByTime(300);
    expect(onClose).not.toHaveBeenCalled();
    expect(el("sheet").classList.contains("tv-large")).toBe(false);
    expect(el("sheet").style.transform).toBe("");
  });
});

describe("the edge swipe back", () => {
  it("a 150 px flick in from the edge clicks Back after the slide", () => {
    const back = vi.fn();
    el("back").addEventListener("click", back);
    edgeSwipe([20, 80, 150]);
    // Released, it slides the rest of the way off.
    expect(el("body").style.transform).toBe("translateX(390px)");
    expect(el("under").style.visibility).toBe("visible");
    vi.advanceTimersByTime(299);
    expect(back).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(back).toHaveBeenCalledTimes(1);
  });

  it("a slow 40 px swipe springs back", () => {
    const back = vi.fn();
    el("back").addEventListener("click", back);
    edgeSwipe([20, 40]);
    vi.advanceTimersByTime(1000);
    expect(back).not.toHaveBeenCalled();
    expect(el("body").style.transform).toBe("");
  });

  it("only from the edge", () => {
    const back = vi.fn();
    el("back").addEventListener("click", back);
    touch("touchstart", el("scroll"), 60, 300, 0);
    touch("touchmove", el("scroll"), 260, 300, 50);
    touch("touchend", el("scroll"), 260, 300, 60);
    vi.advanceTimersByTime(1000);
    expect(back).not.toHaveBeenCalled();
  });
});

describe("slideClose", () => {
  it("is false on the desktop panel", () => {
    setWidth(1280);
    expect(slideClose(document, window)).toBe(false);
    vi.advanceTimersByTime(1000);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("slides the sheet off and closes once, even when called twice", () => {
    expect(slideClose(document, window)).toBe(true);
    expect(slideClose(document, window)).toBe(true);
    expect(el("sheet").classList.contains("tv-large")).toBe(true);
    vi.advanceTimersByTime(299);
    expect(onClose).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("is false once the listeners are gone", () => {
    uninstall();
    uninstall = () => {};
    expect(slideClose(document, window)).toBe(false);
  });
});

describe("keelKeyboard", () => {
  it("a keyboard coming up takes a medium sheet to large on a touch device", () => {
    window.keelKeyboard?.({ kb: 328, secs: 0.25, ease: "ease" });
    expect(el("sheet").dataset["detent"]).toBe("large");
    expect(el("sheet").classList.contains("tv-large")).toBe(true);
  });

  it("slides at the sheet's own pace when the keyboard gives none", () => {
    // The first keyboard after launch: iOS reports a 0 s animation.
    window.keelKeyboard?.({ kb: 396, secs: 0, ease: "ease" });
    // One transition, no "none" in the list (that would void it).
    expect(el("sheet").style.transition).toBe("transform 0.3s cubic-bezier(.32,.72,0,1)");
    expect(el("sheet").style.transform).toBe("translateY(0px)");
    vi.advanceTimersByTime(300);
    expect(el("sheet").style.transform).toBe("");
  });

  it("does nothing elsewhere", () => {
    vi.mocked(currentPlatform).mockReturnValue("other");
    window.keelKeyboard?.({ kb: 328, secs: 0.25, ease: "ease" });
    expect(el("sheet").dataset["detent"]).toBe("medium");
  });

  it("is installed and removed with the listeners", () => {
    expect(typeof window.keelKeyboard).toBe("function");
    uninstall();
    uninstall = () => {};
    expect(window.keelKeyboard).toBeUndefined();
  });
});
