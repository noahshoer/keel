import ObjectiveC
import UIKit
import WebKit

/// No bar above the keyboard. WebKit's has previous, next and a check mark that
/// only closes the keyboard, and it takes room from the task card and its
/// pickers. The bar comes from the web view's content view, so that one view
/// gets a subclass whose `inputAccessoryView` is nil.
enum KeyboardBar {
  static func hide(in webview: WKWebView) {
    guard let content = webview.scrollView.subviews.first(where: { NSStringFromClass(type(of: $0)).hasPrefix("WKContent") })
    else {
      Diag.log("KB bar: no content view")
      return
    }
    let base: AnyClass = type(of: content)
    let name = "Keel_\(NSStringFromClass(base))"
    if let existing = NSClassFromString(name) {
      object_setClass(content, existing)
      return
    }
    guard let sub = objc_allocateClassPair(base, name, 0) else { return }
    let selector = #selector(getter: UIResponder.inputAccessoryView)
    let none: @convention(block) (AnyObject) -> UIView? = { _ in nil }
    let types = class_getInstanceMethod(UIResponder.self, selector).flatMap(method_getTypeEncoding)
    class_addMethod(sub, selector, imp_implementationWithBlock(none), types)
    objc_registerClassPair(sub)
    object_setClass(content, sub)
    Diag.log("KB bar: hidden")
  }
}
