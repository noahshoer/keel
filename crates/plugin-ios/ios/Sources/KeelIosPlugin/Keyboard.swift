import UIKit
import WebKit

/// Keel's keyboard pace, app wide: the keyboard slides in and out at one
/// speed, 10% slower than iOS's own (its windows' layers run at `speed`).
/// The page hears of each move as it starts (`window.keelKeyboard`), so the
/// task sheet moves with the keyboard instead of after it: the web view's
/// visual viewport only resizes once the keyboard is up.
enum KeyboardPace {
  /// The keyboard's animation speed (1 is iOS's own).
  static let speed: Float = 0.9
  /// How long the keyboard takes to come or go, at `speed` (iOS 26 reports
  /// 0.3833 s both ways; updated from each notification).
  private(set) static var seconds = 0.3833 / Double(speed)
  /// The keyboard's curve in CSS, measured from its own animation after
  /// each show (until the first, an approximation).
  private(set) static var ease = "cubic-bezier(.25,.9,.3,1)"

  private static weak var webview: WKWebView?
  private static var observers: [NSObjectProtocol] = []

  static func attach(_ webview: WKWebView) {
    self.webview = webview
    guard observers.isEmpty else { return }
    let nc = NotificationCenter.default
    observers.append(nc.addObserver(forName: UIWindow.didBecomeVisibleNotification, object: nil, queue: .main) { n in
      if let w = n.object as? UIWindow { pace(w) }
    })
    for (name, up) in [(UIResponder.keyboardWillShowNotification, true), (UIResponder.keyboardWillHideNotification, false)] {
      observers.append(nc.addObserver(forName: name, object: nil, queue: .main) { n in moving(n, up: up) })
    }
    windows().forEach(pace)
  }

  /// The keyboard's own windows (the keyboard, and the text effects window
  /// that carries its accessory bar).
  private static func isKeyboard(_ w: UIWindow) -> Bool {
    let name = NSStringFromClass(type(of: w))
    return name.contains("Keyboard") || name.contains("TextEffects")
  }

  private static func windows() -> [UIWindow] {
    UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap(\.windows)
  }

  private static func pace(_ w: UIWindow) {
    guard isKeyboard(w), w.layer.speed != speed else { return }
    w.layer.speed = speed
    Diag.log("KB pace \(NSStringFromClass(type(of: w))) speed \(speed)")
  }

  private static func moving(_ n: Notification, up: Bool) {
    windows().forEach(pace)
    let info = n.userInfo ?? [:]
    let dur = info[UIResponder.keyboardAnimationDurationUserInfoKey] as? Double ?? 0
    if dur > 0 { seconds = dur / Double(speed) }
    var kb = 0.0
    if up, let webview, let end = (info[UIResponder.keyboardFrameEndUserInfoKey] as? NSValue)?.cgRectValue {
      let f = webview.convert(end, from: nil)
      kb = max(0, webview.bounds.maxY - f.minY)
    }
    let secs = dur > 0 ? seconds : 0
    let js = String(format: "window.keelKeyboard?.({ kb: %.1f, secs: %.4f, ease: '%@' })", kb, secs, ease)
    webview?.evaluateJavaScript(js)
    if up && dur > 0 { DispatchQueue.main.async { measure() } }
  }

  /// The curve of the keyboard's slide, read off its animation once it has
  /// been added (a spring sampled into `linear()`, or a cubic Bézier).
  private static func measure() {
    for w in windows() where NSStringFromClass(type(of: w)).contains("Keyboard") {
      guard let (key, anim) = slide(in: w.layer) else { continue }
      if let s = anim as? CASpringAnimation {
        ease = linear(s)
        Diag.log("KB curve \(key) spring m \(s.mass) k \(s.stiffness) c \(s.damping) v \(s.initialVelocity) dur \(s.duration) settle \(s.settlingDuration)")
      } else if let b = anim as? CABasicAnimation, let f = b.timingFunction {
        var p = [Float](repeating: 0, count: 2), q = [Float](repeating: 0, count: 2)
        f.getControlPoint(at: 1, values: &p)
        f.getControlPoint(at: 2, values: &q)
        ease = String(format: "cubic-bezier(%.3f,%.3f,%.3f,%.3f)", p[0], p[1], q[0], q[1])
        Diag.log("KB curve \(key) basic \(ease) dur \(b.duration)")
      } else {
        Diag.log("KB curve \(key) \(type(of: anim)) dur \(anim.duration)")
      }
      return
    }
    Diag.log("KB curve not found")
  }

  /// The first position or transform animation in the layer tree.
  private static func slide(in layer: CALayer) -> (String, CAAnimation)? {
    for key in layer.animationKeys() ?? [] {
      if let a = layer.animation(forKey: key) as? CAPropertyAnimation,
        let path = a.keyPath, path.hasPrefix("position") || path.hasPrefix("transform") || path.hasPrefix("bounds")
      {
        return (path, a)
      }
    }
    for sub in layer.sublayers ?? [] {
      if let found = slide(in: sub) { return found }
    }
    return nil
  }

  /// A spring's progress over its duration as a CSS `linear()` easing.
  private static func linear(_ s: CASpringAnimation) -> String {
    let steps = 40, dt = 1.0 / 2400
    let m = Double(s.mass), k = Double(s.stiffness), c = Double(s.damping)
    var x = 0.0, v = Double(s.initialVelocity), t = 0.0
    var points = ["0"]
    for i in 1...steps {
      let until = s.duration * Double(i) / Double(steps)
      while t < until {
        v += (-k * (x - 1) - c * v) / m * dt
        x += v * dt
        t += dt
      }
      points.append(i == steps ? "1" : String(format: "%.4f", x))
    }
    return "linear(\(points.joined(separator: ",")))"
  }
}
