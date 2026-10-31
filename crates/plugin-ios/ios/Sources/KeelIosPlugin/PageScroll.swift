import UIKit
import WebKit

/// While the task view is open, the page under it stays put. A field
/// focused in the task sheet sits where the keyboard will be, so WebKit
/// scrolls the whole page (the web view's scroll view) to reveal it, and the
/// list under the sheet moves. The sheet lifts itself above the keyboard
/// (`window.keelKeyboard`), so while the page holds the scroll, any move of
/// it is put back inside the same change, before it's drawn. WebKit keeps
/// its keyboard handling otherwise (the visual viewport still resizes), and
/// with the task view closed nothing is held.
enum PageScroll {
  private static weak var webview: WKWebView?
  private static var observation: NSKeyValueObservation?
  /// Where the page is held, or nil when it scrolls freely.
  private static var heldAt: CGPoint?

  static func attach(_ webview: WKWebView) {
    self.webview = webview
  }

  static func hold(_ on: Bool) {
    guard let sv = webview?.scrollView else { return }
    if on {
      if heldAt == nil { heldAt = sv.contentOffset }
      observation = observation ?? sv.observe(\.contentOffset, options: [.new]) { sv, _ in
        // Setting it from here notifies again, with the held value: no loop.
        guard let at = heldAt, sv.contentOffset != at else { return }
        sv.setContentOffset(at, animated: false)
      }
    } else {
      heldAt = nil
      observation?.invalidate()
      observation = nil
    }
    Diag.log("page scroll \(on ? "held" : "free")")
  }
}
