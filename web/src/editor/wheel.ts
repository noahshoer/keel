// The month / year wheels of the date picker's chooser, like iOS
// Calendar's (the Dioxus `WHEEL_JS` in crates/ui/src/pickers.rs).
//
// The month wheel loops with only its twelve rows: `pos` is the
// (fractional) month at the band, and each row is moved to its distance
// from it, wrapped to -6…6, so December sits above January. Dragging, the
// mouse wheel and a tap move `pos`; a flick coasts, then it snaps to a
// month. The year wheel is a plain snap-scrolling list.

/** A row's height, matching `.wheel-item`. */
export const ROW = 36;

/** Row `i`'s distance from the band at `pos`, wrapped into -n/2…n/2. */
export function wrapOffset(i: number, pos: number, n: number): number {
  let d = (((i - pos) % n) + n) % n;
  if (d > n / 2) d -= n;
  return d;
}

/** Where a released drag settles: about a third of a second's coast. */
export function flickTarget(pos: number, velocity: number): number {
  return Math.round(pos - (velocity * 320) / ROW);
}

/** Drives the month column; `onSettle` gets the month (0-based) that lands in the band. */
export function installMonthWheel(
  col: HTMLElement,
  start: number,
  onSettle: (month0: number) => void,
): () => void {
  const rows = [...col.querySelectorAll<HTMLElement>(".wheel-item")];
  const n = rows.length;
  const wrap = (i: number) => ((i % n) + n) % n;
  let pos = start;
  let last = start;
  let anim = 0;
  let snapT: ReturnType<typeof setTimeout> | undefined;
  const draw = () => {
    rows.forEach((el, i) => {
      const d = wrapOffset(i, pos, n);
      el.style.transform = `translateY(${d * ROW}px)`;
      el.style.visibility = Math.abs(d) > 3 ? "hidden" : "";
    });
  };
  const settle = () => {
    pos = wrap(Math.round(pos));
    draw();
    if (pos !== last) {
      last = pos;
      onSettle(pos);
    }
  };
  const glide = (to: number, ms: number) => {
    cancelAnimationFrame(anim);
    const from = pos;
    const t0 = performance.now();
    const step = (t: number) => {
      const k = Math.min(1, (t - t0) / ms);
      pos = from + (to - from) * (1 - (1 - k) ** 3);
      draw();
      if (k < 1) anim = requestAnimationFrame(step);
      else settle();
    };
    anim = requestAnimationFrame(step);
  };
  let y0: number | null = null;
  let yPrev = 0;
  let tPrev = 0;
  let v = 0;
  let moved = 0;
  const down = (e: PointerEvent) => {
    cancelAnimationFrame(anim);
    try {
      col.setPointerCapture(e.pointerId);
    } catch {
      // Capture is a nicety; dragging still works without it.
    }
    y0 = yPrev = e.clientY;
    tPrev = performance.now();
    v = 0;
    moved = 0;
  };
  const move = (e: PointerEvent) => {
    if (y0 === null) return;
    const t = performance.now();
    const dy = e.clientY - yPrev;
    pos -= dy / ROW;
    moved += Math.abs(dy);
    v = dy / Math.max(1, t - tPrev);
    yPrev = e.clientY;
    tPrev = t;
    draw();
  };
  const release = (e: PointerEvent) => {
    if (y0 === null) return;
    y0 = null;
    if (moved < 4) {
      // A tap: spin the tapped month to the band.
      const it = document
        .elementFromPoint(e.clientX, e.clientY)
        ?.closest<HTMLElement>(".wheel-col.loop .wheel-item");
      if (it) {
        glide(pos + wrapOffset(Number(it.dataset["i"]), pos, n), 260);
        return;
      }
    }
    glide(flickTarget(pos, v), 420);
  };
  const wheel = (e: WheelEvent) => {
    e.preventDefault();
    cancelAnimationFrame(anim);
    pos += e.deltaY / ROW;
    draw();
    clearTimeout(snapT);
    snapT = setTimeout(() => glide(Math.round(pos), 160), 110);
  };
  col.addEventListener("pointerdown", down);
  col.addEventListener("pointermove", move);
  col.addEventListener("pointerup", release);
  col.addEventListener("pointercancel", release);
  col.addEventListener("wheel", wheel, { passive: false });
  draw();
  return () => {
    cancelAnimationFrame(anim);
    clearTimeout(snapT);
    col.removeEventListener("pointerdown", down);
    col.removeEventListener("pointermove", move);
    col.removeEventListener("pointerup", release);
    col.removeEventListener("pointercancel", release);
    col.removeEventListener("wheel", wheel);
  };
}

/** Drives the year column; `onSettle` gets the index of the year that lands in the band. */
export function installYearWheel(
  col: HTMLElement,
  startIndex: number,
  onSettle: (index: number) => void,
): () => void {
  col.scrollTop = startIndex * ROW;
  let last = startIndex;
  let t: ReturnType<typeof setTimeout> | undefined;
  const scroll = () => {
    clearTimeout(t);
    t = setTimeout(() => {
      const i = Math.round(col.scrollTop / ROW);
      if (i !== last) {
        last = i;
        onSettle(i);
      }
    }, 110);
  };
  const click = (e: MouseEvent) => {
    const it = (e.target as Element | null)?.closest<HTMLElement>(".wheel-item");
    if (it) col.scrollTo({ top: Number(it.dataset["i"]) * ROW, behavior: "smooth" });
  };
  col.addEventListener("scroll", scroll);
  col.addEventListener("click", click);
  return () => {
    clearTimeout(t);
    col.removeEventListener("scroll", scroll);
    col.removeEventListener("click", click);
  };
}
