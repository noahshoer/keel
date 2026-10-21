import SwiftUI
import UIKit
import WebKit

/// Sits over the web view. SwiftUI content has no UIViews of its own, so a
/// hit test can't tell a glass button from empty space: while the card is
/// closed, touches go to the web view unless they land on a tracked control.
/// While it's open, the overlay takes every touch (a tap outside closes it).
final class PassthroughView: UIView {
  weak var host: UIView?
  var model: CardModel?

  override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
    guard let model, let host else { return nil }
    if model.isOpen { return super.hitTest(point, with: event) ?? host }
    let p = convert(point, to: nil)
    guard model.hitRects.values.contains(where: { $0.contains(p) }) else { return nil }
    return super.hitTest(point, with: event) ?? host
  }
}

extension View {
  /// Records this view's window frame under `key` so touches reach it.
  func touchable(_ key: String, in model: CardModel) -> some View {
    onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { model.hitRects[key] = $0 }
      .onDisappear { model.hitRects[key] = nil }
  }
}

struct OverlayView: View {
  @ObservedObject var model: CardModel

  var body: some View {
    ZStack {
      Color.clear
      if model.isOpen {
        CardView(model: model)
      } else if model.buttonsVisible {
        FloatingButtons(model: model)
      }
    }
    .confirmationDialog(
      model.actionMenu?.title ?? "",
      isPresented: Binding(get: { model.actionMenu != nil }, set: { if !$0 { model.actionMenu = nil } }),
      titleVisibility: .visible,
      presenting: model.actionMenu
    ) { menu in
      ForEach(menu.items, id: \.id) { item in
        Button(item.label, role: item.destructive ? .destructive : nil) {
          model.chooseAction(menu.taskId, item.id)
        }
      }
    }
  }
}

extension UIView {
  func findViewController() -> UIViewController? {
    var r: UIResponder? = self
    while let n = r?.next {
      if let vc = n as? UIViewController { return vc }
      r = n
    }
    return nil
  }
}

/// Adds the overlay over `webview`, filling it. Returns the hosting
/// controller, which the caller keeps alive.
func attachOverlay(model: CardModel, to webview: WKWebView) -> UIViewController {
  let hosting = UIHostingController(rootView: OverlayView(model: model))
  hosting.view.backgroundColor = .clear
  let overlay = PassthroughView(frame: webview.bounds)
  overlay.autoresizingMask = [.flexibleWidth, .flexibleHeight]
  overlay.backgroundColor = .clear
  hosting.view.frame = overlay.bounds
  hosting.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
  overlay.addSubview(hosting.view)
  overlay.host = hosting.view
  overlay.model = model
  webview.addSubview(overlay)
  if let parent = webview.window?.rootViewController ?? webview.findViewController() {
    parent.addChild(hosting)
    hosting.didMove(toParent: parent)
  }
  return hosting
}
