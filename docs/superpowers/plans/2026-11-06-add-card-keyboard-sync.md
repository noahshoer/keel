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
- The card moves by `transform`, not `bottom`: animating `bottom` is layout on
  the page's thread and trailed the keyboard on the phone (the log showed the
  start reaching the page within 5-14 ms, so it wasn't the hop). The layout
  jumps to the final place at once; `rideCard` starts the card from where it
  was (`translateY(delta)` with no transition), then transitions the
  transform away at `secs` and `ease`, and puts its style back afterwards. The
  same trick the sheet and `.keyboard-card` use.
- The curve is the keyboard's real spring, which Swift reads off the text
  effects window's animation (it used to log it and send an approximation).
- The drawer's `useKeyboard` call doesn't pass `paced`: unchanged.
- Reduced motion arrives as `secs: 0`: no transition.

Not checked in the simulator (it has no software keyboard): Noah tries it on
the phone with diagnostics logging on (`keelKeyboard kb … secs … ease …` is
logged by `sheet.ts`).
