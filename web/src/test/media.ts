import { vi } from "vitest";

// A stand-in for `window.matchMedia` that answers by the kind of query. The
// listeners do nothing: the answers never change during a test.

export type MediaAnswers = {
  /** `(pointer: coarse)`: a touch screen. */
  coarse?: boolean;
  /** `(min-width: …)`: the desktop layout. */
  wide?: boolean;
  /** `(prefers-reduced-motion: …)`. */
  reduced?: boolean;
  /** Any other query. */
  other?: boolean;
};

export function stubMedia(answers: MediaAnswers = {}): void {
  const answer = (q: string): boolean => {
    if (q.includes("coarse")) return answers.coarse ?? false;
    if (q.includes("min-width")) return answers.wide ?? false;
    if (q.includes("reduced-motion")) return answers.reduced ?? false;
    return answers.other ?? false;
  };
  vi.stubGlobal("matchMedia", (q: string) => ({
    matches: answer(q),
    media: q,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}
