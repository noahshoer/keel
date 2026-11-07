// The task card rides the keyboard: its bottom stays `gap` above the
// keyboard's top on every frame, except that it never goes below where it
// rests with the keyboard down (above the home indicator). Measured on the
// phone, a card that slid on its own (the keyboard's curve, its own distance,
// started a little ahead) ran up to 70 pt ahead of the keyboard
// (docs/superpowers/plans/2026-11-06-add-card-keyboard.md). So the card's move is
// the keyboard's own path, sampled from the keyboard's spring (`linear()` from
// Swift, keel-ios Keyboard.swift), and played as keyframes on its transform.

import type { Rect } from "./popover";

/** The vertical move of a computed `transform` (`matrix(…)`), 0 when there's none. */
export function translateY(transform: string): number {
  const values = /^matrix\((.+)\)$/.exec(transform)?.[1]?.split(",");
  return Number(values?.[5]) || 0;
}

/**
 * Where the layout puts `el`, without the move its transform is drawing now
 * (the card's ride, `rideCard`).
 */
export function layoutRect(el: HTMLElement): Rect {
  const r = el.getBoundingClientRect();
  const ty = translateY(getComputedStyle(el).transform);
  return { top: r.top - ty, bottom: r.bottom - ty, left: r.left, width: r.width, height: r.height };
}

/** The progress points of a `linear(…)` curve, evenly spaced in time; null for any other curve. */
export function springPoints(ease: string): number[] | null {
  const m = /^linear\((.*)\)$/.exec(ease.trim());
  if (!m?.[1]) return null;
  const points = m[1].split(",").map((p) => Number(p.trim()));
  if (points.length < 2 || points.some((p) => !Number.isFinite(p))) return null;
  return points;
}

export type Glide = {
  /** The keyboard's progress at evenly spaced times (0 to 1). */
  points: number[];
  /** The keyboard's top before and after the move (viewport y). */
  kbFrom: number;
  kbTo: number;
  /** How far above the keyboard the card's bottom sits. */
  gap: number;
  /** The card's bottom with the keyboard down: it never goes lower. */
  rest: number;
  /** Where the card's bottom is drawn now, and where the layout now puts it. */
  from: number;
  final: number;
};

/**
 * The card's transform (`translateY`, px from its final place) at each point.
 * When the card isn't where the path starts (another move was under way),
 * the difference fades out as the keyboard moves, so it starts where it is
 * and still ends in place.
 */
export function gluedPath({ points, kbFrom, kbTo, gap, rest, from, final }: Glide): number[] {
  const bottom = (p: number) => Math.min(kbFrom + (kbTo - kbFrom) * p - gap, rest);
  const off = from - bottom(0);
  return points.map((p) => bottom(p) - final + off * (1 - p));
}
