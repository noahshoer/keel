import UIKit

/// Glass's lines for Keel's diagnostics log, which lives in the keel-ios
/// plugin (`Diag`, a separate Swift package). Each line is posted as a
/// notification that `Diag` writes while logging is on, so neither package
/// depends on the other.
enum GlassLog {
  static let name = Notification.Name("keel.diag.line")

  static func log(_ text: String) {
    NotificationCenter.default.post(name: name, object: nil, userInfo: ["line": text, "source": "swift"])
  }

  private static var kvo: [NSKeyValueObservation] = []

  /// Logs the overlay's frame as it changes (the `HOST frame` lines).
  static func watch(overlay: UIView?) {
    guard let overlay, kvo.isEmpty else { return }
    kvo.append(overlay.observe(\.frame, options: [.new]) { h, _ in
      let f = h.frame
      log(String(format: "HOST frame [%.1f,%.1f %.1fx%.1f] safe ", f.minX, f.minY, f.width, f.height) + "\(h.safeAreaInsets)")
    })
  }
}

/// How long the keyboard takes to come or go, for the card's exit. The
/// keel-ios plugin's `KeyboardPace` sets the keyboard's speed to 0.9 of
/// iOS's own; the card follows the same pace.
enum GlassKeyboard {
  /// `KeyboardPace.speed` in the keel-ios plugin.
  static let speed = 0.9
  private(set) static var seconds = 0.3833 / speed
  private static var observers: [NSObjectProtocol] = []

  static func attach() {
    guard observers.isEmpty else { return }
    let nc = NotificationCenter.default
    for name in [UIResponder.keyboardWillShowNotification, UIResponder.keyboardWillHideNotification] {
      observers.append(nc.addObserver(forName: name, object: nil, queue: .main) { n in
        let dur = n.userInfo?[UIResponder.keyboardAnimationDurationUserInfoKey] as? Double ?? 0
        if dur > 0 { seconds = dur / speed }
      })
    }
  }
}
