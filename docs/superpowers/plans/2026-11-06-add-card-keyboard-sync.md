# The add card rises with the keyboard (2026-11-06)

Problem: tapping "+ Add task" (or the round add button) opens the task card
at once, low on the screen, and the keyboard slides up under it; the card
only jumps to sit on the keyboard when the visual viewport resizes, about
420 ms after `keyboardWillShow` (see `2026-10-29-keyboard-pace.md`). Wanted:
the card and the keyboard rise together, and fall together.

Decisions:
- Swift already calls `window.keelKeyboard({ kb, secs, ease })` as the
  keyboard starts to move, and `installSheet` (always on) receives it. It
  also announces it on `window` as a `keel-keyboard` event (`KEYBOARD_EVENT`,
  in `gestures/sheet-rules.ts`), so the card can follow without a second
  global.
- `useKeyboard(enabled, paced)` (the task card passes `paced`) holds `kb`
  as the inset from the moment the keyboard starts until the viewport has
  caught up, as the sheet's `kbHold` does (`readKeyboard`, the same
  `keyboardInset` rule). While held it reads as up, and `room` is the
  height left above it, so the card doesn't change class or height when the
  viewport resizes.
- The card's `bottom` and `margin-bottom` transition at `secs` and `ease`
  (`KeyboardState.slide`) for as long as the move lasts, so a later move of
  the viewport (a scroll, a picker's height) stays instant. Going down is the
  same move to 0.
- The drawer's `useKeyboard` call doesn't pass `paced`: unchanged.
- Reduced motion arrives as `secs: 0`: no transition.

Not checked in the simulator (it has no software keyboard): Noah tries it on
the phone with diagnostics logging on (`keelKeyboard kb … secs … ease …` is
logged by `sheet.ts`).
