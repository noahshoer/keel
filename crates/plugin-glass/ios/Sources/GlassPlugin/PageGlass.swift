import SwiftUI
import UIKit
import WebKit

/// Glass buttons drawn inside WebKit's own layers (the task sheet's X and
/// •••; the floating menu and search): the web buttons get compositing views
/// of their own (`will-change`, in the page's CSS), and each glass button is
/// added as a subview of its web button's view. Core Animation then moves it
/// with the page in the same frame (the sheet's slide-up, a drag, a settle,
/// the keyboard's pan), and whatever the page draws above it (the task
/// view's scrim and sheet) covers it, with no timing or messages of ours. The glass plays its own press effect, and the
/// tap itself reaches the page: the web button under it takes the click. (A
/// native tap handler on the glass never fires; WebKit takes the touch.)
///
/// WebKit can drop the views (a sub-task's view replaces the header) or not
/// have made them yet (the sheet's first frames), so while shown, each frame
/// checks the glass is still in place and finds the views again if not. If
/// they never turn up, `fallback` runs (the page shows its own icons).
final class PageGlass: NSObject {
  private weak var webview: WKWebView?
  private let name: String
  private let font: Font
  private let fallback: String
  private var wanted: [DrawerButton] = []
  private var placed: [String: UIHostingController<PageGlassButton>] = [:]
  private var link: CADisplayLink?
  private var missing = 0

  /// About a second at 60 fps without finding the web buttons' views.
  private static let giveUpAfter = 60

  /// `fallback`: JavaScript that brings back the page's own icons.
  init(webview: WKWebView?, name: String, font: Font, fallback: String) {
    self.webview = webview
    self.name = name
    self.font = font
    self.fallback = fallback
    super.init()
  }

  func show(_ buttons: [DrawerButton]) {
    // The same buttons with a new symbol (search turning into close) change
    // in place; anything else starts over.
    if buttons.map(\.id) != wanted.map(\.id) {
      removeAll()
    } else {
      for b in buttons { placed[b.id]?.rootView = PageGlassButton(symbol: b.symbol, size: max(b.w, b.h), font: font) }
    }
    wanted = buttons
    missing = 0
    if link == nil {
      let l = CADisplayLink(target: self, selector: #selector(tick))
      l.add(to: .main, forMode: .common)
      link = l
    }
    tick()
  }

  func hide() {
    link?.invalidate()
    link = nil
    wanted = []
    removeAll()
  }

  private func removeAll() {
    placed.values.forEach { $0.view.removeFromSuperview() }
    placed = [:]
  }

  @objc private func tick() {
    guard let webview else { return }
    let lost = wanted.filter { b in
      guard let v = placed[b.id]?.view, let host = v.superview, v.window != nil else { return true }
      return !Self.fits(host, b)
    }
    if lost.isEmpty {
      missing = 0
      return
    }
    let hosts = Self.compositingViews(in: webview)
    var found = true
    for b in lost {
      guard let host = Self.best(for: b, among: hosts, in: webview) else {
        found = false
        continue
      }
      let glass = placed[b.id] ?? makeGlass(b)
      glass.view.frame = host.bounds
      host.addSubview(glass.view)
      placed[b.id] = glass
      Diag.log("GLASS \(name) \(b.id) in \(host.convert(host.bounds, to: nil))")
    }
    if found {
      missing = 0
      return
    }
    missing += 1
    if missing == Self.giveUpAfter {
      Diag.log("GLASS \(name) gave up")
      hide()
      webview.evaluateJavaScript(fallback)
    }
  }

  /// A web button's compositing view: its size, and no glass of ours yet.
  private static func fits(_ v: UIView, _ b: DrawerButton) -> Bool {
    abs(v.bounds.width - b.w) < 1 && abs(v.bounds.height - b.h) < 1
  }

  private static func compositingViews(in root: UIView) -> [UIView] {
    var out: [UIView] = []
    func walk(_ v: UIView) {
      if String(describing: type(of: v)) == "WKCompositingView" { out.append(v) }
      v.subviews.forEach(walk)
    }
    walk(root)
    return out
  }

  /// The view of `b`'s size nearest its measured x (one button is left, the
  /// other right).
  private static func best(for b: DrawerButton, among views: [UIView], in webview: WKWebView) -> UIView? {
    views
      .filter { fits($0, b) }
      .map { ($0, abs($0.convert($0.bounds, to: webview).minX - b.x)) }
      .filter { $0.1 < 8 }
      .min { $0.1 < $1.1 }?
      .0
  }

  private func makeGlass(_ b: DrawerButton) -> UIHostingController<PageGlassButton> {
    let hosting = UIHostingController(rootView: PageGlassButton(symbol: b.symbol, size: max(b.w, b.h), font: font))
    let v = hosting.view!
    v.backgroundColor = .clear
    v.autoresizingMask = [.flexibleWidth, .flexibleHeight]
    return hosting
  }
}

struct PageGlassButton: View {
  let symbol: String
  let size: CGFloat
  let font: Font

  var body: some View {
    Image(systemName: symbol)
      .font(font)
      .contentTransition(.symbolEffect(.replace))
      .foregroundStyle(Color.primary)
      .frame(width: size, height: size)
      .contentShape(.circle)
      .glassEffect(.regular.interactive(), in: .circle)
      .accessibilityHidden(true)
      .ignoresSafeArea()
  }
}
