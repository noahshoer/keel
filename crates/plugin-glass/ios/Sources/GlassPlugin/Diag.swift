import UIKit
import WebKit

/// Keel's diagnostics log: timestamped lines appended to
/// Library/Caches/diag.log in the app's container (`scripts/diag.sh` pulls
/// it). Off by default; the Settings switch (via `GlassPlugin.diag`) or the
/// `KEEL_DIAG=1` environment variable turns it on. When off, nothing is
/// written and nothing is observed. The file rolls over to diag.1.log at
/// ~2 MB, keeping one previous file.
enum Diag {
  static let cap: UInt64 = 2 * 1024 * 1024
  static let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
  static let url = dir.appendingPathComponent("diag.log")
  static let previous = dir.appendingPathComponent("diag.1.log")

  private static let queue = DispatchQueue(label: "keel.diag")
  private static var observers: [NSObjectProtocol] = []
  private static var kvo: [NSKeyValueObservation] = []
  private static var _enabled = ProcessInfo.processInfo.environment["KEEL_DIAG"] == "1"
  private static weak var watched: WKWebView?
  private static weak var watchedHost: UIView?

  static var enabled: Bool { queue.sync { _enabled } }

  /// Switches logging on or off; turning it on starts the native watchers.
  static func setEnabled(_ on: Bool) {
    queue.sync { _enabled = on }
    DispatchQueue.main.async { on ? startWatching() : stopWatching() }
    if on { log("---- diagnostics on") }
  }

  /// One line, written only while logging is on. `source` is "swift" for
  /// native lines; the page and Rust send theirs through `GlassPlugin.diag`.
  static func log(_ text: String, source: String = "swift") {
    guard enabled else { return }
    let line = String(format: "%.3f ", Date().timeIntervalSince1970) + "[\(source)] "
      + text.replacingOccurrences(of: "\r", with: "").replacingOccurrences(of: "\n", with: "\\n") + "\n"
    queue.async {
      guard _enabled, let data = line.data(using: .utf8) else { return }
      let fm = FileManager.default
      if let size = (try? fm.attributesOfItem(atPath: url.path))?[.size] as? UInt64, size >= cap {
        try? fm.removeItem(at: previous)
        try? fm.moveItem(at: url, to: previous)
      }
      if let h = try? FileHandle(forWritingTo: url) {
        h.seekToEndOfFile()
        h.write(data)
        try? h.close()
      } else {
        try? data.write(to: url)
      }
    }
  }

  /// A rectangle as "[x,y wxh]" for log lines.
  static func r(_ r: CGRect) -> String { String(format: "[%.1f,%.1f %.1fx%.1f]", r.minX, r.minY, r.width, r.height) }

  /// Remembers the web view so watching can start whenever logging is on.
  static func attach(_ webview: WKWebView, hosting: UIView?) {
    watched = webview
    watchedHost = hosting
    if enabled { DispatchQueue.main.async { startWatching() } }
  }

  /// Keyboard notifications, the web view's scroll view (which iOS moves for
  /// the keyboard) and the overlay's frame.
  private static func startWatching() {
    stopWatching()
    guard let webview = watched else { return }
    log("watching; webview frame \(r(webview.frame)) window \(r(webview.window?.frame ?? .zero))")
    let nc = NotificationCenter.default
    for name in [UIResponder.keyboardWillShowNotification, UIResponder.keyboardDidShowNotification,
                 UIResponder.keyboardWillHideNotification, UIResponder.keyboardDidHideNotification,
                 UIResponder.keyboardWillChangeFrameNotification] {
      observers.append(nc.addObserver(forName: name, object: nil, queue: .main) { n in
        let end = (n.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? NSValue)?.cgRectValue ?? .zero
        let dur = n.userInfo?[UIResponder.keyboardAnimationDurationUserInfoKey] as? Double ?? 0
        log("KB \(name.rawValue.replacingOccurrences(of: "UIKeyboard", with: "")) end \(r(end)) dur \(dur)")
      })
    }
    let sv = webview.scrollView
    kvo.append(sv.observe(\.contentOffset, options: [.new]) { sv, _ in
      log("SV offset \(String(format: "%.1f,%.1f", sv.contentOffset.x, sv.contentOffset.y)) inset top \(sv.adjustedContentInset.top) bottom \(sv.adjustedContentInset.bottom) size \(String(format: "%.0fx%.0f", sv.contentSize.width, sv.contentSize.height))")
    })
    kvo.append(webview.observe(\.frame, options: [.new]) { w, _ in log("WV frame \(r(w.frame))") })
    if let hosting = watchedHost {
      kvo.append(hosting.observe(\.frame, options: [.new]) { h, _ in log("HOST frame \(r(h.frame)) safe \(h.safeAreaInsets)") })
    }
  }

  private static func stopWatching() {
    observers.forEach(NotificationCenter.default.removeObserver)
    observers = []
    kvo.forEach { $0.invalidate() }
    kvo = []
  }
}
