# Draggable drawer (issue #12 follow-up)

Noah, 2026-11-07: the drawer shouldn't just fly in; it follows the finger and, once released past a point, carries on. Settings sliding away to reveal the drawer behaves the same.

## Decisions
- One generic horizontal pan, `web/src/gestures/pan.ts`: tracks a single finger, decides intent past the slop (sideways wins, vertical is left to scrolling), cancels the scroll once it is a pan, reports progress 0..1 along one direction, and on release says whether to commit.
- Rules in `sheet-rules.ts` (tested first): `panIntent`, `panProgress`, `panCommits` (past half the range, or a flick of at least `FLICK_SPEED` in the pan's direction).
- Drawer (`web/src/shell/drawerDrag.ts`): opening drags right from a closed drawer, closing drags left from an open one. While dragging, `.app[data-drawer-drag]` and `--p` (0 shut, 1 open) drive the sidebar, scrim and the page's parallax in CSS; release settles at the drawer's .26s pace, then React's `drawer` state is set.
- Settings (phone): dragging right moves the screen with the finger over the open sidebar; past the point it carries off (the existing `.leaving` animation continues from where it is), otherwise it springs back.
- `drawerSwipe` and `settingsSwipe` go (the pan replaces them). `installSwipe` stays for the instant ones: left opens Search, right on Search goes back.
- Starts anywhere not blocked (fields, popovers, the task sheet), as the swipe did; phone width only. Reduced motion is not special-cased (the drawer's CSS doesn't either).
