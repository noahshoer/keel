# The add card and the keyboard (2026-11-06)

Problem: the add card stuttered as it came in with the keyboard, the date's
calendar jumped while the keyboard went down, and the first keyboard after
launch snapped in instead of sliding.

How it was found: frame-by-frame traces of the keyboard (its host layer, from a
display link) and the card (its transform, per animation frame), plus 60 fps
screen recordings measured by the card's and keyboard's edges. The tracing
was removed afterwards; the recordings were the truth (the host layer's
samples run a frame or two behind what's drawn).

What it showed and what changed:

- The card started 50 ms into the keyboard's stiff spring: a 96 pt jump on its
  first frame, a frozen frame, then 70 pt ahead of the keyboard. It also
  travelled its own distance (it rests 42 pt above the screen's bottom). Now
  it starts with the keyboard, and its bottom is `min(keyboard top − 8, where
  it rests)` on every frame (`gluedPath`, `web/src/editor/glide.ts`), played
  as keyframes from the keyboard's spring (the `linear()` Swift sends) on its
  transform. Swift's default curve is that spring (k 555.0265, c 47.118,
  0.3833 s), so the first keyboard gets it too.
- A keyboard going down counts as down at once (`held` 0), so the card rides
  it down; a repeated announcement of the same move (iOS sends two or three)
  leaves the ride alone.
- The calendar is `position: fixed` inside the card, and a card with a
  transform is its containing block, so it was drawn off the card while the
  card rode. The card now always has a transform (`translateY(0)` at rest),
  pickers are placed in its coordinates against where the layout puts it
  (`layoutRect`, `followCard`), and while it rides an open picker plays the
  path backwards, so it holds still on the screen. It rises in on
  `translate`/`scale`/`opacity` (`picker-rise`).
- The first keyboard after launch snapped because of the page's keyboard
  primer (a hidden `inputmode="none"` field focused on the first tap), which
  seemed to leave the keyboard's area open. It's gone. Its job (a drop on Add
  section raising the keyboard) moved to keel-ios `PageFocus`, which tells
  WebKit's private `_elementDidFocus:userIsInteracting:…` the person is
  interacting (as Capacitor's Keyboard plugin does). `PageFocus` also makes
  WebKit's content view first responder at launch; otherwise it became one in
  the first tap, and WebKit asked for the keyboard twice (a snap).
- Left as is: a very quick tap on + right after launch still has WebKit ask
  twice, and iOS shows the keyboard with no animation. Tried and dropped: a
  keyboard warm-up (iOS raises no keyboard before the first touch), making the
  text effects window at launch, holding WebKit's second request (it was the
  one iOS needed), focusing 40 ms later, and waiting for WebKit's touch
  request. In that case only, the card rises in 0.15 s (`SNAPPED_RISE_SECS`).
- Also tried and rolled back: starting the card a frame late and the keyboard
  at 0.8×. The card then trailed the keyboard, which looked like it pushed it.
- Opening the date's calendar: the page lowers the keyboard itself (`makeRoom`
  blurs the field), so it starts the card's ride at once with the keyboard's
  last pace (`lowerKeyboardNow`) and blurs on the next frame. Waiting for
  Swift's report, the card set off ~35 ms after the keyboard (a gap opened to
  88 pt); blurring in the same task, the same; holding the card back a frame,
  it trailed again. As is, the card leads by a few points at most. The
  calendar is built only after the keyboard's move has started
  (`afterKeyboardMove`): building it first (~60 ms) held the card back. It
  unfolds upward once built (`sched-card`, 0.25 s, solid: a fade showed the list
  through it), the quick choices fade out first, and
  pickers on the card are placed by their laid-out size (their entrance
  scales them, which shifted them left once it ended).
