import UIKit
import WebKit

/// What shows past the page's end: the list's own background, not black. When
/// iOS scrolls the whole web view up to reveal a field under the keyboard, that
/// space shows under the keyboard's rounded top corners. The page posts its
/// `--page` color (`keelUnderPage`, web/src/shell/underPage.ts) at start and on
/// every theme change, and it goes on the web view's under-page and scroll view
/// backgrounds.
enum UnderPage {
  private static weak var webview: WKWebView?
  private static var installed = false
  private static let handler = Handler()

  static func attach(_ webview: WKWebView) {
    self.webview = webview
    guard !installed else { return }
    installed = true
    webview.configuration.userContentController.add(handler, name: "keelUnderPage")
  }

  /// `#rrggbb`, as the page's tokens are written.
  private static func color(_ hex: String) -> UIColor? {
    let digits = hex.trimmingCharacters(in: .whitespaces).dropFirst(hex.hasPrefix("#") ? 1 : 0)
    guard digits.count == 6, let v = UInt32(digits, radix: 16) else { return nil }
    return UIColor(
      red: CGFloat((v >> 16) & 0xff) / 255, green: CGFloat((v >> 8) & 0xff) / 255,
      blue: CGFloat(v & 0xff) / 255, alpha: 1)
  }

  private final class Handler: NSObject, WKScriptMessageHandler {
    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
      guard let hex = message.body as? String, let c = UnderPage.color(hex), let webview = UnderPage.webview else { return }
      webview.underPageBackgroundColor = c
      webview.scrollView.backgroundColor = c
      Diag.log("under page \(hex)")
    }
  }
}
