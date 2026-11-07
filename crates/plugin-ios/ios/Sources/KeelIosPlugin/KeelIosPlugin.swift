import SwiftRs
import Tauri
import UIKit
import WebKit

/// `enabled` switches the diagnostics log; `line` (with `source`) adds to it.
class DiagArgs: Decodable {
  let enabled: Bool?
  let source: String?
  let line: String?
  let fresh: Bool?
}

/// `on` holds the page's scroll while the task view is open (`PageScroll`).
class PageScrollArgs: Decodable {
  let on: Bool
}

/// `kind` is the vibration to play (`Haptics`): `complete` or `press`.
class HapticArgs: Decodable {
  let kind: String
}

/// What Keel needs from Swift on iOS: the diagnostics log (`Diag`),
/// keyboard pace (`KeyboardPace`), the page's focus and the keyboard
/// (`PageFocus`), the color past the page's end
/// (`UnderPage`), holding the page still under the task
/// view (`PageScroll`), vibrations (`Haptics`) and the simulator-check script runner
/// (`Script`). It draws nothing over the web view.
class KeelIosPlugin: Plugin {
  public override func load(webview: WKWebView) {
    DispatchQueue.main.async {
      Diag.attach(webview)
      KeyboardPace.attach(webview)
      KeyboardBar.hide(in: webview)
      PageFocus.attach(webview)
      UnderPage.attach(webview)
      PageScroll.attach(webview)
      Script.run(webview)
    }
  }

  /// The diagnostics log (see `Diag`): Rust turns it on or off and forwards
  /// the page's and its own lines here.
  @objc public func diag(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(DiagArgs.self)
    if args.fresh == true { Diag.startFresh() }
    if let on = args.enabled { Diag.setEnabled(on) }
    if let line = args.line { Diag.log(line, source: args.source ?? "js") }
    invoke.resolve()
  }

  /// Holds the page's scroll while the task view is open (see `PageScroll`).
  @objc public func pageScroll(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(PageScrollArgs.self)
    DispatchQueue.main.async {
      PageScroll.hold(args.on)
      invoke.resolve()
    }
  }

  /// Plays a vibration (see `Haptics`).
  @objc public func haptic(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(HapticArgs.self)
    DispatchQueue.main.async {
      Haptics.play(args.kind)
      invoke.resolve()
    }
  }
}

@_cdecl("init_plugin_keel_ios")
public func initPlugin() -> Plugin {
  return KeelIosPlugin()
}
