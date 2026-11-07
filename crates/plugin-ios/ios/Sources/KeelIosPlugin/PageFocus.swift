import ObjectiveC
import UIKit
import WebKit

/// The page's focus and the keyboard
/// (docs/superpowers/plans/2026-11-06-add-card-keyboard.md).
///
/// A field the page focuses outside a tap raises the keyboard: a task dropped
/// on Add section focuses the section's name as it lands. WebKit raises it only
/// for a focus while the person is interacting, so its content view is told on
/// every focus that they are, as Capacitor's Keyboard plugin does for
/// `keyboardDisplayRequiresUserAction`. (The page's old way, a hidden field
/// focused on the first tap, made the first keyboard after launch snap in.)
///
/// And the content view takes keyboard focus at launch, with no field focused,
/// so no keyboard shows. Otherwise it took it in the first tap, alongside the
/// field's focus, WebKit asked for the keyboard twice, and iOS snapped it in.
///
/// WebKit's method isn't public: if it's missing, nothing changes and the log
/// says so.
enum PageFocus {
  static let selector = NSSelectorFromString(
    "_elementDidFocus:userIsInteracting:blurPreviousNode:activityStateChanges:userObject:")

  static func attach(_ webview: WKWebView) {
    guard let content = webview.scrollView.subviews.first(where: { NSStringFromClass(type(of: $0)).contains("WKContent") })
    else {
      Diag.log("page focus: no content view")
      return
    }
    hook(content)
    takeFocus(webview, content)
  }

  /// On Keel's subclass of the content view (`KeyboardBar`), so WebKit's own
  /// class is left alone.
  private static func hook(_ content: UIView) {
    let cls: AnyClass = type(of: content)
    guard NSStringFromClass(cls).hasPrefix("Keel_"), let base = class_getSuperclass(cls),
      let method = class_getInstanceMethod(base, selector)
    else {
      Diag.log("page focus: no \(NSStringFromSelector(selector)) on \(NSStringFromClass(cls)), unchanged")
      return
    }
    // The focused element's information (a reference), whether the person is
    // interacting, whether to blur the previous node, the activity state
    // changes (a small option set: a register, passed on whole) and a user
    // object.
    typealias Focus = @convention(c) (AnyObject, Selector, UnsafeRawPointer, Bool, Bool, UInt64, AnyObject?) -> Void
    let own = unsafeBitCast(method_getImplementation(method), to: Focus.self)
    let interacting: @convention(block) (AnyObject, UnsafeRawPointer, Bool, Bool, UInt64, AnyObject?) -> Void = {
      view, info, _, blur, changes, user in
      own(view, selector, info, true, blur, changes, user)
    }
    class_addMethod(cls, selector, imp_implementationWithBlock(interacting), method_getTypeEncoding(method))
  }

  /// Once the web view is in a window.
  private static func takeFocus(_ webview: WKWebView, _ content: UIView, tries: Int = 20) {
    guard webview.window != nil else {
      if tries > 0 {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { takeFocus(webview, content, tries: tries - 1) }
      }
      return
    }
    if !content.isFirstResponder { _ = content.becomeFirstResponder() }
  }
}
