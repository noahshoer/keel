# Keyboard pace and the task sheet (2026-10-29)

Problem: adding a sub-task with the task sheet at medium, the keyboard slides
in and only afterwards the sheet jumps to fit above it. The diag log shows
why: the visual viewport resizes ~420 ms after `keyboardWillShow` (after
`DidShow`), and `--kb` / `--vv-h` come from the visual viewport. On hide the
resize lands ~10 ms after `WillHide`. Both directions report 0.3833 s.

Wanted:
- App wide, the keyboard slides in and out at one speed, 10% slower than iOS.
- Whenever the keyboard comes up in the task view, the sheet goes to its
  large height with it, at the keyboard's pace.
- Once the keyboard closes, the sheet stays large.

Decisions:
- `KeyboardPace` (Swift) runs the keyboard's windows (`UIRemoteKeyboardWindow`,
  `UITextEffectsWindow`) at layer speed 0.9, so show and hide both take
  0.3833 / 0.9 ≈ 0.426 s. The notifications still report iOS's own duration.
- On `WillShow` it calls `window.keelKeyboard({ kb, secs, ease })` at once
  (`kb` is the keyboard's overlap of the web view). The curve is measured
  from the keyboard's own animation after each show and used from then on
  (until the first measurement, an approximation). `WillHide` sends `kb: 0`.
- The page (`SHEET_DRAG_JS`): from medium the sheet switches to large and
  slides up from where it is (`keelSheet.kbExpand`, the rule in Rust).
  Either way its bottom follows the keyboard (`--kb` with a transition).
  `--kb` from the visual viewport doesn't undercut the keyboard's height
  until the viewport has caught up.
- The native card leaves at the keyboard's pace (`KeyboardPace.seconds`)
  instead of 0.25 s.
