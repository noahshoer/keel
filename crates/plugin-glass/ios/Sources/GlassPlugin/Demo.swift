import UIKit
import WebKit

/// Drives the card without touches, for simulator checks:
/// `SIMCTL_CHILD_KEEL_GLASS_DEMO="<selector>|<text>|<submit 0/1>" xcrun simctl launch …`
/// clicks `selector` in the page, types `text` into the title, and submits.
/// Does nothing unless the variable is set.
enum Demo {
  static func run(webview: WKWebView, model: CardModel) {
    guard let spec = ProcessInfo.processInfo.environment["KEEL_GLASS_DEMO"] else { return }
    let parts = spec.components(separatedBy: "|")
    let selector = parts.first ?? ".add-task-row"
    let text = parts.count > 1 ? parts[1] : ""
    let submit = parts.count > 2 && parts[2] == "1"
    DispatchQueue.main.asyncAfter(deadline: .now() + 4) {
      webview.evaluateJavaScript("document.querySelector('\(selector)')?.click()")
      DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
        guard !text.isEmpty, let field = TitleField.AutoFocusField.current else { return }
        field.text = text
        field.sendActions(for: .editingChanged)
        if submit {
          DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { model.send(.submit) }
        }
      }
    }
  }
}
