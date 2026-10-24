import CoreGraphics

/// The task sheet's top while a finger drags it, worked out natively from
/// the finger (no message per frame from the page). The page hands over,
/// once per drag, where the sheet's top and the finger were when the drag
/// took over and the large height's top (it never goes above that), all on
/// screen; it's the page's own rule, `top = largeTop + max(0, from + dy)`.
struct SheetFollow: Equatable {
  let top0: CGFloat
  let finger0: CGFloat
  let floor: CGFloat

  func top(finger: CGFloat) -> CGFloat {
    max(floor, top0 + finger - finger0)
  }
}
