import UIKit

/// Watches the finger on the web view for the task sheet's glass buttons
/// (`SheetFollow`): it never recognizes, so it never cancels, delays or
/// steals a web touch, and it has no pan hysteresis.
final class FingerTracker: UIGestureRecognizer {
  /// The finger's y in the web view (nil when the touch ends).
  var onMove: (CGFloat?) -> Void = { _ in }

  override init(target: Any?, action: Selector?) {
    super.init(target: target, action: action)
    cancelsTouchesInView = false
    delaysTouchesBegan = false
    delaysTouchesEnded = false
  }

  private func report(_ touches: Set<UITouch>, _ event: UIEvent) {
    guard event.allTouches?.count == 1, let t = touches.first, let v = view else { return }
    onMove(t.location(in: v).y)
  }

  override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent) { report(touches, event) }
  override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent) { report(touches, event) }
  override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent) {
    onMove(nil)
    state = .failed
  }
  override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent) {
    onMove(nil)
    state = .failed
  }
  override func canPrevent(_ preventedGestureRecognizer: UIGestureRecognizer) -> Bool { false }
  override func canBePrevented(by preventingGestureRecognizer: UIGestureRecognizer) -> Bool { false }
}
