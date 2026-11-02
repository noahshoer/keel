// What the view header does as the list scrolls. On the phone its title rides
// up into the top row, shrinking as it goes; once it's there the list starts
// going under the header, into a fade. Elsewhere the header holds still and the
// fade starts straight away.
//
// The scroll position reaches the page in whole points and unevenly, so the
// header doesn't copy it. It follows it with a critically damped spring, which
// eases off from still and steadies the wobble, frame by frame. CSS can't do
// this: `animation-timeline` isn't drawn on iOS once the finger lifts, and a
// transition restarts with each update. The values go on the header alone, so a
// frame restyles only the header and not the list.

/** The title is home by this scroll position. A list's spring-back keeps an even
 * pace down to about here and then creeps, and the title shouldn't creep with it.
 * Replaying flicks recorded on the phone, this took the typical return from 600ms
 * to 375ms, as one smooth rise and fall. */
const TITLE_FROM = 8;
/** The title's row in app.css (`--title-row`): the header shrinks by this much. */
const COLLAPSE_BY = 42;
/** How far the list scrolls before the fade is full height. Well under its
 * height in app.css: a fade squeezed into a short scroll is steep enough to read
 * as a line. */
const FADE_FULL_AT = 32;
/** The layout in app.css where the header collapses. */
const COLLAPSES = "(max-width: 700px) and (pointer: coarse)";
/** The spring's time constant in ms. Replaying flicks recorded on the phone, this
 * was the smoothest at the same delay as the plain follow it replaced, about 60ms. */
const FOLLOW_MS = 35;
/** Closer than this is too small to see, on values that run from 0 to 1. */
const LANDED = 0.002;

const unit = (x: number) => Math.min(Math.max(x, 0), 1);

export function headCollapse(scrollTop: number): number {
  // From the second pixel: a short list rests on its one pixel of bounce room.
  return unit((scrollTop - 1) / (COLLAPSE_BY - 1));
}

export function titleMove(scrollTop: number): number {
  return unit((scrollTop - TITLE_FROM) / (COLLAPSE_BY - TITLE_FROM));
}

export function listFade(scrollTop: number, collapses: boolean): number {
  return unit((scrollTop - (collapses ? COLLAPSE_BY : 0)) / FADE_FULL_AT);
}

export type Followed = { value: number; speed: number };

/** One frame of the spring: the exact solution, so frame lengths don't change its path. */
export function followStep(current: Followed, target: number, ms: number): Followed {
  const decay = Math.exp(-ms / FOLLOW_MS);
  const away = current.value - target;
  const pull = (current.speed + away / FOLLOW_MS) * ms;
  const value = target + (away + pull) * decay;
  const speed = (current.speed - pull / FOLLOW_MS) * decay;
  // Per frame at 60 a second, so the two limits are alike.
  return Math.abs(value - target) < LANDED && Math.abs(speed * 16) < LANDED
    ? { value: target, speed: 0 }
    : { value, speed };
}

/** Keeps `--head-collapse`, `--title-move` and `--list-fade` on `target` current; returns the function that stops. */
export function attachHeadScroll(scroller: HTMLElement, target: HTMLElement): () => void {
  const media = typeof matchMedia === "function" ? matchMedia(COLLAPSES) : null;
  const goal = () => {
    const collapses = media?.matches ?? false;
    const top = scroller.scrollTop;
    return {
      collapse: collapses ? headCollapse(top) : 0,
      move: collapses ? titleMove(top) : 0,
      fade: listFade(top, collapses),
    };
  };
  const first = goal();
  let collapse: Followed = { value: first.collapse, speed: 0 };
  let move: Followed = { value: first.move, speed: 0 };
  let fade: Followed = { value: first.fade, speed: 0 };
  const draw = () => {
    target.style.setProperty("--head-collapse", String(collapse.value));
    target.style.setProperty("--title-move", String(move.value));
    target.style.setProperty("--list-fade", String(fade.value));
  };
  let live = true;
  let following = false;
  let last = 0;
  const frame = (now: number) => {
    if (!live) return;
    const aim = goal();
    const ms = now - last;
    last = now;
    collapse = followStep(collapse, aim.collapse, ms);
    move = followStep(move, aim.move, ms);
    fade = followStep(fade, aim.fade, ms);
    draw();
    following = collapse.value !== aim.collapse || move.value !== aim.move || fade.value !== aim.fade;
    if (following) requestAnimationFrame(frame);
  };
  const follow = () => {
    if (following) return;
    following = true;
    last = performance.now();
    requestAnimationFrame(frame);
  };
  draw();
  scroller.addEventListener("scroll", follow, { passive: true });
  media?.addEventListener("change", follow);
  return () => {
    live = false;
    scroller.removeEventListener("scroll", follow);
    media?.removeEventListener("change", follow);
  };
}
